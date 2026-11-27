//! Execution domain model (RFC-0007 Part I).
//!
//! This is a bounded executable host model of the RFC-0007 execution domain
//! semantics: the boundary at which capability enforcement is performed and
//! within which references are valid. It is deliberately **not** a production
//! domain manager, a hardware root of trust, or a bootstrap implementation.
//! RFC-0007 Part II (the 5-stage bootstrap protocol) is hardware/firmware
//! bound and is out of scope for this host model.
//!
//! Scope and declared architecture:
//!
//! - **Class:** execution-domain isolation and lifecycle management.
//! - **Domain model:** domains are the unit of capability isolation. Every
//!   object reference, capability, or address is valid only within the domain
//!   that created it or a domain to which it has been explicitly delegated
//!   (RFC-0007.1). Domains are hierarchical: a parent may create children,
//!   but children may not escape their parent's visibility unless explicitly
//!   granted capability access (RFC-0007.3).
//! - **Lifecycle:** `Created -> Active -> Destroyed`. `Created -> Active` is
//!   the only valid initial transition (RFC-0007.5). A domain transitions to
//!   `Destroyed` when all references to all objects within it are released.
//! - **Hierarchy integrity:** the parent/child graph is a DAG; creating a
//!   cycle is rejected (RFC-0007.3 invariant maintenance, RFC-0007.23).
//! - **Events:** `DomainCreated` on creation and `DomainDestroyed` on
//!   destruction (RFC-0007.6).
//!
//! Explicitly out of scope:
//! - RFC-0007 Part II bootstrap protocol (firmware root of trust, Minimum
//!   Executor, Capability Root, Composition Root).
//! - Hardware-attested root of trust and TPM/TrustZone integration.
//! - RFC-0013 mediation enforcement at the MMU/device boundary.
//! - RFC-0031 event delivery to external consumers (events are recorded
//!   locally).
//! - Any kernel or device-driver integration.

#![allow(clippy::new_without_default)]
#![allow(clippy::unused_self)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]

use aether_types::DomainId;
use std::collections::{HashMap, HashSet};

/// Lifecycle state of an execution domain (RFC-0007.5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainState {
    /// Domain has been created but not yet activated.
    Created,
    /// Domain is active and may host objects and capabilities.
    Active,
    /// Domain is destroyed; all references to its objects are released.
    Destroyed,
}

/// One observable domain event (RFC-0007.6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainEvent {
    /// A domain was created.
    DomainCreated {
        /// The newly created domain.
        domain: DomainId,
        /// Its parent domain, if any.
        parent: Option<DomainId>,
        /// Scheduling time at creation, in nanoseconds.
        timestamp_ns: u64,
    },
    /// A domain was destroyed.
    DomainDestroyed {
        /// The destroyed domain.
        domain: DomainId,
        /// Scheduling time at destruction, in nanoseconds.
        timestamp_ns: u64,
    },
}

/// Typed failures; the model never panics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainError {
    /// A domain with this identifier already exists.
    DuplicateDomain,
    /// No domain with this identifier exists.
    UnknownDomain,
    /// The requested lifecycle transition is not valid from the current state.
    InvalidTransition,
    /// The domain still has child domains; destroy the children first or
    /// explicitly orphan them (RFC-0007.7).
    HasChildren,
    /// Creating this domain would introduce a cycle in the parent/child DAG
    /// (RFC-0007.23).
    CycleDetected,
    /// The domain is not active; the requested operation requires an active
    /// domain.
    DomainNotActive,
    /// A cross-domain reference is not authorized; domain isolation
    /// violated (RFC-0007.1, RFC-0007.8).
    IsolationViolation,
}

/// One registered execution domain.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Domain {
    /// Unique, immutable domain identifier (RFC-0007.9).
    id: DomainId,
    /// Current lifecycle state.
    state: DomainState,
    /// Parent domain, if any. A domain has at most one parent.
    parent: Option<DomainId>,
    /// Child domains created by this domain.
    children: Vec<DomainId>,
    /// Scheduling time at which the domain entered its current state.
    state_since_ns: u64,
}

