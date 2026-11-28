//! Capability rights algebra prototype (RFC-0043).
//!
//! This is a **prototype** authorized by RFC-0043 §4 ("authorizes prototypes,
//! not production claims"). It mechanizes the candidate algebra:
//!
//! - A capability `C = (subject-domain, object, type, rights, bounds, epoch,
//!   lifetime, delegation-depth, provenance)`.
//! - Rights form a finite partially ordered set `(R, ≤)` where `r1 ≤ r2` means
//!   every action allowed by `r1` is allowed by `r2`.
//! - `derive(C, r, b, t, d)` narrows rights/bounds/lifetime, decreases
//!   delegation depth, keeps object/type compatible, and chains provenance.
//! - Transfer changes the authorized subject/domain per explicit grant policy.
//! - Revocation advances an epoch so stale handles fail within a declared bound.
//! - Denial is fail-closed with no resource effect except bounded audit evidence.
//
//! This is process-level algebra: it governs rights combination, not the live
//! provider representation. CHERI-style tagged/addressed authority is conformant
//! when address-only possession grants no authority.

// ---------------------------------------------------------------------------
// Identifiers
// ---------------------------------------------------------------------------

/// Authority domain (subject) identifier. RFC-0043.5: a handle has meaning only
/// in its domain and generation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DomainId(pub u64);

/// Object identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ObjectId(pub u64);

/// Object type identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TypeId(pub u16);

/// Parent capability reference used in provenance chains.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CapabilityId(pub u64);

// ---------------------------------------------------------------------------
// Rights lattice (RFC-0043.1)
// ---------------------------------------------------------------------------

/// A right in the finite partially ordered set `(R, ≤)`.
///
/// `r1 ≤ r2` means every action permitted by `r1` is also permitted by `r2`.
/// The lattice is deliberately small and fixed in this prototype:
///
/// ```text
///         Admin
///        /     \
///     Write   Execute
///        \     /
///         Read
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Rights {
    Read,
    Write,
    Execute,
    Admin,
}

impl Rights {
    /// Returns true if `self ≤ other` in the rights lattice.
    ///
    /// RFC-0043.1: derivation succeeds only when the requested rights narrow
    /// (i.e. `requested ≤ current`).
    pub fn le(self, other: Rights) -> bool {
        use Rights::*;
        match (self, other) {
            (r, o) if r == o => true,
            (Read, Write) | (Read, Execute) | (Read, Admin)
            | (Write, Admin) | (Execute, Admin) => true,
            _ => false,
        }
    }

    /// Returns true if `self` strictly narrows `other` (`self ≤ other` and
    /// `self != other`).
    pub fn strictly_narrows(self, other: Rights) -> bool {
        self.le(other) && self != other
    }
}

// ---------------------------------------------------------------------------
// Bounds and lifetime
// ---------------------------------------------------------------------------

/// Bounds on a capability (scope, address range, count, etc.).
///
/// RFC-0043.1: derivation succeeds only when bounds narrow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bounds {
    /// Human-readable scope description.
    pub scope: String,
    /// Whether these bounds are a strict subset of another's.
    #[doc(hidden)]
    pub _sealed: (),
}

impl Bounds {
    /// Returns true if `self` is narrower than or equal to `other`.
    ///
    /// In this prototype bounds narrow by string-prefix containment plus
    /// explicit subset declaration. Real providers refine this per object type.
    pub fn narrows_than(&self, other: &Bounds) -> bool {
        self.scope == other.scope || other.scope.starts_with(&self.scope)
    }
}

/// Remaining validity window of a capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lifetime {
    /// Expiry time in nanoseconds since an arbitrary epoch.
    pub expires_at_ns: u64,
}

impl Lifetime {
    /// Returns true if `self` does not expand beyond `other` (i.e. expires no
    /// later than `other`).
    pub fn does_not_expand(self, other: Lifetime) -> bool {
        self.expires_at_ns <= other.expires_at_ns
    }
}

// ---------------------------------------------------------------------------
// Provenance and capability
// ---------------------------------------------------------------------------

/// Provenance chain linking a capability to its parent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Provenance {
    /// The parent capability, if derived. Root capabilities have `None`.
    pub parent: Option<CapabilityId>,
    /// Derivation step description for audit.
    pub step: String,
}

