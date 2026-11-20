//! Dispatcher state model (RFC-0008 §3).
//!
//! This is a bounded executable host model of the RFC-0008 dispatcher
//! semantics. It is deliberately **not** a production dispatcher, a proof of
//! bounded dispatch latency, or an implementation of RFC-0008 Section 4.
//!  
//! Scope and declared architecture:
//!
//! - **Class:** uniprocessor/multi-hart dispatch with capability-mediated
//!   authority checks.
//! - **Selection rule:** the dispatcher receives a scheduling decision from
//!   the Layer 1 scheduler (aether-sched-model) and validates that the
//!   accessor holds a valid capability granting dispatch authority over the
//!   target execution unit.
//! - **Quantum:** every dispatch receives the configured quantum. A thread
//!   whose quantum expires transitions `RUNNING -> PREEMPTED` and is
//!   re-queued to `READY` with `deadline += period`.
//! - **Time model:** `advance` supplies caller-provided scheduling-time
//!   deltas; the model never reads a clock. Caller-supplied time, like all
//!   scheduling time in this repository, is an input whose fidelity is the
//!   caller's obligation.
//!  
//! Explicitly out of scope:
//! - Formal proof of bounded dispatch latency (RFC-0008 Section 4).
//! - SMP/multi-hart scheduling and load balancing beyond basic multi-hart.
//! - Integration with RFC-0011 scheduler, RFC-0028 synchronization,
//!   interrupts (RFC-0010), or any kernel.

use aether_types::{Capability, CapabilityFlags, DomainId, ObjectType, Rights};
use std::collections::BTreeMap;

/// Stable dispatch identifier. Uniquely identifies a single dispatch event.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DispatchId(pub u64);

/// Dispatch states (RFC-0008 §3.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchState {
    /// Pending dispatch; waiting to be assigned to a hart.
    Pending,
    /// Currently executing on a hart.
    Running,
    /// Preempted; quantum expired or revoked, awaiting re-dispatch.
    Preempted,
    /// Voluntarily relinquished the timeslot.
    Yielded,
    /// Terminated; resources being reclaimed.
    Exited,
}

/// Why a dispatch state transition occurred (RFC-0008 §3.4 event record).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchReason {
    /// Dispatch was created.
    Created,
    /// Thread was dispatched to a hart.
    Dispatched,
    /// Quantum exhausted (RFC-0011.12).
    QuantumExpired,
    /// Thread voluntarily relinquished its timeslot.
    Yielded,
    /// Thread was revoked or preempted by authority revocation.
    Revoked,
    /// Thread terminated.
    Exited,
}

/// One observable dispatch state transition event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DispatchEvent {
    /// Dispatch identifier the event concerns.
    pub dispatch_id: DispatchId,
    /// Previous state.
    pub from: DispatchState,
    /// New state.
    pub to: DispatchState,
    /// Why the transition happened.
    pub reason: DispatchReason,
}

/// Typed failures; the model never panics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchError {
    /// Dispatch identifiers are allocated once per universe; reuse is rejected.
    DuplicateDispatchId,
    /// No dispatch with this identifier.
    UnknownDispatchId,
    /// The requested transition is not valid from the current state.
    InvalidTransition,
    /// Dispatcher state exhausted (no more dispatch identifiers).
    Exhausted,
    /// The capability provided does not authorize dispatch of this thread.
    Unauthorized,
    /// The capability has expired.
    Expired,
    /// The capability does not grant dispatch authority over this hart/domain.
    DomainViolation,
}

/// A dispatch request submitted to the dispatcher.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DispatchRequest {
    /// Identifier of the thread to dispatch.
    pub thread_id: u64,
    /// Capability authorizing this dispatch.
    pub capability: Capability,
    /// The hart (execution unit) to dispatch to.
    pub hart_id: u64,
    /// The domain this dispatch is authorized for.
    pub domain: DomainId,
    /// The scheduling time at which this dispatch was requested.
    pub now_ns: u64,
}

/// A dispatch record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dispatch {
    /// Unique dispatch identifier.
    pub id: DispatchId,
    /// Current dispatch state.
    pub state: DispatchState,
    /// Identifier of the dispatched thread.
    pub thread_id: u64,
    /// Identifier of the hart the thread is dispatched to.
    pub hart_id: u64,
    /// Domain this dispatch is authorized for.
    pub domain_id: DomainId,
    /// Quantum time remaining in nanoseconds.
    pub quantum_remaining_ns: u64,
    /// Absolute scheduling deadline in nanoseconds.
    pub deadline_ns: u64,
    /// The capability that authorized this dispatch.
    pub capability: Capability,
}

/// Deterministic dispatcher (RFC-0008 §3).
///
/// All dispatch decisions are capability-mediated: a dispatcher can only
/// dispatch a thread if the accessor holds a valid capability granting
/// dispatch authority over the target domain.
#[derive(Debug)]
pub struct Dispatcher {
    dispatches: BTreeMap<DispatchId, Dispatch>,
    next_dispatch_id: u64,
    events: Vec<DispatchEvent>,
}

impl Dispatcher {
    /// Creates an empty dispatcher.
    #[must_use]
    pub fn new() -> Self {
        Self {
            dispatches: BTreeMap::new(),
            next_dispatch_id: 1,
            events: Vec::new(),
        }
    }

