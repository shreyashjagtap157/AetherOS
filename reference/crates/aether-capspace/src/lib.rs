//! Table-backed Software Capability Provider prototype.

use aether_types::{Bounds, CapError, Capability, DomainId, Handle, ObjectType, Rights};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
struct Entry {
    generation: u16,
    capability: Capability,
    lineage: Vec<u64>,
}

#[derive(Default)]
struct Space {
    slots: Vec<Option<Entry>>,
    generations: Vec<u16>,
}

/// Mutable provider owning every live capability table and revocation lineage.
#[derive(Default)]
pub struct CapabilitySystem {
    spaces: HashMap<DomainId, Space>,
    revoked: HashSet<u64>,
    next_lineage: u64,
}

/// Linear witness returned only when the explicit provider bootstrap begins.
pub struct BootstrapAuthority(());

impl CapabilitySystem {
    /// Creates an empty provider.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_lineage: 1,
            ..Self::default()
        }
    }

    /// Starts a fresh provider authority universe with its bootstrap witness.
    #[must_use]
    pub fn bootstrap() -> (Self, BootstrapAuthority) {
        (Self::new(), BootstrapAuthority(()))
    }

    /// Registers a domain. Duplicate registration is harmless.
    pub fn add_domain(&mut self, domain: DomainId) {
        self.spaces.entry(domain).or_default();
    }

    /// Creates an origin capability at the explicit bootstrap boundary.
    pub fn create_root(
        &mut self,
        _bootstrap: &BootstrapAuthority,
        domain: DomainId,
        capability: Capability,
    ) -> Result<Handle, CapError> {
        let lineage = self.alloc_lineage();
        self.insert(domain, capability, vec![lineage])
    }

    /// Derives a same-domain child.
    pub fn derive(
        &mut self,
        domain: DomainId,
        parent: Handle,
        rights: Rights,
        bounds: Bounds,
    ) -> Result<Handle, CapError> {
        let entry = self.resolve(domain, parent)?.clone();
        Self::check_narrowing(&entry.capability, rights, bounds)?;
        let mut lineage = entry.lineage;
        lineage.push(self.alloc_lineage());
        self.insert(
            domain,
            Capability {
                rights,
                bounds,
                ..entry.capability
            },
            lineage,
        )
    }

    /// Transfers an attenuated child into a destination capability space.
    pub fn transfer(
        &mut self,
        source: DomainId,
        parent: Handle,
        destination: DomainId,
        rights: Rights,
        bounds: Bounds,
    ) -> Result<Handle, CapError> {
        let entry = self.resolve(source, parent)?.clone();
        if !entry.capability.rights.contains(Rights::GRANT) {
            return Err(CapError::Rights);
        }
        Self::check_narrowing(&entry.capability, rights, bounds)?;
        let mut lineage = entry.lineage;
        lineage.push(self.alloc_lineage());
        self.insert(
            destination,
            Capability {
                rights,
                bounds,
                ..entry.capability
            },
            lineage,
        )
    }

    /// Marks this lineage revoked. Descendants fail because they retain ancestors.
    pub fn revoke(&mut self, domain: DomainId, handle: Handle) -> Result<(), CapError> {
        let lineage = *self
            .resolve(domain, handle)?
            .lineage
            .last()
            .ok_or(CapError::Missing)?;
        self.revoked.insert(lineage);
        Ok(())
    }

    /// Removes a local handle and advances the slot generation before reuse.
    pub fn remove(&mut self, domain: DomainId, handle: Handle) -> Result<(), CapError> {
        let (encoded_domain, slot, generation) =
            handle.decode().ok_or(CapError::MalformedHandle)?;
        if encoded_domain != domain {
            return Err(CapError::WrongDomain);
        }
        let space = self.spaces.get_mut(&domain).ok_or(CapError::Missing)?;
        let index = usize::try_from(slot).map_err(|_| CapError::Missing)?;
        let entry = space.slots.get(index).ok_or(CapError::Missing)?;
        let entry = entry.as_ref().ok_or_else(|| {
            if space.generations[index] == generation {
                CapError::Missing
            } else {
                CapError::Stale
            }
        })?;
        if entry.generation != generation {
            return Err(CapError::Stale);
        }
        space.slots[index] = None;
        space.generations[index] = next_generation(generation);
        Ok(())
    }

    /// Validates a live handle for a requested operation.
    pub fn authorize(
        &self,
        domain: DomainId,
        handle: Handle,
        object_type: ObjectType,
        rights: Rights,
        bounds: Bounds,
    ) -> Result<Capability, CapError> {
        let entry = self.resolve(domain, handle)?;
        if entry.lineage.iter().any(|id| self.revoked.contains(id)) {
            return Err(CapError::Revoked);
        }
        if entry.capability.object_type != object_type {
            return Err(CapError::Type);
        }
        if !entry.capability.rights.contains(rights) {
            return Err(CapError::Rights);
        }
        if !entry.capability.bounds.contains(bounds) {
            return Err(CapError::Bounds);
        }
        Ok(entry.capability)
    }

    fn resolve(&self, domain: DomainId, handle: Handle) -> Result<&Entry, CapError> {
        let (encoded_domain, slot, generation) =
            handle.decode().ok_or(CapError::MalformedHandle)?;
        if encoded_domain != domain {
            return Err(CapError::WrongDomain);
        }
        let space = self.spaces.get(&domain).ok_or(CapError::Missing)?;
        let index = usize::try_from(slot).map_err(|_| CapError::Missing)?;
        let entry = space.slots.get(index).ok_or(CapError::Missing)?;
        let entry = entry.as_ref().ok_or_else(|| {
            if space.generations[index] == generation {
                CapError::Missing
            } else {
                CapError::Stale
            }
        })?;
        if entry.generation != generation {
            return Err(CapError::Stale);
        }
        Ok(entry)
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

    fn insert(
        &mut self,
        domain: DomainId,
        capability: Capability,
        lineage: Vec<u64>,
    ) -> Result<Handle, CapError> {
        let space = self.spaces.get_mut(&domain).ok_or(CapError::Missing)?;
        let index = if let Some(index) =
            space.slots.iter().enumerate().find_map(|(index, entry)| {
                (entry.is_none() && space.generations[index] != 0).then_some(index)
            }) {
            index
        } else {
            if space.slots.len() > 0x00ff_ffff {
                return Err(CapError::Exhausted);
            }
            space.slots.push(None);
            space.generations.push(1);
            space.slots.len() - 1
        };
        let generation = space.generations[index];
        space.slots[index] = Some(Entry {
            generation,
            capability,
            lineage,
        });
        Handle::encode(
            domain,
            u32::try_from(index).map_err(|_| CapError::Exhausted)?,
            generation,
        )
        .ok_or(CapError::Exhausted)
    }
}

