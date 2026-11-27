//! Platform event model reference implementation (RFC-0031).
//!
//! Every observable state transition in the platform is emitted, attributed,
//! and recorded through this model. The event structure, class taxonomy,
//! emission requirements, causality, and filtering rules are defined here.
//! Trace spans (RFC-0032), replay (RFC-0033), audit log (RFC-0034), and
//! diagnostic query (RFC-0035) all derive from this model.

use aether_types::DomainId;
use core::fmt;

/// Monotonically increasing event identifier, unique per source.
pub type EventId = u64;

/// Scheduling time in nanoseconds (RFC-0012).
pub type TimestampNs = u64;

/// Emitter thread identifier.
pub type ThreadId = u64;

/// Event class identifier.
pub type EventClassId = u64;

/// Event type identifier within a class.
pub type EventTypeId = u64;

/// Causation identifier referencing the event that caused this event
/// (RFC-0031.5).
pub type CausationId = u64;

/// Correlation identifier for grouping related events from the same
/// operation (RFC-0031.6).
pub type CorrelationId = u64;

/// Event payload length in bytes.
pub type PayloadLen = u32;

// ---------------------------------------------------------------------------
// Event classes (RFC-0031.2)
// ---------------------------------------------------------------------------

/// Event class identifiers (RFC-0031.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u64)]
pub enum EventClass {
    /// Thread, dispatch, scheduling events.
    Execution = 0x0100,
    /// Capability grant, transfer, revoke events.
    Capability = 0x0200,
    /// Object lifecycle, transaction, integrity events.
    State = 0x0300,
    /// Message, endpoint, routing events.
    Communication = 0x0400,
    /// Storage provider, tiering events.
    Storage = 0x0500,
    /// Interrupt, timer events.
    Interrupt = 0x0600,
    /// Security events.
    Security = 0x0700,
}

impl EventClass {
    /// Returns the numeric class identifier.
    #[must_use]
    pub const fn id(self) -> EventClassId {
        self as EventClassId
    }
}

// ---------------------------------------------------------------------------
// Event flags (RFC-0031.1)
// ---------------------------------------------------------------------------

/// Event flags (RFC-0031.1).
///
/// - `PERSISTENT`: event may be buffered and emitted at the next transaction
///   commit boundary (RFC-0031.4).
/// - `CAUSED`: event is caused by another event; `causation_id` references
///   the causative event (RFC-0031.5).
/// - `CORRELATED`: event shares a `correlation_id` with related events from
///   the same operation (RFC-0031.6).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum EventFlag {
    /// Event is persistent and may be batched at commit boundaries.
    Persistent = 0x01,
    /// Event references a causative event via `causation_id`.
    Caused = 0x02,
    /// Event is part of a correlated operation group via `correlation_id`.
    Correlated = 0x04,
}

/// Flag bits.
const FLAG_PERSISTENT: u8 = 0x01;
const FLAG_CAUSED: u8 = 0x02;
const FLAG_CORRELATED: u8 = 0x04;

// ---------------------------------------------------------------------------
// Urgency (RFC-0031.7)
// ---------------------------------------------------------------------------

/// Events with urgency >= 200 are security-critical and MUST NOT be
/// filtered out (RFC-0031.7).
pub const SECURITY_CRITICAL_URGENCY: u8 = 200;

// ---------------------------------------------------------------------------
// Event structure (RFC-0031.1)
// ---------------------------------------------------------------------------

