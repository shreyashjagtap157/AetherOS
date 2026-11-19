//! Deterministic uniprocessor Layer 1 scheduler state model (RFC-0011 §3).
//!
//! This is a bounded executable host model of the RFC-0011 scheduler Layer 1
//! semantics. It is deliberately **not** a production scheduler, a proof of
//! bounded decision latency, or an implementation of RFC-0011 Layer 2/3.
//!
//! Scope and declared algorithm class, as required by RFC-0011.2/.3:
//!
//! - **Class:** EDF (Earliest Deadline First), uniprocessor.
//! - **Selection rule:** among READY threads, select the least
//!   `(deadline_ns, tid)`. The pair is a total order, so identical inputs
//!   always select the same thread (RFC-0011.9).
//! - **Admissibility (RFC-0011.7, uniprocessor bound U = 1):** a thread set is
//!   admitted only if `sum(Ei / Pi) <= 1`, evaluated with exact rational
//!   arithmetic (checked `u128` with per-step reduction); on overflow the
//!   model fails closed with [`SchedError::AccountingOverflow`].
//! - **Quantum (RFC-0011.11/.12):** every dispatch receives the unit's
//!   configured quantum. A thread whose quantum expires transitions
//!   `RUNNING -> PREEMPTED` and is re-queued to `READY` with
//!   `deadline += period`.
//! - **Time model:** `advance` supplies caller-provided scheduling-time
//!   deltas; the model never reads a clock. Caller-supplied time, like all
//!   scheduling time in this repository, is an input whose fidelity is the
//!   caller's obligation (see `docs/HANDOFF-RFC-0037-LIFETIME-BOUND.md`).
//!
//! Explicitly out of scope (recorded limitations, not defects):
//!
//! - EEVDF accounting and any machine-checkable proof of bounded decision
//!   latency (RFC-0011.3/.6). Selection cost is linear in READY count here;
//!   no constant-time bound is claimed.
//! - Layer 2 advisory tuning and Layer 3 offline learning (RFC-0011 §4).
//! - SMP/multi-unit scheduling and load balancing.
//! - Integration with dispatch (RFC-0008), synchronization (RFC-0028),
//!   interrupts (RFC-0010), or any kernel.
//! - Priority is stored (0-255, RFC-0011.8) but is **not** consulted by the
//!   EDF selection rule; it is reported for observability only. PEDLF-style
//!   priority-aware selection would be a different, explicitly declared
//!   algorithm class.

use std::collections::BTreeMap;

/// Stable thread identifier. Deterministic tie-break for EDF selection.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ThreadId(pub u64);

/// Layer 1 thread states (RFC-0011.4).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThreadState {
    /// Waiting for dispatch; eligible for selection.
    Ready,
    /// Currently executing on the execution unit.
    Running,
    /// Waiting for a synchronization primitive or I/O.
    Blocked,
    /// Voluntarily relinquished the timeslot; eligible once re-queued.
    Yielded,
    /// Involuntarily descheduled; re-queued to `READY` when its turn comes.
    Preempted,
    /// Terminated; resources being reclaimed.
    Exited,
}

/// Why a state transition occurred (RFC-0011.5 event contents).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionReason {
    /// Thread entered the scheduler.
    Created,
    /// Thread was selected for dispatch.
    Dispatched,
    /// Quantum exhausted (RFC-0011.12).
    QuantumExpired,
    /// Thread voluntarily relinquished its timeslot.
    Yielded,
    /// Thread was re-queued after yielding or preemption.
    Requeued,
    /// Thread blocked on a synchronization primitive or I/O.
    Blocked,
    /// Thread was unblocked and became eligible again.
    Unblocked,
    /// Thread terminated.
    Exited,
    /// Scheduling time passed a thread's deadline (see RFC-0011.6/.7).
    DeadlineMissed,
}

/// One observable state transition, mirroring the RFC-0011.5 event record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedEvent {
    /// Thread the event concerns.
    pub thread: ThreadId,
    /// Previous state.
    pub from: ThreadState,
    /// New state.
    pub to: ThreadState,
    /// Why the transition happened.
    pub reason: TransitionReason,
}

