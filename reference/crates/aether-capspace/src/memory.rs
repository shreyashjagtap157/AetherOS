//! Host reference model for capability-authorized RV64 page mappings.
//!
//! This module is an executable model of a two-stage software-capability
//! contract: the Software Capability Provider authorizes a mapping operation;
//! page-table permissions then mediate each simulated memory access. It is not
//! an MMU implementation and makes no kernel, concurrency, or WCET claim.

use crate::CapabilitySystem;
use aether_types::{Bounds, CapError, DomainId, Handle, ObjectId, ObjectType, Rights};
use std::collections::{HashMap, HashSet};

/// Supervisor address-space identifier used by the host reference model.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AddressSpaceId(u16);

impl AddressSpaceId {
    /// Creates an address-space identifier.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the underlying identifier value.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Hardware hart identifier in the host reference model.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HartId(u32);

impl HartId {
    /// Creates a hart identifier.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the underlying hart identifier.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Sv39 leaf-page size selected explicitly by a mapping request.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PageSize {
    /// Base page (4 KiB).
    Base4KiB,
    /// Level-1 megapage (2 MiB).
    Large2MiB,
    /// Level-2 gigapage (1 GiB).
    Huge1GiB,
}

impl PageSize {
    const SEARCH_ORDER: [Self; 3] = [Self::Huge1GiB, Self::Large2MiB, Self::Base4KiB];

    /// Returns the page size in bytes.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        match self {
            Self::Base4KiB => 4 * 1024,
            Self::Large2MiB => 2 * 1024 * 1024,
            Self::Huge1GiB => 1024 * 1024 * 1024,
        }
    }
}

/// Memory operation checked against the page permissions cached by a hart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryAccess {
    /// Load data from the mapped region.
    Read,
    /// Store data into the mapped region.
    Write,
    /// Fetch an instruction from the mapped region.
    Execute,
}

/// Hardware leaf permissions, deliberately distinct from capability authority.
///
/// These bits describe only the RISC-V PTE R/W/X permissions. They are not
/// RFC-0037 capability-right bits and cannot be used to grant mapping authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PtePermissions(u8);

impl PtePermissions {
    /// Permit load operations.
    pub const READ: Self = Self(1 << 0);
    /// Permit store operations.
    pub const WRITE: Self = Self(1 << 1);
    /// Permit instruction fetches.
    pub const EXECUTE: Self = Self(1 << 2);
    /// No access permissions.
    pub const NONE: Self = Self(0);
    const KNOWN: u8 = Self::READ.0 | Self::WRITE.0 | Self::EXECUTE.0;

    /// Returns the permission bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Returns whether every requested PTE permission is present.
    #[must_use]
    pub const fn contains(self, requested: Self) -> bool {
        self.0 & requested.0 == requested.0
    }

    /// Returns the union of PTE permissions.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Request to map a byte range of a capability-backed object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapRequest {
    /// Address space receiving the mapping.
    pub address_space: AddressSpaceId,
    /// Half-open virtual-address range.
    pub virtual_range: Bounds,
    /// Half-open byte range within the capability-backed object.
    pub resource_range: Bounds,
    /// Explicit hardware leaf-page size.
    pub page_size: PageSize,
    /// Requested PTE permissions. This must contain only READ, WRITE and/or
    /// EXECUTE; MAP is checked separately as the authority to create the map.
    pub permissions: PtePermissions,
}

/// Stable identifier for a mapping created by this host model.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MappingId(u64);

impl MappingId {
    /// Returns the mapping identifier.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Result of a simulated access after translation and PTE permission checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranslatedAccess {
    /// Mapping that supplied the translation.
    pub mapping: MappingId,
    /// Resource object reached by the mapping.
    pub object: ObjectId,
    /// Byte offset within the resource object.
    pub object_offset: u64,
}

/// Summary of a completed capability-subtree revocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RevocationReport {
    /// Number of mappings invalidated by the capability revocation.
    pub mappings_invalidated: usize,
    /// Number of harts for which an invalidation was requested.
    pub hart_invalidations: usize,
}

/// Failure reported by a platform-specific page-table/TLB backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShootdownError {
    /// A target hart or the platform fence mechanism did not complete.
    Unavailable,
}

/// Backend contract for address-space activation, page-table publication, and
/// local/remote translation-cache invalidation.
///
/// `activate_address_space_and_wait` must select the registered page-table root
/// and ASID for the target hart and complete the required local translation
/// synchronization before returning. The dispatch caller must not release the
/// target thread to execution until this call succeeds. The backend must be able
/// to re-establish a known translation state after a previous activation failed.
///
/// `current_hart` identifies the hart executing the page-table operation.
/// `publish_page_table_updates` must make PTE stores visible before any
/// translation invalidation (for example, with the required data fence).
/// `invalidate_hart_and_wait` must execute SFENCE.VMA on the target hart and
/// must not return success until that hart acknowledges completion. A remote
/// SBI RFENCE success indicating only that an IPI was sent is not sufficient.
pub trait TlbShootdown {
    /// Returns the hart executing the mapping or revocation operation.
    fn current_hart(&self) -> HartId;

    /// Selects the registered page-table root and ASID on one hart, then waits
    /// for local activation and the required local translation synchronization.
    ///
    /// # Errors
    /// Returns `ShootdownError::Unavailable` when activation does not complete.
    fn activate_address_space_and_wait(
        &mut self,
        hart: HartId,
        address_space: AddressSpaceId,
    ) -> Result<(), ShootdownError>;

    /// Publishes page-table changes for the specified address-space range.
    ///
    /// # Errors
    /// Returns `ShootdownError::Unavailable` if the backend cannot make the
    /// preceding PTE stores visible to the harts that may use this address space.
    fn publish_page_table_updates(
        &mut self,
        address_space: AddressSpaceId,
        virtual_range: Bounds,
    ) -> Result<(), ShootdownError>;

    /// Invalidates translations on one hart and waits for its acknowledgement.
    ///
    /// # Errors
    /// Returns `ShootdownError::Unavailable` if the target hart does not
    /// execute and acknowledge the required local translation fence.
    fn invalidate_hart_and_wait(
        &mut self,
        hart: HartId,
        address_space: AddressSpaceId,
        virtual_range: Bounds,
    ) -> Result<(), ShootdownError>;
}