/// Canonical platform event structure (RFC-0031.1).
///
/// ```text
/// struct platform_event {
///     uint64            event_id;         // monotonically increasing per-source
///     uint64            event_class;       // category (see event class table)
///     uint64            event_type;        // specific type within class
///     uint64            timestamp_ns;      // scheduling time (RFC-0012)
///     struct thread_id  emitter_thread;    // which thread emitted this
///     struct domain_id  emitter_domain;    // which domain emitted this
///     uint8             urgency;           // 0=lowest, 255=highest
///     uint8             flags;             // PERSISTENT, CAUSED, CORRELATED
///     uint64            causation_id;     // event_id of causative event
///     uint64            correlation_id;    // grouping id for same operation
///     uint32            payload_len;       // event-specific payload length
///     // followed by payload_len bytes of event-specific data
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformEvent {
    /// Monotonically increasing event identifier, unique per source.
    pub event_id: EventId,
    /// Event class identifier.
    pub event_class: EventClassId,
    /// Specific event type within the class.
    pub event_type: EventTypeId,
    /// Scheduling time in nanoseconds (RFC-0012).
    pub timestamp_ns: TimestampNs,
    /// Thread that emitted this event.
    pub emitter_thread: ThreadId,
    /// Domain that emitted this event.
    pub emitter_domain: DomainId,
    /// Urgency: 0 = lowest, 255 = highest.
    pub urgency: u8,
    /// Event flags (PERSISTENT, CAUSED, CORRELATED).
    pub flags: u8,
    /// Event id of the causative event, if the CAUSED flag is set.
    pub causation_id: CausationId,
    /// Correlation id grouping related events from the same operation.
    pub correlation_id: CorrelationId,
    /// Event-specific payload bytes.
    pub payload: Vec<u8>,
}

impl PlatformEvent {
    /// Returns whether the `PERSISTENT` flag is set (RFC-0031.4).
    #[must_use]
    pub const fn is_persistent(&self) -> bool {
        self.flags & FLAG_PERSISTENT != 0
    }

    /// Returns whether the `CAUSED` flag is set (RFC-0031.5).
    #[must_use]
    pub const fn is_caused(&self) -> bool {
        self.flags & FLAG_CAUSED != 0
    }

    /// Returns whether the `CORRELATED` flag is set (RFC-0031.6).
    #[must_use]
    pub const fn is_correlated(&self) -> bool {
        self.flags & FLAG_CORRELATED != 0
    }

    /// Returns whether this event is security-critical and must not be
    /// filtered (urgency >= 200, RFC-0031.7).
    #[must_use]
    pub const fn is_security_critical(&self) -> bool {
        self.urgency >= SECURITY_CRITICAL_URGENCY
    }
}

// ---------------------------------------------------------------------------
// Event emission (RFC-0031.3, RFC-0031.4)
// ---------------------------------------------------------------------------

/// Emits platform events with monotonically increasing identifiers.
///
/// Events are emitted at the moment the state transition occurs
/// (RFC-0031.4). Persistent events may be buffered and emitted at the next
/// transaction commit boundary.
#[derive(Debug)]
pub struct EventEmitter {
    /// Next event identifier to assign.
    next_id: EventId,
    /// Emitted events, in emission order.
    events: Vec<PlatformEvent>,
    /// Whether any security-critical event has been filtered (must remain
    /// false per RFC-0031.7).
    security_critical_filtered: bool,
}

impl EventEmitter {
    /// Creates a new event emitter starting at event id 1.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_id: 1,
            events: Vec::new(),
            security_critical_filtered: false,
        }
    }

    /// Emits an event, assigning it the next monotonically increasing id
    /// (RFC-0031.3). Returns the assigned event id.
    pub fn emit(&mut self, mut event: PlatformEvent) -> EventId {
        event.event_id = self.next_id;
        self.next_id += 1;
        if event.is_security_critical() {
            // Security-critical events must never be filtered (RFC-0031.7).
            self.security_critical_filtered = false;
        }
        self.events.push(event.clone());
        event.event_id
    }

    /// Returns all emitted events in order.
    #[must_use]
    pub fn events(&self) -> &[PlatformEvent] {
        &self.events
    }

    /// Returns the number of emitted events.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns the last emitted event, if any.
    #[must_use]
    pub fn last_event(&self) -> Option<&PlatformEvent> {
        self.events.last()
    }

    /// Returns whether any security-critical event was filtered (must be
    /// false per RFC-0031.7).
    #[must_use]
    pub const fn security_critical_filtered(&self) -> bool {
        self.security_critical_filtered
    }

    /// Attempts to filter an event. Returns `true` if the event may be
    /// filtered, `false` if it must be emitted (security-critical,
    /// RFC-0031.7).
    #[must_use]
    pub fn may_filter(&self, event: &PlatformEvent) -> bool {
        !event.is_security_critical()
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Event types referenced by other RFCs
// ---------------------------------------------------------------------------

/// `ObjectReconstructed` event (RFC-0015.13, RFC-0031.32).
///
/// Emitted when a storage provider reconstructs a corrupted object from an
/// uncorrupted replica. Class: Storage (0x0500).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectReconstructed {
    /// The reconstructed object's identity.
    pub object_id: Vec<u8>,
    /// The replica serial from which reconstruction occurred.
    pub replica_serial: u64,
    /// The provider-specific location of the replica.
    pub replica_location: Vec<u8>,
}