/// Typed failures; the model never panics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedError {
    /// Thread identifiers are allocated once per universe; reuse is rejected.
    DuplicateThread,
    /// No thread with this identifier.
    UnknownThread,
    /// The requested transition is not valid from the current state.
    InvalidTransition,
    /// Admission bound exceeded: `sum(Ei / Pi) > 1` (uniprocessor EDF, U = 1).
    NotAdmissible,
    /// Arithmetic overflow in utilization accounting; failing closed.
    AccountingOverflow,
    /// A thread with a zero period or zero execution budget is meaningless
    /// for deadline scheduling and is rejected.
    InvalidThreadParameters,
    /// A time advance addition overflowed the scheduling clock.
    ClockOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Thread {
    state: ThreadState,
    period_ns: u64,
    budget_ns: u64,
    remaining_budget_ns: u64,
    deadline_ns: u64,
    /// Declared priority (0-255, 255 highest, RFC-0011.8). Stored and
    /// reported; uniprocessor EDF selection is deadline-first by design.
    priority: u8,
    quantum_remaining_ns: u64,
    /// Set once per deadline period so a miss is counted exactly once.
    miss_flagged: bool,
}

/// Deterministic uniprocessor Layer 1 scheduler (RFC-0011 §3).
///
/// All decision-relevant state is explicit; selection iterates the `READY`
/// set and picks the least `(deadline_ns, tid)`. Insertion order therefore
/// cannot influence the outcome.
#[derive(Debug)]
pub struct UniprocessorEdf {
    threads: BTreeMap<ThreadId, Thread>,
    current: Option<ThreadId>,
    now_ns: u64,
    quantum_ns: u64,
    events: Vec<SchedEvent>,
    misses: u64,
}

impl UniprocessorEdf {
    /// Creates an empty scheduler with a fixed per-dispatch quantum.
    #[must_use]
    pub fn new(quantum_ns: u64) -> Self {
        Self {
            threads: BTreeMap::new(),
            current: None,
            now_ns: 0,
            quantum_ns,
            events: Vec::new(),
            misses: 0,
        }
    }

    /// Returns the current scheduling time.
    #[must_use]
    pub const fn now(&self) -> u64 {
        self.now_ns
    }

    /// Returns the number of deadline misses observed so far.
    ///
    /// Under RFC-0011.7 an admissible set with a correct EDF implementation
    /// must never miss; the counter exists so CTS fixtures can assert that.
    #[must_use]
    pub const fn deadline_misses(&self) -> u64 {
        self.misses
    }

    /// Admits a thread, enforcing the exact utilization bound.
    ///
    /// # Errors
    /// Returns [`SchedError::DuplicateThread`], [`SchedError::InvalidThreadParameters`],
    /// [`SchedError::AccountingOverflow`], or [`SchedError::NotAdmissible`].
    pub fn create_thread(
        &mut self,
        tid: ThreadId,
        period_ns: u64,
        budget_ns: u64,
        priority: u8,
    ) -> Result<(), SchedError> {
        if period_ns == 0 || budget_ns == 0 {
            return Err(SchedError::InvalidThreadParameters);
        }
        if self.threads.contains_key(&tid) {
            return Err(SchedError::DuplicateThread);
        }
        // Exact admission over all non-exited threads plus the new thread.
        let mut num: u128 = 0;
        let mut den: u128 = 1;
        for thread in self.threads.values() {
            if thread.state != ThreadState::Exited {
                (num, den) = add_fraction(
                    num,
                    den,
                    u128::from(thread.budget_ns),
                    u128::from(thread.period_ns),
                )?;
            }
        }
        (num, den) = add_fraction(num, den, u128::from(budget_ns), u128::from(period_ns))?;
        if num > den {
            return Err(SchedError::NotAdmissible);
        }
        let deadline_ns = self
            .now_ns
            .checked_add(period_ns)
            .ok_or(SchedError::ClockOverflow)?;
        self.threads.insert(
            tid,
            Thread {
                state: ThreadState::Ready,
                period_ns,
                budget_ns,
                remaining_budget_ns: budget_ns,
                deadline_ns,
                priority,
                quantum_remaining_ns: self.quantum_ns,
                miss_flagged: false,
            },
        );
        self.record(
            tid,
            ThreadState::Ready,
            ThreadState::Ready,
            TransitionReason::Created,
        );
        Ok(())
    }