/// The execution-domain manager (RFC-0007 Part I).
///
/// Enforces domain isolation, domain hierarchy (DAG, no cycles), and the
/// domain lifecycle state machine. All identifiers are unique and immutable
/// (RFC-0007.9). Address–authority orthogonality (RFC-0007.4, I-13) is
/// preserved: domains own address spaces, and addresses are valid only
/// within their creating domain.
pub struct DomainManager {
    domains: HashMap<DomainId, Domain>,
    events: Vec<DomainEvent>,
}

impl DomainManager {
    /// Creates an empty domain manager with no domains.
    #[must_use]
    pub fn new() -> Self {
        Self {
            domains: HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Returns all domain events recorded so far, oldest first.
    #[must_use]
    pub fn events(&self) -> &[DomainEvent] {
        &self.events
    }

    /// Returns the number of domain events recorded.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns the number of registered domains.
    #[must_use]
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }

    /// Returns `true` if the domain exists and is active.
    #[must_use]
    pub fn is_active(&self, id: DomainId) -> bool {
        self.domains
            .get(&id)
            .map_or(false, |d| d.state == DomainState::Active)
    }

    /// Returns `true` if the domain exists.
    #[must_use]
    pub fn contains(&self, id: DomainId) -> bool {
        self.domains.contains_key(&id)
    }

    /// Returns the lifecycle state of the domain, or `UnknownDomain`.
    pub fn state(&self, id: DomainId) -> Result<DomainState, DomainError> {
        self.domains
            .get(&id)
            .map(|d| d.state)
            .ok_or(DomainError::UnknownDomain)
    }

    /// Returns the parent domain of `id`, if any, or `UnknownDomain`.
    pub fn parent_of(&self, id: DomainId) -> Result<Option<DomainId>, DomainError> {
        self.domains
            .get(&id)
            .map(|d| d.parent)
            .ok_or(DomainError::UnknownDomain)
    }

    /// Returns the child domains of `id`, or `UnknownDomain`.
    pub fn children_of(&self, id: DomainId) -> Result<Vec<DomainId>, DomainError> {
        self.domains
            .get(&id)
            .map(|d| d.children.clone())
            .ok_or(DomainError::UnknownDomain)
    }

    /// Creates a new domain in the `Created` state.
    ///
    /// If `parent` is `Some`, the parent must exist and adding `id` as a
    /// child must not introduce a cycle (RFC-0007.23). The new domain's
    /// parent records `id` in its children list (RFC-0007.3).
    ///
    /// # Errors
    /// Returns `DuplicateDomain` if `id` already exists, `UnknownDomain` if
    /// `parent` does not exist, or `CycleDetected` if `id` is an ancestor of
    /// `parent`.
    ///
    /// # Panics
    /// Panics if internal state is inconsistent (a parent that was validated
    /// to exist is missing when registering the child).
    pub fn create_domain(
        &mut self,
        id: DomainId,
        parent: Option<DomainId>,
        now_ns: u64,
    ) -> Result<(), DomainError> {
        if self.domains.contains_key(&id) {
            return Err(DomainError::DuplicateDomain);
        }
        if let Some(parent_id) = parent {
            if !self.domains.contains_key(&parent_id) {
                return Err(DomainError::UnknownDomain);
            }
            // Reject cycles: id must not be an ancestor of parent.
            if self.is_ancestor(id, parent_id)? {
                return Err(DomainError::CycleDetected);
            }
        }

        let domain = Domain {
            id,
            state: DomainState::Created,
            parent,
            children: Vec::new(),
            state_since_ns: now_ns,
        };
        self.domains.insert(id, domain);

        // Register the child with its parent (RFC-0007.3). The parent was
        // validated to exist above.
        if let Some(parent_id) = parent {
            self.domains
                .get_mut(&parent_id)
                .expect("parent domain was validated to exist")
                .children
                .push(id);
        }

        self.events.push(DomainEvent::DomainCreated {
            domain: id,
            parent,
            timestamp_ns: now_ns,
        });
        Ok(())
    }

    /// Activates a domain: `Created -> Active`. This is the only valid
    /// initial transition (RFC-0007.5).
    ///
    /// # Errors
    /// Returns `UnknownDomain` if the domain does not exist, or
    /// `InvalidTransition` if the domain is not in the `Created` state.
    pub fn activate(&mut self, id: DomainId, now_ns: u64) -> Result<(), DomainError> {
        let domain = self
            .domains
            .get_mut(&id)
            .ok_or(DomainError::UnknownDomain)?;
        if domain.state != DomainState::Created {
            return Err(DomainError::InvalidTransition);
        }
        domain.state = DomainState::Active;
        domain.state_since_ns = now_ns;
        Ok(())
    }

    /// Destroys a domain, transitioning it to `Destroyed`.
    ///
    /// A domain may be destroyed only if it has no child domains
    /// (RFC-0007.7): the parent must destroy its children first, or
    /// explicitly orphan them via `orphan_children` before destruction.
    ///
    /// # Errors
    /// Returns `UnknownDomain` if the domain does not exist,
    /// `InvalidTransition` if the domain is not `Active`, or `HasChildren`
    /// if the domain still has children.
    pub fn destroy(&mut self, id: DomainId, now_ns: u64) -> Result<(), DomainError> {
        let domain = self.domains.get(&id).ok_or(DomainError::UnknownDomain)?;
        if domain.state != DomainState::Active {
            return Err(DomainError::InvalidTransition);
        }
        if !domain.children.is_empty() {
            return Err(DomainError::HasChildren);
        }

        // Remove this domain from its parent's children list (RFC-0007.7).
        if let Some(parent_id) = domain.parent {
            if let Some(parent) = self.domains.get_mut(&parent_id) {
                parent.children.retain(|child| *child != id);
            }
        }

        self.domains.remove(&id);
        self.events.push(DomainEvent::DomainDestroyed {
            domain: id,
            timestamp_ns: now_ns,
        });
        Ok(())
    }

    /// Orphans all children of `parent` by clearing their parent pointer,
    /// transferring responsibility to a designated surrogate. This is the
    /// RFC-0007.7 alternative to destroying children before a parent.
    ///
    /// # Errors
    /// Returns `UnknownDomain` if `parent` does not exist.
    ///
    /// # Panics
    /// Panics if internal state is inconsistent (a child recorded in the
    /// parent's children list is missing from the domain table).
    pub fn orphan_children(&mut self, parent: DomainId) -> Result<Vec<DomainId>, DomainError> {
        let children = self
            .domains
            .get(&parent)
            .ok_or(DomainError::UnknownDomain)?
            .children
            .clone();
        for child in &children {
            self.domains
                .get_mut(child)
                .expect("recorded child domain was validated to exist")
                .parent = None;
        }
        self.domains
            .get_mut(&parent)
            .expect("parent domain was validated to exist")
            .children
            .clear();
        Ok(children)
    }

    /// Validates that a reference from `source_domain` to `target_domain` is
    /// permitted under domain isolation (RFC-0007.1). A domain may reference
    /// objects in itself or in a domain that explicitly delegated capability
    /// access; cross-domain references without explicit grant are an
    /// isolation violation.
    ///
    /// `granted` is the set of domains to which `source_domain` has been
    /// explicitly granted capability access. Returns `Ok(())` when the
    /// reference is permitted, `IsolationViolation` otherwise.
    pub fn check_isolation(
        &self,
        source_domain: DomainId,
        target_domain: DomainId,
        granted: &[DomainId],
    ) -> Result<(), DomainError> {
        // Same-domain references are always permitted (RFC-0007.1).
        if source_domain == target_domain {
            return Ok(());
        }
        // Both domains must exist.
        if !self.domains.contains_key(&source_domain) {
            return Err(DomainError::UnknownDomain);
        }
        if !self.domains.contains_key(&target_domain) {
            return Err(DomainError::UnknownDomain);
        }
        // Explicit grant permits cross-domain access (RFC-0007.1, RFC-0007.8).
        if granted.contains(&target_domain) {
            return Ok(());
        }
        Err(DomainError::IsolationViolation)
    }

    /// Returns `true` if `ancestor` is an ancestor of `descendant` in the
    /// parent/child graph.
    pub fn is_ancestor(
        &self,
        ancestor: DomainId,
        descendant: DomainId,
    ) -> Result<bool, DomainError> {
        let mut visited = HashSet::new();
        let mut current = Some(descendant);
        while let Some(id) = current {
            if !visited.insert(id) {
                // Defensive: should not happen in a DAG.
                break;
            }
            if id == ancestor {
                return Ok(true);
            }
            current = self.domains.get(&id).and_then(|d| d.parent);
        }
        Ok(false)
    }
}

impl Default for DomainManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain(value: u16) -> DomainId {
        DomainId::new(value).unwrap()
    }

