//! Lifecycle finite state machine reference implementation (RFC-0018).
//!
//! Every persistent object traverses a mandatory, forward-only lifecycle:
//! `Created → Validated → Referenced → Active → Cold → Archived → Expired → Destroyed`.
//! Transitions are gated by declared preconditions (RFC-0018.6), emit RFC-0031
//! events (RFC-0018.10), the current state of every object is queryable by its
//! identity (RFC-0018.11), and automatic tiering transitions are blocked during
//! a Safety-Critical hazard period (RFC-0018.13).
//!
//! Precondition evidence (integrity verification per RFC-0017, identity
//! assignment per RFC-0015) is supplied by the caller at each transition so the
//! FSM stays decoupled from the concrete integrity and identity implementations.

use aether_event_model::{EventClass, EventEmitter, PlatformEvent, SECURITY_CRITICAL_URGENCY};
use aether_object_identity::ObjectIdentity;
use aether_types::DomainId;
use core::fmt;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// States (RFC-0018.1)
// ---------------------------------------------------------------------------

/// Lifecycle state of a persistent object (RFC-0018.1).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum LifecycleState {
    /// Object instantiated; content being formed; not yet valid.
    Created = 0,
    /// Object content verified (RFC-0017); eligible for commit.
    Validated = 1,
    /// Object is reachable through at least one named reference.
    Referenced = 2,
    /// Object is actively serving reads and writes.
    Active = 3,
    /// Object access frequency has dropped below the warm threshold.
    Cold = 4,
    /// Object explicitly moved to archive storage tier.
    Archived = 5,
    /// A declared retention period has elapsed; awaiting cleanup.
    Expired = 6,
    /// Object explicitly deleted; identity preserved in history only.
    Destroyed = 7,
}

impl LifecycleState {
    /// Returns the numeric discriminant (RFC-0018 data structures).
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Returns whether this is a terminal state (no further transitions).
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, LifecycleState::Destroyed)
    }
}

impl fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleState::Created => write!(f, "Created"),
            LifecycleState::Validated => write!(f, "Validated"),
            LifecycleState::Referenced => write!(f, "Referenced"),
            LifecycleState::Active => write!(f, "Active"),
            LifecycleState::Cold => write!(f, "Cold"),
            LifecycleState::Archived => write!(f, "Archived"),
            LifecycleState::Expired => write!(f, "Expired"),
            LifecycleState::Destroyed => write!(f, "Destroyed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Transition trigger (RFC-0018.10)
// ---------------------------------------------------------------------------

/// What initiated a transition (RFC-0018 data structures: `trigger`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum TransitionTrigger {
    /// Threshold crossed (Active→Cold, Cold→Archived; RFC-0018.4).
    Automatic = 0,
    /// Policy-initiated (compulsory deletion, expiry; RFC-0018.6).
    Policy = 1,
    /// Explicit request (first active operation, delete; RFC-0018.5/.6).
    Explicit = 2,
}

// ---------------------------------------------------------------------------
// Transition outcome (RFC-0018.6)
// ---------------------------------------------------------------------------

/// Why a transition was rejected or recorded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TransitionReason {
    /// The transition succeeded.
    Success = 0,
    /// A declared precondition (integrity or identity) was not satisfied.
    PreconditionNotMet = 1,
    /// The transition is not a permitted forward step (or an illegal backward step).
    BackwardTransitionNotPermitted = 2,
    /// The object is in the terminal `Destroyed` state.
    TerminalState = 3,
    /// Blocked during a declared Safety-Critical hazard period (RFC-0018.13).
    HazardPeriodBlocked = 4,
    /// The object identity is not registered with this machine.
    UnknownObject = 5,
    /// The object is already in the requested state.
    AlreadyInTargetState = 6,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Rejection reason for a refused lifecycle transition (RFC-0018.6, RFC-0018.9).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionError {
    /// The object is already in the requested state.
    AlreadyInTargetState,
    /// A declared precondition is not satisfied (integrity / identity).
    PreconditionNotMet,
    /// The transition is not a permitted forward step (or an illegal backward step).
    BackwardTransitionNotPermitted,
    /// The object is in the terminal `Destroyed` state.
    TerminalState,
    /// Blocked during a declared Safety-Critical hazard period (RFC-0018.13).
    HazardPeriodBlocked,
    /// The object identity is not registered with this machine.
    UnknownObject,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransitionError::AlreadyInTargetState => write!(f, "already in target state"),
            TransitionError::PreconditionNotMet => write!(f, "precondition not met"),
            TransitionError::BackwardTransitionNotPermitted => {
                write!(f, "backward transition not permitted")
            }
            TransitionError::TerminalState => write!(f, "object is destroyed (terminal state)"),
            TransitionError::HazardPeriodBlocked => {
                write!(f, "transition blocked during hazard period")
            }
            TransitionError::UnknownObject => write!(f, "unknown object identity"),
        }
    }
}

