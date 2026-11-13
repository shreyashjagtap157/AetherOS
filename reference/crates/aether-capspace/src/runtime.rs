//! Serialized host façade joining capability authority and address-space state.
//!
//! All operations pass through one mutex protecting both the capability
//! provider and the mapping manager. This provides a single host-side
//! linearization order for access, mapping, dispatch, removal, and revocation.
//! It is a reference/testing boundary only: holding a blocking mutex across
//! fence callbacks is not a production-kernel dispatch strategy. Fence
//! backends must not re-enter this runtime.

use crate::memory::{
    AddressSpaceId, HartId, MapRequest, MappingId, MemoryAccess, MemoryError, MemoryManager,
    RevocationReport, TlbShootdown, TranslatedAccess,
};
use crate::{BootstrapAuthority, CapabilitySystem};
use aether_types::{Bounds, CapError, Capability, DomainId, Handle, Rights};
use std::sync::{Mutex, MutexGuard};

struct RuntimeState {
    capabilities: CapabilitySystem,
    memory: MemoryManager,
}

/// Bootstrap witness scoped to one serialized MemoryRuntime authority universe.
pub struct RuntimeBootstrapAuthority(BootstrapAuthority);

/// Failures produced by the serialized host runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    /// The runtime mutex was poisoned by a panic while state was locked.
    LockPoisoned,
    /// Capability validation or mutation failed.
    Capability(CapError),
    /// Address-space mapping or translation failed.
    Memory(MemoryError),
}

impl From<CapError> for RuntimeError {
    fn from(value: CapError) -> Self {
        Self::Capability(value)
    }
}

impl From<MemoryError> for RuntimeError {
    fn from(value: MemoryError) -> Self {
        Self::Memory(value)
    }
}

/// Serializes the host capability provider and its address-space mappings.
///
/// The runtime deliberately owns both subsystems. Callers cannot obtain a
/// mutable reference to either internal manager, so all supported capability
/// mutations and memory operations share one lock and one linearization order.
///
/// This is stronger than the lower-level MemoryManager API, which remains
/// available for focused single-threaded provider tests. Applications using
/// that lower-level API directly do not inherit this runtime's serialization
/// guarantee.
pub struct MemoryRuntime {
    state: Mutex<RuntimeState>,
}

impl MemoryRuntime {
    /// Creates an empty runtime and its one-time bootstrap witness.
    #[must_use]
    pub fn bootstrap() -> (Self, RuntimeBootstrapAuthority) {
        let (capabilities, authority) = CapabilitySystem::bootstrap();
        (
            Self {
                state: Mutex::new(RuntimeState {
                    capabilities,
                    memory: MemoryManager::new(),
                }),
            },
            RuntimeBootstrapAuthority(authority),
        )
    }