    #[test]
    fn create_and_activate_domain() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("root domain creation should succeed");
        assert_eq!(manager.state(domain(1)), Ok(DomainState::Created));
        assert!(!manager.is_active(domain(1)));
        manager
            .activate(domain(1), 2_000)
            .expect("domain activation should succeed");
        assert_eq!(manager.state(domain(1)), Ok(DomainState::Active));
        assert!(manager.is_active(domain(1)));
    }

    #[test]
    fn duplicate_domain_is_rejected() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("first creation should succeed");
        assert_eq!(
            manager.create_domain(domain(1), None, 2_000),
            Err(DomainError::DuplicateDomain)
        );
    }

    #[test]
    fn child_domain_records_parent_and_hierarchy() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("root creation should succeed");
        manager
            .create_domain(domain(2), Some(domain(1)), 2_000)
            .expect("child creation should succeed");
        manager
            .create_domain(domain(3), Some(domain(2)), 3_000)
            .expect("grandchild creation should succeed");

        assert_eq!(manager.parent_of(domain(2)), Ok(Some(domain(1))));
        assert_eq!(manager.children_of(domain(1)), Ok(vec![domain(2)]));
        assert_eq!(manager.children_of(domain(2)), Ok(vec![domain(3)]));
    }

    #[test]
    fn deep_parent_child_chain_remains_a_dag() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("root creation should succeed");
        manager
            .create_domain(domain(2), Some(domain(1)), 2_000)
            .expect("child creation should succeed");
        manager
            .create_domain(domain(3), Some(domain(2)), 3_000)
            .expect("grandchild creation should succeed");
        manager
            .create_domain(domain(4), Some(domain(1)), 4_000)
            .expect("second child creation should succeed");

        // 1 is an ancestor of 2, 3, and 4; 2 is an ancestor of 3.
        assert!(manager.is_ancestor(domain(1), domain(2)).unwrap());
        assert!(manager.is_ancestor(domain(1), domain(3)).unwrap());
        assert!(manager.is_ancestor(domain(1), domain(4)).unwrap());
        assert!(manager.is_ancestor(domain(2), domain(3)).unwrap());
        assert!(!manager.is_ancestor(domain(3), domain(1)).unwrap());
        assert!(!manager.is_ancestor(domain(2), domain(4)).unwrap());
    }

    #[test]
    fn domain_lifecycle_created_to_active_to_destroyed() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("creation should succeed");
        manager
            .activate(domain(1), 2_000)
            .expect("activation should succeed");
        manager
            .destroy(domain(1), 3_000)
            .expect("destruction should succeed");
        assert_eq!(manager.state(domain(1)), Err(DomainError::UnknownDomain));
    }

    #[test]
    fn cannot_destroy_domain_without_activation() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("creation should succeed");
        // Created -> Destroyed directly is not the valid initial transition.
        assert_eq!(
            manager.destroy(domain(1), 2_000),
            Err(DomainError::InvalidTransition)
        );
    }

    #[test]
    fn cannot_destroy_parent_with_children() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("root creation should succeed");
        manager
            .activate(domain(1), 1_000)
            .expect("root activation should succeed");
        manager
            .create_domain(domain(2), Some(domain(1)), 2_000)
            .expect("child creation should succeed");
        manager
            .activate(domain(2), 2_000)
            .expect("child activation should succeed");
        // Parent cannot be destroyed while it has children.
        assert_eq!(
            manager.destroy(domain(1), 3_000),
            Err(DomainError::HasChildren)
        );
        // Destroy child first, then parent.
        manager
            .destroy(domain(2), 3_000)
            .expect("child destruction should succeed");
        manager
            .destroy(domain(1), 4_000)
            .expect("parent destruction should succeed");
    }

    #[test]
    fn orphaning_children_allows_parent_destruction() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("root creation should succeed");
        manager
            .activate(domain(1), 1_000)
            .expect("root activation should succeed");
        manager
            .create_domain(domain(2), Some(domain(1)), 2_000)
            .expect("child creation should succeed");
        manager
            .activate(domain(2), 2_000)
            .expect("child activation should succeed");
        // Orphan the child, then destroy the parent (RFC-0007.7).
        let orphans = manager
            .orphan_children(domain(1))
            .expect("orphaning should succeed");
        assert_eq!(orphans, vec![domain(2)]);
        assert_eq!(manager.children_of(domain(1)), Ok(Vec::new()));
        manager
            .destroy(domain(1), 3_000)
            .expect("parent destruction after orphan should succeed");
        // The orphaned child has no parent and can be re-parented or destroyed.
        assert_eq!(manager.parent_of(domain(2)), Ok(None));
    }

    #[test]
    fn same_domain_reference_is_permitted() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("creation should succeed");
        assert!(manager.check_isolation(domain(1), domain(1), &[]).is_ok());
    }

    #[test]
    fn cross_domain_reference_without_grant_is_rejected() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("domain 1 creation should succeed");
        manager
            .create_domain(domain(2), None, 1_000)
            .expect("domain 2 creation should succeed");
        assert_eq!(
            manager.check_isolation(domain(1), domain(2), &[]),
            Err(DomainError::IsolationViolation)
        );
    }

    #[test]
    fn cross_domain_reference_with_explicit_grant_is_permitted() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("domain 1 creation should succeed");
        manager
            .create_domain(domain(2), None, 1_000)
            .expect("domain 2 creation should succeed");
        assert!(
            manager
                .check_isolation(domain(1), domain(2), &[domain(2)])
                .is_ok()
        );
    }

    #[test]
    fn events_record_creation_and_destruction() {
        let mut manager = DomainManager::new();
        manager
            .create_domain(domain(1), None, 1_000)
            .expect("creation should succeed");
        manager
            .activate(domain(1), 2_000)
            .expect("activation should succeed");
        manager
            .destroy(domain(1), 3_000)
            .expect("destruction should succeed");
        assert_eq!(manager.event_count(), 2);
        let first = &manager.events()[0];
        match first {
            DomainEvent::DomainCreated {
                domain: d,
                parent: None,
                timestamp_ns: 1_000,
            } if *d == domain(1) => {}
            _ => panic!("expected DomainCreated for domain 1 at 1000 ns, got {first:?}"),
        }
        let last = &manager.events()[1];
        match last {
            DomainEvent::DomainDestroyed {
                domain: d,
                timestamp_ns: 3_000,
            } if *d == domain(1) => {}
            _ => panic!("expected DomainDestroyed for domain 1 at 3000 ns, got {last:?}"),
        }
    }
}
