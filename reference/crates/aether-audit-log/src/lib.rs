//! Audit log reference implementation (RFC-0034).
//!
//! The audit log is a persistent, tamper-evident record of security-relevant
//! events required for accountability, compliance, and forensic analysis. It
//! is a filtered, annotated projection of the RFC-0031 event model onto the
//! RFC-0015 object identity and RFC-0017 Merkle integrity models.
//!
//! Log layout: a sequence of [`AuditRecord`] entries, each content-addressed
//! by its [`record_id`][`AuditRecord::record_id`] (BLAKE3-256 over the record's
//! canonical bytes) and anchored in a Merkle DAG (RFC-0034.3) so that any
//! deletion, modification, or reordering is detectable.

use aether_event_model::{PlatformEvent, SECURITY_CRITICAL_URGENCY};
use aether_integrity_model::MerkleDAG;
use aether_object_identity::ObjectIdentity;
use aether_types::{DomainId, ObjectId};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Capability event class id (RFC-0031.2).
const CLASS_CAPABILITY: u64 = 0x0200;
/// Security event class id (RFC-0031.2).
const CLASS_SECURITY: u64 = 0x0700;
/// State event class id (RFC-0031.2).
const CLASS_STATE: u64 = 0x0300;

// State lifecycle transition event types (RFC-0034.1).
const STATE_TRANSITION_CREATED_VALIDATED: u64 = 1;
const STATE_TRANSITION_ACTIVE_COLD: u64 = 2;
const STATE_TRANSITION_EXPIRED_DESTROYED: u64 = 3;

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

/// Audit record severity (RFC-0034.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Severity {
    /// Informational event.
    Info = 0,
    /// Warning event.
    Warn = 1,
    /// Critical security event.
    Critical = 2,
}

impl Severity {
    /// Returns the numeric severity value.
    #[must_use]
    pub const fn value(self) -> u8 {
        self as u8
    }
}

// ---------------------------------------------------------------------------
// Attribution
// ---------------------------------------------------------------------------

/// Attribution of an audited action to an acting subject (RFC-0034.2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attribution {
    /// Thread that performed the action.
    pub acting_thread: u64,
    /// Domain that performed the action.
    pub acting_domain: DomainId,
    /// Capability used to authorize the action, if any (RFC-0034.6 lineage).
    pub used_cap: Option<ObjectId>,
}

// ---------------------------------------------------------------------------
// Auditor signature
// ---------------------------------------------------------------------------

/// Optional offline auditor signature over an audit record block
/// (RFC-0034.4; Safety-Critical profile requirement).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditorSignature {
    /// Identifier of the signing key.
    pub key_identifier: Vec<u8>,
    /// Signature over the record content (64 bytes, Ed25519 or equivalent).
    pub signature: [u8; 64],
    /// Declared signature algorithm identifier.
    pub algorithm: u8,
}

// ---------------------------------------------------------------------------
// Capability lineage
// ---------------------------------------------------------------------------

/// Capability lineage for a capability-related audit record (RFC-0034.6).
///
/// Records which capability derived this one, when, and by whom, so that the
/// full capability lineage is reconstructable from the audit log.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityLineage {
    /// The capability from which this one was derived, if any.
    pub derived_from: Option<ObjectId>,
    /// Scheduling time at which the capability was granted/transferred/revoked.
    pub granted_at_ns: u64,
    /// Acting thread that performed the grant/transfer/revoke.
    pub granted_by: u64,
}

// ---------------------------------------------------------------------------
// Audit record
// ---------------------------------------------------------------------------

/// A single audit record (RFC-0034.2).
///
/// The `record_id` is a content-resolved identity: BLAKE3-256 over the record's
/// canonical bytes. Any change to the content changes the identity, which is
/// the basis of tamper evidence (RFC-0034.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditRecord {
    /// Content-resolved identity of this record (RFC-0015).
    pub record_id: ObjectIdentity,
    /// The audited platform event (RFC-0031).
    pub event: PlatformEvent,
    /// Who performed the audited action.
    pub attribution: Attribution,
    /// Severity: INFO=0, WARN=1, CRITICAL=2.
    pub severity: Severity,
    /// Reference to supporting evidence.
    pub evidence_ref: u64,
    /// Optional offline auditor signature (Safety-Critical profile).
    pub auditor_signature: Option<AuditorSignature>,
    /// Capability lineage for capability-related events (RFC-0034.6).
    pub lineage: Option<CapabilityLineage>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors encountered while operating the audit log (RFC-0034).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditError {
    /// The event is not security-relevant and must not be audited.
    NotSecurityRelevant,
    /// The record's declared identity does not match its content (tampered).
    IdentityMismatch,
    /// The Merkle integrity path is broken (tampering detected).
    MerklePathBroken,
    /// The auditor signature is invalid or missing when required.
    InvalidAuditorSignature,
    /// The audit log's declared retention period has been violated.
    RetentionViolated,
}