/// Errors from the host reference memory-mapping model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryError {
    /// Capability validation failed.
    Capability(CapError),
    /// The domain has not been assigned an address space.
    AddressSpaceMissing,
    /// The address-space identifier is already assigned to another domain.
    AddressSpaceAlreadyOwned,
    /// The domain is already bound to a different address-space identifier.
    DomainAlreadyAssigned,
    /// The requested address space is not active on the executing hart.
    AddressSpaceNotActive,
    /// Address-space activation failed and the hart remains quarantined.
    AddressSpaceActivationFailed,
    /// A virtual or resource range is empty or reversed.
    InvalidRange,
    /// The virtual range is outside the canonical Sv39 address regions.
    NonCanonicalVirtualRange,
    /// A virtual or resource range is not aligned to the selected page size.
    MisalignedRange,
    /// Virtual and resource ranges have different lengths.
    RangeSizeMismatch,
    /// The requested rights contain no memory permission or contain a
    /// non-memory right.
    UnsupportedPermissions,
    /// RISC-V reserves leaf PTEs that set WRITE without READ.
    InvalidPtePermissions,
    /// The requested virtual range overlaps an existing or pending mapping.
    OverlappingMapping,
    /// The mapping identifier does not exist.
    MappingMissing,
    /// The mapping is not owned by the supplied domain and capability handle.
    MappingOwnerMismatch,
    /// No valid page-table mapping covers the virtual address.
    Unmapped,
    /// The PTE denies the requested load, store, or instruction fetch.
    ProtectionFault,
    /// One or more required hart invalidations have not been acknowledged.
    ShootdownPending,
    /// A new mapping's translation publication/invalidation failed; the
    /// mapping was rolled back and no mapping was installed.
    MappingActivationFailed,
    /// Mapping identifiers cannot be represented any further.
    MappingIdExhausted,
}

