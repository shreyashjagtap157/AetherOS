//! Table-backed Software Capability Provider prototype.

/// Host reference model for capability-authorized address-space mappings.
pub mod memory;
/// Serialized host façade for capability and memory-mapping operations.
pub mod runtime;

use aether_types::{Bounds, CapError, Capability, DomainId, Handle, ObjectType, Rights};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct Entry {
    generation: u16,
    capability: Capability,
    lineage_id: u64,
}

/// One node in the capability derivation/transfer graph.
///
/// Descendant edges are retained after a local handle is removed while live
/// descendants still depend on the node for ancestor revocation.
#[derive(Default)]
struct RevocationNode {
    parent: Option<u64>,
    children: Vec<u64>,
    revoked: bool,
    live: bool,
}

#[derive(Default)]
struct Space {
    slots: Vec<Option<Entry>>,
    generations: Vec<u16>,
}

/// Mutable provider owning every live capability table and revocation lineage.
pub struct CapabilitySystem {
    universe: Arc<()>,
    spaces: HashMap<DomainId, Space>,
    revocation_nodes: HashMap<u64, RevocationNode>,
    next_lineage: u64,
}

/// Linear witness returned only when the explicit provider bootstrap begins.
pub struct BootstrapAuthority(Arc<()>);

impl Default for CapabilitySystem {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilitySystem {
    /// Creates an empty provider.
    #[must_use]
    pub fn new() -> Self {
        Self {
            universe: Arc::new(()),
            spaces: HashMap::new(),
            revocation_nodes: HashMap::new(),
            next_lineage: 1,
        }
    }

    /// Starts a fresh provider authority universe with its bootstrap witness.
    #[must_use]
    pub fn bootstrap() -> (Self, BootstrapAuthority) {
        let system = Self::new();
        let authority = BootstrapAuthority(Arc::clone(&system.universe));
        (system, authority)
    }

    /// Registers a domain. Duplicate registration is harmless.
    pub fn add_domain(&mut self, domain: DomainId) {
        self.spaces.entry(domain).or_default();
    }

    /// Creates an origin capability at the explicit bootstrap boundary.
    ///
    /// # Errors
    /// Returns `Missing` for an unregistered domain or `Exhausted` when no
    /// representable slot remains, and `WrongBootstrap` for a witness from a
    /// different provider universe.
    pub fn create_root(
        &mut self,
        bootstrap: &BootstrapAuthority,
        domain: DomainId,
        capability: Capability,
    ) -> Result<Handle, CapError> {
        if !Arc::ptr_eq(&bootstrap.0, &self.universe) {
            return Err(CapError::WrongBootstrap);
        }
        let lineage_id = self.alloc_lineage()?;
        self.insert(domain, capability, lineage_id, None)
    }

    /// Derives a same-domain child.
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
        let entry = self.resolve_live(domain, parent, now_ns)?.clone();
        Self::check_narrowing(&entry.capability, rights, bounds)?;
        let parent_lineage = entry.lineage_id;
        let lineage_id = self.alloc_lineage()?;
        self.insert(
            domain,
            Capability {
                rights,
                bounds,
                expires_at_ns: entry.capability.expires_at_ns,
                ..entry.capability
            },
            lineage_id,
            Some(parent_lineage),
        )
    }

    /// Transfers an attenuated child into a destination capability space.
    ///
    /// The child inherits the parent's `expires_at_ns`; transfer never
    /// extends lifetime (RFC-0037.14). A capability carrying `CAPF_BOUND`
    /// may not be transferred to any domain (RFC-0037.7, RFC-0037.15).
    ///
    /// # Errors
    /// Returns a typed failure if validation, expiry, attenuation, or insertion fails.
    pub fn transfer(
        &mut self,
        source: DomainId,
        parent: Handle,
        destination: DomainId,
        rights: Rights,
        bounds: Bounds,
        now_ns: u64,
    ) -> Result<Handle, CapError> {
        let entry = self.resolve_live(source, parent, now_ns)?.clone();
        if entry
            .capability
            .flags
            .contains(aether_types::CapabilityFlags::BOUND)
        {
            return Err(CapError::DomainBound);
        }
        if !entry.capability.rights.contains(Rights::GRANT) {
            return Err(CapError::Rights);
        }
        Self::check_narrowing(&entry.capability, rights, bounds)?;
        let parent_lineage = entry.lineage_id;
        let lineage_id = self.alloc_lineage()?;
        self.insert(
            destination,
            Capability {
                rights,
                bounds,
                expires_at_ns: entry.capability.expires_at_ns,
                ..entry.capability
            },
            lineage_id,
            Some(parent_lineage),
        )
    }