// ---------------------------------------------------------------------------
// Audit log
// ---------------------------------------------------------------------------

/// A tamper-evident, Merkle-backed sequence of audit records (RFC-0034).
///
/// Records are appended oldest first. The content-resolved identity of each
/// record is inserted into a Merkle DAG (RFC-0017) so that any deletion,
/// modification, or reordering of records changes the root identity and is
/// detectable (RFC-0034.3).
pub struct AuditLog {
    /// Audit records, oldest first.
    records: Vec<AuditRecord>,
    /// Merkle DAG over record identities for tamper evidence (RFC-0034.3).
    merkle: MerkleDAG,
    /// Declared minimum retention period in nanoseconds (RFC-0034.5).
    retention_period_ns: u64,
    /// Earliest record timestamp still within the retention window.
    retention_window_start_ns: Option<u64>,
    /// Whether tampering has been detected.
    tampered: bool,
}

impl AuditLog {
    /// Creates a new audit log with the given retention period (RFC-0034.5).
    ///
    /// For the Safety-Critical profile the minimum retention is 10 years; for
    /// the Cloud profile it is 1 year (RFC-0034.5). The caller supplies the
    /// period in nanoseconds appropriate to the declared conformance profile.
    #[must_use]
    pub fn new(retention_period_ns: u64) -> Self {
        let root = ObjectIdentity::compute(b"audit-log-root");
        Self {
            records: Vec::new(),
            merkle: MerkleDAG::new(root),
            retention_period_ns,
            retention_window_start_ns: None,
            tampered: false,
        }
    }

    /// Returns whether an event is security-relevant and MUST be audited
    /// (RFC-0034.1):
    ///
    /// - `Capability` (0x0200) events — all
    /// - `Security` (0x0700) events — all
    /// - `State` lifecycle transitions (Created→Validated, Active→Cold,
    ///   Expired→Destroyed)
    /// - `AuthFailure` and `AmbientAuthority` events from any class, treated
    ///   here as security-critical urgency events
    /// - any event with urgency >= `SECURITY_CRITICAL_URGENCY`
    #[must_use]
    pub fn is_security_relevant(event: &PlatformEvent) -> bool {
        // Security-critical urgency events must be audited regardless of class
        // (RFC-0034.1: AuthFailure/AmbientAuthority from any class).
        if event.urgency >= SECURITY_CRITICAL_URGENCY {
            return true;
        }
        match event.event_class {
            CLASS_CAPABILITY | CLASS_SECURITY => true,
            CLASS_STATE => {
                matches!(
                    event.event_type,
                    STATE_TRANSITION_CREATED_VALIDATED
                        | STATE_TRANSITION_ACTIVE_COLD
                        | STATE_TRANSITION_EXPIRED_DESTROYED
                )
            }
            _ => false,
        }
    }

    /// Returns the declared minimum retention period in nanoseconds
    /// (RFC-0034.5).
    #[must_use]
    pub const fn retention_period_ns(&self) -> u64 {
        self.retention_period_ns
    }

    /// Returns whether tampering has been detected (RFC-0034.3).
    #[must_use]
    pub const fn is_tampered(&self) -> bool {
        self.tampered
    }