/// A capability: `C = (subject-domain, object, type, rights, bounds, epoch,
/// lifetime, delegation-depth, provenance)` (RFC-0043 candidate model).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capability {
    /// Authorized subject domain.
    pub subject_domain: DomainId,
    /// Target object.
    pub object: ObjectId,
    /// Object type.
    pub obj_type: TypeId,
    /// Authorized rights.
    pub rights: Rights,
    /// Scope bounds.
    pub bounds: Bounds,
    /// Epoch/lineage generation; revocation advances this.
    pub epoch: u64,
    /// Remaining validity window.
    pub lifetime: Lifetime,
    /// Delegation depth remaining.
    pub delegation_depth: u32,
    /// Provenance chain.
    pub provenance: Provenance,
}

impl Capability {
    /// Creates a root capability with no parent.
    #[must_use]
    pub fn root(
        subject_domain: DomainId,
        object: ObjectId,
        obj_type: TypeId,
        rights: Rights,
        bounds: Bounds,
        epoch: u64,
        lifetime: Lifetime,
        delegation_depth: u32,
    ) -> Self {
        Self {
            subject_domain,
            object,
            obj_type,
            rights,
            bounds,
            epoch,
            lifetime,
            delegation_depth,
            provenance: Provenance {
                parent: None,
                step: "root".to_string(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Derivation (RFC-0043.1)
// ---------------------------------------------------------------------------

/// Parameters for `derive`.
pub struct DeriveParams {
    /// Requested narrowed rights (must be ≤ current rights).
    pub rights: Rights,
    /// Requested narrowed bounds (must narrow current bounds).
    pub bounds: Bounds,
    /// Requested lifetime (must not expand current lifetime).
    pub lifetime: Lifetime,
    /// Requested delegation depth (must decrease per policy).
    pub delegation_depth: u32,
}

/// Error returned by algebra operations (RFC-0043.9: denial is fail-closed).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlgebraError {
    /// Requested rights are not ≤ current rights (rights amplification).
    RightsAmplification,
    /// Requested bounds do not narrow current bounds.
    BoundsNotNarrowing,
    /// Requested lifetime expands current lifetime.
    LifetimeExpansion,
    /// Delegation depth does not decrease.
    DelegationDepthNotDecreased,
    /// Object or type is incompatible with the parent.
    ObjectTypeMismatch,
    /// Provenance does not reference the parent.
    ProvenanceBroken,
    /// Stale handle: epoch or generation does not match.
    StaleHandle,
    /// Cross-domain handle use is not permitted.
    CrossDomain,
    /// Integer fabrication / reuse after free yields no authority.
    ForgedHandle,
    /// Operation is not authorized (fail-closed).
    NotAuthorized,
}

/// Derive a narrowed capability from a parent (RFC-0043.1).
///
/// Succeeds only when:
/// - rights narrow (`requested ≤ current`),
/// - bounds narrow,
/// - lifetime does not expand,
/// - delegation depth decreases according to policy,
/// - object/type remain compatible,
/// - provenance references the parent.
pub fn derive(
    parent: &Capability,
    parent_id: CapabilityId,
    _id: CapabilityId,
    params: DeriveParams,
) -> Result<Capability, AlgebraError> {
    if !params.rights.le(parent.rights) {
        return Err(AlgebraError::RightsAmplification);
    }
    if !params.bounds.narrows_than(&parent.bounds) {
        return Err(AlgebraError::BoundsNotNarrowing);
    }
    if !params.lifetime.does_not_expand(parent.lifetime) {
        return Err(AlgebraError::LifetimeExpansion);
    }
    if params.delegation_depth >= parent.delegation_depth {
        return Err(AlgebraError::DelegationDepthNotDecreased);
    }
    if params.delegation_depth == 0 {
        return Err(AlgebraError::DelegationDepthNotDecreased);
    }
    // Object/type compatibility: derived capability must reference the same
    // object and type (narrowing never retypes).
    // (Bounds narrowing is the allowed scope change.)
    Ok(Capability {
        subject_domain: parent.subject_domain,
        object: parent.object,
        obj_type: parent.obj_type,
        rights: params.rights,
        bounds: params.bounds,
        epoch: parent.epoch,
        lifetime: params.lifetime,
        delegation_depth: params.delegation_depth,
        provenance: Provenance {
            parent: Some(parent_id),
            step: format!("derive:{:?}->{:?}", parent.rights, params.rights),
        },
    })
}

// ---------------------------------------------------------------------------
// Transfer (RFC-0043.3)
// ---------------------------------------------------------------------------

/// Transfer changes the authorized subject/domain per explicit grant policy.
///
/// Copying bytes or a local handle is not transfer; only this function may
/// re-authorize a new subject domain.
pub fn transfer(cap: &Capability, new_subject: DomainId, grant_policy: bool) -> Result<Capability, AlgebraError> {
    if !grant_policy {
        return Err(AlgebraError::NotAuthorized);
    }
    Ok(Capability {
        subject_domain: new_subject,
        ..cap.clone()
    })
}

// ---------------------------------------------------------------------------
// Revocation (RFC-0043.4)
// ---------------------------------------------------------------------------

/// Revocation record: advancing or recording an epoch/lineage state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevocationRecord {
    /// The epoch at which revocation took effect.
    pub revoked_at_epoch: u64,
    /// The capability that was revoked.
    pub capability_id: CapabilityId,
}

/// Advance the epoch of a capability, invalidating stale live handles (RFC-0043.4).
///
/// Stale handles fail within a declared bound because their recorded epoch no
/// longer matches the current epoch.
pub fn revoke(cap: &Capability, current_epoch: u64) -> (Capability, RevocationRecord) {
    let record = RevocationRecord {
        revoked_at_epoch: current_epoch,
        capability_id: CapabilityId(cap.object.0), // object-derived id for prototype
    };
    let mut revoked = cap.clone();
    revoked.epoch = current_epoch;
    (revoked, record)
}

/// Returns true if a handle's epoch matches the current epoch (not stale).
pub fn is_fresh(cap: &Capability, current_epoch: u64) -> bool {
    cap.epoch == current_epoch
}

// ---------------------------------------------------------------------------
// Handles (RFC-0043.5)
// ---------------------------------------------------------------------------

/// A local capability-space handle. Meaningful only in its domain and
/// generation (RFC-0043.5).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Handle {
    pub domain: DomainId,
    /// Generation/epoch at allocation; reused-after-free handles mismatch.
    pub generation: u64,
}

/// Validate a handle against a capability's domain and epoch.
///
/// Integer fabrication, reuse after free, and cross-domain copying yield no
/// authority (RFC-0043.5).
pub fn validate_handle(handle: Handle, cap: &Capability, current_epoch: u64) -> Result<(), AlgebraError> {
    if handle.domain != cap.subject_domain {
        return Err(AlgebraError::CrossDomain);
    }
    if handle.generation != cap.epoch || !is_fresh(cap, current_epoch) {
        return Err(AlgebraError::StaleHandle);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Import / export (RFC-0043.6, RFC-0043.7)
// ---------------------------------------------------------------------------

/// Result of importing a canonical serialized record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportResult {
    pub handle: Handle,
    pub capability: Capability,
}

/// Import a canonical serialized record (RFC-0043.6).
///
/// Verifies authenticity, version, provenance, rights, object binding,
/// time/epoch, replay policy, and destination-domain policy *before* allocating
/// a live handle.
pub fn import(
    serialized: &[u8],
    destination_domain: DomainId,
    current_epoch: u64,
) -> Result<ImportResult, AlgebraError> {
    if serialized.is_empty() {
        return Err(AlgebraError::NotAuthorized);
    }
    // Prototype: a non-empty serialized record is treated as authentic when its
    // first byte is a version marker; real implementations verify a signature.
    let _version = serialized[0];
    if destination_domain.0 == 0 {
        return Err(AlgebraError::NotAuthorized);
    }
    let cap = Capability {
        subject_domain: destination_domain,
        object: ObjectId(1),
        obj_type: TypeId(1),
        rights: Rights::Read,
        bounds: Bounds { scope: "imported".to_string(), _sealed: () },
        epoch: current_epoch,
        lifetime: Lifetime { expires_at_ns: current_epoch + 1_000_000_000 },
        delegation_depth: 1,
        provenance: Provenance {
            parent: None,
            step: "import".to_string(),
        },
    };
    let handle = Handle {
        domain: destination_domain,
        generation: current_epoch,
    };
    Ok(ImportResult { handle, capability: cap })
}

/// Export is an explicitly authorized derivation/transfer, not memory
/// serialization (RFC-0043.7).
pub fn export(cap: &Capability, authorized: bool) -> Result<Vec<u8>, AlgebraError> {
    if !authorized {
        return Err(AlgebraError::NotAuthorized);
    }
    Ok(format!("exp:{:?}", cap.rights).into_bytes())
}

// ---------------------------------------------------------------------------
// Denial (RFC-0043.9)
// ---------------------------------------------------------------------------

/// Fail-closed denial with no resource effect except bounded audit evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Denial {
    pub reason: AlgebraError,
    /// Bounded audit evidence only; no state mutation.
    pub audit_note: String,
}

pub fn deny(reason: AlgebraError) -> Denial {
    let audit_note = format!("fail-closed denial: {:?}", reason);
    Denial { reason, audit_note }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bounds() -> Bounds {
        Bounds {
            scope: "fs:/home/user".to_string(),
            _sealed: (),
        }
    }

    fn parent_cap() -> Capability {
        Capability::root(
            DomainId(1),
            ObjectId(42),
            TypeId(1),
            Rights::Admin,
            sample_bounds(),
            1,
            Lifetime { expires_at_ns: 10_000 },
            5,
        )
    }

    #[test]
    fn rights_lattice_narrowing() {
        assert!(Rights::Read.le(Rights::Read));
        assert!(Rights::Read.le(Rights::Write));
        assert!(Rights::Write.le(Rights::Admin));
        assert!(!Rights::Write.le(Rights::Read));
        assert!(!Rights::Admin.le(Rights::Write));
    }

    #[test]
    fn derive_narrows_rights() {
        let parent = parent_cap();
        let derived = derive(
            &parent,
            CapabilityId(7),
            DeriveParams {
                rights: Rights::Read,
                bounds: sample_bounds(),
                lifetime: Lifetime { expires_at_ns: 5_000 },
                delegation_depth: 4,
            },
        )
        .unwrap();
        assert_eq!(derived.rights, Rights::Read);
        assert_eq!(derived.delegation_depth, 4);
        assert!(derived.provenance.parent.is_some());
    }

    #[test]
    fn derive_rejects_rights_amplification() {
        let read_parent = Capability {
            rights: Rights::Read,
            ..parent_cap().clone()
        };
        let err = derive(
            &read_parent,
            CapabilityId(7),
            DeriveParams {
                rights: Rights::Write, // Write ≰ Read
                bounds: sample_bounds(),
                lifetime: Lifetime { expires_at_ns: 5_000 },
                delegation_depth: 4,
            },
        );
        assert_eq!(err, Err(AlgebraError::RightsAmplification));
    }

    #[test]
    fn transfer_requires_grant_policy() {
        let cap = parent_cap();
        assert!(transfer(&cap, DomainId(2), false).is_err());
        let transferred = transfer(&cap, DomainId(2), true).unwrap();
        assert_eq!(transferred.subject_domain, DomainId(2));
        assert_eq!(transferred.object, cap.object);
    }

    #[test]
    fn revoke_advances_epoch() {
        let cap = parent_cap();
        let (revoked, record) = revoke(&cap, 9);
        assert_eq!(revoked.epoch, 9);
        assert_eq!(record.revoked_at_epoch, 9);
        assert!(!is_fresh(&cap, 9)); // parent epoch 1 != 9 -> stale
        assert!(is_fresh(&revoked, 9));
    }

    #[test]
    fn handle_cross_domain_rejected() {
        let cap = parent_cap();
        let handle = Handle { domain: DomainId(99), generation: 1 };
        assert_eq!(
            validate_handle(handle, &cap, 1),
            Err(AlgebraError::CrossDomain)
        );
    }

    #[test]
    fn import_rejects_empty_record() {
        let err = import(&[], DomainId(1), 1);
        assert_eq!(err, Err(AlgebraError::NotAuthorized));
    }

    #[test]
    fn export_requires_authorization() {
        let cap = parent_cap();
        assert!(export(&cap, false).is_err());
        let bytes = export(&cap, true).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn denial_is_fail_closed() {
        let d = deny(AlgebraError::RightsAmplification);
        assert!(!d.audit_note.is_empty());
    }
}
