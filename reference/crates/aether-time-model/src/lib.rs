//! Time model (RFC-0012).
//!
//! This is a bounded executable host model of the RFC-0012 time-model
//! semantics: monotonic and scheduling time sources, a timer facility
//! (one-shot / periodic / cancellation) whose fires are delivered as
//! interrupts (RFC-0010), per-thread deadline accounting for EDF
//! scheduling (RFC-0011), monotonicity enforcement, and suspend/resume of
//! scheduling time. It is deliberately **not** a hardware clock, a
//! real-world/wall-clock implementation, or a distributed time-sync
//! protocol (multi-domain time is deferred to v2 per RFC-0012.4).
//!
//! Scope and declared architecture:
//!
//! - **Class:** programmable time sources and timers, caller-supplied
//!   scheduling time (the model never reads a clock).
//! - **Time sources (RFC-0012.1/.2):** monotonic raw (durations) and
//!   scheduling time (quantum/deadline accounting); additional sources
//!   may be registered with declared provenance.
//! - **Resolution (RFC-0012.3):** every source declares a tick resolution
//!   of at most 1 nanosecond; registration rejects coarser resolutions.
//! - **Timer facility (RFC-0012.5):** one-shot (fires once at or after a
//!   deadline), periodic (fires at a declared interval), and cancellation
//!   of a pending timer.
//! - **Timer delivery (RFC-0012.6):** timer fires are delivered as
//!   interrupts through an `aether_interrupt_model::InterruptRouter`,
//!   whose routing table must explicitly bind each timer's vector.
//! - **Deadline accounting (RFC-0012.7/.8):** per-thread deadline register
//!   compared against the scheduling clock; a miss emits a
//!   `DeadlineMissed` event identifying the thread and degree of miss.
//! - **Monotonicity (RFC-0012.9):** monotonic raw never goes backward; a
//!   backward jump is corrected forward to the last observed value and a
//!   `TimeSourceFault` event is recorded.
//! - **Suspend/resume (RFC-0012.10):** scheduling time may be suspended
//!   and is resumed from the exact point of suspension, never reset.
//!
//! Explicitly out of scope:
//!
//! - Wall-clock / real-world time and cross-domain time synchronization
//!   (RFC-0012.2/.4, v2).
//! - Hardware clock discipline, drift compensation, or TAI/UTC leap
//!   handling.
//! - Integration with any kernel, scheduler unit, or dispatch beyond the
//!   interrupt delivery hook.

#![allow(clippy::new_without_default)]
#![allow(clippy::unused_self)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]

use aether_interrupt_model::{InterruptRouter, IrqVector};
use aether_types::{Capability, DomainId};
use std::collections::{BTreeMap, BTreeSet};

/// Identifier for a registered time source.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct TimeSourceId(pub u64);

/// Identifier for a timer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct TimerId(pub u64);

/// Identifier for a thread tracked by the deadline register.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct ThreadId(pub u64);

/// Kinds of time source (RFC-0012.1/.2).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub enum TimeSourceKind {
    /// Hardware counter that increments monotonically without jumps.
    MonotonicRaw,
    /// Time used for quantum and deadline accounting; pausable when idle.
    SchedulingTime,
    /// Wall-clock time (declared provenance only).
    WallClock,
    /// High-resolution performance counter (declared provenance only).
    Performance,
}

/// Declaration of a time source (RFC-0012.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeSourceDescriptor {
    /// Kind of time source (RFC-0012.1/.2).
    pub kind: TimeSourceKind,
    /// Declared tick resolution in nanoseconds; MUST be <= 1 (RFC-0012.3).
    pub resolution_ns: u64,
    /// Drift in parts per million; 0 = perfect.
    pub drift_ppm: u64,
}

/// Timer firing mode (RFC-0012.5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerMode {
    /// Fires exactly once at or after the deadline.
    OneShot,
    /// Fires at a regular interval with a declared jitter bound.
    Periodic,
}