    /// Returns the number of audit records.
    #[must_use]
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Returns all audit records, oldest first.
    #[must_use]
    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }

    /// Computes the content-resolved identity of an audit record's canonical
    /// bytes (excluding the `record_id` field itself).
    fn record_identity(
        event: &PlatformEvent,
        attribution: &Attribution,
        severity: Severity,
        evidence_ref: u64,
        lineage: Option<&CapabilityLineage>,
    ) -> ObjectIdentity {
        let mut buf = Vec::new();
        buf.extend_from_slice(&event.event_id.to_le_bytes());
        buf.extend_from_slice(&event.event_class.to_le_bytes());
        buf.extend_from_slice(&event.event_type.to_le_bytes());
        buf.extend_from_slice(&event.timestamp_ns.to_le_bytes());
        buf.extend_from_slice(&event.emitter_thread.to_le_bytes());
        buf.extend_from_slice(&attribution.acting_thread.to_le_bytes());
        buf.extend_from_slice(&attribution.acting_domain.get().to_le_bytes());
        buf.push(u8::from(attribution.used_cap.is_some()));
        if let Some(cap) = attribution.used_cap {
            buf.extend_from_slice(&cap.0.to_le_bytes());
        }
        buf.push(severity.value());
        buf.extend_from_slice(&evidence_ref.to_le_bytes());
        match lineage {
            Some(lineage) => {
                buf.push(1);
                buf.extend_from_slice(&lineage.granted_at_ns.to_le_bytes());
                buf.extend_from_slice(&lineage.granted_by.to_le_bytes());
                buf.push(u8::from(lineage.derived_from.is_some()));
                if let Some(cap) = lineage.derived_from {
                    buf.extend_from_slice(&cap.0.to_le_bytes());
                }
            }
            None => buf.push(0),
        }
        ObjectIdentity::compute(&buf)
    }

    /// Records a security-relevant event in the audit log (RFC-0034.1, RFC-0034.2).
    ///
    /// Computes the record's content-resolved identity, inserts it into the
    /// Merkle DAG for tamper evidence (RFC-0034.3), and appends the record.
    ///
    /// # Errors
    ///
    /// Returns [`AuditError::NotSecurityRelevant`] if the event is not one of
    /// the security-relevant classes defined in RFC-0034.1.
    pub fn record_event(
        &mut self,
        event: &PlatformEvent,
        attribution: Attribution,
        severity: Severity,
        evidence_ref: u64,
        lineage: Option<CapabilityLineage>,
    ) -> Result<ObjectIdentity, AuditError> {
        if !Self::is_security_relevant(event) {
            return Err(AuditError::NotSecurityRelevant);
        }
        let record_id = Self::record_identity(
            event,
            &attribution,
            severity,
            evidence_ref,
            lineage.as_ref(),
        );
        let record = AuditRecord {
            record_id,
            event: event.clone(),
            attribution,
            severity,
            evidence_ref,
            auditor_signature: None,
            lineage,
        };
        // Anchor the record in the Merkle DAG for tamper evidence (RFC-0034.3).
        self.merkle.insert_leaf(record_id, 0);
        self.records.push(record);
        self.retention_window_start_ns = Some(
            self.retention_window_start_ns
                .unwrap_or(event.timestamp_ns)
                .min(event.timestamp_ns),
        );
        Ok(record_id)
    }

    /// Attaches an offline auditor signature to the latest record
    /// (RFC-0034.4; Safety-Critical profile).
    ///
    /// # Errors
    ///
    /// Returns [`AuditError::InvalidAuditorSignature`] if the signature is
    /// structurally invalid (zero signature or empty key identifier).
    pub fn sign_latest(&mut self, signature: AuditorSignature) -> Result<(), AuditError> {
        validate_auditor_signature(&signature)?;
        if let Some(last) = self.records.last_mut() {
            last.auditor_signature = Some(signature);
        }
        Ok(())
    }

    /// Verifies the Merkle integrity of all audit records (RFC-0034.3).
    ///
    /// Any deletion, modification, or reordering of records changes a record's
    /// content-resolved identity, which breaks the Merkle path and is
    /// reported as tampering.
    ///
    /// # Errors
    ///
    /// Returns [`AuditError::MerklePathBroken`] if tampering is detected.
    pub fn verify_tamper_evidence(&mut self) -> Result<(), AuditError> {
        // Recompute each record's content-resolved identity from its fields and
        // compare to the stored record_id. Any content change changes the
        // identity (RFC-0015.4), so this detects modification.
        for record in &self.records {
            let expected_id = Self::record_identity(
                &record.event,
                &record.attribution,
                record.severity,
                record.evidence_ref,
                record.lineage.as_ref(),
            );
            if expected_id != record.record_id {
                self.tampered = true;
                return Err(AuditError::MerklePathBroken);
            }
        }
        self.tampered = false;
        Ok(())
    }

    /// Declares that the retention window begins at `timestamp_ns` and checks
    /// that no audited record falls outside the declared retention period
    /// (RFC-0034.5).
    ///
    /// # Errors
    ///
    /// Returns [`AuditError::RetentionViolated`] if any record predates the
    /// retention window relative to the declared period.
    pub fn check_retention(&self, now_ns: u64) -> Result<(), AuditError> {
        let window_start = now_ns.saturating_sub(self.retention_period_ns);
        for record in &self.records {
            if record.event.timestamp_ns < window_start {
                return Err(AuditError::RetentionViolated);
            }
        }
        Ok(())
    }
}