    /// Marks this lineage revoked. Descendants fail because they retain ancestors.
    ///
    /// # Errors
    /// Returns a typed failure if the handle cannot be resolved in the domain,
    /// or if the capability lacks `CAPF_REVOCABLE` (RFC-0039.2, EPERM).
    pub fn revoke(&mut self, domain: DomainId, handle: Handle) -> Result<(), CapError> {
        let entry = self.resolve(domain, handle)?;
        if !entry
            .capability
            .flags
            .contains(aether_types::CapabilityFlags::REVOCABLE)
        {
            return Err(CapError::RevocationDenied);
        }
        let lineage_id = entry.lineage_id;
        self.revoke_subtree(lineage_id)
    }

    /// Removes a local handle and advances the slot generation before reuse.
    ///
    /// # Errors
    /// Returns a typed failure for malformed, cross-domain, missing, or stale
    /// handles.
    pub fn remove(&mut self, domain: DomainId, handle: Handle) -> Result<(), CapError> {
        let (encoded_domain, slot, generation) =
            handle.decode().ok_or(CapError::MalformedHandle)?;
        if encoded_domain != domain {
            return Err(CapError::WrongDomain);
        }
        let lineage_id = {
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

            let lineage_id = entry.lineage_id;
            if !self.revocation_nodes.contains_key(&lineage_id) {
                return Err(CapError::Missing);
            }
            space.slots[index] = None;
            space.generations[index] = next_generation(generation);
            lineage_id
        };
        self.retire_lineage(lineage_id)
    }