impl std::error::Error for TransitionError {}

// ---------------------------------------------------------------------------
// Policy configuration (RFC-0018.12)
// ---------------------------------------------------------------------------

/// Threshold parameters governing automatic tiering transitions.
///
/// All fields are configurable per object, per storage class, or globally
/// (RFC-0018.12). These defaults document the out-of-the-box values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecyclePolicy {
    /// Accesses per second below which the object cools toward `Cold`.
    pub warm_threshold: u64,
    /// Seconds below `warm_threshold` before `Active → Cold` (RFC-0018.4).
    pub cold_cool_seconds: u64,
    /// Accesses per second below which the object is archived.
    pub archive_threshold: u64,
    /// Seconds below `archive_threshold` before `Cold → Archived` (RFC-0018.4).
    pub archive_wait_seconds: u64,
    /// Seconds in `Archived` before `Archived → Expired` (RFC-0018.6).
    pub retention_seconds: u64,
}

impl Default for LifecyclePolicy {
    fn default() -> Self {
        Self {
            warm_threshold: 10,
            cold_cool_seconds: 300,
            archive_threshold: 1,
            archive_wait_seconds: 3_600,
            retention_seconds: 2_592_000, // 30 days
        }
    }
}

// ---------------------------------------------------------------------------
// Transition record (RFC-0018 data structures)
// ---------------------------------------------------------------------------

/// Record of a single lifecycle state transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleTransitionRecord {
    /// The object whose state changed.
    pub object_id: ObjectIdentity,
    /// State before the transition.
    pub from_state: LifecycleState,
    /// State after the transition.
    pub to_state: LifecycleState,
    /// Scheduling time of the transition in nanoseconds (RFC-0012).
    pub timestamp_ns: u64,
    /// What initiated the transition (RFC-0018.10).
    pub trigger: TransitionTrigger,
    /// Outcome reason code (success or rejection reason).
    pub reason_code: TransitionReason,
}

// ---------------------------------------------------------------------------
// Event type codes emitted for lifecycle transitions
// ---------------------------------------------------------------------------

const EVT_CREATED_VALIDATED: u64 = 1;
const EVT_VALIDATED_REFERENCED: u64 = 2;
const EVT_REFERENCED_ACTIVE: u64 = 3;
const EVT_ACTIVE_COLD: u64 = 4;
const EVT_COLD_ARCHIVED: u64 = 5;
const EVT_ARCHIVED_EXPIRED: u64 = 6;
const EVT_EXPIRED_DESTROYED: u64 = 7;
const EVT_DESTROYED_COMPULSORY: u64 = 8;
const EVT_ACTIVE_CREATED_RECOVERY: u64 = 9;
const EVT_TRANSITION_REJECTED: u64 = 10;

/// The RFC-0031 event class used for all lifecycle transition events (State).
const LIFECYCLE_EVENT_CLASS: u64 = EventClass::State.id();

/// Domain that emits lifecycle events.
const LIFECYCLE_EMITTER_DOMAIN: u16 = 1;