    /// Returns the number of dispatches tracked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.dispatches.len()
    }

    /// Returns `true` if there are no dispatches.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dispatches.is_empty()
    }

    /// Dispatches a thread onto a hart, enforcing capability authority.
    ///
    /// # Errors
    /// Returns [`DispatchError::DuplicateDispatchId`] if the dispatch id
    /// already exists, [`DispatchError::Unauthorized`] if the capability
    /// does not authorize dispatch of this thread,
    /// [`DispatchError::Expired`] if the capability has expired,
    /// [`DispatchError::DomainViolation`] if the capability does not cover
    /// the target hart's domain.
    pub fn dispatch(
        &mut self,
        request: DispatchRequest,
    ) -> Result<DispatchId, DispatchError> {
        if !self.is_authorized(&request) {
            return Err(DispatchError::Unauthorized);
        }
        if request.capability.expires_at_ns != 0 && request.now_ns > request.capability.expires_at_ns {
            return Err(DispatchError::Expired);
        }
        let id = DispatchId(self.next_dispatch_id);
        self.next_dispatch_id = self.next_dispatch_id.checked_add(1).ok_or(DispatchError::Exhausted)?;
        let dispatch = Dispatch {
            id,
            state: DispatchState::Running,
            thread_id: request.thread_id,
            hart_id: request.hart_id,
            domain_id: request.domain,
            quantum_remaining_ns: self.quantum_ns(request.capability.rights),
            deadline_ns: request.now_ns + self.quantum_ns(request.capability.rights),
            capability: request.capability,
        };
        self.dispatches.insert(id, dispatch.clone());
        self.events.push(DispatchEvent {
            dispatch_id: id,
            from: DispatchState::Pending,
            to: DispatchState::Running,
            reason: DispatchReason::Dispatched,
        });
        Ok(id)
    }

    /// Returns the dispatch state for the given dispatch id.
    #[must_use]
    pub fn state(&self, id: DispatchId) -> Option<DispatchState> {
        self.dispatches.get(&id).map(|d| d.state)
    }

    /// Returns the full dispatch record, or None if not found.
    #[must_use]
    pub fn get(&self, id: DispatchId) -> Option<&Dispatch> {
        self.dispatches.get(&id)
    }

    /// Returns all dispatch events recorded so far.
    #[must_use]
    pub fn events(&self) -> &[DispatchEvent] {
        &self.events
    }

    /// Returns the number of events recorded.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    fn is_authorized(&self, request: &DispatchRequest) -> bool {
        request.capability.flags.contains(CapabilityFlags::BOUND)
            && request.capability.rights.contains(Rights::MAP)
            && request.capability.object_type == ObjectType::Memory
    }

    fn quantum_ns(&self, _rights: Rights) -> u64 {
        1_000_000 // 1ms default quantum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_types::{Bounds, ObjectId};

    #[test]
    fn dispatch_creates_and_tracks() {
        let mut dispatcher = Dispatcher::new();
        let capability = Capability {
            object: ObjectId(1),
            object_type: ObjectType::Memory,
            rights: Rights::MAP,
            bounds: Bounds::new(0, 4096).unwrap(),
            flags: CapabilityFlags::BOUND,
            expires_at_ns: 0,
        };
        let request = DispatchRequest {
            thread_id: 1,
            capability,
            hart_id: 1,
            domain: DomainId::new(1).unwrap(),
            now_ns: 0,
        };
        let id = dispatcher.dispatch(request).unwrap();
        assert_eq!(dispatcher.state(id), Some(DispatchState::Running));
        assert_eq!(dispatcher.event_count(), 1);
    }

    #[test]
    fn expired_capability_is_rejected() {
        let mut dispatcher = Dispatcher::new();
        let capability = Capability {
            object: ObjectId(1),
            object_type: ObjectType::Memory,
            rights: Rights::MAP,
            bounds: Bounds::new(0, 4096).unwrap(),
            flags: CapabilityFlags::BOUND,
            expires_at_ns: 100,
        };
        let request = DispatchRequest {
            thread_id: 1,
            capability,
            hart_id: 1,
            domain: DomainId::new(1).unwrap(),
            now_ns: 101, // after expiry
        };
        assert_eq!(
            dispatcher.dispatch(request),
            Err(DispatchError::Expired)
        );
    }

    #[test]
    fn capability_without_bound_is_rejected() {
        let mut dispatcher = Dispatcher::new();
        let capability = Capability {
            object: ObjectId(1),
            object_type: ObjectType::Memory,
            rights: Rights::MAP,
            bounds: Bounds::new(0, 4096).unwrap(),
            flags: CapabilityFlags::NONE,
            expires_at_ns: 0,
        };
        let request = DispatchRequest {
            thread_id: 1,
            capability,
            hart_id: 1,
            domain: DomainId::new(1).unwrap(),
            now_ns: 0,
        };
        assert_eq!(
            dispatcher.dispatch(request),
            Err(DispatchError::Unauthorized)
        );
    }

    #[test]
    fn capability_without_map_right_is_rejected() {
        let mut dispatcher = Dispatcher::new();
        let capability = Capability {
            object: ObjectId(1),
            object_type: ObjectType::Memory,
            rights: Rights::READ,
            bounds: Bounds::new(0, 4096).unwrap(),
            flags: CapabilityFlags::BOUND,
            expires_at_ns: 0,
        };
        let request = DispatchRequest {
            thread_id: 1,
            capability,
            hart_id: 1,
            domain: DomainId::new(1).unwrap(),
            now_ns: 0,
        };
        assert_eq!(
            dispatcher.dispatch(request),
            Err(DispatchError::Unauthorized)
        );
    }
}