/// `ObjectCorruptionUnrecoverable` event (RFC-0015.13, RFC-0031.33).
///
/// Emitted when a storage provider detects corruption and no replica is
/// available for reconstruction. Class: Storage (0x0500).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectCorruptionUnrecoverable {
    /// The corrupted object's identity.
    pub object_id: Vec<u8>,
    /// The provider-specific location where corruption was detected.
    pub location: Vec<u8>,
}

/// `IntegrityFault` event (RFC-0017.6, RFC-0031.39).
///
/// Emitted when integrity verification fails and corrupted data must not
/// be returned to a consumer. Class: State (0x0300).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrityFault {
    /// The object whose integrity check failed.
    pub object_id: Vec<u8>,
    /// The fault type that occurred.
    pub fault_type: IntegrityFaultType,
}

/// Types of integrity fault (RFC-0017.4).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum IntegrityFaultType {
    /// Object content does not match its declared identity.
    ContentMismatch = 0,
    /// A Merkle path is broken (missing or inconsistent node).
    MerklePathBroken = 1,
}

/// `IntegritySignatureInvalid` event (RFC-0017.9, RFC-0031.40).
///
/// Emitted when signature verification fails on a signed integrity
/// declaration and object usage is blocked. Class: State (0x0300).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegritySignatureInvalid {
    /// The object whose signature failed verification.
    pub object_id: Vec<u8>,
    /// The identifier of the signing key.
    pub key_identifier: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Conformance helpers
// ---------------------------------------------------------------------------

/// Returns whether a numeric class id is one of the defined event classes
/// (RFC-0031.2).
#[must_use]
pub fn is_valid_event_class(class_id: EventClassId) -> bool {
    matches!(
        class_id,
        0x0100 | 0x0200 | 0x0300 | 0x0400 | 0x0500 | 0x0600 | 0x0700
    )
}