    /// Returns the declared priority (RFC-0011.8) or `None` for unknown thread.
    #[must_use]
    pub fn priority(&self, tid: ThreadId) -> Option<u8> {
        self.threads.get(&tid).map(|thread| thread.priority)
    }

    /// Returns the thread's current state, or `None` for unknown thread.
    #[must_use]
    pub fn state(&self, tid: ThreadId) -> Option<ThreadState> {
        self.threads.get(&tid).map(|thread| thread.state)
    }

    /// Returns the thread's absolute deadline, or `None` for unknown thread.
    #[must_use]
    pub fn deadline(&self, tid: ThreadId) -> Option<u64> {
        self.threads.get(&tid).map(|thread| thread.deadline_ns)
    }

    /// Selects and dispatches the next thread (RFC-0011.9).
    ///
    /// Returns `None` when no thread is READY.
    ///
    /// # Errors
    /// Returns [`SchedError::InvalidTransition`] while another thread is
    /// running; the current thread must block, yield, exit, or exhaust its
    /// quantum first.
    pub fn dispatch_next(&mut self) -> Result<Option<ThreadId>, SchedError> {
        if self.current.is_some() {
            return Err(SchedError::InvalidTransition);
        }
        let next = self
            .threads
            .iter()
            .filter(|(_, thread)| thread.state == ThreadState::Ready)
            .min_by_key(|(tid, thread)| (thread.deadline_ns, **tid))
            .map(|(tid, _)| *tid);
        if let Some(tid) = next {
            let thread = self
                .threads
                .get_mut(&tid)
                .ok_or(SchedError::UnknownThread)?;
            let from = thread.state;
            thread.state = ThreadState::Running;
            thread.quantum_remaining_ns = self.quantum_ns;
            self.current = Some(tid);
            self.record(
                tid,
                from,
                ThreadState::Running,
                TransitionReason::Dispatched,
            );
        }
        Ok(next)
    }

    /// Advances the scheduling clock by `dt_ns` (RFC-0012 deltas).
    ///
    /// Consumes the running thread's quantum and budget, preempts and
    /// re-queues on quantum exhaustion, and records deadline misses against
    /// pre-tick deadlines, so a miss is never dropped when quantum expiry
    /// and deadline crossing coincide in the same call. Misses are counted
    /// at most once per thread per deadline period; caller deltas that span
    /// several periods do not retroactively itemize skipped periods.
    ///
    /// # Errors
    /// Returns [`SchedError::ClockOverflow`] on arithmetic overflow. The call
    /// is atomic: on overflow no state is mutated and no event is recorded.
    pub fn advance(&mut self, dt_ns: u64) -> Result<(), SchedError> {
        let now_ns = self
            .now_ns
            .checked_add(dt_ns)
            .ok_or(SchedError::ClockOverflow)?;

        // Pre-validate every transition this call would perform, so the
        // mutation phase below cannot fail (and cannot strand a thread in
        // PREEMPTED).
        if let Some(tid) = self.current {
            let thread = self.threads.get(&tid).ok_or(SchedError::UnknownThread)?;
            if thread.quantum_remaining_ns.saturating_sub(dt_ns) == 0 {
                thread
                    .deadline_ns
                    .checked_add(thread.period_ns)
                    .ok_or(SchedError::ClockOverflow)?;
            }
        }
        let missed = self.miss_candidates(now_ns);

        self.now_ns = now_ns;

        if let Some(tid) = self.current {
            let thread = self
                .threads
                .get_mut(&tid)
                .ok_or(SchedError::UnknownThread)?;
            thread.quantum_remaining_ns = thread.quantum_remaining_ns.saturating_sub(dt_ns);
            thread.remaining_budget_ns = thread.remaining_budget_ns.saturating_sub(dt_ns);
            if thread.quantum_remaining_ns == 0 {
                thread.state = ThreadState::Preempted;
                self.record(
                    tid,
                    ThreadState::Running,
                    ThreadState::Preempted,
                    TransitionReason::QuantumExpired,
                );
                self.current = None;
                self.requeue(tid, TransitionReason::Requeued)?;
            }
        }

        for (tid, state) in missed {
            self.misses = self.misses.saturating_add(1);
            self.record(tid, state, state, TransitionReason::DeadlineMissed);
        }
        Ok(())
    }

