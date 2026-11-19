//! Abstract capability model used as the differential oracle.

use aether_types::{Bounds, CapError, Capability, DomainId, Handle, ObjectType, Rights};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct Record {
    generation: u16,
    capability: Capability,
    lineage: Vec<u64>,
}

/// Simple map-based semantic oracle. It intentionally has no slot reuse.
pub struct CapabilityModel {
    universe: Arc<()>,
    next_handle: u32,
    next_lineage: u64,
    domains: HashSet<DomainId>,
    records: HashMap<(DomainId, u32), Record>,
    revoked: HashSet<u64>,
}

/// Linear witness returned only when the explicit model bootstrap begins.
pub struct BootstrapAuthority(Arc<()>);

impl Default for CapabilityModel {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityModel {
    /// Creates an empty model.
    #[must_use]
    pub fn new() -> Self {
        Self {
            universe: Arc::new(()),
            next_handle: 1,
            next_lineage: 1,
            domains: HashSet::new(),
            records: HashMap::new(),
            revoked: HashSet::new(),
        }
    }

    /// Starts a fresh authority universe and returns its unique bootstrap witness.
    #[must_use]
    pub fn bootstrap() -> (Self, BootstrapAuthority) {
        let model = Self::new();
        let authority = BootstrapAuthority(Arc::clone(&model.universe));
        (model, authority)
    }

    /// Registers a domain. Duplicate registration is harmless.
    pub fn add_domain(&mut self, domain: DomainId) {
        self.domains.insert(domain);
    }

    /// Creates an origin capability. This represents the explicit bootstrap boundary.
    ///
    /// # Errors
    /// Returns `WrongBootstrap` if the witness came from another model universe.
    pub fn create_root(
        &mut self,
        bootstrap: &BootstrapAuthority,
        domain: DomainId,
        capability: Capability,
    ) -> Result<Handle, CapError> {
        if !Arc::ptr_eq(&bootstrap.0, &self.universe) {
            return Err(CapError::WrongBootstrap);
        }
        let lineage = self.alloc_lineage()?;
        self.insert(domain, capability, vec![lineage])
    }

    /// Derives a same-domain child with narrowed rights and bounds.
    ///
    /// The child inherits the parent's `expires_at_ns`; derivation never
    /// extends lifetime (RFC-0037.14, least-authority attenuation).
    ///
    /// # Errors
    /// Returns a typed failure if the parent is invalid, expired, or
    /// attenuation fails.
    pub fn derive(
        &mut self,
        domain: DomainId,
        parent: Handle,
        rights: Rights,
        bounds: Bounds,
        now_ns: u64,
    ) -> Result<Handle, CapError> {
        let record = self.resolve_live(domain, parent, now_ns)?.clone();
        Self::check_narrowing(&record.capability, rights, bounds)?;
        let mut lineage = record.lineage;
        lineage.push(self.alloc_lineage()?);
        self.insert(
            domain,
            Capability {
                rights,
                bounds,
                expires_at_ns: record.capability.expires_at_ns,
                ..record.capability
            },
            lineage,
        )
    }

    /// Transfers an attenuated child to another domain.
    ///
    /// The child inherits the parent's `expires_at_ns`; transfer never
    /// extends lifetime (RFC-0037.14). A capability carrying `CAPF_BOUND`
    /// may not be transferred to any domain (RFC-0037.7, RFC-0037.15).
    ///
    /// # Errors
    /// Returns a typed failure if the parent is invalid, expired or bound,
    /// lacks `GRANT`, or the requested authority is not a subset.
    pub fn transfer(
        &mut self,
        source: DomainId,
        parent: Handle,
        destination: DomainId,
        rights: Rights,
        bounds: Bounds,
        now_ns: u64,
    ) -> Result<Handle, CapError> {
        let record = self.resolve_live(source, parent, now_ns)?.clone();
        if record
            .capability
            .flags
            .contains(aether_types::CapabilityFlags::BOUND)
        {
            return Err(CapError::DomainBound);
        }
        if !record.capability.rights.contains(Rights::GRANT) {
            return Err(CapError::Rights);
        }
        Self::check_narrowing(&record.capability, rights, bounds)?;
        let mut lineage = record.lineage;
        lineage.push(self.alloc_lineage()?);
        self.insert(
            destination,
            Capability {
                rights,
                bounds,
                expires_at_ns: record.capability.expires_at_ns,
                ..record.capability
            },
            lineage,
        )
    }

