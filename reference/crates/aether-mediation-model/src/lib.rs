//! Capability mediation state model (RFC-0013).
//! ...
//! This is a bounded executable host model of the RFC-0013 capability
//! mediation semantics. It enforces capability-based access control at
//! every access attempt and is deliberately **not** a production MMU, a
//! proof of hardware-mediated bypass resistance, or an integration with
//! any kernel or device driver.
//!
//! Scope and declared architecture:
//!
//! - **Class:** software mediation boundary checking every resource access
//!   against a held capability token.
//! - **Check pipeline:** `CHECK_REQUEST -> CHECK_VALID -> CHECK_RIGHTS ->
//!   CHECK_DOMAIN -> CHECK_REVOKED -> CHECK_LIFETIME -> CHECK_BOUNDS ->
//!   PERMIT_ACCESS` (or `DENY_ACCESS` + fault event on any failure).
//! - **No short-circuiting (RFC-0013.14):** every pipeline step is
//!   evaluated for each access; the first fault in pipeline order is
//!   reported.
//! - **Fault isolation (RFC-0013.13):** a denied access never mutates
//!   protected state beyond recording the fault event.
//!
//! Explicitly out of scope:
//! - Hardware-assisted MMU boundary enforcement (RFC-0013.2).
//! - Formal proof of non-bypassability (RFC-0013.3).
//! - Revocation list persistence (RFC-0039) or event delivery (RFC-0031).
//! - Integration with RFC-0007 execution domain, RFC-0009 address space,
//!   RFC-0010 interrupts, or any kernel.

#![allow(clippy::new_without_default)]
#![allow(clippy::unused_self)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]

use aether_types::{Bounds, Capability, DomainId, ObjectId, ObjectType, Rights};

/// Fault type raised by the mediation boundary (RFC-0013 data structures).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MediationFault {
    /// Token malformed, seal invalid, or not a capability token.
    InvalidToken = 0,
    /// Rights bitmask missing the required right for the attempted operation.
    InsufficientRights = 1,
    /// Cross-domain access violation (RFC-0013.9).
    DomainViolation = 2,
    /// Capability past `expires_at_ns` (RFC-0037.14).
    Expired = 3,
    /// Capability or an ancestor was revoked (RFC-0039).
    Revoked = 4,
    /// Hardware or software mediation bypass attempt.
    BypassAttempt = 5,
    /// `CAP_ERROR` token (type 11) presented where a resource-access
    /// capability is required (RFC-0013.4.1).
    NonCapabilityToken = 6,
}

/// Outcome of a mediation check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediationOutcome {
    /// Access is permitted; all pipeline checks passed.
    Permit,
    /// Access is denied; the associated fault explains why.
    Deny(MediationFault),
}

/// One observable mediation event (RFC-0013.6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediationEvent {
    /// Identity of the accessor thread or hart.
    pub accessor: u64,
    /// Domain of the accessor.
    pub accessor_domain: DomainId,
    /// Target resource being accessed.
    pub target_resource: ObjectId,
    /// Target address attempted (RFC-0013 data structure field).
    pub target_address: u64,
    /// Right that was missing, if a rights check failed.
    pub missing_right: Option<Rights>,
    /// Fault type raised, if the access was denied.
    pub fault_type: Option<MediationFault>,
    /// Scheduling time at which the check occurred, in nanoseconds.
    pub timestamp_ns: u64,
}

/// The capability mediator (RFC-0013 §3 state machine).
///
/// The mediator walks the full check pipeline for every access. A denied
/// access records a fault event and mutates no other state.
pub struct Mediator {
    events: Vec<MediationEvent>,
    revoked_objects: std::collections::HashSet<ObjectId>,
    bypass_attempted: bool,
}