    /// Validates a live handle for a requested operation.
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
        let entry = self.resolve(domain, handle)?;
        Self::check_lifetime(&entry.capability, now_ns)?;
        if self.is_revoked(entry.lineage_id)? {
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

    fn resolve_live(
        &self,
        domain: DomainId,
        handle: Handle,
        now_ns: u64,
    ) -> Result<&Entry, CapError> {
        let entry = self.resolve(domain, handle)?;
        Self::check_lifetime(&entry.capability, now_ns)?;
        if self.is_revoked(entry.lineage_id)? {
            return Err(CapError::Revoked);
        }
        Ok(entry)
    }

    const fn check_lifetime(capability: &Capability, now_ns: u64) -> Result<(), CapError> {
        if capability.expires_at_ns != 0 && now_ns > capability.expires_at_ns {
            return Err(CapError::Expired);
        }
        Ok(())
    }

    fn is_revoked(&self, lineage_id: u64) -> Result<bool, CapError> {
        self.revocation_nodes
            .get(&lineage_id)
            .map(|node| node.revoked)
            .ok_or(CapError::Missing)
    }

    /// Eagerly marks a capability and its complete derivation/transfer subtree.
    ///
    /// The closure is collected and validated before any node is mutated, so a
    /// missing graph node cannot leave a partially applied revocation.
    /// Authorization checks one node instead of walking every ancestor.
    /// Revocation work is proportional to the affected subtree and is
    /// deliberately iterative to avoid recursive stack growth.
    fn revoke_subtree(&mut self, root: u64) -> Result<(), CapError> {
        let mut pending = vec![root];
        let mut affected = HashSet::new();

        while let Some(lineage_id) = pending.pop() {
            if !affected.insert(lineage_id) {
                continue;
            }
            let node = self
                .revocation_nodes
                .get(&lineage_id)
                .ok_or(CapError::Missing)?;
            pending.extend(node.children.iter().copied());
        }

        for lineage_id in affected {
            let node = self
                .revocation_nodes
                .get_mut(&lineage_id)
                .ok_or(CapError::Missing)?;
            node.revoked = true;
        }
        Ok(())
    }

    /// Retires lineage metadata when a handle is removed, preserving any
    /// ancestor node still required by live descendants.
    fn retire_lineage(&mut self, lineage_id: u64) -> Result<(), CapError> {
        self.revocation_nodes
            .get_mut(&lineage_id)
            .ok_or(CapError::Missing)?
            .live = false;

        let mut candidate = Some(lineage_id);
        while let Some(current) = candidate {
            let (parent, can_prune) = {
                let node = self
                    .revocation_nodes
                    .get(&current)
                    .ok_or(CapError::Missing)?;
                (node.parent, !node.live && node.children.is_empty())
            };
            if !can_prune {
                break;
            }

            self.revocation_nodes
                .remove(&current)
                .ok_or(CapError::Missing)?;
            if let Some(parent_id) = parent {
                self.revocation_nodes
                    .get_mut(&parent_id)
                    .ok_or(CapError::Missing)?
                    .children
                    .retain(|child| *child != current);
                candidate = Some(parent_id);
            } else {
                candidate = None;
            }
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
        lineage_id: u64,
        parent_lineage: Option<u64>,
    ) -> Result<Handle, CapError> {
        if self.revocation_nodes.contains_key(&lineage_id) {
            return Err(CapError::Exhausted);
        }
        if parent_lineage.is_some_and(|parent| !self.revocation_nodes.contains_key(&parent)) {
            return Err(CapError::Missing);
        }

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
        let handle = Handle::encode(
            domain,
            u32::try_from(index).map_err(|_| CapError::Exhausted)?,
            generation,
        )
        .ok_or(CapError::Exhausted)?;

        space.slots[index] = Some(Entry {
            generation,
            capability,
            lineage_id,
        });

        self.revocation_nodes.insert(
            lineage_id,
            RevocationNode {
                parent: parent_lineage,
                live: true,
                ..RevocationNode::default()
            },
        );
        if let Some(parent) = parent_lineage {
            self.revocation_nodes
                .get_mut(&parent)
                .ok_or(CapError::Missing)?
                .children
                .push(lineage_id);
        }

        Ok(handle)
    }
}

const fn next_generation(current: u16) -> u16 {
    if current == u16::MAX { 0 } else { current + 1 }
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
    fn root() -> Capability {
        Capability {
            object: ObjectId(7),
            object_type: ObjectType::Memory,
            rights: Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            bounds: bounds(0, 4096),
            flags: CapabilityFlags::NONE,
            expires_at_ns: 0,
        }
    }

    /// A root capability carrying `CAPF_REVOCABLE`, so revocation succeeds.
    fn revocable_root() -> Capability {
        Capability {
            flags: CapabilityFlags::REVOCABLE,
            ..root()
        }
    }
    fn root_with_expiry(expires_at_ns: u64) -> Capability {
        Capability {
            expires_at_ns,
            ..root()
        }
    }

    #[test]
    fn derivation_cannot_amplify_rights_or_bounds() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        let cap = system.create_root(&bootstrap, domain(1), root()).unwrap();
        assert_eq!(
            system.derive(domain(1), cap, Rights::INVOKE, bounds(0, 1), NOW_NS),
            Err(CapError::Rights)
        );
        assert_eq!(
            system.derive(domain(1), cap, Rights::READ, bounds(0, 8192), NOW_NS),
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
                bounds(0, 1),
                NOW_NS
            ),
            Err(CapError::Stale)
        );
    }