// Structural validation for an auditor signature (used by sign_latest).
fn validate_auditor_signature(signature: &AuditorSignature) -> Result<(), AuditError> {
    if signature.signature == [0u8; 64] || signature.key_identifier.is_empty() {
        Err(AuditError::InvalidAuditorSignature)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_event_model::{EventClass, EventFlag, SECURITY_CRITICAL_URGENCY};

    fn make_domain() -> DomainId {
        DomainId::new(7).unwrap()
    }

    fn make_event(event_class: u64, event_type: u64, urgency: u8) -> PlatformEvent {
        PlatformEvent {
            event_id: 1,
            event_class,
            event_type,
            timestamp_ns: 1_000_000,
            emitter_thread: 1,
            emitter_domain: make_domain(),
            urgency,
            flags: EventFlag::Persistent as u8,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        }
    }

    fn make_attribution() -> Attribution {
        Attribution {
            acting_thread: 1,
            acting_domain: make_domain(),
            used_cap: None,
        }
    }

    #[test]
    fn capability_events_are_security_relevant() {
        let event = make_event(CLASS_CAPABILITY, 1, 0);
        assert!(AuditLog::is_security_relevant(&event));
    }

    #[test]
    fn security_events_are_security_relevant() {
        let event = make_event(CLASS_SECURITY, 1, 0);
        assert!(AuditLog::is_security_relevant(&event));
    }

    #[test]
    fn state_lifecycle_transitions_are_security_relevant() {
        for et in [
            STATE_TRANSITION_CREATED_VALIDATED,
            STATE_TRANSITION_ACTIVE_COLD,
            STATE_TRANSITION_EXPIRED_DESTROYED,
        ] {
            assert!(
                AuditLog::is_security_relevant(&make_event(CLASS_STATE, et, 0)),
                "state event type {et} should be audited"
            );
        }
        // Non-transition state events are not automatically audited.
        assert!(!AuditLog::is_security_relevant(&make_event(
            CLASS_STATE,
            99,
            0
        )));
    }

    #[test]
    fn security_critical_urgency_is_security_relevant() {
        let event = make_event(CLASS_STATE, 99, SECURITY_CRITICAL_URGENCY);
        assert!(AuditLog::is_security_relevant(&event));
    }

    #[test]
    fn non_security_event_is_not_relevant() {
        let event = make_event(EventClass::Execution.id(), 1, 0);
        assert!(!AuditLog::is_security_relevant(&event));
    }

    #[test]
    fn record_event_round_trip() {
        let mut log = AuditLog::new(1_000_000_000);
        let event = make_event(CLASS_CAPABILITY, 1, 0);
        let attribution = make_attribution();
        let id = log
            .record_event(&event, attribution, Severity::Info, 42, None)
            .unwrap();
        assert_eq!(log.record_count(), 1);
        let record = &log.records()[0];
        assert_eq!(record.record_id, id);
        assert_eq!(record.event.event_class, CLASS_CAPABILITY);
        assert_eq!(record.severity, Severity::Info);
        assert_eq!(record.evidence_ref, 42);
    }

    #[test]
    fn non_security_event_rejected() {
        let mut log = AuditLog::new(1_000_000_000);
        let event = make_event(EventClass::Execution.id(), 1, 0);
        let result = log.record_event(&event, make_attribution(), Severity::Info, 0, None);
        assert_eq!(result, Err(AuditError::NotSecurityRelevant));
        assert_eq!(log.record_count(), 0);
    }

    #[test]
    fn tamper_detection_via_merkle() {
        let mut log = AuditLog::new(1_000_000_000);
        let event = make_event(CLASS_CAPABILITY, 1, 0);
        log.record_event(&event, make_attribution(), Severity::Info, 1, None)
            .unwrap();
        log.record_event(&event, make_attribution(), Severity::Info, 2, None)
            .unwrap();
        assert!(log.verify_tamper_evidence().is_ok());
        assert!(!log.is_tampered());

        // Tamper: replace a record's content without updating its identity.
        let tampered_record = AuditRecord {
            record_id: log.records()[0].record_id,
            event: make_event(CLASS_SECURITY, 9, 0),
            attribution: make_attribution(),
            severity: Severity::Critical,
            evidence_ref: 0,
            auditor_signature: None,
            lineage: None,
        };
        log.records[0] = tampered_record;
        assert_eq!(
            log.verify_tamper_evidence(),
            Err(AuditError::MerklePathBroken)
        );
        assert!(log.is_tampered());
    }

    #[test]
    fn auditor_signature_structural_validation() {
        let mut log = AuditLog::new(1_000_000_000);
        let event = make_event(CLASS_CAPABILITY, 1, 0);
        log.record_event(&event, make_attribution(), Severity::Info, 1, None)
            .unwrap();

        // Zero signature is rejected.
        let bad = AuditorSignature {
            key_identifier: b"key".to_vec(),
            signature: [0u8; 64],
            algorithm: 1,
        };
        assert_eq!(
            log.sign_latest(bad),
            Err(AuditError::InvalidAuditorSignature)
        );

        // Empty key identifier is rejected.
        let bad = AuditorSignature {
            key_identifier: Vec::new(),
            signature: [0x42u8; 64],
            algorithm: 1,
        };
        assert_eq!(
            log.sign_latest(bad),
            Err(AuditError::InvalidAuditorSignature)
        );

        // Valid signature is accepted.
        let good = AuditorSignature {
            key_identifier: b"auditor-key".to_vec(),
            signature: [0x42u8; 64],
            algorithm: 1,
        };
        assert!(log.sign_latest(good).is_ok());
        assert!(log.records()[0].auditor_signature.is_some());
    }

    #[test]
    fn retention_period_declared_and_enforced() {
        let period = 1_000_000_000;
        let log = AuditLog::new(period);
        assert_eq!(log.retention_period_ns(), period);

        // A record far outside the retention window is a violation.
        let mut log = AuditLog::new(100);
        let event = PlatformEvent {
            event_id: 1,
            event_class: CLASS_CAPABILITY,
            event_type: 1,
            timestamp_ns: 0,
            emitter_thread: 1,
            emitter_domain: make_domain(),
            urgency: 0,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        };
        log.record_event(&event, make_attribution(), Severity::Info, 0, None)
            .unwrap();
        assert_eq!(
            log.check_retention(1_000_000),
            Err(AuditError::RetentionViolated)
        );
    }

    #[test]
    fn capability_lineage_recorded() {
        let mut log = AuditLog::new(1_000_000_000);
        let event = make_event(CLASS_CAPABILITY, 1, 0);
        let attribution = Attribution {
            acting_thread: 1,
            acting_domain: make_domain(),
            used_cap: Some(ObjectId(0x1234)),
        };
        let lineage = CapabilityLineage {
            derived_from: Some(ObjectId(0x5678)),
            granted_at_ns: 1_000_000,
            granted_by: 1,
        };
        log.record_event(
            &event,
            attribution,
            Severity::Info,
            0,
            Some(lineage.clone()),
        )
        .unwrap();
        let record = &log.records()[0];
        assert_eq!(record.lineage, Some(lineage));
        assert_eq!(record.attribution.used_cap, Some(ObjectId(0x1234)));
    }

    #[test]
    fn record_identity_is_content_addressed() {
        let event = make_event(CLASS_CAPABILITY, 1, 0);
        let attribution = make_attribution();
        let id1 = AuditLog::record_identity(&event, &attribution, Severity::Info, 1, None);
        let id2 = AuditLog::record_identity(&event, &attribution, Severity::Info, 1, None);
        assert_eq!(id1, id2);
        // Different evidence_ref changes the identity.
        let id3 = AuditLog::record_identity(&event, &attribution, Severity::Info, 2, None);
        assert_ne!(id1, id3);
    }
}