/// Declaration of a timer (RFC-0012.5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimerDescriptor {
    /// Absolute deadline in scheduling-time nanoseconds at which to fire.
    pub deadline_ns: u64,
    /// Firing mode: one-shot or periodic (RFC-0012.5).
    pub mode: TimerMode,
    /// Relevant for `Periodic`: interval between firings.
    pub period_ns: u64,
    /// Declared upper bound on delivery jitter, in nanoseconds.
    pub jitter_bound_ns: u64,
    /// Interrupt vector on which the timer fire is delivered (RFC-0012.6).
    pub vector: IrqVector,
    /// Domain to which the timer fire is routed (RFC-0012.6).
    pub domain: DomainId,
    /// Handler capability invoked on delivery.
    pub handler_cap: Capability,
}

/// One observable time event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeEvent {
    /// A timer fired at `timestamp_ns`.
    TimerFired {
        /// The timer that fired.
        timer_id: TimerId,
        /// Scheduling time at firing, in nanoseconds.
        timestamp_ns: u64,
    },
    /// A thread's deadline was missed by `degree_ns` (RFC-0012.8).
    DeadlineMissed {
        /// The thread that missed its deadline.
        thread_id: ThreadId,
        /// The missed deadline, in nanoseconds.
        deadline_ns: u64,
        /// Scheduling time when the miss was detected, in nanoseconds.
        now_ns: u64,
        /// Degree of the miss (`now_ns - deadline_ns`), in nanoseconds.
        degree_ns: u64,
    },
    /// A time source exhibited a non-monotonic anomaly (RFC-0012.9).
    TimeSourceFault {
        /// The faulty time source.
        source: TimeSourceId,
        /// Scheduling time when the anomaly was detected, in nanoseconds.
        timestamp_ns: u64,
    },
}

/// Failures of the time model; the model never panics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeError {
    /// No registered source with this identifier.
    UnknownSource,
    /// No timer with this identifier.
    UnknownTimer,
    /// A timer with this identifier already exists.
    TimerAlreadyExists,
    /// Scheduling-time arithmetic overflowed.
    ClockOverflow,
    /// A source resolution exceeds 1 nanosecond (RFC-0012.3).
    InvalidResolution,
}

/// One registered time source and its current value.
struct TimeSource {
    /// Declared tick resolution in nanoseconds (MUST be <= 1, RFC-0012.3).
    resolution_ns: u64,
    /// Current value of the source in nanoseconds.
    value: u64,
}

/// One programmed timer.
struct Timer {
    deadline_ns: u64,
    mode: TimerMode,
    period_ns: u64,
    vector: IrqVector,
    domain: DomainId,
    cancelled: bool,
}

/// The time model (RFC-0012).
pub struct TimeModel {
    sources: BTreeMap<TimeSourceId, TimeSource>,
    timers: BTreeMap<TimerId, Timer>,
    deadlines: BTreeMap<ThreadId, u64>,
    /// Threads whose current deadline has already been reported as missed,
    /// so a miss is counted once per deadline registration.
    missed_deadlines: BTreeSet<ThreadId>,
    next_source_id: u64,
    next_timer_id: u64,
    events: Vec<TimeEvent>,
    monotonic_raw_id: TimeSourceId,
    scheduling_time_id: TimeSourceId,
    scheduling_suspended: bool,
}

impl TimeModel {
    /// Creates a time model with the two RFC-0012.1 sources pre-registered:
    /// monotonic raw (id 0) and scheduling time (id 1), each at 1 ns
    /// resolution and zero drift.
    #[must_use]
    pub fn new() -> Self {
        let mut sources = BTreeMap::new();
        sources.insert(
            TimeSourceId(0),
            TimeSource {
                resolution_ns: 1,
                value: 0,
            },
        );
        sources.insert(
            TimeSourceId(1),
            TimeSource {
                resolution_ns: 1,
                value: 0,
            },
        );
        Self {
            sources,
            timers: BTreeMap::new(),
            deadlines: BTreeMap::new(),
            missed_deadlines: BTreeSet::new(),
            next_source_id: 2,
            next_timer_id: 0,
            events: Vec::new(),
            monotonic_raw_id: TimeSourceId(0),
            scheduling_time_id: TimeSourceId(1),
            scheduling_suspended: false,
        }
    }

