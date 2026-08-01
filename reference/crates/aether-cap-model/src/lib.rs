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
    /// # Errors
    /// Returns a typed failure if the parent is invalid or attenuation fails.
    pub fn derive(
        &mut self,
        domain: DomainId,
        parent: Handle,
        rights: Rights,
        bounds: Bounds,
    ) -> Result<Handle, CapError> {
        let record = self.resolve_live(domain, parent)?.clone();
        Self::check_narrowing(&record.capability, rights, bounds)?;
        let mut lineage = record.lineage;
        lineage.push(self.alloc_lineage()?);
        self.insert(
            domain,
            Capability {
                rights,
                bounds,
                ..record.capability
            },
            lineage,
        )
    }

    /// Transfers an attenuated child to another domain.
    ///
    /// # Errors
    /// Returns a typed failure if the parent is invalid, lacks `GRANT`, or
    /// the requested authority is not a subset.
    pub fn transfer(
        &mut self,
        source: DomainId,
        parent: Handle,
        destination: DomainId,
        rights: Rights,
        bounds: Bounds,
    ) -> Result<Handle, CapError> {
        let record = self.resolve_live(source, parent)?.clone();
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
    /// # Errors
    /// Returns the precise handle, domain, generation, revocation, type,
    /// rights, or bounds validation failure.
    pub fn authorize(
        &self,
        domain: DomainId,
        handle: Handle,
        object_type: ObjectType,
        rights: Rights,
        bounds: Bounds,
    ) -> Result<Capability, CapError> {
        let record = self.resolve(domain, handle)?;
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

    fn resolve_live(&self, domain: DomainId, handle: Handle) -> Result<&Record, CapError> {
        let record = self.resolve(domain, handle)?;
        if record.lineage.iter().any(|id| self.revoked.contains(id)) {
            return Err(CapError::Revoked);
        }
        Ok(record)
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
    use aether_types::ObjectId;

    #[test]
    fn altered_generation_is_stale() {
        let domain = DomainId::new(1).unwrap();
        let bounds = Bounds::new(0, 64).unwrap();
        let (mut model, bootstrap) = CapabilityModel::bootstrap();
        model.add_domain(domain);
        let handle = model
            .create_root(
                &bootstrap,
                domain,
                Capability {
                    object: ObjectId(1),
                    object_type: ObjectType::Memory,
                    rights: Rights::READ,
                    bounds,
                },
            )
            .unwrap();
        let (_, slot, generation) = handle.decode().unwrap();
        let forged = Handle::encode(domain, slot, generation + 1).unwrap();
        assert_eq!(
            model.authorize(domain, forged, ObjectType::Memory, Rights::READ, bounds),
            Err(CapError::Stale)
        );
    }

    #[test]
    fn bootstrap_witness_is_bound_to_its_model() {
        let (mut first, first_bootstrap) = CapabilityModel::bootstrap();
        let (mut second, second_bootstrap) = CapabilityModel::bootstrap();
        let domain = DomainId::new(1).unwrap();
        let capability = Capability {
            object: ObjectId(1),
            object_type: ObjectType::Memory,
            rights: Rights::READ,
            bounds: Bounds::new(0, 1).unwrap(),
        };
        first.add_domain(domain);
        second.add_domain(domain);
        assert_eq!(
            first.create_root(&second_bootstrap, domain, capability),
            Err(CapError::WrongBootstrap)
        );
        assert!(
            second
                .create_root(&second_bootstrap, domain, capability)
                .is_ok()
        );
        assert!(
            first
                .create_root(&first_bootstrap, domain, capability)
                .is_ok()
        );
    }

    #[test]
    fn identifier_exhaustion_fails_closed() {
        let domain = DomainId::new(1).unwrap();
        let capability = Capability {
            object: ObjectId(1),
            object_type: ObjectType::Memory,
            rights: Rights::READ,
            bounds: Bounds::new(0, 1).unwrap(),
        };
        let (mut slots, slot_bootstrap) = CapabilityModel::bootstrap();
        slots.add_domain(domain);
        slots.next_handle = 0x0100_0000;
        assert_eq!(
            slots.create_root(&slot_bootstrap, domain, capability),
            Err(CapError::Exhausted)
        );
        let (mut lineages, lineage_bootstrap) = CapabilityModel::bootstrap();
        lineages.add_domain(domain);
        lineages.next_lineage = u64::MAX;
        assert_eq!(
            lineages.create_root(&lineage_bootstrap, domain, capability),
            Err(CapError::Exhausted)
        );
    }

    #[test]
    fn missing_domains_and_revoked_parents_fail_closed() {
        let domain = DomainId::new(1).unwrap();
        let missing = DomainId::new(2).unwrap();
        let capability = Capability {
            object: ObjectId(1),
            object_type: ObjectType::Memory,
            rights: Rights::READ.union(Rights::GRANT),
            bounds: Bounds::new(0, 8).unwrap(),
        };
        let (mut model, bootstrap) = CapabilityModel::bootstrap();
        model.add_domain(domain);
        assert_eq!(
            model.create_root(&bootstrap, missing, capability),
            Err(CapError::Missing)
        );
        let root = model.create_root(&bootstrap, domain, capability).unwrap();
        model.revoke(domain, root).unwrap();
        assert_eq!(
            model.derive(domain, root, Rights::READ, Bounds::new(0, 1).unwrap()),
            Err(CapError::Revoked)
        );
        assert_eq!(
            model.transfer(
                domain,
                root,
                missing,
                Rights::READ,
                Bounds::new(0, 1).unwrap()
            ),
            Err(CapError::Revoked)
        );
    }
}