    /// Revokes a capability lineage and all descendants, including transfers.
    ///
    /// # Errors
    /// Returns a typed failure if the handle cannot be resolved in the domain.
    pub fn revoke(&mut self, domain: DomainId, handle: Handle) -> Result<(), CapError> {
        let lineage = *self
            .resolve(domain, handle)?
            .lineage
            .last()
            .ok_or(CapError::Missing)?;
        self.revoked.insert(lineage);
        Ok(())
    }

    /// Checks authority and returns the semantic capability.
    ///
    /// `now_ns` is the current scheduling time in nanoseconds; expiry is
    /// enforced per RFC-0037.14 (`expires_at_ns != 0 && now_ns > expires_at_ns`).
    /// The boundary is intentionally strict: a capability whose `expires_at_ns`
    /// equals `now_ns` is still valid, and expires only at the next instant.
    ///
    /// # Errors
    /// Returns the precise handle, domain, generation, revocation, expiry,
    /// type, rights, or bounds validation failure.
    pub fn authorize(
        &self,
        domain: DomainId,
        handle: Handle,
        object_type: ObjectType,
        rights: Rights,
        bounds: Bounds,
        now_ns: u64,
    ) -> Result<Capability, CapError> {
        let record = self.resolve(domain, handle)?;
        Self::check_lifetime(&record.capability, now_ns)?;
        if record.lineage.iter().any(|id| self.revoked.contains(id)) {
            return Err(CapError::Revoked);
        }
        if record.capability.object_type != object_type {
            return Err(CapError::Type);
        }
        if !record.capability.rights.contains(rights) {
            return Err(CapError::Rights);
        }
        if !record.capability.bounds.contains(bounds) {
            return Err(CapError::Bounds);
        }
        Ok(record.capability)
    }

    fn resolve(&self, domain: DomainId, handle: Handle) -> Result<&Record, CapError> {
        let (encoded_domain, slot, generation) =
            handle.decode().ok_or(CapError::MalformedHandle)?;
        if encoded_domain != domain {
            return Err(CapError::WrongDomain);
        }
        let record = self.records.get(&(domain, slot)).ok_or(CapError::Missing)?;
        if record.generation != generation {
            return Err(CapError::Stale);
        }
        Ok(record)
    }

    fn resolve_live(
        &self,
        domain: DomainId,
        handle: Handle,
        now_ns: u64,
    ) -> Result<&Record, CapError> {
        let record = self.resolve(domain, handle)?;
        Self::check_lifetime(&record.capability, now_ns)?;
        if record.lineage.iter().any(|id| self.revoked.contains(id)) {
            return Err(CapError::Revoked);
        }
        Ok(record)
    }

    const fn check_lifetime(capability: &Capability, now_ns: u64) -> Result<(), CapError> {
        if capability.expires_at_ns != 0 && now_ns > capability.expires_at_ns {
            return Err(CapError::Expired);
        }
        Ok(())
    }

    fn check_narrowing(
        parent: &Capability,
        rights: Rights,
        bounds: Bounds,
    ) -> Result<(), CapError> {
        if !parent.rights.contains(rights) {
            return Err(CapError::Rights);
        }
        if !parent.bounds.contains(bounds) {
            return Err(CapError::Bounds);
        }
        Ok(())
    }

    fn alloc_lineage(&mut self) -> Result<u64, CapError> {
        let id = self.next_lineage;
        self.next_lineage = self
            .next_lineage
            .checked_add(1)
            .ok_or(CapError::Exhausted)?;
        Ok(id)
    }