    /// Returns all time events recorded so far, oldest first.
    #[must_use]
    pub fn events(&self) -> &[TimeEvent] {
        &self.events
    }

    /// Returns the number of time events recorded.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns the current value of a time source, or `UnknownSource`.
    pub fn read(&self, source: TimeSourceId) -> Result<u64, TimeError> {
        self.sources
            .get(&source)
            .map(|s| s.value)
            .ok_or(TimeError::UnknownSource)
    }

    /// Returns the declared resolution of a time source, or `UnknownSource`.
    pub fn resolution(&self, source: TimeSourceId) -> Result<u64, TimeError> {
        self.sources
            .get(&source)
            .map(|s| s.resolution_ns)
            .ok_or(TimeError::UnknownSource)
    }

    /// Registers a new time source with the given descriptor (RFC-0012.1/.3).
    ///
    /// Rejects resolutions coarser than 1 nanosecond (RFC-0012.3).
    ///
    /// # Errors
    /// Returns `InvalidResolution` if `resolution_ns > 1`.
    pub fn register_source(
        &mut self,
        descriptor: &TimeSourceDescriptor,
    ) -> Result<TimeSourceId, TimeError> {
        if descriptor.resolution_ns > 1 {
            return Err(TimeError::InvalidResolution);
        }
        let id = TimeSourceId(self.next_source_id);
        self.next_source_id += 1;
        self.sources.insert(
            id,
            TimeSource {
                resolution_ns: descriptor.resolution_ns,
                value: 0,
            },
        );
        Ok(id)
    }

    /// Advances the monotonic raw source to `now_ns`, enforcing monotonicity
    /// (RFC-0012.9). A backward jump is corrected forward to the last
    /// observed value and a `TimeSourceFault` event is recorded.
    ///
    /// # Errors
    /// Returns `UnknownSource` if the monotonic raw source is not registered.
    pub fn advance_monotonic_raw(&mut self, now_ns: u64) -> Result<(), TimeError> {
        let Some(src) = self.sources.get_mut(&self.monotonic_raw_id) else {
            return Err(TimeError::UnknownSource);
        };
        if now_ns < src.value {
            // RFC-0012.9: correct forward to the last observed value; never
            // allow a backward jump. Record the anomaly.
            self.events.push(TimeEvent::TimeSourceFault {
                source: self.monotonic_raw_id,
                timestamp_ns: now_ns,
            });
            return Ok(());
        }
        src.value = now_ns;
        Ok(())
    }

    /// Suspends the scheduling time source (RFC-0012.10). While suspended,
    /// scheduling time does not advance.
    pub fn suspend_scheduling_time(&mut self) -> Result<(), TimeError> {
        self.scheduling_suspended = true;
        Ok(())
    }

    /// Resumes the scheduling time source from the exact point of
    /// suspension; it is never reset (RFC-0012.10).
    pub fn resume_scheduling_time(&mut self) -> Result<(), TimeError> {
        self.scheduling_suspended = false;
        Ok(())
    }

    /// Returns `true` if scheduling time is suspended.
    #[must_use]
    pub fn is_scheduling_suspended(&self) -> bool {
        self.scheduling_suspended
    }

    /// Advances the scheduling time source to `now_ns` unless suspended
    /// (RFC-0012.10).
    ///
    /// # Errors
    /// Returns `UnknownSource` if the scheduling time source is not registered.
    pub fn advance_scheduling_time(&mut self, now_ns: u64) -> Result<(), TimeError> {
        if self.scheduling_suspended {
            return Ok(());
        }
        let Some(src) = self.sources.get_mut(&self.scheduling_time_id) else {
            return Err(TimeError::UnknownSource);
        };
        if now_ns >= src.value {
            src.value = now_ns;
        }
        Ok(())
    }