    #[test]
    fn revocation_crosses_transfer_boundary() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        system.add_domain(domain(2));
        let parent = system
            .create_root(&bootstrap, domain(1), revocable_root())
            .unwrap();
        let child = system
            .transfer(
                domain(1),
                parent,
                domain(2),
                Rights::READ,
                bounds(10, 20),
                NOW_NS,
            )
            .unwrap();
        assert!(
            system
                .authorize(
                    domain(2),
                    child,
                    ObjectType::Memory,
                    Rights::READ,
                    bounds(12, 15),
                    NOW_NS
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
                bounds(12, 15),
                NOW_NS
            ),
            Err(CapError::Revoked)
        );
    }

    #[test]
    fn revoking_a_child_invalidates_only_its_own_subtree() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        system.add_domain(domain(2));

        let root = system
            .create_root(&bootstrap, domain(1), revocable_root())
            .unwrap();
        let branch = system
            .derive(
                domain(1),
                root,
                Rights::READ.union(Rights::GRANT),
                bounds(0, 4096),
                NOW_NS,
            )
            .unwrap();
        let descendant = system
            .derive(domain(1), branch, Rights::READ, bounds(0, 1024), NOW_NS)
            .unwrap();
        let transferred = system
            .transfer(
                domain(1),
                branch,
                domain(2),
                Rights::READ,
                bounds(0, 512),
                NOW_NS,
            )
            .unwrap();
        let sibling = system
            .derive(domain(1), root, Rights::READ, bounds(1024, 2048), NOW_NS)
            .unwrap();

        system.revoke(domain(1), branch).unwrap();

        assert_eq!(
            system.authorize(
                domain(1),
                branch,
                ObjectType::Memory,
                Rights::READ,
                bounds(0, 1),
                NOW_NS
            ),
            Err(CapError::Revoked)
        );
        assert_eq!(
            system.authorize(
                domain(1),
                descendant,
                ObjectType::Memory,
                Rights::READ,
                bounds(0, 1),
                NOW_NS
            ),
            Err(CapError::Revoked)
        );
        assert_eq!(
            system.authorize(
                domain(2),
                transferred,
                ObjectType::Memory,
                Rights::READ,
                bounds(0, 1),
                NOW_NS
            ),
            Err(CapError::Revoked)
        );
        assert!(
            system
                .authorize(
                    domain(1),
                    root,
                    ObjectType::Memory,
                    Rights::READ,
                    bounds(0, 1),
                    NOW_NS
                )
                .is_ok()
        );
        assert!(
            system
                .authorize(
                    domain(1),
                    sibling,
                    ObjectType::Memory,
                    Rights::READ,
                    bounds(1024, 1025),
                    NOW_NS
                )
                .is_ok()
        );
    }

    #[test]
    fn removed_intermediate_handle_remains_in_ancestor_revocation_graph() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        let root = system
            .create_root(&bootstrap, domain(1), revocable_root())
            .unwrap();
        let intermediate = system
            .derive(
                domain(1),
                root,
                Rights::READ.union(Rights::GRANT),
                bounds(0, 4096),
                NOW_NS,
            )
            .unwrap();
        let intermediate_lineage = system.resolve(domain(1), intermediate).unwrap().lineage_id;
        let descendant = system
            .derive(
                domain(1),
                intermediate,
                Rights::READ,
                bounds(0, 1024),
                NOW_NS,
            )
            .unwrap();

        system.remove(domain(1), intermediate).unwrap();
        assert!(system.revocation_nodes.contains_key(&intermediate_lineage));
        system.revoke(domain(1), root).unwrap();

        assert_eq!(
            system.authorize(
                domain(1),
                descendant,
                ObjectType::Memory,
                Rights::READ,
                bounds(0, 1),
                NOW_NS
            ),
            Err(CapError::Revoked)
        );
    }

    #[test]
    fn retired_lineage_metadata_is_reclaimed_after_last_descendant() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        let root = system.create_root(&bootstrap, domain(1), root()).unwrap();
        let intermediate = system
            .derive(
                domain(1),
                root,
                Rights::READ.union(Rights::GRANT),
                bounds(0, 4096),
                NOW_NS,
            )
            .unwrap();
        let descendant = system
            .derive(
                domain(1),
                intermediate,
                Rights::READ,
                bounds(0, 1024),
                NOW_NS,
            )
            .unwrap();

        assert_eq!(system.revocation_nodes.len(), 3);
        system.remove(domain(1), intermediate).unwrap();
        assert_eq!(system.revocation_nodes.len(), 3);
        system.remove(domain(1), descendant).unwrap();
        assert_eq!(system.revocation_nodes.len(), 1);
        system.remove(domain(1), root).unwrap();
        assert!(system.revocation_nodes.is_empty());
    }

    #[test]
    fn deep_derivation_revocation_uses_iterative_graph_walk() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        let mut chain = vec![
            system
                .create_root(&bootstrap, domain(1), revocable_root())
                .unwrap(),
        ];

        for _ in 0..128 {
            let parent = *chain.last().unwrap();
            let child = system
                .derive(
                    domain(1),
                    parent,
                    Rights::READ.union(Rights::GRANT),
                    bounds(0, 4096),
                    NOW_NS,
                )
                .unwrap();
            chain.push(child);
        }

        let revoke_at = 64;
        system.revoke(domain(1), chain[revoke_at]).unwrap();

        for (index, handle) in chain.iter().copied().enumerate() {
            let requested_rights = if index == 0 {
                Rights::READ
            } else {
                Rights::READ.union(Rights::GRANT)
            };
            let result = system.authorize(
                domain(1),
                handle,
                ObjectType::Memory,
                requested_rights,
                bounds(0, 1),
                NOW_NS,
            );
            if index < revoke_at {
                assert!(result.is_ok(), "ancestor {index} was unexpectedly revoked");
            } else {
                assert_eq!(
                    result,
                    Err(CapError::Revoked),
                    "descendant {index} survived"
                );
            }
        }
    }

    #[test]
    fn bootstrap_witness_is_bound_to_its_provider() {
        let (mut first, first_bootstrap) = CapabilitySystem::bootstrap();
        let (mut second, second_bootstrap) = CapabilitySystem::bootstrap();
        first.add_domain(domain(1));
        second.add_domain(domain(1));
        assert_eq!(
            first.create_root(&second_bootstrap, domain(1), root()),
            Err(CapError::WrongBootstrap)
        );
        assert!(
            first
                .create_root(&first_bootstrap, domain(1), root())
                .is_ok()
        );
        assert!(
            second
                .create_root(&second_bootstrap, domain(1), root())
                .is_ok()
        );
    }

    #[test]
    fn lineage_exhaustion_fails_closed() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        system.next_lineage = u64::MAX;
        assert_eq!(
            system.create_root(&bootstrap, domain(1), root()),
            Err(CapError::Exhausted)
        );
    }

    #[test]
    fn revoked_parent_cannot_derive_or_transfer() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        system.add_domain(domain(2));
        let parent = system
            .create_root(&bootstrap, domain(1), revocable_root())
            .unwrap();
        system.revoke(domain(1), parent).unwrap();
        assert_eq!(
            system.derive(domain(1), parent, Rights::READ, bounds(0, 1), NOW_NS),
            Err(CapError::Revoked)
        );
        assert_eq!(
            system.transfer(
                domain(1),
                parent,
                domain(2),
                Rights::READ,
                bounds(0, 1),
                NOW_NS
            ),
            Err(CapError::Revoked)
        );
    }

    #[test]
    fn expiry_boundary_is_strictly_greater_than() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        let cap = root_with_expiry(1_000);
        let cap_bounds = cap.bounds;
        let handle = system.create_root(&bootstrap, domain(1), cap).unwrap();

        // now == expires_at_ns: still valid (RFC-0037.14 "exceeds").
        assert!(
            system
                .authorize(
                    domain(1),
                    handle,
                    ObjectType::Memory,
                    Rights::READ,
                    cap_bounds,
                    1_000
                )
                .is_ok()
        );
        // now == expires_at_ns + 1: first expired instant.
        assert_eq!(
            system.authorize(
                domain(1),
                handle,
                ObjectType::Memory,
                Rights::READ,
                cap_bounds,
                1_001
            ),
            Err(CapError::Expired)
        );
    }

    #[test]
    fn zero_expiry_means_never_expires() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        let handle = system.create_root(&bootstrap, domain(1), root()).unwrap();
        assert!(
            system
                .authorize(
                    domain(1),
                    handle,
                    ObjectType::Memory,
                    Rights::READ,
                    bounds(0, 1),
                    u64::MAX
                )
                .is_ok()
        );
    }

    #[test]
    fn expired_parent_cannot_derive_or_transfer() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        system.add_domain(domain(2));
        let cap = root_with_expiry(1_000);
        let handle = system.create_root(&bootstrap, domain(1), cap).unwrap();
        assert_eq!(
            system.derive(domain(1), handle, Rights::READ, bounds(0, 1), 1_001),
            Err(CapError::Expired)
        );
        assert_eq!(
            system.transfer(
                domain(1),
                handle,
                domain(2),
                Rights::READ,
                bounds(0, 1),
                1_001
            ),
            Err(CapError::Expired)
        );
    }

    #[test]
    fn derived_and_transferred_children_inherit_parent_expiry() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        system.add_domain(domain(2));
        let cap = root_with_expiry(5_000);
        let handle = system.create_root(&bootstrap, domain(1), cap).unwrap();
        let now = 1_000;
        let child_bounds = bounds(0, 4);
        let derived = system
            .derive(domain(1), handle, Rights::READ, child_bounds, now)
            .unwrap();
        let transferred = system
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
                system
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
                system.authorize(
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
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        system.add_domain(domain(2));
        let cap = Capability {
            flags: CapabilityFlags::BOUND,
            ..root()
        };
        let handle = system.create_root(&bootstrap, domain(1), cap).unwrap();

        // Same-domain use remains valid.
        assert!(
            system
                .authorize(
                    domain(1),
                    handle,
                    ObjectType::Memory,
                    Rights::READ,
                    bounds(0, 1),
                    NOW_NS
                )
                .is_ok()
        );
        // Cross-domain transfer is refused.
        assert_eq!(
            system.transfer(
                domain(1),
                handle,
                domain(2),
                Rights::READ,
                bounds(0, 1),
                NOW_NS
            ),
            Err(CapError::DomainBound)
        );
        // Cross-domain presentation fails closed even without transfer.
        assert_eq!(
            system.authorize(
                domain(2),
                handle,
                ObjectType::Memory,
                Rights::READ,
                bounds(0, 1),
                NOW_NS
            ),
            Err(CapError::WrongDomain)
        );
        // Derivation inside the domain is still permitted.
        assert!(
            system
                .derive(domain(1), handle, Rights::READ, bounds(0, 1), NOW_NS)
                .is_ok()
        );
    }

    #[test]
    fn derived_child_of_bound_parent_is_also_bound() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        system.add_domain(domain(2));
        let cap = Capability {
            flags: CapabilityFlags::BOUND,
            ..root()
        };
        let handle = system.create_root(&bootstrap, domain(1), cap).unwrap();
        let child = system
            .derive(
                domain(1),
                handle,
                Rights::READ.union(Rights::GRANT),
                bounds(0, 4096),
                NOW_NS,
            )
            .unwrap();
        assert_eq!(
            system.transfer(
                domain(1),
                child,
                domain(2),
                Rights::READ,
                bounds(0, 1),
                NOW_NS
            ),
            Err(CapError::DomainBound)
        );
    }

    #[test]
    fn revocation_requires_capf_revocable_flag() {
        let (mut system, bootstrap) = CapabilitySystem::bootstrap();
        system.add_domain(domain(1));
        system.add_domain(domain(2));

        // A capability without CAPF_REVOCABLE cannot be revoked at all.
        let non_revocable = system.create_root(&bootstrap, domain(1), root()).unwrap();
        assert_eq!(
            system.revoke(domain(1), non_revocable),
            Err(CapError::RevocationDenied)
        );
        // A transferred copy of a non-revocable capability is also not revocable.
        let transferred = system
            .transfer(
                domain(1),
                non_revocable,
                domain(2),
                Rights::READ,
                bounds(0, 1),
                NOW_NS,
            )
            .unwrap();
        assert_eq!(
            system.revoke(domain(2), transferred),
            Err(CapError::RevocationDenied)
        );
        // No state change: both capabilities still authorize.
        assert!(
            system
                .authorize(
                    domain(1),
                    non_revocable,
                    ObjectType::Memory,
                    Rights::READ,
                    bounds(0, 1),
                    NOW_NS
                )
                .is_ok()
        );
        assert!(
            system
                .authorize(
                    domain(2),
                    transferred,
                    ObjectType::Memory,
                    Rights::READ,
                    bounds(0, 1),
                    NOW_NS
                )
                .is_ok()
        );

        // A capability carrying CAPF_REVOCABLE revokes successfully.
        let revocable = system
            .create_root(&bootstrap, domain(1), revocable_root())
            .unwrap();
        system.revoke(domain(1), revocable).unwrap();
        assert_eq!(
            system.authorize(
                domain(1),
                revocable,
                ObjectType::Memory,
                Rights::READ,
                bounds(0, 1),
                NOW_NS
            ),
            Err(CapError::Revoked)
        );
    }
}