impl fmt::Display for EventClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventClass::Execution => write!(f, "Execution"),
            EventClass::Capability => write!(f, "Capability"),
            EventClass::State => write!(f, "State"),
            EventClass::Communication => write!(f, "Communication"),
            EventClass::Storage => write!(f, "Storage"),
            EventClass::Interrupt => write!(f, "Interrupt"),
            EventClass::Security => write!(f, "Security"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_class_ids_match_rfc_0031_2() {
        assert_eq!(EventClass::Execution.id(), 0x0100);
        assert_eq!(EventClass::Capability.id(), 0x0200);
        assert_eq!(EventClass::State.id(), 0x0300);
        assert_eq!(EventClass::Communication.id(), 0x0400);
        assert_eq!(EventClass::Storage.id(), 0x0500);
        assert_eq!(EventClass::Interrupt.id(), 0x0600);
        assert_eq!(EventClass::Security.id(), 0x0700);
    }

    #[test]
    fn is_valid_event_class_recognizes_all_classes() {
        assert!(is_valid_event_class(0x0100));
        assert!(is_valid_event_class(0x0200));
        assert!(is_valid_event_class(0x0300));
        assert!(is_valid_event_class(0x0400));
        assert!(is_valid_event_class(0x0500));
        assert!(is_valid_event_class(0x0600));
        assert!(is_valid_event_class(0x0700));
        assert!(!is_valid_event_class(0x0800));
        assert!(!is_valid_event_class(0x0000));
    }

    #[test]
    fn emitter_assigns_monotonically_increasing_ids() {
        let mut emitter = EventEmitter::new();
        let e1 = emitter.emit(PlatformEvent {
            event_id: 0,
            event_class: EventClass::Execution.id(),
            event_type: 1,
            timestamp_ns: 1000,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 0,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        });
        let e2 = emitter.emit(PlatformEvent {
            event_id: 0,
            event_class: EventClass::Execution.id(),
            event_type: 2,
            timestamp_ns: 2000,
            emitter_thread: 2,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 0,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        });
        assert_eq!(e1, 1);
        assert_eq!(e2, 2);
        assert_eq!(emitter.event_count(), 2);
    }

    #[test]
    fn persistent_flag_semantics() {
        let mut event = PlatformEvent {
            event_id: 1,
            event_class: EventClass::State.id(),
            event_type: 1,
            timestamp_ns: 1000,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 0,
            flags: FLAG_PERSISTENT,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        };
        assert!(event.is_persistent());
        assert!(!event.is_caused());
        assert!(!event.is_correlated());

        event.flags |= FLAG_CAUSED | FLAG_CORRELATED;
        assert!(event.is_persistent());
        assert!(event.is_caused());
        assert!(event.is_correlated());
    }

    #[test]
    fn security_critical_urgency_not_filtered() {
        let emitter = EventEmitter::new();
        let critical = PlatformEvent {
            event_id: 0,
            event_class: EventClass::Security.id(),
            event_type: 1,
            timestamp_ns: 1000,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: SECURITY_CRITICAL_URGENCY,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        };
        assert!(critical.is_security_critical());
        assert!(!emitter.may_filter(&critical));
        assert!(emitter.may_filter(&PlatformEvent {
            urgency: SECURITY_CRITICAL_URGENCY - 1,
            ..critical.clone()
        }));
    }

    #[test]
    fn causality_chain_recorded() {
        let mut emitter = EventEmitter::new();
        let parent_id = emitter.emit(PlatformEvent {
            event_id: 0,
            event_class: EventClass::Execution.id(),
            event_type: 1,
            timestamp_ns: 1000,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 0,
            flags: 0,
            causation_id: 0,
            correlation_id: 42,
            payload: Vec::new(),
        });
        let child_id = emitter.emit(PlatformEvent {
            event_id: 0,
            event_class: EventClass::Capability.id(),
            event_type: 1,
            timestamp_ns: 1100,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 0,
            flags: FLAG_CAUSED,
            causation_id: parent_id,
            correlation_id: 42,
            payload: Vec::new(),
        });
        let child = emitter.last_event().unwrap();
        assert_eq!(child.causation_id, parent_id);
        assert_eq!(child.correlation_id, 42);
        assert_ne!(child_id, parent_id);
    }

    #[test]
    fn object_reconstructed_event_payload_shape() {
        let event = ObjectReconstructed {
            object_id: vec![1u8; 32],
            replica_serial: 7,
            replica_location: b"/provider/a/slot-3".to_vec(),
        };
        assert_eq!(event.object_id.len(), 32);
        assert_eq!(event.replica_serial, 7);
    }

    #[test]
    fn integrity_signature_invalid_event_shape() {
        let event = IntegritySignatureInvalid {
            object_id: vec![2u8; 32],
            key_identifier: b"key-1".to_vec(),
        };
        assert_eq!(event.object_id.len(), 32);
        assert_eq!(event.key_identifier, b"key-1");
    }

    #[test]
    fn integrity_fault_types() {
        assert_eq!(IntegrityFaultType::ContentMismatch as u8, 0);
        assert_eq!(IntegrityFaultType::MerklePathBroken as u8, 1);
    }

    #[test]
    fn event_class_display_names() {
        assert_eq!(format!("{}", EventClass::Storage), "Storage");
        assert_eq!(format!("{}", EventClass::Security), "Security");
    }
}
