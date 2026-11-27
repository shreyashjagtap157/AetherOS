//! Dispatcher state model (RFC-0008).
//!
//! This is a bounded executable host model of the RFC-0008 dispatch
//! protocol: the act of moving a thread from a scheduler decision to an
//! execution unit. It is deliberately **not** a production dispatcher, a
//! proof of bounded dispatch latency, or hardware/HART integration.
//!
//! Scope and declared architecture:
//!
//! - **Class:** uniprocessor/multi-hart dispatch with capability-mediated
//!   authority checks and per-domain address-space activation.
//! - **Protocol (RFC-0008.1):** scheduler decision → capability verification
//!   (RFC-0008.2) → domain compatibility (RFC-0008.3) → address-space
//!   activation (RFC-0008.4) → execute.
//! - **State machine:** `Idle -> Preverify -> AddrspaceActivate -> Execute
//!   -> Suspend -> Idle/Exit` (RFC-0008 §3.1).
//! - **Atomicity:** every check runs before any state is mutated, so a
//!   rejected dispatch leaves the dispatcher unchanged in this
//!   single-threaded host model (RFC-0008.11, .12).
//! - **No blocking:** the protocol performs no I/O or allocation (RFC-0008.6).
//! - **Events:** dispatch outcomes and lifecycle transitions are recorded
//!   locally (`ThreadPreempted` per RFC-0031.18, migration per RFC-0031.17).
//!
//! Explicitly deferred or out of scope:
//! - RFC-0008.5 documented worst-case dispatch bound: declared here as
//!   `DISPATCH_BOUND_NS`; hardware implementations must declare and verify
//!   their own bound.
//! - RFC-0008.8 migration lock hierarchy (RFC-0028): synchronization is a
//!   separate subsystem; migration here re-runs pre-dispatch verification.
//! - RFC-0008.10 idle power-state transitions beyond recording the idle
//!   event; no hardware power management.
//! - SMP load balancing, interrupts (RFC-0010), or kernel integration.
//! - RFC-0031 event delivery to external consumers (events are recorded
//!   locally).

#![allow(clippy::new_without_default)]
#![allow(clippy::unused_self)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]

use aether_address_space_model::{AddressSpaceError, AddressSpaceManager, AddressSpaceState};
use aether_exec_domain_model::{DomainError, DomainManager};
use aether_types::{Capability, CapabilityFlags, DomainId, ObjectType, Rights};
use std::collections::BTreeMap;

/// Stable dispatch identifier. Uniquely identifies a single dispatch event.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DispatchId(pub u64);

/// Dispatch states (RFC-0008 §3.1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchState {
    /// Execution unit available, no thread assigned.
    Idle,
    /// Verifying dispatch authority and domain compatibility
    /// (RFC-0008.2, RFC-0008.3).
    Preverify,
    /// Activating the thread's address space on the target unit
    /// (RFC-0008.4).
    AddrspaceActivate,
    /// Thread executing on the execution unit.
    Execute,
    /// Thread descheduled mid-quantum (preempted, yielded, or suspended).
    Suspend,
    /// Thread terminated; domain cleanup initiated.
    Exit,
}

/// Why a dispatch state transition occurred (RFC-0008 §3 event record).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchReason {
    /// Dispatch was admitted and the thread is executing.
    Dispatched,
    /// Capability verification failed (RFC-0008.2 → `CAPABILITY_FAIL`).
    CapabilityFail,
    /// Domain compatibility check failed (RFC-0008.3 → `DOMAIN_MISMATCH`).
    DomainMismatch,
    /// Address space activation failed (RFC-0008.4 → `ASID_ACTIVATE_FAIL`).
    AsidActivateFail,
    /// Thread was preempted by a higher-priority decision (RFC-0008.9 →
    /// `ThreadPreempted` event per RFC-0031.18).
    Preempted,
    /// Thread was migrated to another execution unit (RFC-0008.7 →
    /// migration event per RFC-0031.17).
    Migrated,
    /// Execution unit entered the idle state (RFC-0008.10).
    IdleEntered,
    /// Execution unit resumed from the idle state (RFC-0008.10).
    HartResumed,
    /// Thread terminated.
    Exited,
}