    /// Blocks the running thread (RFC-0011.4: `RUNNING -> BLOCKED`).
    ///
    /// # Errors
    /// Returns [`SchedError::InvalidTransition`] when no thread is running.
    pub fn block(&mut self) -> Result<ThreadId, SchedError> {
        let tid = self.take_current()?;
        let thread = self
            .threads
            .get_mut(&tid)
            .ok_or(SchedError::UnknownThread)?;
        thread.state = ThreadState::Blocked;
        self.record(
            tid,
            ThreadState::Running,
            ThreadState::Blocked,
            TransitionReason::Blocked,
        );
        Ok(tid)
    }

    /// Re-admits a blocked thread to `READY` (RFC-0011.4: `BLOCKED -> READY`).
    ///
    /// The thread's deadline and consumption for the current period are
    /// unchanged; its quantum is refreshed for the next dispatch.
    ///
    /// # Errors
    /// Returns [`SchedError::UnknownThread`] or [`SchedError::InvalidTransition`]
    /// when the thread is not blocked.
    pub fn unblock(&mut self, tid: ThreadId) -> Result<(), SchedError> {
        let thread = self
            .threads
            .get_mut(&tid)
            .ok_or(SchedError::UnknownThread)?;
        if thread.state != ThreadState::Blocked {
            return Err(SchedError::InvalidTransition);
        }
        thread.state = ThreadState::Ready;
        thread.quantum_remaining_ns = self.quantum_ns;
        self.record(
            tid,
            ThreadState::Blocked,
            ThreadState::Ready,
            TransitionReason::Unblocked,
        );
        Ok(())
    }

    /// Voluntarily relinquishes the running thread's timeslot
    /// (RFC-0011.4: `RUNNING -> YIELDED`). Call [`Self::requeue_yielded`] to
    /// make it eligible again.
    ///
    /// # Errors
    /// Returns [`SchedError::InvalidTransition`] when no thread is running.
    pub fn yield_now(&mut self) -> Result<ThreadId, SchedError> {
        let tid = self.take_current()?;
        let thread = self
            .threads
            .get_mut(&tid)
            .ok_or(SchedError::UnknownThread)?;
        thread.state = ThreadState::Yielded;
        self.record(
            tid,
            ThreadState::Running,
            ThreadState::Yielded,
            TransitionReason::Yielded,
        );
        Ok(tid)
    }

    /// Re-queues every `YIELDED` thread to `READY`, advancing its deadline
    /// by one period (RFC-0011.4/.12 semantics). Returns the number re-queued.
    ///
    /// # Errors
    /// Returns an error only on internal inconsistency, including
    /// [`SchedError::ClockOverflow`] on deadline arithmetic overflow (the
    /// call is atomic: on overflow no thread is re-queued).
    pub fn requeue_yielded(&mut self) -> Result<u64, SchedError> {
        let yielded: Vec<ThreadId> = self
            .threads
            .iter()
            .filter(|(_, thread)| thread.state == ThreadState::Yielded)
            .map(|(tid, _)| *tid)
            .collect();
        // Pre-validate so the requeue below cannot fail partway through.
        for &tid in &yielded {
            let thread = self.threads.get(&tid).ok_or(SchedError::UnknownThread)?;
            thread
                .deadline_ns
                .checked_add(thread.period_ns)
                .ok_or(SchedError::ClockOverflow)?;
        }
        let mut requeued = 0_u64;
        for tid in yielded {
            self.requeue(tid, TransitionReason::Requeued)?;
            requeued = requeued.saturating_add(1);
        }
        Ok(requeued)
    }

    /// Terminates a thread (RFC-0011.4). Exiting is permitted from any
    /// non-`EXITED` state; `EXITED` threads free their admission share.
    ///
    /// # Errors
    /// Returns [`SchedError::UnknownThread`] or [`SchedError::InvalidTransition`].
    pub fn exit(&mut self, tid: ThreadId) -> Result<(), SchedError> {
        let thread = self
            .threads
            .get_mut(&tid)
            .ok_or(SchedError::UnknownThread)?;
        if thread.state == ThreadState::Exited {
            return Err(SchedError::InvalidTransition);
        }
        let from = thread.state;
        thread.state = ThreadState::Exited;
        if self.current == Some(tid) {
            self.current = None;
        }
        self.record(tid, from, ThreadState::Exited, TransitionReason::Exited);
        Ok(())
    }