    /// Programs a timer (RFC-0012.5). The timer's interrupt routing must be
    /// configured in the supplied router's table before delivery (RFC-0012.6).
    ///
    /// # Errors
    /// Returns `TimerAlreadyExists` if a timer with the same identifier
    /// already exists.
    pub fn create_timer(&mut self, descriptor: &TimerDescriptor) -> Result<TimerId, TimeError> {
        let id = TimerId(self.next_timer_id);
        self.next_timer_id += 1;
        let timer = Timer {
            deadline_ns: descriptor.deadline_ns,
            mode: descriptor.mode,
            period_ns: descriptor.period_ns,
            vector: descriptor.vector,
            domain: descriptor.domain,
            cancelled: false,
        };
        self.timers.insert(id, timer);
        Ok(id)
    }

    /// Cancels a pending timer before it fires (RFC-0012.5).
    ///
    /// # Errors
    /// Returns `UnknownTimer` if no timer with this identifier exists.
    pub fn cancel_timer(&mut self, id: TimerId) -> Result<(), TimeError> {
        let Some(timer) = self.timers.get_mut(&id) else {
            return Err(TimeError::UnknownTimer);
        };
        timer.cancelled = true;
        Ok(())
    }

    /// Registers a per-thread deadline in the deadline register
    /// (RFC-0012.7). The deadline is compared against the scheduling clock on
    /// each `tick`. A previously reported miss for this thread is cleared so
    /// the new deadline is re-armed.
    pub fn set_deadline(&mut self, thread: ThreadId, deadline_ns: u64) {
        self.deadlines.insert(thread, deadline_ns);
        self.missed_deadlines.remove(&thread);
    }

    /// Returns a thread's registered deadline, if any (RFC-0012.7).
    #[must_use]
    pub fn deadline(&self, thread: ThreadId) -> Option<u64> {
        self.deadlines.get(&thread).copied()
    }

    /// Advances scheduling time, delivers due timers as interrupts through
    /// `router` (RFC-0012.6), and checks for deadline misses (RFC-0012.7/.8).
    ///
    /// Returns the events generated by this tick (timer fires and deadline
    /// misses); all events are also recorded internally.
    ///
    /// # Errors
    /// Returns `ClockOverflow` if deadline or period arithmetic overflows,
    /// or `UnknownSource` if the scheduling time source is missing.
    pub fn tick(
        &mut self,
        now_ns: u64,
        router: &mut InterruptRouter,
    ) -> Result<Vec<TimeEvent>, TimeError> {
        self.advance_scheduling_time(now_ns)?;

        let mut generated: Vec<TimeEvent> = Vec::new();

        // RFC-0012.5/.6: fire due timers as interrupts.
        let due: Vec<TimerId> = self
            .timers
            .iter()
            .filter(|(_, t)| !t.cancelled && t.deadline_ns <= now_ns)
            .map(|(id, _)| *id)
            .collect();
        for id in due {
            let (vector, domain, mode) = {
                let timer = self.timers.get(&id).ok_or(TimeError::UnknownTimer)?;
                (timer.vector, timer.domain, timer.mode)
            };
            let _outcome = router.deliver(vector, domain, now_ns);
            let event = TimeEvent::TimerFired {
                timer_id: id,
                timestamp_ns: now_ns,
            };
            self.events.push(event);
            generated.push(event);
            match mode {
                TimerMode::OneShot => {
                    self.timers.remove(&id);
                }
                TimerMode::Periodic => {
                    let new_deadline = self
                        .timers
                        .get(&id)
                        .and_then(|t| t.deadline_ns.checked_add(t.period_ns))
                        .ok_or(TimeError::ClockOverflow)?;
                    if let Some(t) = self.timers.get_mut(&id) {
                        t.deadline_ns = new_deadline;
                    }
                }
            }
        }

        // RFC-0012.7/.8: deadline miss detection.
        let mut missed: Vec<(ThreadId, u64, u64)> = Vec::new();
        for (&thread, &deadline) in &self.deadlines {
            if now_ns > deadline && !self.missed_deadlines.contains(&thread) {
                let degree = now_ns
                    .checked_sub(deadline)
                    .ok_or(TimeError::ClockOverflow)?;
                missed.push((thread, deadline, degree));
                self.missed_deadlines.insert(thread);
            }
        }
        for (thread, deadline, degree) in missed {
            let event = TimeEvent::DeadlineMissed {
                thread_id: thread,
                deadline_ns: deadline,
                now_ns,
                degree_ns: degree,
            };
            self.events.push(event);
            generated.push(event);
        }

        Ok(generated)
    }
}