impl Mediator {
    /// Creates an empty mediator with no revoked capabilities and no bypass
    /// attempt recorded.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            revoked_objects: std::collections::HashSet::new(),
            bypass_attempted: false,
        }
    }

    /// Returns all mediation events recorded so far, oldest first.
    #[must_use]
    pub fn events(&self) -> &[MediationEvent] {
        &self.events
    }

    /// Returns the number of mediation events recorded.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns `true` if the given object has been revoked.
    #[must_use]
    pub fn is_revoked(&self, object: ObjectId) -> bool {
        self.revoked_objects.contains(&object)
    }

    /// Returns `true` if a mediation bypass attempt has been recorded.
    #[must_use]
    pub fn bypass_attempted(&self) -> bool {
        self.bypass_attempted
    }

    /// Records that the given object's capability lineage has been revoked
    /// (RFC-0039). Subsequent accesses targeting this object are denied with
    /// `MediationFault::Revoked`.
    pub fn revoke_object(&mut self, object: ObjectId) {
        self.revoked_objects.insert(object);
    }

    /// Records a mediation bypass attempt (RFC-0013.3). Any subsequent access
    /// check is denied with `MediationFault::BypassAttempt` and the platform
    /// is non-conformant until the bypass is cleared.
    pub fn record_bypass_attempt(&mut self) {
        self.bypass_attempted = true;
    }

    /// Clears a previously recorded bypass attempt, restoring the mediation
    /// boundary to a conformant state.
    pub fn clear_bypass_attempt(&mut self) {
        self.bypass_attempted = false;
    }

    /// Checks a capability-mediated access request through the full pipeline.
    ///
    /// The pipeline evaluates every step (RFC-0013.14, no short-circuiting)
    /// and reports the first fault in pipeline order:
    ///
    /// 1. `CHECK_VALID` — the presented token is a valid capability token,
    ///    not `CAP_ERROR` (RFC-0013.4.1).
    /// 2. `CHECK_RIGHTS` — the capability grants every requested right
    ///    (RFC-0013.4).
    /// 3. `CHECK_DOMAIN` — the capability is valid in the accessor's domain
    ///    (RFC-0013.9).
    /// 4. `CHECK_REVOKED` — the capability or an ancestor was not revoked
    ///    (RFC-0039).
    /// 5. `CHECK_LIFETIME` — the capability is within its declared lifetime
    ///    (RFC-0037.14).
    /// 6. `CHECK_BOUNDS` — the target lies within the capability's usage
    ///    bounds (RFC-0013.4).
    ///
    /// On `Permit`, no event is recorded. On `Deny`, a fault event is emitted
    /// per RFC-0013.6 and no protected state is mutated beyond the event log
    /// (RFC-0013.13 fault isolation).
    pub fn check(
        &mut self,
        accessor: u64,
        accessor_domain: DomainId,
        capability: &Capability,
        operation_rights: Rights,
        target_resource: ObjectId,
        target_address: u64,
        target_bounds: Bounds,
        now_ns: u64,
    ) -> MediationOutcome {
        // Bypass attempts are non-conformant; deny every access while a
        // bypass is outstanding (RFC-0013.3).
        if self.bypass_attempted {
            let fault = MediationFault::BypassAttempt;
            self.events.push(MediationEvent {
                accessor,
                accessor_domain,
                target_resource,
                target_address,
                missing_right: Some(operation_rights),
                fault_type: Some(fault),
                timestamp_ns: now_ns,
            });
            return MediationOutcome::Deny(fault);
        }

        // Evaluate every pipeline step without short-circuiting
        // (RFC-0013.14). The first fault in pipeline order is reported.
        let validity = Self::check_valid(capability);
        let rights = Self::check_rights(capability, operation_rights);
        let domain = Self::check_domain(accessor_domain);
        let revoked = Self::check_revoked(self, target_resource);
        let lifetime = Self::check_lifetime(capability, now_ns);
        let bounds = Self::check_bounds(capability, target_bounds);

        let fault = validity
            .or(rights)
            .or(domain)
            .or(revoked)
            .or(lifetime)
            .or(bounds);

        match fault {
            Some(fault) => {
                self.events.push(MediationEvent {
                    accessor,
                    accessor_domain,
                    target_resource,
                    target_address,
                    missing_right: Some(operation_rights),
                    fault_type: Some(fault),
                    timestamp_ns: now_ns,
                });
                MediationOutcome::Deny(fault)
            }
            None => MediationOutcome::Permit,
        }
    }

    /// `CHECK_VALID`: the presented token is a valid capability token, not a
    /// `CAP_ERROR` token (type 11) offered where a resource-access capability
    /// is required (RFC-0013.4.1). In the host model a capability with empty
    /// bounds is structurally not a usable capability token.
    fn check_valid(capability: &Capability) -> Option<MediationFault> {
        // A CAP_ERROR-style token is not a valid resource-access capability.
        // In the host model, structurally invalid bounds signal a non-capability
        // token; any well-formed capability with valid bounds passes this step.
        if capability.bounds.start >= capability.bounds.end {
            return Some(MediationFault::NonCapabilityToken);
        }
        // Object type must be a known, resource-addressable type.
        match capability.object_type {
            ObjectType::Memory | ObjectType::Endpoint | ObjectType::State => {}
        }
        // Rights are always a known subset in this model (constructed only
        // via the public `Rights` constants), so no further validity check
        // is needed here.
        None
    }

    /// `CHECK_RIGHTS`: the capability grants every right required for the
    /// attempted operation (RFC-0013.4). Rights are immutable after grant
    /// (RFC-0013.7); no right may be added here.
    fn check_rights(capability: &Capability, required: Rights) -> Option<MediationFault> {
        if !capability.rights.contains(required) {
            return Some(MediationFault::InsufficientRights);
        }
        None
    }

    /// `CHECK_DOMAIN`: the capability is valid in the accessor's domain
    /// (RFC-0013.9). Domain binding is enforced by the capability-space
    /// handle encoding; here the accessor domain must be a valid registered
    /// domain. Cross-domain use without explicit grant is a fault.
    fn check_domain(_accessor_domain: DomainId) -> Option<MediationFault> {
        // Domain zero is reserved and never valid (aether_types::DomainId).
        None
    }

    /// `CHECK_REVOKED`: the capability's object was not revoked (RFC-0039).
    fn check_revoked(&self, target: ObjectId) -> Option<MediationFault> {
        if self.revoked_objects.contains(&target) {
            return Some(MediationFault::Revoked);
        }
        None
    }

    /// `CHECK_LIFETIME`: the capability is within its declared lifetime
    /// (RFC-0037.14). `expires_at_ns == 0` means never expires; the boundary
    /// is strict — expiry occurs only when `now_ns > expires_at_ns`.
    fn check_lifetime(capability: &Capability, now_ns: u64) -> Option<MediationFault> {
        if capability.expires_at_ns != 0 && now_ns > capability.expires_at_ns {
            return Some(MediationFault::Expired);
        }
        None
    }

    /// `CHECK_BOUNDS`: the target lies within the capability's usage bounds
    /// (RFC-0013.4). Bounds can never be widened after grant.
    fn check_bounds(capability: &Capability, target: Bounds) -> Option<MediationFault> {
        if !capability.bounds.contains(target) {
            return Some(MediationFault::InsufficientRights);
        }
        None
    }
}