/// One observable dispatch state transition event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchEvent {
    /// Dispatch identifier the event concerns.
    pub dispatch_id: DispatchId,
    /// Previous state.
    pub from: DispatchState,
    /// New state.
    pub to: DispatchState,
    /// Why the transition happened.
    pub reason: DispatchReason,
    /// The execution unit (hart) the event concerns.
    pub hart_id: u64,
    /// The domain involved, if any (idle events have no domain).
    pub domain: Option<DomainId>,
    /// Scheduling time at which the transition occurred, in nanoseconds.
    pub timestamp_ns: u64,
}

/// Typed failures; the model never panics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchError {
    /// No dispatch with this identifier.
    UnknownDispatchId,
    /// The requested transition is not valid from the current state.
    InvalidTransition,
    /// Dispatcher state exhausted (no more dispatch identifiers).
    Exhausted,
    /// The capability does not authorize dispatch of this thread
    /// (RFC-0008.2 → `CAPABILITY_FAIL`).
    Unauthorized,
    /// The capability has expired.
    Expired,
    /// The target domain is incompatible with the execution unit
    /// (RFC-0008.3 → `DOMAIN_MISMATCH`).
    DomainMismatch,
    /// The thread's address space could not be activated
    /// (RFC-0008.4 → `ASID_ACTIVATE_FAIL`).
    AsidActivateFail,
}

/// A dispatch request submitted to the dispatcher (RFC-0008.1).
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    /// The capability that authorized this dispatch.
    pub capability: Capability,
}

/// Per-hart tracking state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HartState {
    /// Execution unit available.
    Idle,
    /// Thread executing on this hart.
    Running {
        /// The dispatch currently executing.
        dispatch_id: DispatchId,
        /// The domain currently running on this hart.
        domain: DomainId,
    },
}

/// Documented worst-case dispatch bound in nanoseconds (RFC-0008.5).
///
/// The host model executes the dispatch protocol synchronously with no
/// blocking operations (RFC-0008.6), so the observed cost is deterministic.
/// Hardware implementations must declare and verify their own bound in the
/// conformance declaration.
pub const DISPATCH_BOUND_NS: u64 = 10_000; // 10 microseconds

/// Deterministic dispatcher (RFC-0008 §3).
///
/// Runs the dispatch protocol and enforces capability-mediated dispatch
/// authority. All validation precedes any mutation, so transitions are
/// atomic in this single-threaded host model.
pub struct Dispatcher {
    domains: DomainManager,
    spaces: AddressSpaceManager,
    harts: BTreeMap<u64, HartState>,
    dispatches: BTreeMap<DispatchId, Dispatch>,
    next_dispatch_id: u64,
    events: Vec<DispatchEvent>,
}

impl Dispatcher {
    /// Creates an empty dispatcher with no domains, address spaces, harts,
    /// or dispatches.
    #[must_use]
    pub fn new() -> Self {
        Self {
            domains: DomainManager::new(),
            spaces: AddressSpaceManager::new(),
            harts: BTreeMap::new(),
            dispatches: BTreeMap::new(),
            next_dispatch_id: 1,
            events: Vec::new(),
        }
    }