/// Event type for a successful transition between two states.
fn event_type_for(from: LifecycleState, to: LifecycleState) -> u64 {
    match (from, to) {
        (LifecycleState::Created, LifecycleState::Validated) => EVT_CREATED_VALIDATED,
        (LifecycleState::Validated, LifecycleState::Referenced) => EVT_VALIDATED_REFERENCED,
        (LifecycleState::Referenced, LifecycleState::Active) => EVT_REFERENCED_ACTIVE,
        (LifecycleState::Active, LifecycleState::Cold) => EVT_ACTIVE_COLD,
        (LifecycleState::Cold, LifecycleState::Archived) => EVT_COLD_ARCHIVED,
        (LifecycleState::Archived, LifecycleState::Expired) => EVT_ARCHIVED_EXPIRED,
        (LifecycleState::Expired, LifecycleState::Destroyed) => EVT_EXPIRED_DESTROYED,
        (LifecycleState::Active, LifecycleState::Created) => EVT_ACTIVE_CREATED_RECOVERY,
        (_, LifecycleState::Destroyed) => EVT_DESTROYED_COMPULSORY,
        _ => EVT_TRANSITION_REJECTED,
    }
}

/// Serialize a transition record into an event payload (36 bytes).
fn transition_payload(
    object_id: &ObjectIdentity,
    from: LifecycleState,
    to: LifecycleState,
    trigger: TransitionTrigger,
    reason: TransitionReason,
) -> Vec<u8> {
    let mut p = object_id.bytes().to_vec(); // 32 bytes
    p.push(from as u8);
    p.push(to as u8);
    p.push(trigger as u8);
    p.push(reason as u8);
    p
}

// ---------------------------------------------------------------------------
// Lifecycle machine
// ---------------------------------------------------------------------------

/// Tracks the lifecycle state of persistent objects and enforces the
/// mandatory forward-only transition path (RFC-0018.3).
pub struct LifecycleMachine {
    /// Current state keyed by object identity.
    states: BTreeMap<ObjectIdentity, LifecycleState>,
    /// Per-object policy configuration (RFC-0018.12).
    policies: BTreeMap<ObjectIdentity, LifecyclePolicy>,
    /// Successful transitions, oldest first.
    records: Vec<LifecycleTransitionRecord>,
    /// Access count within the current state, used for automatic tiering.
    access_count: BTreeMap<ObjectIdentity, u64>,
    /// Scheduling time at which the object entered its current state.
    state_entered_ns: BTreeMap<ObjectIdentity, u64>,
    /// Whether a Safety-Critical hazard period is declared (RFC-0018.13).
    hazard_active: bool,
    /// Emits RFC-0031 events for every transition and rejection.
    emitter: EventEmitter,
}