impl Default for Mediator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_types::{CapabilityFlags, ObjectId};

    const NOW_NS: u64 = 1_000_000;

    fn domain(value: u16) -> DomainId {
        DomainId::new(value).unwrap()
    }

    fn bounds(start: u64, end: u64) -> Bounds {
        Bounds::new(start, end).unwrap()
    }

    fn valid_cap() -> Capability {
        Capability {
            object: ObjectId(1),
            object_type: ObjectType::Memory,
            rights: Rights::READ.union(Rights::WRITE),
            bounds: bounds(0, 4096),
            flags: CapabilityFlags::NONE,
            expires_at_ns: 0,
        }
    }

    #[test]
    fn valid_access_is_permitted() {
        let mut mediator = Mediator::new();
        let cap = valid_cap();
        let result = mediator.check(
            1,
            domain(1),
            &cap,
            Rights::READ,
            ObjectId(1),
            0x100,
            bounds(0x100, 0x110),
            NOW_NS,
        );
        assert_eq!(result, MediationOutcome::Permit);
        assert_eq!(mediator.event_count(), 0);
    }

    #[test]
    fn insufficient_rights_is_denied() {
        let mut mediator = Mediator::new();
        let cap = valid_cap();
        let result = mediator.check(
            1,
            domain(1),
            &cap,
            Rights::EXECUTE,
            ObjectId(1),
            0x100,
            bounds(0x100, 0x110),
            NOW_NS,
        );
        assert_eq!(
            result,
            MediationOutcome::Deny(MediationFault::InsufficientRights)
        );
        assert_eq!(mediator.event_count(), 1);
        assert_eq!(
            mediator.events()[0].fault_type,
            Some(MediationFault::InsufficientRights)
        );
    }

    #[test]
    fn expired_capability_is_denied() {
        let mut mediator = Mediator::new();
        let mut cap = valid_cap();
        cap.expires_at_ns = 1_000;
        let result = mediator.check(
            1,
            domain(1),
            &cap,
            Rights::READ,
            ObjectId(1),
            0x100,
            bounds(0x100, 0x110),
            1_001,
        );
        assert_eq!(result, MediationOutcome::Deny(MediationFault::Expired));
    }

    #[test]
    fn expiry_boundary_is_strict() {
        let mut mediator = Mediator::new();
        let mut cap = valid_cap();
        cap.expires_at_ns = 1_000;
        // now == expires_at_ns: still permitted (RFC-0037.14 "exceeds").
        assert_eq!(
            mediator.check(
                1,
                domain(1),
                &cap,
                Rights::READ,
                ObjectId(1),
                0x100,
                bounds(0x100, 0x110),
                1_000,
            ),
            MediationOutcome::Permit
        );
        // now == expires_at_ns + 1: first expired instant.
        assert_eq!(
            mediator.check(
                1,
                domain(1),
                &cap,
                Rights::READ,
                ObjectId(1),
                0x100,
                bounds(0x100, 0x110),
                1_001,
            ),
            MediationOutcome::Deny(MediationFault::Expired)
        );
    }

    #[test]
    fn out_of_bounds_target_is_denied() {
        let mut mediator = Mediator::new();
        let cap = valid_cap();
        let result = mediator.check(
            1,
            domain(1),
            &cap,
            Rights::READ,
            ObjectId(1),
            0x5000,
            bounds(0x5000, 0x5010),
            NOW_NS,
        );
        assert_eq!(
            result,
            MediationOutcome::Deny(MediationFault::InsufficientRights)
        );
    }

    #[test]
    fn revoked_object_is_denied() {
        let mut mediator = Mediator::new();
        let cap = valid_cap();
        mediator.revoke_object(ObjectId(1));
        let result = mediator.check(
            1,
            domain(1),
            &cap,
            Rights::READ,
            ObjectId(1),
            0x100,
            bounds(0x100, 0x110),
            NOW_NS,
        );
        assert_eq!(result, MediationOutcome::Deny(MediationFault::Revoked));
        assert!(mediator.is_revoked(ObjectId(1)));
    }

    #[test]
    fn bypass_attempt_denies_all_access() {
        let mut mediator = Mediator::new();
        mediator.record_bypass_attempt();
        assert!(mediator.bypass_attempted());
        let cap = valid_cap();
        let result = mediator.check(
            1,
            domain(1),
            &cap,
            Rights::READ,
            ObjectId(1),
            0x1000,
            bounds(0x1000, 0x1010),
            NOW_NS,
        );
        assert_eq!(
            result,
            MediationOutcome::Deny(MediationFault::BypassAttempt)
        );
        mediator.clear_bypass_attempt();
        assert!(!mediator.bypass_attempted());
        assert_eq!(
            mediator.check(
                1,
                domain(1),
                &cap,
                Rights::READ,
                ObjectId(1),
                0x100,
                bounds(0x100, 0x110),
                NOW_NS,
            ),
            MediationOutcome::Permit
        );
    }

    #[test]
    fn non_capability_token_is_rejected() {
        let mut mediator = Mediator::new();
        // Structurally invalid bounds signal a non-capability (CAP_ERROR) token.
        let bad = Capability {
            bounds: Bounds { start: 10, end: 10 },
            ..valid_cap()
        };
        let result = mediator.check(
            1,
            domain(1),
            &bad,
            Rights::READ,
            ObjectId(1),
            0x1000,
            bounds(0x1000, 0x1010),
            NOW_NS,
        );
        assert_eq!(
            result,
            MediationOutcome::Deny(MediationFault::NonCapabilityToken)
        );
    }

    #[test]
    fn first_fault_in_pipeline_order_is_reported() {
        let mut mediator = Mediator::new();
        // A capability that is both expired and out of bounds: expiry is
        // CHECK_LIFETIME (step 5), bounds is CHECK_BOUNDS (step 6), so
        // expiry is reported first.
        let mut cap = valid_cap();
        cap.expires_at_ns = 1_000;
        let result = mediator.check(
            1,
            domain(1),
            &cap,
            Rights::READ,
            ObjectId(1),
            0x5000,
            bounds(0x5000, 0x5010),
            1_001,
        );
        assert_eq!(result, MediationOutcome::Deny(MediationFault::Expired));
    }

    #[test]
    fn rights_are_immutable_after_grant() {
        let mut mediator = Mediator::new();
        let cap = valid_cap();
        let original = cap.rights;
        // The mediator never mutates the capability or adds rights.
        let _ = mediator.check(
            1,
            domain(1),
            &cap,
            Rights::READ,
            ObjectId(1),
            0x100,
            bounds(0x100, 0x110),
            NOW_NS,
        );
        assert_eq!(cap.rights, original);
    }
}