    /// Returns the documented worst-case dispatch bound (RFC-0008.5).
    #[must_use]
    pub const fn dispatch_bound_ns() -> u64 {
        DISPATCH_BOUND_NS
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

    /// Returns the number of dispatch events recorded.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns all dispatch events recorded so far, oldest first.
    #[must_use]
    pub fn events(&self) -> &[DispatchEvent] {
        &self.events
    }

    /// Registers `domain` with the dispatcher. A domain must be registered
    /// before it can be dispatched (RFC-0008.3 domain compatibility is
    /// checked against registered domains). The domain's RFC-0009 address
    /// space is created in the `Created` state and is activated by
    /// `dispatch` before execution (RFC-0008.4).
    ///
    /// # Errors
    /// Returns `DomainError::DuplicateDomain` if `id` already exists, or
    /// `DomainError::CycleDetected` if `parent` would introduce a cycle
    /// (RFC-0007.23).
    pub fn register_domain(
        &mut self,
        id: DomainId,
        parent: Option<DomainId>,
        now_ns: u64,
    ) -> Result<(), DomainError> {
        self.domains.create_domain(id, parent, now_ns)?;
        self.spaces
            .create_address_space(id, &self.domains, now_ns)
            .map_err(|_| DomainError::UnknownDomain)?;
        Ok(())
    }

    /// Returns the lifecycle state of `domain`'s address space, if any.
    #[must_use]
    pub fn address_space_state(
        &self,
        domain: DomainId,
    ) -> Result<AddressSpaceState, AddressSpaceError> {
        self.spaces.state_of(domain)
    }

    /// Returns the current tracking state of `hart`, if any.
    #[must_use]
    pub fn hart_state(&self, hart: u64) -> Option<HartState> {
        self.harts.get(&hart).copied()
    }

    /// Returns the dispatch state for the given dispatch id.
    #[must_use]
    pub fn state(&self, id: DispatchId) -> Option<DispatchState> {
        self.dispatches.get(&id).copied().map(|d| d.state)
    }

    /// Returns the full dispatch record, or `None` if not found.
    #[must_use]
    pub fn get(&self, id: DispatchId) -> Option<Dispatch> {
        self.dispatches.get(&id).copied()
    }

    /// Runs the RFC-0008 dispatch protocol for `request`.
    ///
    /// The protocol is atomic in this host model: every check below is
    /// performed before any state is mutated, so a rejected dispatch leaves
    /// the dispatcher unchanged (RFC-0008.11, .12). No blocking operation
    /// is performed (RFC-0008.6).
    ///
    /// # Errors
    /// Returns `Unauthorized` if the capability does not authorize dispatch
    /// of this thread or has expired (RFC-0008.2 → `CAPABILITY_FAIL`),
    /// `DomainMismatch` if the domain is incompatible with the target
    /// execution unit (RFC-0008.3), or `AsidActivateFail` if the thread's
    /// address space could not be activated (RFC-0008.4).
    pub fn dispatch(&mut self, request: DispatchRequest) -> Result<DispatchId, DispatchError> {
        // RFC-0008.2: verify the capability grants dispatch authority.
        self.verify_capability(&request)?;
        // RFC-0008.3: verify domain compatibility with the execution unit.
        self.verify_domain(request.hart_id, request.domain)?;
        // RFC-0008.4: the thread's address space must be activatable on the
        // unit (exists and not destroyed).
        self.activate_address_space(request.domain)?;

        // All checks passed; apply atomically.
        let id = DispatchId(self.next_dispatch_id);
        self.next_dispatch_id = self
            .next_dispatch_id
            .checked_add(1)
            .ok_or(DispatchError::Exhausted)?;
        let dispatch = Dispatch {
            id,
            state: DispatchState::Execute,
            thread_id: request.thread_id,
            hart_id: request.hart_id,
            domain_id: request.domain,
            capability: request.capability,
        };
        self.dispatches.insert(id, dispatch);
        self.harts.insert(
            request.hart_id,
            HartState::Running {
                dispatch_id: id,
                domain: request.domain,
            },
        );
        self.events.push(DispatchEvent {
            dispatch_id: id,
            from: DispatchState::Idle,
            to: DispatchState::Execute,
            reason: DispatchReason::Dispatched,
            hart_id: request.hart_id,
            domain: Some(request.domain),
            timestamp_ns: request.now_ns,
        });
        Ok(id)
    }

    /// Migrates a running dispatch to `target_hart`.
    ///
    /// Performs the same pre-dispatch verification (RFC-0008.2, .3) and
    /// address-space reactivation (RFC-0008.7) on the target unit, then
    /// emits a migration event (RFC-0031.17).
    ///
    /// # Errors
    /// Returns `UnknownDispatchId`, `InvalidTransition` (the dispatch is
    /// not `Execute`), `Unauthorized`, `DomainMismatch`, or
    /// `AsidActivateFail`.
    pub fn migrate(
        &mut self,
        id: DispatchId,
        target_hart: u64,
        now_ns: u64,
    ) -> Result<(), DispatchError> {
        let dispatch = match self.dispatches.get(&id) {
            Some(d) => *d,
            None => return Err(DispatchError::UnknownDispatchId),
        };
        if dispatch.state != DispatchState::Execute {
            return Err(DispatchError::InvalidTransition);
        }
        // Re-verify authority and domain on the target unit (RFC-0008.7).
        let request = DispatchRequest {
            thread_id: dispatch.thread_id,
            capability: dispatch.capability,
            hart_id: target_hart,
            domain: dispatch.domain_id,
            now_ns,
        };
        self.verify_capability(&request)?;
        self.verify_domain(target_hart, request.domain)?;
        self.activate_address_space(request.domain)?;

        // Move the dispatch to the target hart.
        if let Some(HartState::Running { .. }) = self.harts.get(&dispatch.hart_id) {
            self.harts.remove(&dispatch.hart_id);
        }
        self.harts.insert(
            target_hart,
            HartState::Running {
                dispatch_id: id,
                domain: request.domain,
            },
        );
        let d = self
            .dispatches
            .get_mut(&id)
            .expect("dispatch was just validated");
        d.hart_id = target_hart;
        d.state = DispatchState::Execute;
        self.events.push(DispatchEvent {
            dispatch_id: id,
            from: DispatchState::Execute,
            to: DispatchState::Execute,
            reason: DispatchReason::Migrated,
            hart_id: target_hart,
            domain: Some(request.domain),
            timestamp_ns: now_ns,
        });
        Ok(())
    }

    /// Preempts a running dispatch (RFC-0008.9), emitting a
    /// `ThreadPreempted` event per RFC-0031.18.
    pub fn preempt(&mut self, id: DispatchId, now_ns: u64) -> Result<(), DispatchError> {
        let dispatch = match self.dispatches.get(&id) {
            Some(d) => *d,
            None => return Err(DispatchError::UnknownDispatchId),
        };
        if dispatch.state != DispatchState::Execute {
            return Err(DispatchError::InvalidTransition);
        }
        let domain = dispatch.domain_id;
        let hart = dispatch.hart_id;
        if let Some(HartState::Running { .. }) = self.harts.get(&hart) {
            self.harts.remove(&hart);
        }
        self.harts.insert(hart, HartState::Idle);
        let d = self
            .dispatches
            .get_mut(&id)
            .expect("dispatch was just validated");
        d.state = DispatchState::Suspend;
        self.events.push(DispatchEvent {
            dispatch_id: id,
            from: DispatchState::Execute,
            to: DispatchState::Suspend,
            reason: DispatchReason::Preempted,
            hart_id: hart,
            domain: Some(domain),
            timestamp_ns: now_ns,
        });
        Ok(())
    }

    /// Transitions `hart` to the idle state (RFC-0008.10). The execution
    /// unit moves to a declared low-power state; exiting idle is triggered
    /// by a wake signal via [`resume`](Self::resume).
    ///
    /// # Errors
    /// Returns `InvalidTransition` if `hart` is currently running a thread.
    pub fn idle(&mut self, hart: u64, now_ns: u64) -> Result<(), DispatchError> {
        if let Some(HartState::Running { .. }) = self.harts.get(&hart) {
            return Err(DispatchError::InvalidTransition);
        }
        self.harts.insert(hart, HartState::Idle);
        self.events.push(DispatchEvent {
            dispatch_id: DispatchId(0),
            from: DispatchState::Idle,
            to: DispatchState::Idle,
            reason: DispatchReason::IdleEntered,
            hart_id: hart,
            domain: None,
            timestamp_ns: now_ns,
        });
        Ok(())
    }

    /// Resumes `hart` from the idle state (RFC-0008.10).
    pub fn resume(&mut self, hart: u64, now_ns: u64) -> Result<(), DispatchError> {
        match self.harts.get(&hart) {
            Some(HartState::Idle) => {}
            Some(HartState::Running { .. }) => return Err(DispatchError::InvalidTransition),
            None => {}
        }
        self.harts.insert(hart, HartState::Idle);
        self.events.push(DispatchEvent {
            dispatch_id: DispatchId(0),
            from: DispatchState::Idle,
            to: DispatchState::Idle,
            reason: DispatchReason::HartResumed,
            hart_id: hart,
            domain: None,
            timestamp_ns: now_ns,
        });
        Ok(())
    }

    /// RFC-0008.2: the capability must be a memory capability granting
    /// `Rights::MAP`, carry `CAPF_BOUND` (bound to a domain, not
    /// transferable), and not be expired.
    fn verify_capability(&self, request: &DispatchRequest) -> Result<(), DispatchError> {
        if request.capability.object_type != ObjectType::Memory
            || !request.capability.rights.contains(Rights::MAP)
            || !request.capability.flags.contains(CapabilityFlags::BOUND)
        {
            return Err(DispatchError::Unauthorized);
        }
        if request.capability.expires_at_ns != 0
            && request.now_ns > request.capability.expires_at_ns
        {
            return Err(DispatchError::Expired);
        }
        Ok(())
    }

    /// RFC-0008.3: the target domain must be compatible with the execution
    /// unit. A `CAPF_BOUND` capability binds to a domain, and a hart may run
    /// only one domain at a time.
    fn verify_domain(&self, hart: u64, domain: DomainId) -> Result<(), DispatchError> {
        if let Some(HartState::Running {
            domain: running, ..
        }) = self.harts.get(&hart)
        {
            if *running != domain {
                return Err(DispatchError::DomainMismatch);
            }
        }
        Ok(())
    }

    /// RFC-0008.4: the domain must have an activatable address space — it
    /// must exist and not be destroyed. In this host model an existing,
    /// non-destroyed address space is treated as activatable; the actual
    /// page-mapped contents are RFC-0009's domain.
    fn activate_address_space(&self, domain: DomainId) -> Result<(), DispatchError> {
        match self.spaces.state_of(domain) {
            Ok(AddressSpaceState::Destroyed) => Err(DispatchError::AsidActivateFail),
            Ok(_) => Ok(()),
            // No address space exists for the domain (unknown domain or
            // space never created): nothing to activate.
            Err(_) => Err(DispatchError::AsidActivateFail),
        }
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain(value: u16) -> DomainId {
        DomainId::new(value).unwrap()
    }

    fn dispatch_cap() -> Capability {
        Capability {
            object: aether_types::ObjectId(1),
            object_type: ObjectType::Memory,
            rights: Rights::MAP,
            bounds: aether_types::Bounds::new(0, 4096).unwrap(),
            flags: CapabilityFlags::BOUND,
            expires_at_ns: 0,
        }
    }

    fn request(thread_id: u64, hart: u64, dom: DomainId, now: u64) -> DispatchRequest {
        DispatchRequest {
            thread_id,
            capability: dispatch_cap(),
            hart_id: hart,
            domain: dom,
            now_ns: now,
        }
    }

    #[test]
    fn dispatch_admits_authorized_thread() {
        let mut disp = Dispatcher::new();
        disp.register_domain(domain(1), None, 1_000).unwrap();
        let id = disp
            .dispatch(request(1, 1, domain(1), 2_000))
            .expect("authorized dispatch should succeed");
        assert_eq!(disp.state(id), Some(DispatchState::Execute));
        assert_eq!(
            disp.hart_state(1),
            Some(HartState::Running {
                dispatch_id: id,
                domain: domain(1)
            })
        );
        assert_eq!(disp.event_count(), 1);
        assert_eq!(disp.events()[0].reason, DispatchReason::Dispatched);
    }

    #[test]
    fn dispatch_without_map_capability_is_rejected() {
        let mut disp = Dispatcher::new();
        disp.register_domain(domain(1), None, 1_000).unwrap();
        let mut cap = dispatch_cap();
        cap.flags = CapabilityFlags::NONE; // not BOUND
        let req = DispatchRequest {
            thread_id: 1,
            capability: cap,
            hart_id: 1,
            domain: domain(1),
            now_ns: 2_000,
        };
        assert_eq!(disp.dispatch(req), Err(DispatchError::Unauthorized));
        assert_eq!(disp.len(), 0);
        // Rejected dispatch leaves no observable state.
        assert_eq!(disp.state(DispatchId(1)), None);
    }

    #[test]
    fn expired_capability_is_rejected() {
        let mut disp = Dispatcher::new();
        disp.register_domain(domain(1), None, 1_000).unwrap();
        let mut cap = dispatch_cap();
        cap.expires_at_ns = 1_500;
        let req = DispatchRequest {
            thread_id: 1,
            capability: cap,
            hart_id: 1,
            domain: domain(1),
            now_ns: 1_600,
        };
        assert_eq!(disp.dispatch(req), Err(DispatchError::Expired));
    }

    #[test]
    fn domain_mismatch_on_occupied_hart_is_rejected() {
        let mut disp = Dispatcher::new();
        disp.register_domain(domain(1), None, 1_000).unwrap();
        disp.register_domain(domain(2), None, 1_000).unwrap();
        // hart 1 runs domain 1.
        disp.dispatch(request(1, 1, domain(1), 2_000)).unwrap();
        // dispatching domain 2 to the same hart while domain 1 runs is a
        // domain mismatch (RFC-0008.3).
        assert_eq!(
            disp.dispatch(request(2, 1, domain(2), 3_000)),
            Err(DispatchError::DomainMismatch)
        );
        // hart 1 still runs domain 1.
        assert_eq!(
            disp.hart_state(1),
            Some(HartState::Running {
                dispatch_id: DispatchId(1),
                domain: domain(1)
            })
        );
    }

    #[test]
    fn unknown_domain_cannot_be_activated() {
        let mut disp = Dispatcher::new();
        assert_eq!(
            disp.dispatch(request(1, 1, domain(9), 2_000)),
            Err(DispatchError::AsidActivateFail)
        );
    }

    #[test]
    fn migrate_reruns_verification_on_target() {
        let mut disp = Dispatcher::new();
        disp.register_domain(domain(1), None, 1_000).unwrap();
        let id = disp
            .dispatch(request(1, 1, domain(1), 2_000))
            .expect("initial dispatch should succeed");
        // Migrate to an idle hart running the same domain: allowed.
        disp.migrate(id, 2, 3_000)
            .expect("migration should succeed");
        assert_eq!(
            disp.hart_state(2),
            Some(HartState::Running {
                dispatch_id: id,
                domain: domain(1)
            })
        );
        // A migration event was emitted (RFC-0031.17).
        assert!(
            disp.events()
                .iter()
                .any(|e| e.reason == DispatchReason::Migrated)
        );
    }

    #[test]
    fn preempt_emits_thread_preempted_event() {
        let mut disp = Dispatcher::new();
        disp.register_domain(domain(1), None, 1_000).unwrap();
        let id = disp
            .dispatch(request(1, 1, domain(1), 2_000))
            .expect("dispatch should succeed");
        disp.preempt(id, 3_000).expect("preemption should succeed");
        assert_eq!(disp.state(id), Some(DispatchState::Suspend));
        assert_eq!(disp.hart_state(1), Some(HartState::Idle));
        assert!(
            disp.events()
                .iter()
                .any(|e| e.reason == DispatchReason::Preempted)
        );
    }

    #[test]
    fn idle_and_resume_cycle() {
        let mut disp = Dispatcher::new();
        disp.idle(1, 1_000).expect("idle should succeed");
        assert_eq!(disp.hart_state(1), Some(HartState::Idle));
        assert!(
            disp.events()
                .iter()
                .any(|e| e.reason == DispatchReason::IdleEntered)
        );
        disp.resume(1, 2_000).expect("resume should succeed");
        assert!(
            disp.events()
                .iter()
                .any(|e| e.reason == DispatchReason::HartResumed)
        );
    }

    #[test]
    fn dispatch_bound_is_documented() {
        // RFC-0008.5: the bound is declared and verifiable.
        let bound = Dispatcher::dispatch_bound_ns();
        assert!(bound > 0);
        assert_eq!(bound, 10_000);
    }
}