    fn insert(
        &mut self,
        domain: DomainId,
        capability: Capability,
        lineage: Vec<u64>,
    ) -> Result<Handle, CapError> {
        if !self.domains.contains(&domain) {
            return Err(CapError::Missing);
        }
        let slot = self.next_handle;
        let handle = Handle::encode(domain, slot, 1).ok_or(CapError::Exhausted)?;
        self.next_handle = self.next_handle.checked_add(1).ok_or(CapError::Exhausted)?;
        self.records.insert(
            (domain, slot),
            Record {
                generation: 1,
                capability,
                lineage,
            },
        );
        Ok(handle)
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

    fn capability(rights: Rights, expires_at_ns: u64, flags: CapabilityFlags) -> Capability {
        Capability {
            object: ObjectId(1),
            object_type: ObjectType::Memory,
            rights,
            bounds: Bounds::new(0, 8).unwrap(),
            flags,
            expires_at_ns,
        }
    }

    #[test]
    fn altered_generation_is_stale() {
        let (mut model, bootstrap) = CapabilityModel::bootstrap();
        model.add_domain(domain(1));
        let cap = capability(Rights::READ, 0, CapabilityFlags::NONE);
        let bounds = cap.bounds;
        let handle = model.create_root(&bootstrap, domain(1), cap).unwrap();
        let (_, slot, generation) = handle.decode().unwrap();
        let forged = Handle::encode(domain(1), slot, generation + 1).unwrap();
        assert_eq!(
            model.authorize(
                domain(1),
                forged,
                ObjectType::Memory,
                Rights::READ,
                bounds,
                NOW_NS
            ),
            Err(CapError::Stale)
        );
    }

    #[test]
    fn bootstrap_witness_is_bound_to_its_model() {
        let (mut first, first_bootstrap) = CapabilityModel::bootstrap();
        let (mut second, second_bootstrap) = CapabilityModel::bootstrap();
        let cap = capability(Rights::READ, 0, CapabilityFlags::NONE);
        first.add_domain(domain(1));
        second.add_domain(domain(1));
        assert_eq!(
            first.create_root(&second_bootstrap, domain(1), cap),
            Err(CapError::WrongBootstrap)
        );
        assert!(
            second
                .create_root(&second_bootstrap, domain(1), cap)
                .is_ok()
        );
        assert!(first.create_root(&first_bootstrap, domain(1), cap).is_ok());
    }

    #[test]
    fn identifier_exhaustion_fails_closed() {
        let cap = capability(Rights::READ, 0, CapabilityFlags::NONE);
        let (mut slots, slot_bootstrap) = CapabilityModel::bootstrap();
        slots.add_domain(domain(1));
        slots.next_handle = 0x0100_0000;
        assert_eq!(
            slots.create_root(&slot_bootstrap, domain(1), cap),
            Err(CapError::Exhausted)
        );
        let (mut lineages, lineage_bootstrap) = CapabilityModel::bootstrap();
        lineages.add_domain(domain(1));
        lineages.next_lineage = u64::MAX;
        assert_eq!(
            lineages.create_root(&lineage_bootstrap, domain(1), cap),
            Err(CapError::Exhausted)
        );
    }

    #[test]
    fn missing_domains_and_revoked_parents_fail_closed() {
        let capability = capability(Rights::READ.union(Rights::GRANT), 0, CapabilityFlags::NONE);
        let (mut model, bootstrap) = CapabilityModel::bootstrap();
        model.add_domain(domain(1));
        assert_eq!(
            model.create_root(&bootstrap, domain(2), capability),
            Err(CapError::Missing)
        );
        let root = model
            .create_root(&bootstrap, domain(1), capability)
            .unwrap();
        model.revoke(domain(1), root).unwrap();
        assert_eq!(
            model.derive(
                domain(1),
                root,
                Rights::READ,
                Bounds::new(0, 1).unwrap(),
                NOW_NS
            ),
            Err(CapError::Revoked)
        );
        assert_eq!(
            model.transfer(
                domain(1),
                root,
                domain(2),
                Rights::READ,
                Bounds::new(0, 1).unwrap(),
                NOW_NS
            ),
            Err(CapError::Revoked)
        );
    }

    #[test]
    fn expiry_boundary_is_strictly_greater_than() {
        let capability = capability(Rights::READ, 1_000, CapabilityFlags::NONE);
        let bounds = capability.bounds;
        let (mut model, bootstrap) = CapabilityModel::bootstrap();
        model.add_domain(domain(1));
        let handle = model
            .create_root(&bootstrap, domain(1), capability)
            .unwrap();

        // now == expires_at_ns: still valid (RFC-0037.14 "exceeds").
        assert!(
            model
                .authorize(
                    domain(1),
                    handle,
                    ObjectType::Memory,
                    Rights::READ,
                    bounds,
                    1_000
                )
                .is_ok()
        );
        // now == expires_at_ns + 1: first expired instant.
        assert_eq!(
            model.authorize(
                domain(1),
                handle,
                ObjectType::Memory,
                Rights::READ,
                bounds,
                1_001
            ),
            Err(CapError::Expired)
        );
    }

    #[test]
    fn zero_expiry_means_never_expires() {
        let capability = capability(Rights::READ, 0, CapabilityFlags::NONE);
        let bounds = capability.bounds;
        let (mut model, bootstrap) = CapabilityModel::bootstrap();
        model.add_domain(domain(1));
        let handle = model
            .create_root(&bootstrap, domain(1), capability)
            .unwrap();
        assert!(
            model
                .authorize(
                    domain(1),
                    handle,
                    ObjectType::Memory,
                    Rights::READ,
                    bounds,
                    u64::MAX
                )
                .is_ok()
        );
    }

    #[test]
    fn expired_parent_cannot_derive_or_transfer() {
        let capability = capability(
            Rights::READ.union(Rights::GRANT),
            1_000,
            CapabilityFlags::NONE,
        );
        let (mut model, bootstrap) = CapabilityModel::bootstrap();
        model.add_domain(domain(1));
        model.add_domain(domain(2));
        let handle = model
            .create_root(&bootstrap, domain(1), capability)
            .unwrap();
        assert_eq!(
            model.derive(
                domain(1),
                handle,
                Rights::READ,
                Bounds::new(0, 1).unwrap(),
                1_001
            ),
            Err(CapError::Expired)
        );
        assert_eq!(
            model.transfer(
                domain(1),
                handle,
                domain(2),
                Rights::READ,
                Bounds::new(0, 1).unwrap(),
                1_001
            ),
            Err(CapError::Expired)
        );
    }

    #[test]
    fn derived_and_transferred_children_inherit_parent_expiry() {
        let capability = capability(
            Rights::READ.union(Rights::GRANT),
            5_000,
            CapabilityFlags::NONE,
        );
        let child_bounds = Bounds::new(0, 4).unwrap();
        let (mut model, bootstrap) = CapabilityModel::bootstrap();
        model.add_domain(domain(1));
        model.add_domain(domain(2));
        let handle = model
            .create_root(&bootstrap, domain(1), capability)
            .unwrap();
        let now = 1_000;
        let derived = model
            .derive(domain(1), handle, Rights::READ, child_bounds, now)
            .unwrap();
        let transferred = model
            .transfer(
                domain(1),
                handle,
                domain(2),
                Rights::READ,
                child_bounds,
                now,
            )
            .unwrap();

        for (bound_domain, child) in [(domain(1), derived), (domain(2), transferred)] {
            assert!(
                model
                    .authorize(
                        bound_domain,
                        child,
                        ObjectType::Memory,
                        Rights::READ,
                        child_bounds,
                        5_000
                    )
                    .is_ok()
            );
            assert_eq!(
                model.authorize(
                    bound_domain,
                    child,
                    ObjectType::Memory,
                    Rights::READ,
                    child_bounds,
                    5_001
                ),
                Err(CapError::Expired)
            );
        }
    }

    #[test]
    fn bound_capability_cannot_be_transferred() {
        let capability = capability(Rights::READ.union(Rights::GRANT), 0, CapabilityFlags::BOUND);
        let bounds = capability.bounds;
        let (mut model, bootstrap) = CapabilityModel::bootstrap();
        model.add_domain(domain(1));
        model.add_domain(domain(2));
        let handle = model
            .create_root(&bootstrap, domain(1), capability)
            .unwrap();

        // Same-domain use remains valid.
        assert!(
            model
                .authorize(
                    domain(1),
                    handle,
                    ObjectType::Memory,
                    Rights::READ,
                    bounds,
                    NOW_NS
                )
                .is_ok()
        );
        // Cross-domain transfer is refused.
        assert_eq!(
            model.transfer(
                domain(1),
                handle,
                domain(2),
                Rights::READ,
                Bounds::new(0, 1).unwrap(),
                NOW_NS
            ),
            Err(CapError::DomainBound)
        );
        // Derivation inside the domain is still permitted.
        assert!(
            model
                .derive(
                    domain(1),
                    handle,
                    Rights::READ,
                    Bounds::new(0, 1).unwrap(),
                    NOW_NS
                )
                .is_ok()
        );
    }

    #[test]
    fn derived_child_of_bound_parent_is_also_bound() {
        let capability = capability(Rights::READ.union(Rights::GRANT), 0, CapabilityFlags::BOUND);
        let (mut model, bootstrap) = CapabilityModel::bootstrap();
        model.add_domain(domain(1));
        model.add_domain(domain(2));
        let handle = model
            .create_root(&bootstrap, domain(1), capability)
            .unwrap();
        let child = model
            .derive(
                domain(1),
                handle,
                Rights::READ.union(Rights::GRANT),
                Bounds::new(0, 4).unwrap(),
                NOW_NS,
            )
            .unwrap();
        assert_eq!(
            model.transfer(
                domain(1),
                child,
                domain(2),
                Rights::READ,
                Bounds::new(0, 1).unwrap(),
                NOW_NS
            ),
            Err(CapError::DomainBound)
        );
    }
}