impl From<CapError> for MemoryError {
    fn from(value: CapError) -> Self {
        Self::Capability(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct PteKey {
    address_space: AddressSpaceId,
    page_size: PageSize,
    virtual_base: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Pte {
    mapping: MappingId,
    object: ObjectId,
    resource_base: u64,
    virtual_base: u64,
    permissions: PtePermissions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MappingRecord {
    domain: DomainId,
    handle: Handle,
    address_space: AddressSpaceId,
    virtual_range: Bounds,
    resource_range: Bounds,
    page_size: PageSize,
    permissions: PtePermissions,
    invalidated: bool,
}

#[derive(Clone, Debug)]
struct PendingInvalidation {
    address_space: AddressSpaceId,
    virtual_range: Bounds,
    pending_harts: HashSet<HartId>,
    publication_pending: bool,
}

/// Host-side model of capability-authorized mappings, PTE permissions and TLB
/// invalidation. It intentionally models only one provider and one host memory
/// object abstraction; it is not a target page-table implementation.
pub struct MemoryManager {
    domain_asids: HashMap<DomainId, AddressSpaceId>,
    asid_domains: HashMap<AddressSpaceId, DomainId>,
    page_table: HashMap<PteKey, Pte>,
    mappings: HashMap<MappingId, MappingRecord>,
    tlb: HashMap<(HartId, PteKey), Pte>,
    /// Harts that may retain translations for an address space, including
    /// harts activated for dispatch before their first simulated memory access.
    resident_harts: HashMap<AddressSpaceId, HashSet<HartId>>,
    /// Address space currently selected on each hart for dispatch/execution.
    active_harts: HashMap<HartId, AddressSpaceId>,
    /// Harts whose last activation may have partially changed hardware state.
    activation_quarantine: HashSet<HartId>,
    pending: HashMap<MappingId, PendingInvalidation>,
    blocked_harts: HashSet<HartId>,
    next_mapping_id: u64,
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self {
            domain_asids: HashMap::new(),
            asid_domains: HashMap::new(),
            page_table: HashMap::new(),
            mappings: HashMap::new(),
            tlb: HashMap::new(),
            resident_harts: HashMap::new(),
            active_harts: HashMap::new(),
            activation_quarantine: HashSet::new(),
            pending: HashMap::new(),
            blocked_harts: HashSet::new(),
            next_mapping_id: 1,
        }
    }
}

impl MemoryManager {
    /// Creates an empty memory-mapping model.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Assigns one unique address-space identifier to a domain.
    ///
    /// # Errors
    /// Returns `DomainAlreadyAssigned` or `AddressSpaceAlreadyOwned` when
    /// either side of the one-domain/one-ASID relationship is already used.
    pub fn register_address_space(
        &mut self,
        domain: DomainId,
        address_space: AddressSpaceId,
    ) -> Result<(), MemoryError> {
        if let Some(existing) = self.domain_asids.get(&domain) {
            return if *existing == address_space {
                Ok(())
            } else {
                Err(MemoryError::DomainAlreadyAssigned)
            };
        }
        if self.asid_domains.contains_key(&address_space) {
            return Err(MemoryError::AddressSpaceAlreadyOwned);
        }
        self.domain_asids.insert(domain, address_space);
        self.asid_domains.insert(address_space, domain);
        Ok(())
    }

    /// Activates a domain's address space at the dispatch boundary.
    ///
    /// The caller must not let the dispatched thread execute until this method
    /// returns `Ok`. A hart is recorded as potentially resident before invoking
    /// the backend because a failed activation may have partially changed the
    /// hardware translation state. Such a hart is quarantined until a later
    /// activation succeeds. Residency is retained across context switches so
    /// later map/unmap/revocation operations still invalidate stale ASID entries.
    ///
    /// # Errors
    /// Returns `AddressSpaceMissing` for an unregistered domain,
    /// `ShootdownPending` while this hart or target address space has an
    /// incomplete invalidation, or `AddressSpaceActivationFailed` when the
    /// backend cannot complete activation. A failed activation never authorizes
    /// simulated execution.
    pub fn activate_for_dispatch(
        &mut self,
        domain: DomainId,
        hart: HartId,
        fences: &mut impl TlbShootdown,
    ) -> Result<AddressSpaceId, MemoryError> {
        let address_space = *self
            .domain_asids
            .get(&domain)
            .ok_or(MemoryError::AddressSpaceMissing)?;

        if self.pending.values().any(|pending| {
            pending.address_space == address_space || pending.pending_harts.contains(&hart)
        }) {
            return Err(MemoryError::ShootdownPending);
        }

        if self.active_harts.get(&hart) == Some(&address_space)
            && !self.activation_quarantine.contains(&hart)
        {
            return Ok(address_space);
        }

        self.resident_harts
            .entry(address_space)
            .or_default()
            .insert(hart);
        if let Some(previous) = self.active_harts.remove(&hart) {
            self.resident_harts
                .entry(previous)
                .or_default()
                .insert(hart);
        }

        match fences.activate_address_space_and_wait(hart, address_space) {
            Ok(()) => {
                self.active_harts.insert(hart, address_space);
                self.activation_quarantine.remove(&hart);
                self.refresh_blocked_harts();
                Ok(address_space)
            }
            Err(_) => {
                self.active_harts.remove(&hart);
                self.activation_quarantine.insert(hart);
                self.refresh_blocked_harts();
                Err(MemoryError::AddressSpaceActivationFailed)
            }
        }
    }

    /// Creates a hardware-style mapping after validating the Software
    /// Capability Provider authority and exact page-granular bounds.
    ///
    /// The capability must grant MAP plus every requested PTE permission.
    /// The page table then enforces READ/WRITE/EXECUTE on each simulated access.
    /// PTE publication and local/remote translation invalidation complete before
    /// the new mapping is returned to its caller.
    ///
    /// # Errors
    /// Returns a capability, alignment, overlap, address-space, rights, or
    /// representability error without partially installing PTEs. If translation
    /// activation fails, the new PTEs are rolled back; incomplete rollback
    /// remains quarantined and returns `ShootdownPending`.
    pub fn map(
        &mut self,
        capabilities: &CapabilitySystem,
        domain: DomainId,
        handle: Handle,
        request: MapRequest,
        fences: &mut impl TlbShootdown,
    ) -> Result<MappingId, MemoryError> {
        if self.domain_asids.get(&domain) != Some(&request.address_space) {
            return Err(MemoryError::AddressSpaceMissing);
        }
        Self::validate_permissions(request.permissions)?;
        Self::validate_ranges(
            request.virtual_range,
            request.resource_range,
            request.page_size,
        )?;
        if self.mappings.values().any(|mapping| {
            mapping.address_space == request.address_space
                && Self::overlaps(mapping.virtual_range, request.virtual_range)
        }) {
            return Err(MemoryError::OverlappingMapping);
        }

        let required = Self::required_capability_rights(request.permissions);
        let capability = capabilities.authorize(
            domain,
            handle,
            ObjectType::Memory,
            required,
            request.resource_range,
        )?;

        let mapping = MappingId(self.next_mapping_id);
        self.next_mapping_id = self
            .next_mapping_id
            .checked_add(1)
            .ok_or(MemoryError::MappingIdExhausted)?;

        let mut new_ptes = Vec::new();
        let mut virtual_base = request.virtual_range.start;
        let mut resource_base = request.resource_range.start;
        while virtual_base < request.virtual_range.end {
            let key = PteKey {
                address_space: request.address_space,
                page_size: request.page_size,
                virtual_base,
            };
            if self.page_table.contains_key(&key) {
                return Err(MemoryError::OverlappingMapping);
            }
            new_ptes.push((
                key,
                Pte {
                    mapping,
                    object: capability.object,
                    resource_base,
                    virtual_base,
                    permissions: request.permissions,
                },
            ));
            let step = request.page_size.bytes();
            virtual_base = virtual_base
                .checked_add(step)
                .ok_or(MemoryError::RangeSizeMismatch)?;
            resource_base = resource_base
                .checked_add(step)
                .ok_or(MemoryError::RangeSizeMismatch)?;
        }

        // All validation has completed. Install the PTE set and its authority
        // record together; no fallible operation follows these insertions.
        for (key, pte) in new_ptes {
            self.page_table.insert(key, pte);
        }
        self.mappings.insert(
            mapping,
            MappingRecord {
                domain,
                handle,
                address_space: request.address_space,
                virtual_range: request.virtual_range,
                resource_range: request.resource_range,
                page_size: request.page_size,
                permissions: request.permissions,
                invalidated: false,
            },
        );

        if self.activate_mapping(mapping, fences).is_err() {
            return match self.invalidate_mapping(mapping, fences) {
                Ok(()) => Err(MemoryError::MappingActivationFailed),
                Err(MemoryError::ShootdownPending) => Err(MemoryError::ShootdownPending),
                Err(error) => Err(error),
            };
        }

        Ok(mapping)
    }

    /// Simulates a memory operation using the page-table entry or a hart-local
    /// TLB entry. It does not consult the capability table on every access.
    ///
    /// # Errors
    /// Returns `ShootdownPending` for a quarantined hart or range,
    /// `Unmapped` for an absent mapping, or `ProtectionFault` when the PTE
    /// does not grant the requested access.
    pub fn access(
        &mut self,
        hart: HartId,
        address_space: AddressSpaceId,
        virtual_address: u64,
        operation: MemoryAccess,
    ) -> Result<TranslatedAccess, MemoryError> {
        if self.blocked_harts.contains(&hart)
            || self.pending.values().any(|pending| {
                pending.address_space == address_space
                    && Self::contains_address(pending.virtual_range, virtual_address)
            })
        {
            return Err(MemoryError::ShootdownPending);
        }
        if self.active_harts.get(&hart) != Some(&address_space) {
            return Err(MemoryError::AddressSpaceNotActive);
        }

        let (key, pte) = self
            .lookup_tlb(hart, address_space, virtual_address)
            .or_else(|| self.lookup_page_table(address_space, virtual_address))
            .ok_or(MemoryError::Unmapped)?;

        self.tlb.insert((hart, key), pte);
        self.resident_harts
            .entry(address_space)
            .or_default()
            .insert(hart);

        let requested = match operation {
            MemoryAccess::Read => PtePermissions::READ,
            MemoryAccess::Write => PtePermissions::WRITE,
            MemoryAccess::Execute => PtePermissions::EXECUTE,
        };
        if !pte.permissions.contains(requested) {
            return Err(MemoryError::ProtectionFault);
        }

        let offset = pte
            .resource_base
            .checked_add(virtual_address - pte.virtual_base)
            .ok_or(MemoryError::RangeSizeMismatch)?;
        Ok(TranslatedAccess {
            mapping: pte.mapping,
            object: pte.object,
            object_offset: offset,
        })
    }

    /// Unmaps one mapping owned by the supplied still-live memory capability.
    ///
    /// # Errors
    /// Returns an ownership/capability error, or `ShootdownPending` until
    /// page-table writes are visible and every resident hart acknowledges the
    /// requested local translation fence.
    pub fn unmap(
        &mut self,
        capabilities: &CapabilitySystem,
        domain: DomainId,
        handle: Handle,
        mapping: MappingId,
        fences: &mut impl TlbShootdown,
    ) -> Result<(), MemoryError> {
        let record = *self
            .mappings
            .get(&mapping)
            .ok_or(MemoryError::MappingMissing)?;
        if record.domain != domain || record.handle != handle || record.invalidated {
            return Err(MemoryError::MappingOwnerMismatch);
        }
        capabilities.authorize(
            domain,
            handle,
            ObjectType::Memory,
            Self::required_capability_rights(record.permissions),
            record.resource_range,
        )?;
        self.invalidate_mapping(mapping, fences)
    }

    /// Revokes a capability subtree and invalidates every mapping whose backing
    /// authority became invalid, including mappings created in destination
    /// domains through capability transfer.
    ///
    /// A failed fence leaves the affected range blocked and returns
    /// `ShootdownPending`; callers must not resume any blocked hart until
    /// `retry_pending_shootdowns` completes.
    ///
    /// # Errors
    /// Returns a capability error or `ShootdownPending` when any required
    /// page-table publication or hart acknowledgement has not completed.
    pub fn revoke_capability(
        &mut self,
        capabilities: &mut CapabilitySystem,
        domain: DomainId,
        handle: Handle,
        fences: &mut impl TlbShootdown,
    ) -> Result<RevocationReport, MemoryError> {
        capabilities.revoke(domain, handle)?;

        let invalid_mappings: Vec<MappingId> = self
            .mappings
            .iter()
            .filter_map(|(mapping_id, record)| {
                let result = capabilities.authorize(
                    record.domain,
                    record.handle,
                    ObjectType::Memory,
                    Self::required_capability_rights(record.permissions),
                    record.resource_range,
                );
                result.is_err().then_some(*mapping_id)
            })
            .collect();

        let mut hart_invalidations = 0_usize;
        let mut incomplete = false;
        for mapping in &invalid_mappings {
            if let Some(record) = self.mappings.get(mapping) {
                let mut targets = self
                    .resident_harts
                    .get(&record.address_space)
                    .cloned()
                    .unwrap_or_default();
                targets.insert(fences.current_hart());
                hart_invalidations = hart_invalidations.saturating_add(targets.len());
            }
            if self.invalidate_mapping(*mapping, fences).is_err() {
                incomplete = true;
            }
        }

        if incomplete {
            return Err(MemoryError::ShootdownPending);
        }
        Ok(RevocationReport {
            mappings_invalidated: invalid_mappings.len(),
            hart_invalidations,
        })
    }

    /// Removes a capability handle and all mappings created directly from it.
    ///
    /// Descendant capabilities are not revoked by handle removal; their mappings
    /// remain valid. Use `revoke_capability` when the entire descendant subtree
    /// must be invalidated.
    ///
    /// # Errors
    /// Returns a capability error or `ShootdownPending` if translation
    /// invalidation has not completed on every resident hart.
    pub fn remove_capability(
        &mut self,
        capabilities: &mut CapabilitySystem,
        domain: DomainId,
        handle: Handle,
        fences: &mut impl TlbShootdown,
    ) -> Result<usize, MemoryError> {
        capabilities.remove(domain, handle)?;

        let mappings: Vec<MappingId> = self
            .mappings
            .iter()
            .filter_map(|(mapping, record)| {
                (record.domain == domain && record.handle == handle).then_some(*mapping)
            })
            .collect();

        let count = mappings.len();
        let mut incomplete = false;
        for mapping in mappings {
            if self.invalidate_mapping(mapping, fences).is_err() {
                incomplete = true;
            }
        }
        if incomplete {
            Err(MemoryError::ShootdownPending)
        } else {
            Ok(count)
        }
    }

    /// Retries incomplete page-table publication and hart-fence operations.
    ///
    /// # Errors
    /// Returns `ShootdownPending` while any target hart remains unacknowledged.
    pub fn retry_pending_shootdowns(
        &mut self,
        fences: &mut impl TlbShootdown,
    ) -> Result<(), MemoryError> {
        let mapping_ids: Vec<MappingId> = self.pending.keys().copied().collect();
        let mut incomplete = false;
        for mapping in mapping_ids {
            if self.progress_invalidation(mapping, fences).is_err() {
                incomplete = true;
            }
        }
        if incomplete {
            Err(MemoryError::ShootdownPending)
        } else {
            Ok(())
        }
    }

    /// Returns the number of mappings that are currently awaiting invalidation.
    #[must_use]
    pub fn pending_invalidation_count(&self) -> usize {
        self.pending.len()
    }

    fn required_capability_rights(permissions: PtePermissions) -> Rights {
        let mut required = Rights::MAP;
        if permissions.contains(PtePermissions::READ) {
            required = required.union(Rights::READ);
        }
        if permissions.contains(PtePermissions::WRITE) {
            required = required.union(Rights::WRITE);
        }
        if permissions.contains(PtePermissions::EXECUTE) {
            required = required.union(Rights::EXECUTE);
        }
        required
    }

    fn validate_permissions(permissions: PtePermissions) -> Result<(), MemoryError> {
        if permissions.bits() == 0 || permissions.bits() & !PtePermissions::KNOWN != 0 {
            return Err(MemoryError::UnsupportedPermissions);
        }
        if permissions.contains(PtePermissions::WRITE)
            && !permissions.contains(PtePermissions::READ)
        {
            // RISC-V reserves leaf PTEs with W=1 and R=0.
            return Err(MemoryError::InvalidPtePermissions);
        }
        Ok(())
    }

    fn validate_ranges(
        virtual_range: Bounds,
        resource_range: Bounds,
        page_size: PageSize,
    ) -> Result<(), MemoryError> {
        if virtual_range.start >= virtual_range.end || resource_range.start >= resource_range.end {
            return Err(MemoryError::InvalidRange);
        }

        let last_virtual_byte = virtual_range.end - 1;
        let start_sign = (virtual_range.start >> 38) & 1;
        let end_sign = (last_virtual_byte >> 38) & 1;
        if start_sign != end_sign
            || !Self::is_sv39_canonical(virtual_range.start)
            || !Self::is_sv39_canonical(last_virtual_byte)
        {
            return Err(MemoryError::NonCanonicalVirtualRange);
        }

        let size = page_size.bytes();
        if virtual_range.start % size != 0
            || virtual_range.end % size != 0
            || resource_range.start % size != 0
            || resource_range.end % size != 0
        {
            return Err(MemoryError::MisalignedRange);
        }
        if virtual_range.end - virtual_range.start != resource_range.end - resource_range.start {
            return Err(MemoryError::RangeSizeMismatch);
        }
        Ok(())
    }

    /// Checks the Sv39 sign-extension rule for a 64-bit virtual address.
    const fn is_sv39_canonical(address: u64) -> bool {
        let sign = (address >> 38) & 1;
        let upper = address >> 39;
        if sign == 0 {
            upper == 0
        } else {
            upper == 0x01ff_ffff
        }
    }

    fn overlaps(left: Bounds, right: Bounds) -> bool {
        left.start < right.end && right.start < left.end
    }

    fn contains_address(range: Bounds, address: u64) -> bool {
        range.start <= address && address < range.end
    }

    fn lookup_tlb(
        &self,
        hart: HartId,
        address_space: AddressSpaceId,
        address: u64,
    ) -> Option<(PteKey, Pte)> {
        for page_size in PageSize::SEARCH_ORDER {
            let bytes = page_size.bytes();
            let key = PteKey {
                address_space,
                page_size,
                virtual_base: address & !(bytes - 1),
            };
            if let Some(pte) = self.tlb.get(&(hart, key)) {
                return Some((key, *pte));
            }
        }
        None
    }

    fn lookup_page_table(
        &self,
        address_space: AddressSpaceId,
        address: u64,
    ) -> Option<(PteKey, Pte)> {
        for page_size in PageSize::SEARCH_ORDER {
            let bytes = page_size.bytes();
            let key = PteKey {
                address_space,
                page_size,
                virtual_base: address & !(bytes - 1),
            };
            if let Some(pte) = self.page_table.get(&key) {
                return Some((key, *pte));
            }
        }
        None
    }

    /// Publishes a newly installed mapping and flushes both the executing hart
    /// and every hart known to have run this address space.
    fn activate_mapping(
        &mut self,
        mapping: MappingId,
        fences: &mut impl TlbShootdown,
    ) -> Result<(), MemoryError> {
        let record = *self
            .mappings
            .get(&mapping)
            .ok_or(MemoryError::MappingMissing)?;

        fences
            .publish_page_table_updates(record.address_space, record.virtual_range)
            .map_err(|_| MemoryError::ShootdownPending)?;

        let mut targets = self
            .resident_harts
            .get(&record.address_space)
            .cloned()
            .unwrap_or_default();
        targets.insert(fences.current_hart());

        for hart in targets {
            fences
                .invalidate_hart_and_wait(hart, record.address_space, record.virtual_range)
                .map_err(|_| MemoryError::ShootdownPending)?;
            self.tlb
                .retain(|(cached_hart, _), pte| !(*cached_hart == hart && pte.mapping == mapping));
        }
        Ok(())
    }

    fn invalidate_mapping(
        &mut self,
        mapping: MappingId,
        fences: &mut impl TlbShootdown,
    ) -> Result<(), MemoryError> {
        let record = *self
            .mappings
            .get(&mapping)
            .ok_or(MemoryError::MappingMissing)?;
        if record.invalidated {
            return if self.pending.contains_key(&mapping) {
                Err(MemoryError::ShootdownPending)
            } else {
                Ok(())
            };
        }

        let mut virtual_base = record.virtual_range.start;
        while virtual_base < record.virtual_range.end {
            self.page_table.remove(&PteKey {
                address_space: record.address_space,
                page_size: record.page_size,
                virtual_base,
            });
            virtual_base = virtual_base
                .checked_add(record.page_size.bytes())
                .ok_or(MemoryError::RangeSizeMismatch)?;
        }
        if let Some(stored) = self.mappings.get_mut(&mapping) {
            stored.invalidated = true;
        }

        let mut pending_harts = self
            .resident_harts
            .get(&record.address_space)
            .cloned()
            .unwrap_or_default();
        pending_harts.insert(fences.current_hart());
        self.pending.insert(
            mapping,
            PendingInvalidation {
                address_space: record.address_space,
                virtual_range: record.virtual_range,
                pending_harts,
                publication_pending: true,
            },
        );
        self.refresh_blocked_harts();
        self.progress_invalidation(mapping, fences)
    }

    fn progress_invalidation(
        &mut self,
        mapping: MappingId,
        fences: &mut impl TlbShootdown,
    ) -> Result<(), MemoryError> {
        let (address_space, virtual_range, publication_pending, pending_harts) = {
            let pending = self
                .pending
                .get(&mapping)
                .ok_or(MemoryError::MappingMissing)?;
            (
                pending.address_space,
                pending.virtual_range,
                pending.publication_pending,
                pending.pending_harts.iter().copied().collect::<Vec<_>>(),
            )
        };

        if publication_pending {
            if fences
                .publish_page_table_updates(address_space, virtual_range)
                .is_err()
            {
                return Err(MemoryError::ShootdownPending);
            }
            if let Some(pending) = self.pending.get_mut(&mapping) {
                pending.publication_pending = false;
            }
        }

        for hart in pending_harts {
            if fences
                .invalidate_hart_and_wait(hart, address_space, virtual_range)
                .is_ok()
            {
                self.tlb.retain(|(cached_hart, _), pte| {
                    !(*cached_hart == hart && pte.mapping == mapping)
                });
                if let Some(pending) = self.pending.get_mut(&mapping) {
                    pending.pending_harts.remove(&hart);
                }
            }
        }

        let completed = self.pending.get(&mapping).is_some_and(|pending| {
            !pending.publication_pending && pending.pending_harts.is_empty()
        });
        if completed {
            self.pending.remove(&mapping);
            self.mappings.remove(&mapping);
            self.refresh_blocked_harts();
            Ok(())
        } else {
            self.refresh_blocked_harts();
            Err(MemoryError::ShootdownPending)
        }
    }

    fn refresh_blocked_harts(&mut self) {
        self.blocked_harts.clear();
        self.blocked_harts
            .extend(self.activation_quarantine.iter().copied());
        for pending in self.pending.values() {
            self.blocked_harts
                .extend(pending.pending_harts.iter().copied());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BootstrapAuthority;
    use aether_types::Capability;

    #[derive(Default)]
    struct FakeFences {
        fail_once: HashSet<HartId>,
        fail_activation_once: HashSet<HartId>,
        published: usize,
        activations: Vec<(HartId, AddressSpaceId)>,
        calls: Vec<(HartId, AddressSpaceId, Bounds)>,
    }

    impl TlbShootdown for FakeFences {
        fn current_hart(&self) -> HartId {
            HartId::new(0)
        }

        fn activate_address_space_and_wait(
            &mut self,
            hart: HartId,
            address_space: AddressSpaceId,
        ) -> Result<(), ShootdownError> {
            self.activations.push((hart, address_space));
            if self.fail_activation_once.remove(&hart) {
                Err(ShootdownError::Unavailable)
            } else {
                Ok(())
            }
        }

        fn publish_page_table_updates(
            &mut self,
            _address_space: AddressSpaceId,
            _virtual_range: Bounds,
        ) -> Result<(), ShootdownError> {
            self.published += 1;
            Ok(())
        }

        fn invalidate_hart_and_wait(
            &mut self,
            hart: HartId,
            address_space: AddressSpaceId,
            virtual_range: Bounds,
        ) -> Result<(), ShootdownError> {
            self.calls.push((hart, address_space, virtual_range));
            if self.fail_once.remove(&hart) {
                Err(ShootdownError::Unavailable)
            } else {
                Ok(())
            }
        }
    }

    fn domain(value: u16) -> DomainId {
        DomainId::new(value).unwrap()
    }

    fn bounds(start: u64, end: u64) -> Bounds {
        Bounds::new(start, end).unwrap()
    }

    fn setup(
        rights: Rights,
    ) -> (
        CapabilitySystem,
        BootstrapAuthority,
        MemoryManager,
        Handle,
        DomainId,
        AddressSpaceId,
    ) {
        let (mut capabilities, bootstrap) = CapabilitySystem::bootstrap();
        let domain = domain(1);
        let address_space = AddressSpaceId::new(7);
        capabilities.add_domain(domain);
        let handle = capabilities
            .create_root(
                &bootstrap,
                domain,
                Capability {
                    object: ObjectId(41),
                    object_type: ObjectType::Memory,
                    rights,
                    bounds: bounds(0, 2 * 1024 * 1024),
                },
            )
            .unwrap();
        let mut memory = MemoryManager::new();
        memory
            .register_address_space(domain, address_space)
            .unwrap();
        (
            capabilities,
            bootstrap,
            memory,
            handle,
            domain,
            address_space,
        )
    }

    fn activate_test_hart(memory: &mut MemoryManager, domain: DomainId, hart: HartId) {
        memory
            .activate_for_dispatch(domain, hart, &mut FakeFences::default())
            .expect("dispatch address-space activation should succeed");
    }

    fn map_request(
        memory: &mut MemoryManager,
        capabilities: &CapabilitySystem,
        domain: DomainId,
        handle: Handle,
        request: MapRequest,
    ) -> Result<MappingId, MemoryError> {
        memory.map(
            capabilities,
            domain,
            handle,
            request,
            &mut FakeFences::default(),
        )
    }

    fn request(
        address_space: AddressSpaceId,
        virtual_start: u64,
        resource_start: u64,
        page_size: PageSize,
        permissions: PtePermissions,
    ) -> MapRequest {
        let size = page_size.bytes();
        MapRequest {
            address_space,
            virtual_range: bounds(virtual_start, virtual_start + size),
            resource_range: bounds(resource_start, resource_start + size),
            page_size,
            permissions,
        }
    }

    #[test]
    fn dispatch_activation_is_required_and_registers_residency_before_first_access() {
        let rights = Rights::MAP.union(Rights::READ);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        let hart = HartId::new(9);
        assert_eq!(
            memory.access(hart, asid, 0x4000, MemoryAccess::Read),
            Err(MemoryError::AddressSpaceNotActive)
        );

        let mut fences = FakeFences::default();
        assert_eq!(
            memory.activate_for_dispatch(domain, hart, &mut fences),
            Ok(asid)
        );
        assert_eq!(fences.activations, vec![(hart, asid)]);
        assert!(memory
            .resident_harts
            .get(&asid)
            .is_some_and(|harts| harts.contains(&hart)));

        memory
            .map(
                &capabilities,
                domain,
                handle,
                request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::READ),
                &mut fences,
            )
            .unwrap();
        assert!(fences
            .calls
            .contains(&(hart, asid, bounds(0x4000, 0x5000))));
        assert!(memory
            .access(hart, asid, 0x4000, MemoryAccess::Read)
            .is_ok());
    }

    #[test]
    fn dispatch_switch_reactivates_address_space_before_access() {
        let rights = Rights::MAP.union(Rights::READ);
        let (capabilities, _bootstrap, mut memory, handle, domain, first_asid) = setup(rights);
        let second_domain = domain(2);
        let second_asid = AddressSpaceId::new(8);
        memory
            .register_address_space(second_domain, second_asid)
            .unwrap();
        map_request(
            &mut memory,
            &capabilities,
            domain,
            handle,
            request(first_asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::READ),
        )
        .unwrap();

        let hart = HartId::new(10);
        activate_test_hart(&mut memory, domain, hart);
        assert!(memory
            .access(hart, first_asid, 0x4000, MemoryAccess::Read)
            .is_ok());
        activate_test_hart(&mut memory, second_domain, hart);
        assert_eq!(
            memory.access(hart, first_asid, 0x4000, MemoryAccess::Read),
            Err(MemoryError::AddressSpaceNotActive)
        );
        assert_eq!(memory.active_harts.get(&hart), Some(&second_asid));
        assert!(memory
            .resident_harts
            .get(&first_asid)
            .is_some_and(|harts| harts.contains(&hart)));
        assert!(memory
            .resident_harts
            .get(&second_asid)
            .is_some_and(|harts| harts.contains(&hart)));

        activate_test_hart(&mut memory, domain, hart);
        assert!(memory
            .access(hart, first_asid, 0x4000, MemoryAccess::Read)
            .is_ok());
    }

    #[test]
    fn failed_dispatch_activation_quarantines_hart_until_retry_succeeds() {
        let rights = Rights::MAP.union(Rights::READ);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        map_request(
            &mut memory,
            &capabilities,
            domain,
            handle,
            request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::READ),
        )
        .unwrap();

        let hart = HartId::new(11);
        let mut fences = FakeFences::default();
        fences.fail_activation_once.insert(hart);
        assert_eq!(
            memory.activate_for_dispatch(domain, hart, &mut fences),
            Err(MemoryError::AddressSpaceActivationFailed)
        );
        assert_eq!(
            memory.access(hart, asid, 0x4000, MemoryAccess::Read),
            Err(MemoryError::ShootdownPending)
        );
        assert_eq!(memory.active_harts.get(&hart), None);
        assert!(memory.activation_quarantine.contains(&hart));

        assert_eq!(
            memory.activate_for_dispatch(domain, hart, &mut fences),
            Ok(asid)
        );
        assert!(!memory.activation_quarantine.contains(&hart));
        assert!(memory
            .access(hart, asid, 0x4000, MemoryAccess::Read)
            .is_ok());
    }

    #[test]
    fn mapping_rejects_noncanonical_and_malformed_ranges() {
        let rights = Rights::MAP.union(Rights::READ);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        let mut fences = FakeFences::default();

        let mut request = request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::READ);
        request.virtual_range = Bounds {
            start: 0x5000,
            end: 0x4000,
        };
        assert_eq!(
            memory.map(&capabilities, domain, handle, request, &mut fences),
            Err(MemoryError::InvalidRange)
        );

        request.virtual_range = bounds(0x4000_0000_0000, 0x4000_0000_1000);
        assert_eq!(
            memory.map(&capabilities, domain, handle, request, &mut fences),
            Err(MemoryError::NonCanonicalVirtualRange)
        );
    }

    #[test]
    fn mapping_requires_map_right_and_page_aligned_authority() {
        let rights = Rights::READ.union(Rights::WRITE);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        let mut request = request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::READ);
        assert_eq!(
            map_request(&mut memory, &capabilities, domain, handle, request),
            Err(MemoryError::Capability(CapError::Rights))
        );

        request.resource_range = bounds(1, 4097);
        assert_eq!(
            map_request(&mut memory, &capabilities, domain, handle, request),
            Err(MemoryError::MisalignedRange)
        );
    }

    #[test]
    fn map_publishes_page_tables_and_invalidates_the_executing_hart() {
        let rights = Rights::MAP.union(Rights::READ);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        let mut fences = FakeFences::default();

        memory
            .map(
                &capabilities,
                domain,
                handle,
                request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::READ),
                &mut fences,
            )
            .unwrap();

        assert_eq!(fences.published, 1);
        assert_eq!(
            fences.calls,
            vec![(HartId::new(0), asid, bounds(0x4000, 0x5000))]
        );
    }

    #[test]
    fn failed_map_activation_rolls_back_the_pte() {
        let rights = Rights::MAP.union(Rights::READ);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        let mut fences = FakeFences::default();
        fences.fail_once.insert(HartId::new(0));

        assert_eq!(
            memory.map(
                &capabilities,
                domain,
                handle,
                request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::READ),
                &mut fences,
            ),
            Err(MemoryError::MappingActivationFailed)
        );
        assert_eq!(memory.pending_invalidation_count(), 0);
        activate_test_hart(&mut memory, domain, HartId::new(0));
        assert_eq!(
            memory.access(HartId::new(0), asid, 0x4000, MemoryAccess::Read),
            Err(MemoryError::Unmapped)
        );
    }