const fn next_generation(current: u16) -> u16 {
    if current == u16::MAX { 0 } else { current + 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_types::ObjectId;

    fn domain(value: u16) -> DomainId {
        DomainId::new(value).unwrap()
    }
    fn bounds(start: u64, end: u64) -> Bounds {
        Bounds::new(start, end).unwrap()
    }
    fn root() -> Capability {
        Capability {
            object: ObjectId(7),
            object_type: ObjectType::Memory,
            rights: Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            bounds: bounds(0, 4096),
        }
    }

    #[test]
    fn derivation_cannot_amplify_rights_or_bounds() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        let cap = system.create_root(&bootstrap, domain(1), root()).unwrap();
        assert_eq!(
            system.derive(domain(1), cap, Rights::INVOKE, bounds(0, 1)),
            Err(CapError::Rights)
        );
        assert_eq!(
            system.derive(domain(1), cap, Rights::READ, bounds(0, 8192)),
            Err(CapError::Bounds)
        );
    }

    #[test]
    fn stale_handle_never_resurrects_on_immediate_reuse() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        let old = system.create_root(&bootstrap, domain(1), root()).unwrap();
        system.remove(domain(1), old).unwrap();
        let new = system.create_root(&bootstrap, domain(1), root()).unwrap();
        assert_ne!(old, new);
        assert_eq!(
            system.authorize(
                domain(1),
                old,
                ObjectType::Memory,
                Rights::READ,
                bounds(0, 1)
            ),
            Err(CapError::Stale)
        );
    }

    #[test]
    fn revocation_crosses_transfer_boundary() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        system.add_domain(domain(2));
        let parent = system.create_root(&bootstrap, domain(1), root()).unwrap();
        let child = system
            .transfer(domain(1), parent, domain(2), Rights::READ, bounds(10, 20))
            .unwrap();
        assert!(
            system
                .authorize(
                    domain(2),
                    child,
                    ObjectType::Memory,
                    Rights::READ,
                    bounds(12, 15)
                )
                .is_ok()
        );
        system.revoke(domain(1), parent).unwrap();
        assert_eq!(
            system.authorize(
                domain(2),
                child,
                ObjectType::Memory,
                Rights::READ,
                bounds(12, 15)
            ),
            Err(CapError::Revoked)
        );
    }
}