    /// Drains the event log (RFC-0011.5). Events are also retained per call.
    #[must_use]
    pub fn take_events(&mut self) -> Vec<SchedEvent> {
        std::mem::take(&mut self.events)
    }

    /// Returns the reduced exact utilization `sum(Ei / Pi)` of all
    /// non-exited threads as a `(numerator, denominator)` pair.
    ///
    /// # Errors
    /// Returns [`SchedError::AccountingOverflow`] on arithmetic overflow.
    pub fn utilization(&self) -> Result<(u128, u128), SchedError> {
        let mut num: u128 = 0;
        let mut den: u128 = 1;
        for thread in self.threads.values() {
            if thread.state != ThreadState::Exited {
                (num, den) = add_fraction(
                    num,
                    den,
                    u128::from(thread.budget_ns),
                    u128::from(thread.period_ns),
                )?;
            }
        }
        Ok((num, den))
    }

    fn take_current(&mut self) -> Result<ThreadId, SchedError> {
        self.current.take().ok_or(SchedError::InvalidTransition)
    }

    /// Collects (and marks) the threads whose deadline the scheduling clock
    /// has passed, evaluating against pre-tick deadlines.
    fn miss_candidates(&mut self, now_ns: u64) -> Vec<(ThreadId, ThreadState)> {
        let mut missed = Vec::new();
        for (&tid, thread) in &mut self.threads {
            if thread.state == ThreadState::Exited {
                continue;
            }
            if now_ns > thread.deadline_ns && !thread.miss_flagged {
                thread.miss_flagged = true;
                missed.push((tid, thread.state));
            }
        }
        missed
    }

    fn requeue(&mut self, tid: ThreadId, reason: TransitionReason) -> Result<(), SchedError> {
        let thread = self
            .threads
            .get_mut(&tid)
            .ok_or(SchedError::UnknownThread)?;
        let from = thread.state;
        thread.deadline_ns = thread
            .deadline_ns
            .checked_add(thread.period_ns)
            .ok_or(SchedError::ClockOverflow)?;
        thread.quantum_remaining_ns = self.quantum_ns;
        thread.state = ThreadState::Ready;
        thread.miss_flagged = false;
        self.record(tid, from, ThreadState::Ready, reason);
        Ok(())
    }