    #[test]
    fn empty_pte_permissions_are_rejected_without_mapping() {
        let rights = Rights::MAP.union(Rights::READ);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        let mut fences = FakeFences::default();
        assert_eq!(
            memory.map(
                &capabilities,
                domain,
                handle,
                request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::NONE),
                &mut fences,
            ),
            Err(MemoryError::UnsupportedPermissions)
        );
        assert!(memory.mappings.is_empty());
        assert!(memory.page_table.is_empty());
    }

    #[test]
    fn requested_pte_rights_must_all_be_granted_by_the_capability() {
        let rights = Rights::MAP.union(Rights::READ);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        let mut fences = FakeFences::default();
        let permissions = PtePermissions::READ.union(PtePermissions::WRITE);
        assert_eq!(
            memory.map(
                &capabilities,
                domain,
                handle,
                request(asid, 0x4000, 0, PageSize::Base4KiB, permissions),
                &mut fences,
            ),
            Err(MemoryError::Capability(CapError::Rights))
        );
        assert!(memory.mappings.is_empty());
        assert!(memory.page_table.is_empty());
    }

    #[test]
    fn pte_permissions_are_checked_independently_of_capability_lookup() {
        let rights = Rights::MAP.union(Rights::READ).union(Rights::WRITE);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        let mapping = map_request(
            &mut memory,
            &capabilities,
            domain,
            handle,
            request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::READ),
        )
        .unwrap();
        let hart = HartId::new(1);
        activate_test_hart(&mut memory, domain, hart);
        assert_eq!(
            memory
                .access(hart, asid, 0x4000, MemoryAccess::Read)
                .unwrap()
                .mapping,
            mapping
        );
        assert_eq!(
            memory.access(hart, asid, 0x4000, MemoryAccess::Write),
            Err(MemoryError::ProtectionFault)
        );
        assert_eq!(
            map_request(
                &mut memory,
                &capabilities,
                domain,
                handle,
                request(
                    asid,
                    0x8000,
                    4096,
                    PageSize::Base4KiB,
                    PtePermissions::WRITE
                ),
            ),
            Err(MemoryError::InvalidPtePermissions)
        );
    }

    #[test]
    fn execute_is_a_distinct_page_permission_and_unmapped_addresses_fault() {
        let rights = Rights::MAP.union(Rights::EXECUTE);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        map_request(
            &mut memory,
            &capabilities,
            domain,
            handle,
            request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::EXECUTE),
        )
        .unwrap();
        let hart = HartId::new(1);
        activate_test_hart(&mut memory, domain, hart);
        assert!(
            memory
                .access(hart, asid, 0x4000, MemoryAccess::Execute)
                .is_ok()
        );
        assert_eq!(
            memory.access(hart, asid, 0x4000, MemoryAccess::Read),
            Err(MemoryError::ProtectionFault)
        );
        assert_eq!(
            memory.access(hart, asid, 0x9000, MemoryAccess::Read),
            Err(MemoryError::Unmapped)
        );
    }

    #[test]
    fn high_canonical_sv39_mapping_is_accepted() {
        let rights = Rights::MAP.union(Rights::READ);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        let mut fences = FakeFences::default();
        let mapping = memory
            .map(
                &capabilities,
                domain,
                handle,
                MapRequest {
                    address_space: asid,
                    virtual_range: bounds(0xffff_ffff_ffff_e000, 0xffff_ffff_ffff_f000),
                    resource_range: bounds(0, 4096),
                    page_size: PageSize::Base4KiB,
                    permissions: PtePermissions::READ,
                },
                &mut fences,
            )
            .unwrap();

        activate_test_hart(&mut memory, domain, HartId::new(1));
        let translated = memory
            .access(
                HartId::new(1),
                asid,
                0xffff_ffff_ffff_e123,
                MemoryAccess::Read,
            )
            .unwrap();
        assert_eq!(translated.mapping, mapping);
        assert_eq!(translated.object_offset, 0x123);
    }

    #[test]
    fn all_sv39_leaf_page_sizes_keep_exact_object_offsets() {
        let rights = Rights::MAP.union(Rights::READ);
        let domain = domain(1);
        let asid = AddressSpaceId::new(7);
        // Use a capability spanning a 1 GiB region so all Sv39 leaf sizes
        // can be exercised.
        // 1 GiB region so all Sv39 leaf sizes can be exercised.
        let (mut capabilities, bootstrap) = CapabilitySystem::bootstrap();
        capabilities.add_domain(domain);
        let handle = capabilities
            .create_root(
                &bootstrap,
                domain,
                Capability {
                    object: ObjectId(42),
                    object_type: ObjectType::Memory,
                    rights,
                    bounds: bounds(0, 1024 * 1024 * 1024),
                },
            )
            .unwrap();
        let mut memory = MemoryManager::new();
        memory.register_address_space(domain, asid).unwrap();
        activate_test_hart(&mut memory, domain, HartId::new(1));

        for (index, page_size) in [PageSize::Base4KiB, PageSize::Large2MiB, PageSize::Huge1GiB]
            .into_iter()
            .enumerate()
        {
            let bytes = page_size.bytes();
            let virtual_base = bytes * (u64::try_from(index).unwrap() + 1);
            map_request(
                &mut memory,
                &capabilities,
                domain,
                handle,
                request(asid, virtual_base, 0, page_size, PtePermissions::READ),
            )
            .unwrap();
            let access = memory
                .access(
                    HartId::new(1),
                    asid,
                    virtual_base + bytes - 1,
                    MemoryAccess::Read,
                )
                .unwrap();
            assert_eq!(access.object, ObjectId(42));
            assert_eq!(access.object_offset, bytes - 1);
        }
    }

    #[test]
    fn address_space_identifiers_are_unique_per_domain() {
        let mut memory = MemoryManager::new();
        memory
            .register_address_space(domain(1), AddressSpaceId::new(7))
            .unwrap();
        assert_eq!(
            memory.register_address_space(domain(2), AddressSpaceId::new(7)),
            Err(MemoryError::AddressSpaceAlreadyOwned)
        );
        assert_eq!(
            memory.register_address_space(domain(1), AddressSpaceId::new(8)),
            Err(MemoryError::DomainAlreadyAssigned)
        );
    }

    // This is intentionally an end-to-end scenario: keeping setup, mapping,
    // revocation and postcondition checks together makes the cross-domain
    // authority propagation invariant directly auditable.
    #[allow(clippy::too_many_lines)]
    #[test]
    fn revocation_invalidates_derived_and_transferred_mappings() {
        let root_rights = Rights::MAP
            .union(Rights::READ)
            .union(Rights::WRITE)
            .union(Rights::GRANT);
        let (mut capabilities, _bootstrap, mut memory, root, first_domain, first_asid) =
            setup(root_rights);
        let second_domain = domain(2);
        let second_asid = AddressSpaceId::new(8);
        capabilities.add_domain(second_domain);
        memory
            .register_address_space(second_domain, second_asid)
            .unwrap();

        let branch = capabilities
            .derive(
                first_domain,
                root,
                Rights::MAP
                    .union(Rights::READ)
                    .union(Rights::WRITE)
                    .union(Rights::GRANT),
                bounds(0, 4096),
            )
            .unwrap();
        let transferred = capabilities
            .transfer(
                first_domain,
                branch,
                second_domain,
                Rights::MAP.union(Rights::READ),
                bounds(0, 4096),
            )
            .unwrap();
        let sibling = capabilities
            .derive(
                first_domain,
                root,
                Rights::MAP.union(Rights::READ),
                bounds(4096, 8192),
            )
            .unwrap();

        map_request(
            &mut memory,
            &capabilities,
            first_domain,
            branch,
            request(
                first_asid,
                0x4000,
                0,
                PageSize::Base4KiB,
                PtePermissions::READ,
            ),
        )
        .unwrap();
        map_request(
            &mut memory,
            &capabilities,
            second_domain,
            transferred,
            request(
                second_asid,
                0x8000,
                0,
                PageSize::Base4KiB,
                PtePermissions::READ,
            ),
        )
        .unwrap();
        map_request(
            &mut memory,
            &capabilities,
            first_domain,
            sibling,
            request(
                first_asid,
                0x5000,
                4096,
                PageSize::Base4KiB,
                PtePermissions::READ,
            ),
        )
        .unwrap();

        let first_hart = HartId::new(1);
        let second_hart = HartId::new(2);
        activate_test_hart(&mut memory, first_domain, first_hart);
        activate_test_hart(&mut memory, second_domain, second_hart);
        assert!(
            memory
                .access(first_hart, first_asid, 0x4000, MemoryAccess::Read)
                .is_ok()
        );
        assert!(
            memory
                .access(first_hart, first_asid, 0x5000, MemoryAccess::Read)
                .is_ok()
        );
        assert!(
            memory
                .access(second_hart, second_asid, 0x8000, MemoryAccess::Read)
                .is_ok()
        );

        let mut fences = FakeFences::default();
        let report = memory
            .revoke_capability(&mut capabilities, first_domain, branch, &mut fences)
            .unwrap();
        assert_eq!(report.mappings_invalidated, 2);
        assert_eq!(report.hart_invalidations, 4);
        assert_eq!(fences.calls.len(), 4);
        assert_eq!(
            memory.access(first_hart, first_asid, 0x4000, MemoryAccess::Read),
            Err(MemoryError::Unmapped)
        );
        assert_eq!(
            memory.access(second_hart, second_asid, 0x8000, MemoryAccess::Read),
            Err(MemoryError::Unmapped)
        );
        assert!(
            memory
                .access(first_hart, first_asid, 0x5000, MemoryAccess::Read)
                .is_ok()
        );
    }

    #[test]
    fn removing_a_handle_unmaps_only_its_direct_mapping() {
        let rights = Rights::MAP.union(Rights::READ).union(Rights::GRANT);
        let (mut capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        let child = capabilities
            .derive(
                domain,
                handle,
                Rights::MAP.union(Rights::READ),
                bounds(0, 4096),
            )
            .unwrap();
        map_request(
            &mut memory,
            &capabilities,
            domain,
            handle,
            request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::READ),
        )
        .unwrap();
        map_request(
            &mut memory,
            &capabilities,
            domain,
            child,
            request(asid, 0x5000, 0, PageSize::Base4KiB, PtePermissions::READ),
        )
        .unwrap();
        let hart = HartId::new(5);
        activate_test_hart(&mut memory, domain, hart);
        assert!(
            memory
                .access(hart, asid, 0x4000, MemoryAccess::Read)
                .is_ok()
        );
        assert!(
            memory
                .access(hart, asid, 0x5000, MemoryAccess::Read)
                .is_ok()
        );

        let mut fences = FakeFences::default();
        assert_eq!(
            memory.remove_capability(&mut capabilities, domain, handle, &mut fences),
            Ok(1)
        );
        assert_eq!(
            memory.access(hart, asid, 0x4000, MemoryAccess::Read),
            Err(MemoryError::Unmapped)
        );
        assert!(
            memory
                .access(hart, asid, 0x5000, MemoryAccess::Read)
                .is_ok()
        );
    }

    #[test]
    fn failed_hart_fence_blocks_access_until_retry_acknowledges() {
        let rights = Rights::MAP.union(Rights::READ).union(Rights::GRANT);
        let (mut capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        map_request(
            &mut memory,
            &capabilities,
            domain,
            handle,
            request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::READ),
        )
        .unwrap();
        let hart = HartId::new(3);
        activate_test_hart(&mut memory, domain, hart);
        assert!(
            memory
                .access(hart, asid, 0x4000, MemoryAccess::Read)
                .is_ok()
        );

        let mut fences = FakeFences::default();
        fences.fail_once.insert(hart);
        assert_eq!(
            memory.revoke_capability(&mut capabilities, domain, handle, &mut fences),
            Err(MemoryError::ShootdownPending)
        );
        assert_eq!(memory.pending_invalidation_count(), 1);
        assert_eq!(
            memory.activate_for_dispatch(domain, hart, &mut fences),
            Err(MemoryError::ShootdownPending)
        );
        assert_eq!(
            memory.access(hart, asid, 0x4000, MemoryAccess::Read),
            Err(MemoryError::ShootdownPending)
        );

        memory.retry_pending_shootdowns(&mut fences).unwrap();
        assert_eq!(memory.pending_invalidation_count(), 0);
        assert_eq!(
            memory.access(hart, asid, 0x4000, MemoryAccess::Read),
            Err(MemoryError::Unmapped)
        );
    }

    #[test]
    fn explicit_unmap_flushes_only_the_selected_mapping() {
        let rights = Rights::MAP.union(Rights::READ);
        let (capabilities, _bootstrap, mut memory, handle, domain, asid) = setup(rights);
        let first = map_request(
            &mut memory,
            &capabilities,
            domain,
            handle,
            request(asid, 0x4000, 0, PageSize::Base4KiB, PtePermissions::READ),
        )
        .unwrap();
        map_request(
            &mut memory,
            &capabilities,
            domain,
            handle,
            request(asid, 0x5000, 4096, PageSize::Base4KiB, PtePermissions::READ),
        )
        .unwrap();
        let hart = HartId::new(4);
        activate_test_hart(&mut memory, domain, hart);
        assert!(
            memory
                .access(hart, asid, 0x4000, MemoryAccess::Read)
                .is_ok()
        );
        assert!(
            memory
                .access(hart, asid, 0x5000, MemoryAccess::Read)
                .is_ok()
        );

        let mut fences = FakeFences::default();
        memory
            .unmap(&capabilities, domain, handle, first, &mut fences)
            .unwrap();
        assert_eq!(
            memory.access(hart, asid, 0x4000, MemoryAccess::Read),
            Err(MemoryError::Unmapped)
        );
        assert!(
            memory
                .access(hart, asid, 0x5000, MemoryAccess::Read)
                .is_ok()
        );
        assert_eq!(fences.calls.len(), 2);
    }
}
