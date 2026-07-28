//! Abstract capability model used as the differential oracle.

use aether_types::{Bounds, CapError, Capability, DomainId, Handle, ObjectType, Rights};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
struct Record {
    capability: Capability,
    lineage: Vec<u64>,
}

/// Simple map-based semantic oracle. It intentionally has no slot reuse.
#[derive(Default)]
pub struct CapabilityModel {
    next_handle: u32,
    next_lineage: u64,
    records: HashMap<(DomainId, u32), Record>,
    revoked: HashSet<u64>,
}

/// Linear witness returned only when the explicit model bootstrap begins.
pub struct BootstrapAuthority(());

impl CapabilityModel {
    /// Creates an empty model.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_handle: 1,
            next_lineage: 1,
            ..Self::default()
        }
    }

    /// Starts a fresh authority universe and returns its unique bootstrap witness.
    #[must_use]
    pub fn bootstrap() -> (Self, BootstrapAuthority) {
        (Self::new(), BootstrapAuthority(()))
    }

    /// Creates an origin capability. This represents the explicit bootstrap boundary.
    pub fn create_root(
        &mut self,
        _bootstrap: &BootstrapAuthority,
        domain: DomainId,
        capability: Capability,
    ) -> Handle {
        let lineage = self.alloc_lineage();
        self.insert(domain, capability, vec![lineage])
    }

    /// Derives a same-domain child with narrowed rights and bounds.
    pub fn derive(
        &mut self,
        domain: DomainId,
        parent: Handle,
        rights: Rights,
        bounds: Bounds,
    ) -> Result<Handle, CapError> {
        let record = self.resolve(domain, parent)?.clone();
        Self::check_narrowing(&record.capability, rights, bounds)?;
        let mut lineage = record.lineage;
        lineage.push(self.alloc_lineage());
        Ok(self.insert(
            domain,
            Capability {
                rights,
                bounds,
                ..record.capability
            },
            lineage,
        ))
    }

    /// Transfers an attenuated child to another domain.
    pub fn transfer(
        &mut self,
        source: DomainId,
        parent: Handle,
        destination: DomainId,
        rights: Rights,
        bounds: Bounds,
    ) -> Result<Handle, CapError> {
        let record = self.resolve(source, parent)?.clone();
        if !record.capability.rights.contains(Rights::GRANT) {
            return Err(CapError::Rights);
        }
        Self::check_narrowing(&record.capability, rights, bounds)?;
        let mut lineage = record.lineage;
        lineage.push(self.alloc_lineage());
        Ok(self.insert(
            destination,
            Capability {
                rights,
                bounds,
                ..record.capability
            },
            lineage,
        ))
    }

    /// Revokes a capability lineage and all descendants, including transfers.
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
        let (encoded_domain, slot, _) = handle.decode().ok_or(CapError::MalformedHandle)?;
        if encoded_domain != domain {
            return Err(CapError::WrongDomain);
        }
        self.records.get(&(domain, slot)).ok_or(CapError::Missing)
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

    fn alloc_lineage(&mut self) -> u64 {
        let id = self.next_lineage;
        self.next_lineage += 1;
        id
    }

    fn insert(&mut self, domain: DomainId, capability: Capability, lineage: Vec<u64>) -> Handle {
        let slot = self.next_handle;
        self.next_handle += 1;
        self.records.insert(
            (domain, slot),
            Record {
                capability,
                lineage,
            },
        );
        Handle::encode(domain, slot, 1).expect("model slot remains representable")
    }
}