impl Default for LifecycleMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl LifecycleMachine {
    /// Creates a machine with no registered objects.
    #[must_use]
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            policies: BTreeMap::new(),
            records: Vec::new(),
            access_count: BTreeMap::new(),
            state_entered_ns: BTreeMap::new(),
            hazard_active: false,
            emitter: EventEmitter::new(),
        }
    }

    /// Registers a new persistent object in the initial `Created` state
    /// (RFC-0018.2).
    pub fn register_object(&mut self, object_id: ObjectIdentity, policy: LifecyclePolicy) {
        self.states.insert(object_id, LifecycleState::Created);
        self.policies.insert(object_id, policy);
        self.access_count.insert(object_id, 0);
        self.state_entered_ns.insert(object_id, 0);
    }

    /// Returns the current lifecycle state of `object_id`, if registered
    /// (RFC-0018.11).
    #[must_use]
    pub fn state_of(&self, object_id: &ObjectIdentity) -> Option<LifecycleState> {
        self.states.get(object_id).copied()
    }

    /// Returns the policy configured for `object_id`, if any.
    #[must_use]
    pub fn policy_of(&self, object_id: &ObjectIdentity) -> Option<&LifecyclePolicy> {
        self.policies.get(object_id)
    }

    /// Returns whether a Safety-Critical hazard period is declared
    /// (RFC-0018.13).
    #[must_use]
    pub fn hazard_active(&self) -> bool {
        self.hazard_active
    }

    /// Declares or clears a Safety-Critical hazard period. While active,
    /// automatic `Active → Cold` and `Cold → Archived` transitions are blocked
    /// (RFC-0018.13).
    pub fn set_hazard(&mut self, active: bool) {
        self.hazard_active = active;
    }

    /// Returns all successful transition records, oldest first.
    #[must_use]
    pub fn transition_records(&self) -> &[LifecycleTransitionRecord] {
        &self.records
    }

    /// Returns the number of RFC-0031 events emitted (transitions and rejections).
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.emitter.event_count()
    }

    /// Returns all emitted RFC-0031 events in order.
    #[must_use]
    pub fn events(&self) -> &[PlatformEvent] {
        self.emitter.events()
    }

    // -----------------------------------------------------------------------
    // Core transition (RFC-0018.3, RFC-0018.6, RFC-0018.9)
    // -----------------------------------------------------------------------

    /// Attempts a lifecycle transition for `object_id`.
    ///
    /// `integrity_verified` supplies the RFC-0017 precondition for
    /// `Created → Validated`; `identity_assigned` supplies the RFC-0015
    /// precondition for `Validated → Referenced`. The caller is responsible
    /// for asserting "no held capabilities" before `Expired → Destroyed`
    /// (RFC-0018.6).
    ///
    /// # Errors
    ///
    /// Returns [`TransitionError`] when the object is unknown, already in the
    /// target state, in a terminal state, the transition violates the
    /// forward-only rule, a declared precondition is not satisfied, or the
    /// transition is blocked during a Safety-Critical hazard period.
    pub fn transition(
        &mut self,
        object_id: ObjectIdentity,
        to_state: LifecycleState,
        trigger: TransitionTrigger,
        timestamp_ns: u64,
        integrity_verified: bool,
        identity_assigned: bool,
    ) -> Result<LifecycleTransitionRecord, TransitionError> {
        let from = self
            .states
            .get(&object_id)
            .copied()
            .ok_or(TransitionError::UnknownObject)?;

        if from == to_state {
            let reason = TransitionReason::AlreadyInTargetState;
            self.emit_rejection(&object_id, from, to_state, trigger, timestamp_ns, reason);
            return Err(TransitionError::AlreadyInTargetState);
        }

        if from.is_terminal() {
            let reason = TransitionReason::TerminalState;
            self.emit_rejection(&object_id, from, to_state, trigger, timestamp_ns, reason);
            return Err(TransitionError::TerminalState);
        }

        // Forward-only: only the next state on the mandatory path, recovery
        // Active→Created, or any→Destroyed are permitted (RFC-0018.3, RFC-0018.8, RFC-0018.9).
        if !Self::is_permitted(from, to_state) {
            let reason = TransitionReason::BackwardTransitionNotPermitted;
            self.emit_rejection(&object_id, from, to_state, trigger, timestamp_ns, reason);
            return Err(TransitionError::BackwardTransitionNotPermitted);
        }

        // Declared preconditions (RFC-0018.6).
        let precondition_met = match (from, to_state) {
            (LifecycleState::Created, LifecycleState::Validated) => integrity_verified,
            (LifecycleState::Validated, LifecycleState::Referenced) => identity_assigned,
            _ => true,
        };
        if !precondition_met {
            let reason = TransitionReason::PreconditionNotMet;
            self.emit_rejection(&object_id, from, to_state, trigger, timestamp_ns, reason);
            return Err(TransitionError::PreconditionNotMet);
        }

        // Safety-Critical hazard blocking (RFC-0018.13).
        if self.hazard_active
            && matches!(
                (from, to_state),
                (LifecycleState::Active, LifecycleState::Cold)
                    | (LifecycleState::Cold, LifecycleState::Archived)
            )
        {
            let reason = TransitionReason::HazardPeriodBlocked;
            self.emit_rejection(&object_id, from, to_state, trigger, timestamp_ns, reason);
            return Err(TransitionError::HazardPeriodBlocked);
        }

        let record = LifecycleTransitionRecord {
            object_id,
            from_state: from,
            to_state,
            timestamp_ns,
            trigger,
            reason_code: TransitionReason::Success,
        };

        self.states.insert(object_id, to_state);
        self.access_count.insert(object_id, 0);
        self.state_entered_ns.insert(object_id, timestamp_ns);
        self.records.push(record.clone());

        self.emit_transition(&record);
        Ok(record)
    }

    /// Records an active read or write against `object_id`. The first active
    /// operation on a `Referenced` object promotes it to `Active`
    /// (RFC-0018.5). Access counts feed automatic tiering (RFC-0018.4).
    ///
    /// # Errors
    ///
    /// Returns [`TransitionError::UnknownObject`] if `object_id` is not
    /// registered, or [`TransitionError::TerminalState`] if the object is
    /// already `Destroyed`.
    ///
    /// # Panics
    ///
    /// Panics if `object_id` is registered in `states` but missing from
    /// `access_count` — an invariant violation, since both maps are populated
    /// together in [`LifecycleMachine::register_object`].
    pub fn record_access(
        &mut self,
        object_id: ObjectIdentity,
        timestamp_ns: u64,
    ) -> Result<(), TransitionError> {
        let from = self
            .states
            .get(&object_id)
            .copied()
            .ok_or(TransitionError::UnknownObject)?;

        if from.is_terminal() {
            return Err(TransitionError::TerminalState);
        }

        *self.access_count.get_mut(&object_id).unwrap() += 1;

        if from == LifecycleState::Referenced {
            return self
                .transition(
                    object_id,
                    LifecycleState::Active,
                    TransitionTrigger::Explicit,
                    timestamp_ns,
                    true,
                    true,
                )
                .map(|_| ());
        }

        Ok(())
    }

    /// Evaluates automatic tiering transitions for every registered object as
    /// of `timestamp_ns` (RFC-0018.4). `Active → Cold` and `Cold → Archived`
    /// are blocked while a hazard period is declared (RFC-0018.13).
    pub fn check_automatic_transitions(
        &mut self,
        timestamp_ns: u64,
    ) -> Vec<LifecycleTransitionRecord> {
        let mut applied = Vec::new();
        let ids: Vec<ObjectIdentity> = self.states.keys().copied().collect();

        for object_id in ids {
            let Some(policy) = self.policies.get(&object_id) else {
                continue;
            };
            let Some(from) = self.states.get(&object_id).copied() else {
                continue;
            };
            if from.is_terminal() {
                continue;
            }
            let entered = self.state_entered_ns.get(&object_id).copied().unwrap_or(0);
            let age = timestamp_ns.saturating_sub(entered);
            let count = self.access_count.get(&object_id).copied().unwrap_or(0);

            let to_state = match from {
                LifecycleState::Active
                    if count < policy.warm_threshold && age >= policy.cold_cool_seconds =>
                {
                    LifecycleState::Cold
                }
                LifecycleState::Cold
                    if count < policy.archive_threshold && age >= policy.archive_wait_seconds =>
                {
                    LifecycleState::Archived
                }
                LifecycleState::Archived if age >= policy.retention_seconds => {
                    LifecycleState::Expired
                }
                _ => continue,
            };

            // Safety-Critical hazard blocking (RFC-0018.13): automatic tiering
            // (Active→Cold, Cold→Archived) is blocked during a hazard period.
            let is_tiering = matches!(
                (from, to_state),
                (LifecycleState::Active, LifecycleState::Cold)
                    | (LifecycleState::Cold, LifecycleState::Archived)
            );
            if self.hazard_active && is_tiering {
                continue;
            }

            if let Ok(record) = self.transition(
                object_id,
                to_state,
                TransitionTrigger::Automatic,
                timestamp_ns,
                true,
                true,
            ) {
                applied.push(record);
            }
        }
        applied
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Only permitted forward steps on the mandatory path, plus recovery and
    /// compulsory destruction (RFC-0018.3, RFC-0018.8, RFC-0018.9).
    fn is_permitted(from: LifecycleState, to: LifecycleState) -> bool {
        let from_d = from as u8;
        let to_d = to as u8;
        // Immediate next state on the mandatory path.
        if to_d == from_d + 1 {
            return true;
        }
        // Recovery: Active → Created (the only permitted backward step).
        if from == LifecycleState::Active && to == LifecycleState::Created {
            return true;
        }
        // Compulsory or expiry-driven destruction from any non-terminal state.
        if to == LifecycleState::Destroyed {
            return true;
        }
        false
    }

    fn emit_transition(&mut self, record: &LifecycleTransitionRecord) {
        self.emitter.emit(PlatformEvent {
            event_id: 0,
            event_class: LIFECYCLE_EVENT_CLASS,
            event_type: event_type_for(record.from_state, record.to_state),
            timestamp_ns: record.timestamp_ns,
            emitter_thread: 0,
            emitter_domain: DomainId::new(LIFECYCLE_EMITTER_DOMAIN).unwrap(),
            urgency: SECURITY_CRITICAL_URGENCY,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: transition_payload(
                &record.object_id,
                record.from_state,
                record.to_state,
                record.trigger,
                record.reason_code,
            ),
        });
    }

    fn emit_rejection(
        &mut self,
        object_id: &ObjectIdentity,
        from: LifecycleState,
        to: LifecycleState,
        trigger: TransitionTrigger,
        timestamp_ns: u64,
        reason: TransitionReason,
    ) {
        self.emitter.emit(PlatformEvent {
            event_id: 0,
            event_class: LIFECYCLE_EVENT_CLASS,
            event_type: EVT_TRANSITION_REJECTED,
            timestamp_ns,
            emitter_thread: 0,
            emitter_domain: DomainId::new(LIFECYCLE_EMITTER_DOMAIN).unwrap(),
            urgency: SECURITY_CRITICAL_URGENCY,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: transition_payload(object_id, from, to, trigger, reason),
        });
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_id(bytes: [u8; 32]) -> ObjectIdentity {
        ObjectIdentity(bytes)
    }

    fn new_machine() -> LifecycleMachine {
        let mut m = LifecycleMachine::new();
        m.register_object(make_id([1u8; 32]), LifecyclePolicy::default());
        m
    }

    #[test]
    fn new_object_starts_in_created_state() {
        let m = new_machine();
        assert_eq!(
            m.state_of(&make_id([1u8; 32])),
            Some(LifecycleState::Created)
        );
    }

    #[test]
    fn mandatory_transition_path_is_forward_only() {
        let mut m = new_machine();
        let id = make_id([1u8; 32]);
        let states = [
            LifecycleState::Validated,
            LifecycleState::Referenced,
            LifecycleState::Active,
            LifecycleState::Cold,
            LifecycleState::Archived,
            LifecycleState::Expired,
            LifecycleState::Destroyed,
        ];
        let mut ts = 1_000_000u64;
        for (i, target) in states.into_iter().enumerate() {
            ts += 1_000_000;
            assert!(
                m.transition(id, target, TransitionTrigger::Explicit, ts, true, true,)
                    .is_ok()
            );
            assert_eq!(m.state_of(&id), Some(target));
            assert_eq!(i + 1, m.transition_records().len());
        }
    }

    #[test]
    fn created_to_validated_requires_integrity() {
        let mut m = new_machine();
        let id = make_id([1u8; 32]);
        assert!(
            m.transition(
                id,
                LifecycleState::Validated,
                TransitionTrigger::Explicit,
                1_000_000,
                false,
                true,
            )
            .is_err()
        );
        assert_eq!(m.state_of(&id), Some(LifecycleState::Created));
    }

    #[test]
    fn validated_to_referenced_requires_identity() {
        let mut m = new_machine();
        let id = make_id([1u8; 32]);
        m.transition(
            id,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .unwrap();
        // Reset identity precondition by re-registering? Instead test that
        // identity_assigned=false rejects.
        let id2 = make_id([2u8; 32]);
        m.register_object(id2, LifecyclePolicy::default());
        m.transition(
            id2,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .unwrap();
        assert!(
            m.transition(
                id2,
                LifecycleState::Referenced,
                TransitionTrigger::Explicit,
                2_000_000,
                true,
                false,
            )
            .is_err()
        );
        assert_eq!(m.state_of(&id2), Some(LifecycleState::Validated));
    }

    #[test]
    fn backward_transitions_rejected_except_active_to_created() {
        let mut m = new_machine();
        let id = make_id([1u8; 32]);
        m.transition(
            id,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .unwrap();
        m.transition(
            id,
            LifecycleState::Referenced,
            TransitionTrigger::Explicit,
            2_000_000,
            true,
            true,
        )
        .unwrap();
        m.transition(
            id,
            LifecycleState::Active,
            TransitionTrigger::Explicit,
            3_000_000,
            true,
            true,
        )
        .unwrap();
        // Backward Active → Validated is rejected.
        assert!(
            m.transition(
                id,
                LifecycleState::Validated,
                TransitionTrigger::Explicit,
                4_000_000,
                true,
                true,
            )
            .is_err()
        );
        // Recovery Active → Created is the only permitted backward step.
        assert!(
            m.transition(
                id,
                LifecycleState::Created,
                TransitionTrigger::Explicit,
                5_000_000,
                true,
                true,
            )
            .is_ok()
        );
        assert_eq!(m.state_of(&id), Some(LifecycleState::Created));
    }

    #[test]
    fn destroyed_is_terminal() {
        let mut m = new_machine();
        let id = make_id([1u8; 32]);
        m.transition(
            id,
            LifecycleState::Destroyed,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .unwrap();
        assert!(
            m.transition(
                id,
                LifecycleState::Expired,
                TransitionTrigger::Explicit,
                2_000_000,
                true,
                true,
            )
            .is_err()
        );
        assert_eq!(m.state_of(&id), Some(LifecycleState::Destroyed));
    }

    #[test]
    fn transition_events_emitted() {
        let mut m = new_machine();
        let id = make_id([1u8; 32]);
        m.transition(
            id,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .unwrap();
        assert_eq!(m.event_count(), 1);
        let event = m.events().last().unwrap();
        assert_eq!(event.event_class, LIFECYCLE_EVENT_CLASS);
        assert_eq!(event.event_type, EVT_CREATED_VALIDATED);
        assert_eq!(event.payload.len(), 36);
        assert_eq!(&event.payload[..32], id.as_ref());
    }

    #[test]
    fn rejected_transitions_are_recorded_as_events() {
        let mut m = new_machine();
        let id = make_id([1u8; 32]);
        // Destroyed → anything rejected; still emits a rejection event.
        m.transition(
            id,
            LifecycleState::Destroyed,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .unwrap();
        assert!(
            m.transition(
                id,
                LifecycleState::Expired,
                TransitionTrigger::Explicit,
                2_000_000,
                true,
                true,
            )
            .is_err()
        );
        assert_eq!(m.event_count(), 2);
        let last = m.events().last().unwrap();
        assert_eq!(last.event_type, EVT_TRANSITION_REJECTED);
    }

    #[test]
    fn hazard_blocks_automatic_tiering() {
        let mut m = new_machine();
        let id = make_id([1u8; 32]);
        // Reach Active.
        m.transition(
            id,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .unwrap();
        m.transition(
            id,
            LifecycleState::Referenced,
            TransitionTrigger::Explicit,
            2_000_000,
            true,
            true,
        )
        .unwrap();
        m.transition(
            id,
            LifecycleState::Active,
            TransitionTrigger::Explicit,
            3_000_000,
            true,
            true,
        )
        .unwrap();
        m.set_hazard(true);
        // Far future, well past cold_cool_seconds, zero accesses.
        let applied = m.check_automatic_transitions(10_000_000_000);
        assert!(applied.is_empty());
        assert_eq!(m.state_of(&id), Some(LifecycleState::Active));
        m.set_hazard(false);
        let applied = m.check_automatic_transitions(10_000_000_000);
        assert_eq!(applied.len(), 1);
        assert_eq!(m.state_of(&id), Some(LifecycleState::Cold));
    }

    #[test]
    fn record_access_promotes_referenced_to_active() {
        let mut m = new_machine();
        let id = make_id([1u8; 32]);
        m.transition(
            id,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .unwrap();
        m.transition(
            id,
            LifecycleState::Referenced,
            TransitionTrigger::Explicit,
            2_000_000,
            true,
            true,
        )
        .unwrap();
        assert_eq!(m.state_of(&id), Some(LifecycleState::Referenced));
        m.record_access(id, 3_000_000).unwrap();
        assert_eq!(m.state_of(&id), Some(LifecycleState::Active));
    }

    #[test]
    fn unknown_object_rejected() {
        let mut m = new_machine();
        let id = make_id([9u8; 32]);
        assert!(
            m.transition(
                id,
                LifecycleState::Validated,
                TransitionTrigger::Explicit,
                1_000_000,
                true,
                true,
            )
            .is_err()
        );
    }
}