impl Default for TimeModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_types::{Bounds, CapabilityFlags, ObjectId, ObjectType, Rights};

    fn bind_cap() -> Capability {
        Capability {
            object: ObjectId(0xBEEF),
            object_type: ObjectType::Memory,
            rights: Rights::MAP,
            bounds: Bounds::new(0, 4096).unwrap(),
            flags: CapabilityFlags::BOUND,
            expires_at_ns: 0,
        }
    }

    fn thread(value: u64) -> ThreadId {
        ThreadId(value)
    }

    fn timer_descriptor(deadline_ns: u64, vector: IrqVector, domain: DomainId) -> TimerDescriptor {
        TimerDescriptor {
            deadline_ns,
            mode: TimerMode::OneShot,
            period_ns: 0,
            jitter_bound_ns: 1000,
            vector,
            domain,
            handler_cap: bind_cap(),
        }
    }

    #[test]
    fn new_registers_monotonic_and_scheduling_sources_at_one_ns() {
        let model = TimeModel::new();
        assert_eq!(model.read(TimeSourceId(0)).unwrap(), 0);
        assert_eq!(model.read(TimeSourceId(1)).unwrap(), 0);
        assert_eq!(model.resolution(TimeSourceId(0)).unwrap(), 1);
        assert_eq!(model.resolution(TimeSourceId(1)).unwrap(), 1);
        assert!(!model.is_scheduling_suspended());
    }

    #[test]
    fn coarse_resolution_is_rejected() {
        let mut model = TimeModel::new();
        assert_eq!(
            model.register_source(&TimeSourceDescriptor {
                kind: TimeSourceKind::WallClock,
                resolution_ns: 2,
                drift_ppm: 0,
            }),
            Err(TimeError::InvalidResolution)
        );
        assert_eq!(
            model.register_source(&TimeSourceDescriptor {
                kind: TimeSourceKind::WallClock,
                resolution_ns: 1,
                drift_ppm: 0,
            }),
            Ok(TimeSourceId(2))
        );
    }

    #[test]
    fn monotonic_never_goes_backward() {
        let mut model = TimeModel::new();
        model.advance_monotonic_raw(5000).unwrap();
        assert_eq!(model.read(TimeSourceId(0)).unwrap(), 5000);
        // Attempted backward jump is corrected forward (stays at 5000).
        model.advance_monotonic_raw(1000).unwrap();
        assert_eq!(model.read(TimeSourceId(0)).unwrap(), 5000);
        assert!(
            model
                .events()
                .iter()
                .any(|e| matches!(e, TimeEvent::TimeSourceFault { .. }))
        );
    }

    #[test]
    fn scheduling_time_pauses_while_suspended() {
        let mut model = TimeModel::new();
        let mut router = InterruptRouter::new();
        model.advance_scheduling_time(1000).unwrap();
        assert_eq!(model.read(TimeSourceId(1)).unwrap(), 1000);
        model.suspend_scheduling_time().unwrap();
        model.advance_scheduling_time(5000).unwrap();
        assert_eq!(model.read(TimeSourceId(1)).unwrap(), 1000);
        assert!(model.is_scheduling_suspended());
        // Resume continues from the exact point of suspension, not reset.
        model.resume_scheduling_time().unwrap();
        model.advance_scheduling_time(6000).unwrap();
        assert_eq!(model.read(TimeSourceId(1)).unwrap(), 6000);
        assert!(!model.is_scheduling_suspended());
    }

    #[test]
    fn one_shot_timer_fires_once_and_is_consumed() {
        let mut model = TimeModel::new();
        let mut router = InterruptRouter::new();
        router
            .register_domain(DomainId::new(1).unwrap(), None, 0)
            .unwrap();
        let id = model
            .create_timer(&timer_descriptor(
                1000,
                IrqVector(3),
                DomainId::new(1).unwrap(),
            ))
            .unwrap();
        // Before deadline: not fired.
        assert_eq!(model.tick(500, &mut router).unwrap().len(), 0);
        // At/after deadline: fires once.
        let events = model.tick(1000, &mut router).unwrap();
        assert_eq!(
            events,
            vec![TimeEvent::TimerFired {
                timer_id: id,
                timestamp_ns: 1000
            }]
        );
        // A later tick does not refire a one-shot timer.
        assert_eq!(model.tick(2000, &mut router).unwrap().len(), 0);
    }

    #[test]
    fn periodic_timer_reschedules() {
        let mut model = TimeModel::new();
        let mut router = InterruptRouter::new();
        router
            .register_domain(DomainId::new(1).unwrap(), None, 0)
            .unwrap();
        let id = model
            .create_timer(&TimerDescriptor {
                deadline_ns: 1000,
                mode: TimerMode::Periodic,
                period_ns: 500,
                jitter_bound_ns: 100,
                vector: IrqVector(4),
                domain: DomainId::new(1).unwrap(),
                handler_cap: bind_cap(),
            })
            .unwrap();
        let e1 = model.tick(1000, &mut router).unwrap();
        assert_eq!(e1.len(), 1);
        // Next fire at deadline + period = 1500.
        let e2 = model.tick(1500, &mut router).unwrap();
        assert_eq!(e2.len(), 1);
        if let TimeEvent::TimerFired { timestamp_ns, .. } = e2[0] {
            assert_eq!(timestamp_ns, 1500);
        } else {
            panic!("expected TimerFired, got {e2:?}");
        }
    }

    #[test]
    fn cancelled_timer_does_not_fire() {
        let mut model = TimeModel::new();
        let mut router = InterruptRouter::new();
        router
            .register_domain(DomainId::new(1).unwrap(), None, 0)
            .unwrap();
        let id = model
            .create_timer(&timer_descriptor(
                1000,
                IrqVector(5),
                DomainId::new(1).unwrap(),
            ))
            .unwrap();
        model.cancel_timer(id).unwrap();
        assert_eq!(model.tick(2000, &mut router).unwrap().len(), 0);
    }

    #[test]
    fn deadline_miss_is_recorded_with_degree() {
        let mut model = TimeModel::new();
        let mut router = InterruptRouter::new();
        model.set_deadline(thread(1), 1000);
        // Advance past the deadline: a miss is recorded with degree 300.
        let events = model.tick(1300, &mut router).unwrap();
        assert_eq!(
            events,
            vec![TimeEvent::DeadlineMissed {
                thread_id: thread(1),
                deadline_ns: 1000,
                now_ns: 1300,
                degree_ns: 300
            }]
        );
        // A miss is counted once per registered deadline.
        assert_eq!(model.tick(1400, &mut router).unwrap().len(), 0);
    }

    #[test]
    fn unknown_source_and_timer_are_rejected() {
        let mut model = TimeModel::new();
        assert_eq!(model.read(TimeSourceId(99)), Err(TimeError::UnknownSource));
        assert_eq!(
            model.cancel_timer(TimerId(99)),
            Err(TimeError::UnknownTimer)
        );
    }
}