    fn record(
        &mut self,
        thread: ThreadId,
        from: ThreadState,
        to: ThreadState,
        reason: TransitionReason,
    ) {
        self.events.push(SchedEvent {
            thread,
            from,
            to,
            reason,
        });
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a.max(1)
}

fn add_fraction(
    num: u128,
    den: u128,
    add_num: u128,
    add_den: u128,
) -> Result<(u128, u128), SchedError> {
    let n = num
        .checked_mul(add_den)
        .and_then(|left| {
            add_num
                .checked_mul(den)
                .and_then(|right| left.checked_add(right))
        })
        .ok_or(SchedError::AccountingOverflow)?;
    let d = den
        .checked_mul(add_den)
        .ok_or(SchedError::AccountingOverflow)?;
    let g = gcd(n, d);
    Ok((n / g, d / g))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_admission_boundary_accepts_utilization_one() {
        let mut sched = UniprocessorEdf::new(1_000);
        // 1/2 + 1/3 + 1/6 = 1 exactly: admissible for uniprocessor EDF.
        sched.create_thread(ThreadId(1), 2, 1, 10).unwrap();
        sched.create_thread(ThreadId(2), 3, 1, 10).unwrap();
        sched.create_thread(ThreadId(3), 6, 1, 10).unwrap();
        assert_eq!(sched.utilization().unwrap(), (1, 1));
        // One more nanosecond of demand in any period is not admissible.
        assert_eq!(
            sched.create_thread(ThreadId(4), 1_000_000, 1, 10),
            Err(SchedError::NotAdmissible)
        );
    }

    #[test]
    fn admission_counts_exited_threads_as_free_capacity() {
        let mut sched = UniprocessorEdf::new(1_000);
        sched.create_thread(ThreadId(1), 3, 2, 10).unwrap();
        // 2/3 + 2/3 > 1: not admissible.
        assert_eq!(
            sched.create_thread(ThreadId(2), 3, 2, 10),
            Err(SchedError::NotAdmissible)
        );
        sched.exit(ThreadId(1)).unwrap();
        assert!(sched.create_thread(ThreadId(2), 3, 2, 10).is_ok());
        assert_eq!(sched.utilization().unwrap(), (2, 3));
    }

    #[test]
    fn duplicate_and_invalid_threads_fail_closed() {
        let mut sched = UniprocessorEdf::new(1_000);
        assert_eq!(
            sched.create_thread(ThreadId(1), 0, 1, 10),
            Err(SchedError::InvalidThreadParameters)
        );
        assert_eq!(
            sched.create_thread(ThreadId(1), 1, 0, 10),
            Err(SchedError::InvalidThreadParameters)
        );
        sched.create_thread(ThreadId(1), 10, 1, 10).unwrap();
        assert_eq!(
            sched.create_thread(ThreadId(1), 10, 1, 10),
            Err(SchedError::DuplicateThread)
        );
    }

    #[test]
    fn selection_is_deterministic_under_permuted_insertion() {
        fn decisions(order: &[(u64, u64, u64)]) -> Vec<ThreadId> {
            let mut sched = UniprocessorEdf::new(1_000);
            for &(tid, period, budget) in order {
                sched
                    .create_thread(ThreadId(tid), period, budget, 10)
                    .unwrap();
            }
            let mut picked = Vec::new();
            for _ in 0..3 {
                picked.push(sched.dispatch_next().unwrap().unwrap());
                sched.yield_now().unwrap();
                sched.requeue_yielded().unwrap();
            }
            picked
        }
        let a = decisions(&[(1, 10, 1), (2, 10, 1), (3, 10, 1)]);
        let b = decisions(&[(3, 10, 1), (1, 10, 1), (2, 10, 1)]);
        assert_eq!(a, b);
        // All three share deadline 10; tie-break is by tid, so 1, 2, 3 repeat.
        assert_eq!(a, vec![ThreadId(1), ThreadId(2), ThreadId(3)]);
    }

    #[test]
    fn quantum_expiry_preempts_and_requeues_with_new_deadline() {
        let mut sched = UniprocessorEdf::new(5);
        sched.create_thread(ThreadId(1), 100, 100, 10).unwrap();
        assert_eq!(sched.dispatch_next().unwrap(), Some(ThreadId(1)));
        assert_eq!(sched.state(ThreadId(1)), Some(ThreadState::Running));
        sched.advance(5).unwrap();
        assert_eq!(sched.state(ThreadId(1)), Some(ThreadState::Ready));
        assert_eq!(sched.deadline(ThreadId(1)), Some(200));
        let events = sched.take_events();
        let reasons: Vec<TransitionReason> = events.iter().map(|e| e.reason).collect();
        assert!(reasons.contains(&TransitionReason::QuantumExpired));
        assert!(reasons.contains(&TransitionReason::Requeued));
        // Deadline miss must not be charged for an in-flight period.
        assert_eq!(sched.deadline_misses(), 0);
    }

    #[test]
    fn blocked_thread_is_not_eligible_until_unblocked() {
        let mut sched = UniprocessorEdf::new(1_000);
        sched.create_thread(ThreadId(1), 100, 50, 10).unwrap();
        sched.create_thread(ThreadId(2), 100, 50, 10).unwrap();
        assert_eq!(sched.dispatch_next().unwrap(), Some(ThreadId(1)));
        sched.block().unwrap();
        assert_eq!(sched.dispatch_next().unwrap(), Some(ThreadId(2)));
        sched.block().unwrap();
        assert_eq!(sched.dispatch_next().unwrap(), None);
        sched.unblock(ThreadId(1)).unwrap();
        assert_eq!(sched.dispatch_next().unwrap(), Some(ThreadId(1)));
        assert_eq!(
            sched.unblock(ThreadId(1)),
            Err(SchedError::InvalidTransition)
        );
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        let mut sched = UniprocessorEdf::new(1_000);
        assert_eq!(sched.dispatch_next().unwrap(), None);
        assert_eq!(sched.block(), Err(SchedError::InvalidTransition));
        assert_eq!(sched.yield_now(), Err(SchedError::InvalidTransition));
        sched.create_thread(ThreadId(1), 100, 100, 10).unwrap();
        sched.dispatch_next().unwrap();
        assert_eq!(sched.dispatch_next(), Err(SchedError::InvalidTransition));
        sched.exit(ThreadId(1)).unwrap();
        assert_eq!(sched.exit(ThreadId(1)), Err(SchedError::InvalidTransition));
        assert_eq!(sched.dispatch_next().unwrap(), None);
    }

    #[test]
    fn deadline_miss_is_counted_once_per_period() {
        let mut sched = UniprocessorEdf::new(1_000);
        sched.create_thread(ThreadId(1), 100, 100, 10).unwrap();
        sched.dispatch_next().unwrap();
        sched.block().unwrap();
        sched.advance(150).unwrap(); // now = 150 > deadline 100, while blocked
        assert_eq!(sched.deadline_misses(), 1);
        sched.advance(10).unwrap(); // same deadline period: counted once
        assert_eq!(sched.deadline_misses(), 1);

        // Requeue advances the deadline to 200 and re-arms the flag.
        sched.unblock(ThreadId(1)).unwrap();
        sched.dispatch_next().unwrap();
        sched.yield_now().unwrap();
        sched.requeue_yielded().unwrap();
        assert_eq!(sched.deadline(ThreadId(1)), Some(200));
        sched.advance(60).unwrap(); // now = 210 > deadline 200
        assert_eq!(sched.deadline_misses(), 2);
    }

    #[test]
    fn priority_extremes_are_accepted() {
        let mut sched = UniprocessorEdf::new(1_000);
        sched.create_thread(ThreadId(1), 10, 1, 0).unwrap();
        sched.create_thread(ThreadId(2), 10, 1, 255).unwrap();
        assert_eq!(sched.priority(ThreadId(1)), Some(0));
        assert_eq!(sched.priority(ThreadId(2)), Some(255));
    }

    #[test]
    fn deadline_miss_is_charged_when_quantum_expiry_crosses_the_deadline() {
        let mut sched = UniprocessorEdf::new(8);
        sched.create_thread(ThreadId(1), 10, 10, 10).unwrap();
        sched.dispatch_next().unwrap();
        sched.advance(7).unwrap(); // now = 7, quantum remaining 1, deadline 10
        // Quantum expiry and deadline crossing coincide in one call.
        sched.advance(6).unwrap(); // now = 13
        assert_eq!(sched.deadline_misses(), 1);
        assert_eq!(sched.state(ThreadId(1)), Some(ThreadState::Ready));
        assert_eq!(sched.deadline(ThreadId(1)), Some(20));
    }

    #[test]
    fn clock_overflow_pre_validation_leaves_state_untouched() {
        let mut sched = UniprocessorEdf::new(1);
        sched
            .create_thread(ThreadId(1), u64::MAX, u64::MAX, 10)
            .unwrap();
        sched.dispatch_next().unwrap();
        // Quantum expires in the same call that would requeue with a
        // deadline overflowing u64; the call must fail atomically.
        assert_eq!(sched.advance(1), Err(SchedError::ClockOverflow));
        assert_eq!(sched.state(ThreadId(1)), Some(ThreadState::Running));
        assert_eq!(sched.now(), 0);

        sched.yield_now().unwrap();
        sched.yield_now().unwrap_err(); // not running anymore; still Yielded
        assert_eq!(sched.requeue_yielded(), Err(SchedError::ClockOverflow));
        assert_eq!(sched.state(ThreadId(1)), Some(ThreadState::Yielded));
    }

    #[test]
    fn yield_two_step_transition_is_visible() {
        let mut sched = UniprocessorEdf::new(1_000);
        sched.create_thread(ThreadId(1), 100, 100, 10).unwrap();
        sched.dispatch_next().unwrap();
        sched.yield_now().unwrap();
        assert_eq!(sched.state(ThreadId(1)), Some(ThreadState::Yielded));
        // Not eligible while in Yielded.
        assert_eq!(sched.dispatch_next().unwrap(), None);
        assert_eq!(sched.requeue_yielded().unwrap(), 1);
        assert_eq!(sched.state(ThreadId(1)), Some(ThreadState::Ready));
        assert_eq!(sched.deadline(ThreadId(1)), Some(200));
    }
}