    fn lock(&self) -> Result<MutexGuard<'_, RuntimeState>, RuntimeError> {
        self.state.lock().map_err(|_| RuntimeError::LockPoisoned)
    }

    /// Registers a domain and its unique address-space identifier atomically
    /// with respect to all other runtime operations.
    ///
    /// # Errors
    /// Returns an address-space registration error or a poisoned-lock error.
    pub fn register_domain(
        &self,
        domain: DomainId,
        address_space: AddressSpaceId,
    ) -> Result<(), RuntimeError> {
        let mut state = self.lock()?;
        state.memory.register_address_space(domain, address_space)?;
        state.capabilities.add_domain(domain);
        Ok(())
    }

    /// Creates a root capability using this runtime's bootstrap witness.
    ///
    /// # Errors
    /// Returns a capability validation/insertion error or a poisoned-lock error.
    pub fn create_root(
        &self,
        bootstrap: &RuntimeBootstrapAuthority,
        domain: DomainId,
        capability: Capability,
    ) -> Result<Handle, RuntimeError> {
        let mut state = self.lock()?;
        Ok(state
            .capabilities
            .create_root(&bootstrap.0, domain, capability)?)
    }

    /// Derives an attenuated capability within one domain.
    ///
    /// # Errors
    /// Returns a capability validation/insertion error or a poisoned-lock error.
    pub fn derive(
        &self,
        domain: DomainId,
        parent: Handle,
        rights: Rights,
        bounds: Bounds,
    ) -> Result<Handle, RuntimeError> {
        let mut state = self.lock()?;
        Ok(state.capabilities.derive(domain, parent, rights, bounds)?)
    }

    /// Transfers an attenuated capability into another registered domain.
    ///
    /// # Errors
    /// Returns a capability validation/insertion error or a poisoned-lock error.
    pub fn transfer(
        &self,
        source: DomainId,
        parent: Handle,
        destination: DomainId,
        rights: Rights,
        bounds: Bounds,
    ) -> Result<Handle, RuntimeError> {
        let mut state = self.lock()?;
        Ok(state
            .capabilities
            .transfer(source, parent, destination, rights, bounds)?)
    }

    /// Activates a domain's address space at the serialized dispatch boundary.
    ///
    /// The caller must not release a thread to execution until this method
    /// returns successfully.
    ///
    /// # Errors
    /// Returns an address-space activation/shootdown error or a poisoned-lock
    /// error.
    pub fn activate_for_dispatch(
        &self,
        domain: DomainId,
        hart: HartId,
        fences: &mut impl TlbShootdown,
    ) -> Result<AddressSpaceId, RuntimeError> {
        let mut state = self.lock()?;
        Ok(state.memory.activate_for_dispatch(domain, hart, fences)?)
    }

    /// Creates a capability-authorized mapping under the same serialization
    /// boundary used by access and revocation.
    ///
    /// # Errors
    /// Returns a capability, mapping, shootdown, or poisoned-lock error.
    pub fn map(
        &self,
        domain: DomainId,
        handle: Handle,
        request: MapRequest,
        fences: &mut impl TlbShootdown,
    ) -> Result<MappingId, RuntimeError> {
        let mut state = self.lock()?;
        let RuntimeState {
            capabilities,
            memory,
        } = &mut *state;
        Ok(memory.map(capabilities, domain, handle, request, fences)?)
    }

    /// Performs a memory access with live capability revalidation and PTE
    /// permission enforcement while excluding concurrent runtime mutations.
    ///
    /// # Errors
    /// Returns an authority, translation, permission, shootdown, or poisoned-lock
    /// error.
    pub fn access(
        &self,
        hart: HartId,
        address_space: AddressSpaceId,
        virtual_address: u64,
        operation: MemoryAccess,
    ) -> Result<TranslatedAccess, RuntimeError> {
        let mut state = self.lock()?;
        let RuntimeState {
            capabilities,
            memory,
        } = &mut *state;
        Ok(memory.access(
            capabilities,
            hart,
            address_space,
            virtual_address,
            operation,
        )?)
    }

    /// Unmaps a mapping while excluding concurrent access and dispatch.
    ///
    /// # Errors
    /// Returns an authority, mapping, shootdown, or poisoned-lock error.
    pub fn unmap(
        &self,
        domain: DomainId,
        handle: Handle,
        mapping: MappingId,
        fences: &mut impl TlbShootdown,
    ) -> Result<(), RuntimeError> {
        let mut state = self.lock()?;
        let RuntimeState {
            capabilities,
            memory,
        } = &mut *state;
        Ok(memory.unmap(capabilities, domain, handle, mapping, fences)?)
    }

    /// Removes one local capability and all mappings created directly from it.
    ///
    /// # Errors
    /// Returns a capability, mapping, shootdown, or poisoned-lock error.
    pub fn remove_capability(
        &self,
        domain: DomainId,
        handle: Handle,
        fences: &mut impl TlbShootdown,
    ) -> Result<usize, RuntimeError> {
        let mut state = self.lock()?;
        let RuntimeState {
            capabilities,
            memory,
        } = &mut *state;
        Ok(memory.remove_capability(capabilities, domain, handle, fences)?)
    }

    /// Revokes a capability subtree and invalidates all mappings that lose
    /// authority, in one serialized host operation.
    ///
    /// # Errors
    /// Returns a capability, shootdown, or poisoned-lock error.
    pub fn revoke_capability(
        &self,
        domain: DomainId,
        handle: Handle,
        fences: &mut impl TlbShootdown,
    ) -> Result<RevocationReport, RuntimeError> {
        let mut state = self.lock()?;
        let RuntimeState {
            capabilities,
            memory,
        } = &mut *state;
        Ok(memory.revoke_capability(capabilities, domain, handle, fences)?)
    }

    /// Retries every pending translation invalidation while excluding access
    /// and dispatch through this runtime.
    ///
    /// # Errors
    /// Returns ShootdownPending until every required acknowledgement arrives,
    /// or LockPoisoned if the runtime mutex is poisoned.
    pub fn retry_pending_shootdowns(
        &self,
        fences: &mut impl TlbShootdown,
    ) -> Result<(), RuntimeError> {
        let mut state = self.lock()?;
        Ok(state.memory.retry_pending_shootdowns(fences)?)
    }

    /// Returns the number of mappings awaiting invalidation.
    ///
    /// # Errors
    /// Returns LockPoisoned if the runtime mutex is poisoned.
    pub fn pending_invalidation_count(&self) -> Result<usize, RuntimeError> {
        let state = self.lock()?;
        Ok(state.memory.pending_invalidation_count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{PageSize, PtePermissions, ShootdownError};
    use aether_types::ObjectType;
    use std::sync::{Arc, Barrier};
    use std::thread;

    struct ImmediateFences;

    impl TlbShootdown for ImmediateFences {
        fn current_hart(&self) -> HartId {
            HartId::new(0)
        }

        fn activate_address_space_and_wait(
            &mut self,
            _hart: HartId,
            _address_space: AddressSpaceId,
        ) -> Result<(), ShootdownError> {
            Ok(())
        }

        fn publish_page_table_updates(
            &mut self,
            _address_space: AddressSpaceId,
            _virtual_range: Bounds,
        ) -> Result<(), ShootdownError> {
            Ok(())
        }

        fn invalidate_hart_and_wait(
            &mut self,
            _hart: HartId,
            _address_space: AddressSpaceId,
            _virtual_range: Bounds,
        ) -> Result<(), ShootdownError> {
            Ok(())
        }
    }

    fn domain(value: u16) -> DomainId {
        DomainId::new(value).expect("test domain must be non-zero")
    }

    fn bounds(start: u64, end: u64) -> Bounds {
        Bounds::new(start, end).expect("test bounds must be non-empty")
    }

    fn prepared_runtime() -> (Arc<MemoryRuntime>, Handle, AddressSpaceId, HartId) {
        let (runtime, bootstrap) = MemoryRuntime::bootstrap();
        let runtime = Arc::new(runtime);
        let owner = domain(1);
        let address_space = AddressSpaceId::new(23);
        runtime
            .register_domain(owner, address_space)
            .expect("domain registration should succeed");
        let handle = runtime
            .create_root(
                &bootstrap,
                owner,
                Capability {
                    object: aether_types::ObjectId(123),
                    object_type: ObjectType::Memory,
                    rights: Rights::MAP.union(Rights::READ).union(Rights::GRANT),
                    bounds: bounds(0, 4096),
                },
            )
            .expect("root capability should be created");
        let hart = HartId::new(3);
        runtime
            .activate_for_dispatch(owner, hart, &mut ImmediateFences)
            .expect("address space should activate");
        runtime
            .map(
                owner,
                handle,
                MapRequest {
                    address_space,
                    virtual_range: bounds(0x8000, 0x9000),
                    resource_range: bounds(0, 4096),
                    page_size: PageSize::Base4KiB,
                    permissions: PtePermissions::READ,
                },
                &mut ImmediateFences,
            )
            .expect("mapping should install");
        (runtime, handle, address_space, hart)
    }

    #[test]
    fn serialized_runtime_rejects_access_after_concurrent_revocation() {
        let (runtime, handle, address_space, hart) = prepared_runtime();
        assert!(
            runtime
                .access(hart, address_space, 0x8000, MemoryAccess::Read)
                .is_ok()
        );

        let barrier = Arc::new(Barrier::new(3));
        let reader_runtime = Arc::clone(&runtime);
        let reader_barrier = Arc::clone(&barrier);
        let reader = thread::spawn(move || {
            reader_barrier.wait();
            reader_runtime.access(hart, address_space, 0x8000, MemoryAccess::Read)
        });

        let revoker_runtime = Arc::clone(&runtime);
        let revoker_barrier = Arc::clone(&barrier);
        let revoker = thread::spawn(move || {
            revoker_barrier.wait();
            revoker_runtime.revoke_capability(domain(1), handle, &mut ImmediateFences)
        });

        barrier.wait();
        let racing_access = reader.join().expect("reader thread must not panic");
        let revoke_result = revoker.join().expect("revoker thread must not panic");
        assert!(
            revoke_result.is_ok(),
            "revocation must complete while serialized against access"
        );
        assert!(
            racing_access.is_ok()
                || matches!(
                    racing_access,
                    Err(RuntimeError::Memory(MemoryError::Capability(CapError::Revoked)))
                        | Err(RuntimeError::Memory(MemoryError::Unmapped))
                ),
            "the racing access must either linearize before revocation or be denied"
        );

        assert!(
            matches!(
                runtime.access(hart, address_space, 0x8000, MemoryAccess::Read),
                Err(RuntimeError::Memory(MemoryError::Unmapped))
                    | Err(RuntimeError::Memory(MemoryError::Capability(CapError::Revoked)))
            ),
            "no access may succeed after revocation has returned"
        );
    }

    #[test]
    fn serialized_runtime_keeps_capability_and_mapping_state_together() {
        let (runtime, handle, address_space, hart) = prepared_runtime();
        let report = runtime
            .revoke_capability(domain(1), handle, &mut ImmediateFences)
            .expect("revocation and mapping invalidation should complete");
        assert_eq!(report.mappings_invalidated, 1);
        assert_eq!(
            runtime.access(hart, address_space, 0x8000, MemoryAccess::Read),
            Err(RuntimeError::Memory(MemoryError::Unmapped))
        );
        assert_eq!(runtime.pending_invalidation_count(), Ok(0));
    }
}
