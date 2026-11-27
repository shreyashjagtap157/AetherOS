//! Address space model (RFC-0009).
//!
//! This is a bounded executable host model of the RFC-0009 address-space
//! semantics: one capability-addressed address space per execution domain
//! (RFC-0007), with mapping and unmapping mediated by MAP capabilities. It is
//! deliberately **not** a hardware MMU, a page-table implementation, or a
//! kernel memory manager; hardware-specific MMU formats and physical page
//! table structures are out of scope.
//!
//! Scope and declared architecture:
//!
//! - **Class:** per-domain capability-addressed memory isolation.
//! - **Address space identity:** each domain has exactly one address space.
//!   Its ASID is derived from the domain identifier by a deterministic
//!   one-way function, is immutable for the domain's lifetime, and ASIDs are
//!   disjoint by construction (RFC-0009.1, RFC-0009.2).
//! - **Mapping:** mapping a memory region requires a capability carrying
//!   `Rights::MAP` for a memory object; the access rights granted by a
//!   mapping are enforced on every memory access, and a thread cannot
//!   elevate its own rights (RFC-0009.3, RFC-0009.4, RFC-0009.6, RFC-0009.7).
//! - **Page sizes:** base page (4 KiB), large page (2 MiB) and huge page (1
//!   GiB) classes are supported; page-size selection is explicit in every
//!   mapping request (RFC-0009.8, RFC-0009.9).
//! - **Atomicity:** map/unmap either apply in full or not at all; no
//!   partially applied mapping is ever observable (RFC-0009.5).
//! - **Unmapping:** unmap flushes pending writes and invalidates the mapping
//!   before returning, and emits a `RegionUnmapped` event (RFC-0009.12,
//!   RFC-0009.13, RFC-0031.21).
//! - **Lifecycle:** `Created -> Active -> Empty -> Destroyed`; `Destroyed` is
//!   reachable only from `Empty` (RFC-0009.15).
//!
//! Explicitly deferred (MAY-level requirements with no approved
//! implementation in this host model):
//! - RFC-0009.10 NUMA topology exposure: not implemented; no HPC-profile
//!   conformance claim is made.
//! - RFC-0009.11 copy-on-write mappings: not implemented.
//! - RFC-0009.14 named address-space slices: not implemented.
//!
//! Explicitly out of scope:
//! - Hardware MMU page-table formats and TLB shootdown.
//! - Integration with any kernel, dispatcher, or device driver.
//! - RFC-0031 event delivery to external consumers (events are recorded
//!   locally).

#![allow(clippy::new_without_default)]
#![allow(clippy::unused_self)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]

use aether_exec_domain_model::DomainManager;
use aether_types::{Bounds, Capability, CapabilityFlags, DomainId, ObjectId, ObjectType, Rights};
use std::collections::HashMap;

/// Page-size class of a mapping; the size is explicit in every mapping
/// request (RFC-0009.9).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PageSize {
    /// Base page: 4 KiB (RFC-0009.8).
    Base4KiB,
    /// Large page: 2 MiB (RFC-0009.8).
    Large2MiB,
    /// Huge page: 1 GiB (RFC-0009.8, recommended).
    Huge1GiB,
}

impl PageSize {
    /// Returns the page size in bytes.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        match self {
            PageSize::Base4KiB => 4 * 1024,
            PageSize::Large2MiB => 2 * 1024 * 1024,
            PageSize::Huge1GiB => 1024 * 1024 * 1024,
        }
    }
}

/// Rights granted by a single mapping (RFC-0009.4).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MappingRights {
    /// Read the mapped region.
    pub read: bool,
    /// Write the mapped region.
    pub write: bool,
    /// Execute instructions from the mapped region.
    pub execute: bool,
    /// Copy-on-write semantics requested (RFC-0009.11, deferred).
    pub copy_on_write: bool,
    /// Caching disabled for the mapping.
    pub nocache: bool,
}

impl MappingRights {
    /// Empty rights set: a mapping that authorizes no access.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            read: false,
            write: false,
            execute: false,
            copy_on_write: false,
            nocache: false,
        }
    }
}

/// A mapping request: virtual range, backing resource, page size, and the
/// access rights the mapping grants. Page-size selection is explicit
/// (RFC-0009.9).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapRequest {
    /// Virtual address range `[start, end)` to map; must be page-aligned.
    pub virtual_range: Bounds,
    /// Backing resource range `[start, end)` authorized by the MAP
    /// capability; must be page-aligned and covered by the capability's
    /// bounds.
    pub resource_range: Bounds,
    /// Page size class; explicit in every request (RFC-0009.9).
    pub page_size: PageSize,
    /// Access rights granted by this mapping (a subset of the capability's
    /// rights).
    pub rights: MappingRights,
}

/// Kind of a memory access attempt (RFC-0009.6).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MemoryAccess {
    Read,
    Write,
    Execute,
}

/// Address-space identifier: a deterministic one-way derivation of the
/// owning domain's identifier (RFC-0009.1). ASIDs are immutable for the
/// lifetime of the domain and disjoint by construction (RFC-0009.2).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct AddressSpaceId(u64);

impl AddressSpaceId {
    /// Derives the ASID for `domain` by a deterministic one-way function.
    ///
    /// The derivation is injective over 16-bit domain identifiers, so no two
    /// domains share an ASID (RFC-0009.2). This is the host-model derivation;
    /// it is not a cryptographic function.
    #[must_use]
    pub const fn from_domain(domain: DomainId) -> Self {
        let mut bits = domain.get() as u64;
        bits = bits.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        bits ^= bits >> 32;
        Self(bits)
    }

    /// Returns the integer representation.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Address-space lifecycle state (RFC-0009 state machine).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressSpaceState {
    /// Address space created for the domain; no regions mapped.
    Created,
    /// At least one region mapped and accessible.
    Active,
    /// All regions unmapped; the address space can be remapped or destroyed.
    Empty,
    /// Domain destroyed; all resources released. Reachable only from
    /// `Empty` (RFC-0009.15).
    Destroyed,
}

/// One installed mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
struct RegionMapping {
    /// First mapped virtual byte.
    vaddr: u64,
    /// Mapped size in bytes.
    size: u64,
    /// Page size class of the mapping.
    page_size: PageSize,
    /// Access rights enforced for this mapping.
    rights: MappingRights,
    /// Backing memory object.
    resource: ObjectId,
}

/// One observable address-space event (RFC-0009.13; `RegionUnmapped` per
/// RFC-0031.21).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AddressSpaceEvent {
    /// An address space was created for a domain.
    AddressSpaceCreated {
        /// The owning domain.
        domain: DomainId,
        /// The derived ASID.
        asid: AddressSpaceId,
        /// Scheduling time at creation, in nanoseconds.
        timestamp_ns: u64,
    },
    /// A region was mapped.
    RegionMapped {
        /// The owning domain.
        domain: DomainId,
        /// The address space.
        asid: AddressSpaceId,
        /// First mapped virtual byte.
        vaddr: u64,
        /// Mapped size in bytes.
        size: u64,
        /// Page size class of the mapping.
        page_size: PageSize,
        /// Scheduling time at mapping, in nanoseconds.
        timestamp_ns: u64,
    },
    /// A region was unmapped (RFC-0031.21).
    RegionUnmapped {
        /// The owning domain.
        domain: DomainId,
        /// The address space.
        asid: AddressSpaceId,
        /// First unmapped virtual byte.
        vaddr: u64,
        /// Unmapped size in bytes.
        size: u64,
        /// Scheduling time at unmapping, in nanoseconds.
        timestamp_ns: u64,
    },
    /// An address space was destroyed.
    AddressSpaceDestroyed {
        /// The destroyed domain.
        domain: DomainId,
        /// Its ASID.
        asid: AddressSpaceId,
        /// Scheduling time at destruction, in nanoseconds.
        timestamp_ns: u64,
    },
}

/// Failures of the address-space model; the model never panics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressSpaceError {
    /// The domain is not registered with the execution-domain manager.
    UnknownDomain,
    /// An address space already exists for this domain.
    DuplicateAddressSpace,
    /// No address space exists for this domain yet.
    NoAddressSpace,
    /// The requested lifecycle transition is not permitted by the
    /// RFC-0009 state machine.
    InvalidLifecycleTransition,
    /// The address space is destroyed; no operation is permitted on it.
    AddressSpaceDestroyed,
    /// The address space has no mapped regions.
    AddressSpaceEmpty,
    /// The requested address range is not mapped.
    Unmapped,
    /// The presented capability lacks `Rights::MAP` or is not a memory
    /// capability.
    MissingMapCapability,
    /// The requested access is not granted by the mapping; no
    /// self-elevation is permitted (RFC-0009.7).
    InsufficientRights,
    /// The capability's bounds do not cover the resource range being mapped.
    BoundsViolation,
    /// A virtual or resource range is not aligned to the mapping's page
    /// size.
    NotPageAligned,
    /// The mapping overlaps an existing mapping in the same address space.
    RegionOverlap,
    /// Unmap of a range without an exact installed mapping.
    RegionNotMapped,
    /// The page-size class is not supported.
    InvalidPageSize,
    /// An address range overflows the 64-bit virtual address space.
    AddressOverflow,
}

/// One recorded flush+invalidation (RFC-0009.12: both complete before unmap
/// returns).
struct InvalidationRecord {
    /// The address space whose mapping was invalidated.
    asid: AddressSpaceId,
    /// First invalidated virtual byte.
    vaddr: u64,
    /// Invalidated size in bytes.
    size: u64,
    /// Scheduling time at invalidation, in nanoseconds.
    timestamp_ns: u64,
}

/// One domain's address space: its ASID, lifecycle state, and installed
/// regions.
struct AddressSpace {
    /// Owning domain.
    domain: DomainId,
    /// Immutable ASID derived from the domain (RFC-0009.1).
    asid: AddressSpaceId,
    /// Current lifecycle state.
    state: AddressSpaceState,
    /// Installed mappings.
    regions: Vec<RegionMapping>,
    /// Scheduling time at which the space entered its current state.
    state_since_ns: u64,
    /// Scheduling time of the most recent successful access.
    last_access_ns: u64,
}

/// The address-space manager (RFC-0009).
///
/// Maintains one address space per domain (RFC-0009.1), keeps ASIDs disjoint
/// by construction (RFC-0009.2), and enforces capability-mediated mapping
/// rights on every access (RFC-0009.3, RFC-0009.4, RFC-0009.6, RFC-0009.7).
/// map/unmap updates are atomic in this host model: an update either applies
/// in full or not at all, and no partially applied mapping is ever
/// observable (RFC-0009.5).
pub struct AddressSpaceManager {
    spaces: HashMap<DomainId, AddressSpace>,
    events: Vec<AddressSpaceEvent>,
    invalidations: Vec<InvalidationRecord>,
}

impl AddressSpaceManager {
    /// Creates an empty address-space manager with no address spaces.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spaces: HashMap::new(),
            events: Vec::new(),
            invalidations: Vec::new(),
        }
    }

    /// Returns all address-space events recorded so far, oldest first.
    #[must_use]
    pub fn events(&self) -> &[AddressSpaceEvent] {
        &self.events
    }

    /// Returns the number of address-space events recorded.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns the number of registered address spaces.
    #[must_use]
    pub fn space_count(&self) -> usize {
        self.spaces.len()
    }

    /// Returns `true` if an address space exists for `domain`.
    #[must_use]
    pub fn contains_domain(&self, domain: DomainId) -> bool {
        self.spaces.contains_key(&domain)
    }

    /// Returns the ASID of `domain`'s address space, or `NoAddressSpace` if
    /// none has been created. ASIDs are immutable (RFC-0009.1).
    #[must_use]
    pub fn asid_of(&self, domain: DomainId) -> Result<AddressSpaceId, AddressSpaceError> {
        self.spaces
            .get(&domain)
            .map(|space| space.asid)
            .ok_or(AddressSpaceError::NoAddressSpace)
    }

    /// Returns the lifecycle state of `domain`'s address space, or
    /// `NoAddressSpace` if none exists.
    #[must_use]
    pub fn state_of(&self, domain: DomainId) -> Result<AddressSpaceState, AddressSpaceError> {
        self.spaces
            .get(&domain)
            .map(|space| space.state)
            .ok_or(AddressSpaceError::NoAddressSpace)
    }

    /// Records of flush+invalidation pairs completed by `unmap` (RFC-0009.12:
    /// both complete before the unmap operation returns).
    #[must_use]
    pub fn invalidations(&self) -> &[InvalidationRecord] {
        &self.invalidations
    }

    /// Returns the number of flush+invalidation records.
    #[must_use]
    pub fn invalidation_count(&self) -> usize {
        self.invalidations.len()
    }

    /// Creates the address space of `domain` in the `Created` state.
    ///
    /// The domain must be registered with the execution-domain manager
    /// (RFC-0007), and no address space may exist for it yet
    /// (RFC-0009.1). The ASID is derived deterministically from the domain
    /// identifier (RFC-0009.1) and is unique across domains (RFC-0009.2).
    ///
    /// # Errors
    /// Returns `UnknownDomain` if `domain` is not registered with
    /// `domain_manager`, or `DuplicateAddressSpace` if an address space
    /// already exists for `domain`.
    pub fn create_address_space(
        &mut self,
        domain: DomainId,
        domain_manager: &DomainManager,
        now_ns: u64,
    ) -> Result<AddressSpaceId, AddressSpaceError> {
        if !domain_manager.contains(domain) {
            return Err(AddressSpaceError::UnknownDomain);
        }
        if self.spaces.contains_key(&domain) {
            return Err(AddressSpaceError::DuplicateAddressSpace);
        }
        let asid = AddressSpaceId::from_domain(domain);
        let space = AddressSpace {
            domain,
            asid,
            state: AddressSpaceState::Created,
            regions: Vec::new(),
            state_since_ns: now_ns,
            last_access_ns: now_ns,
        };
        self.spaces.insert(domain, space);
        self.events.push(AddressSpaceEvent::AddressSpaceCreated {
            domain,
            asid,
            timestamp_ns: now_ns,
        });
        Ok(asid)
    }

    /// Maps `request.virtual_range` into `domain`'s address space, backed by
    /// `request.resource_range`, under `capability`.
    ///
    /// The capability must be a memory capability carrying `Rights::MAP`
    /// (RFC-0009.3), its bounds must cover the resource range, and every
    /// requested access right must be granted by the capability — rights are
    /// attenuated, never elevated (RFC-0009.4, RFC-0009.7). Ranges must be
    /// aligned to the explicitly selected page size (RFC-0009.9) and must
    /// not overlap an existing mapping.
    ///
    /// The update is atomic: all validation runs before any state is
    /// mutated, so a rejected mapping leaves the address space unchanged
    /// (RFC-0009.5). The first mapping transitions `Created -> Active`
    /// (RFC-0009 state machine).
    ///
    /// # Errors
    /// Returns `NoAddressSpace` if no address space exists for `domain`,
    /// `AddressSpaceDestroyed` if it is destroyed, `MissingMapCapability`
    /// if the capability is not a memory capability carrying `Rights::MAP`,
    /// `InsufficientRights` if the capability does not grant every requested
    /// access right, `BoundsViolation` if the capability bounds do not cover
    /// the resource range, `NotPageAligned` if a range is not aligned to the
    /// page size, `RegionOverlap` if it overlaps an existing mapping, or
    /// `AddressOverflow` if a range overflows the address space.
    pub fn map(
        &mut self,
        domain: DomainId,
        capability: &Capability,
        request: MapRequest,
        now_ns: u64,
    ) -> Result<(), AddressSpaceError> {
        let space = self
            .spaces
            .get_mut(&domain)
            .ok_or(AddressSpaceError::NoAddressSpace)?;
        if space.state == AddressSpaceState::Destroyed {
            return Err(AddressSpaceError::AddressSpaceDestroyed);
        }

        // RFC-0009.3: a valid MAP capability for a memory object is required.
        if capability.object_type != ObjectType::Memory || !capability.rights.contains(Rights::MAP)
        {
            return Err(AddressSpaceError::MissingMapCapability);
        }

        let page = request.page_size;
        let page_bytes = page.bytes();

        let start = request.virtual_range.start;
        let end = request.virtual_range.end;
        let size = end
            .checked_sub(start)
            .ok_or(AddressSpaceError::AddressOverflow)?;
        if size == 0 || start % page_bytes != 0 || size % page_bytes != 0 {
            return Err(AddressSpaceError::NotPageAligned);
        }
        let map_end = start
            .checked_add(size)
            .ok_or(AddressSpaceError::AddressOverflow)?;

        let res_start = request.resource_range.start;
        let res_end = request.resource_range.end;
        let res_size = res_end
            .checked_sub(res_start)
            .ok_or(AddressSpaceError::AddressOverflow)?;
        if res_size == 0 || res_start % page_bytes != 0 || res_size % page_bytes != 0 {
            return Err(AddressSpaceError::NotPageAligned);
        }

        // RFC-0009.4 / RFC-0009.7: requested rights are attenuated to the
        // capability's grants; no self-elevation.
        let requested = access_rights(request.rights);
        if requested == Rights::NONE || !capability.rights.contains(requested) {
            return Err(AddressSpaceError::InsufficientRights);
        }
        // RFC-0009.3: the capability must authorize the resource range.
        if !capability.bounds.contains(request.resource_range) {
            return Err(AddressSpaceError::BoundsViolation);
        }

        // No overlap with existing mappings.
        for region in &space.regions {
            let region_end = region.vaddr + region.size;
            if !(map_end <= region.vaddr || region_end <= start) {
                return Err(AddressSpaceError::RegionOverlap);
            }
        }

        // Atomic update: validation is complete; apply in full.
        space.regions.push(RegionMapping {
            vaddr: start,
            size,
            page_size: page,
            rights: request.rights,
            resource: capability.object,
        });
        space.state = AddressSpaceState::Active;
        space.state_since_ns = now_ns;
        self.events.push(AddressSpaceEvent::RegionMapped {
            domain,
            asid: space.asid,
            vaddr: start,
            size,
            page_size: page,
            timestamp_ns: now_ns,
        });
        Ok(())
    }

    /// Unmaps the exact range `[vaddr, vaddr + size)` from `domain`'s
    /// address space.
    ///
    /// RFC-0009.12: pending writes are flushed and the mapping is
    /// invalidated before this operation returns; both are recorded here.
    /// RFC-0009.13: a `RegionUnmapped` event is emitted. When this was the
    /// last mapped region the space transitions `Active -> Empty` (and only
    /// `Empty` may transition to `Destroyed`, RFC-0009.15).
    ///
    /// # Errors
    /// Returns `NoAddressSpace` if no address space exists for `domain`,
    /// `AddressSpaceDestroyed` if it is destroyed, `AddressSpaceEmpty` if
    /// there are no mapped regions, or `RegionNotMapped` if no exact
    /// mapping covers the requested range.
    pub fn unmap(
        &mut self,
        domain: DomainId,
        vaddr: u64,
        size: u64,
        now_ns: u64,
    ) -> Result<(), AddressSpaceError> {
        let space = self
            .spaces
            .get_mut(&domain)
            .ok_or(AddressSpaceError::NoAddressSpace)?;
        if space.state == AddressSpaceState::Destroyed {
            return Err(AddressSpaceError::AddressSpaceDestroyed);
        }
        if space.regions.is_empty() {
            return Err(AddressSpaceError::AddressSpaceEmpty);
        }
        let _ = vaddr
            .checked_add(size)
            .ok_or(AddressSpaceError::AddressOverflow)?;
        if size == 0 {
            return Err(AddressSpaceError::NotPageAligned);
        }

        let position = space
            .regions
            .iter()
            .position(|region| region.vaddr == vaddr && region.size == size)
            .ok_or(AddressSpaceError::RegionNotMapped)?;
        let region = space.regions.remove(position);

        // RFC-0009.12: flush pending writes and invalidate the mapping
        // before unmap returns.
        self.invalidations.push(InvalidationRecord {
            asid: space.asid,
            vaddr: region.vaddr,
            size: region.size,
            timestamp_ns: now_ns,
        });

        if space.regions.is_empty() {
            space.state = AddressSpaceState::Empty;
            space.state_since_ns = now_ns;
        }
        self.events.push(AddressSpaceEvent::RegionUnmapped {
            domain,
            asid: space.asid,
            vaddr: region.vaddr,
            size: region.size,
            timestamp_ns: now_ns,
        });
        Ok(())
    }

    /// Checks the memory access `[vaddr, vaddr + length)` performed by a
    /// dispatched thread in `domain`'s address space against the MAP
    /// capability covering the range (RFC-0009.6, RFC-0009.7).
    ///
    /// Access requires an active address space with a single mapping
    /// covering the entire range that grants the requested access right;
    /// otherwise the access faults. No access can elevate rights beyond
    /// what the mapping grants.
    ///
    /// # Errors
    /// Returns `NoAddressSpace` if no address space exists for `domain`,
    /// `AddressSpaceDestroyed` if it is destroyed, `AddressSpaceEmpty` if
    /// it has no mapped regions, `Unmapped` if no mapping covers the range,
    /// or `InsufficientRights` if the mapping does not grant the access.
    pub fn access(
        &mut self,
        domain: DomainId,
        vaddr: u64,
        length: u64,
        access: MemoryAccess,
        now_ns: u64,
    ) -> Result<(), AddressSpaceError> {
        let space = match self.spaces.get_mut(&domain) {
            Some(space) => space,
            None => return Err(AddressSpaceError::NoAddressSpace),
        };
        match space.state {
            AddressSpaceState::Destroyed => return Err(AddressSpaceError::AddressSpaceDestroyed),
            AddressSpaceState::Created | AddressSpaceState::Empty => {
                return Err(AddressSpaceError::AddressSpaceEmpty);
            }
            AddressSpaceState::Active => {}
        }
        if length == 0 {
            return Ok(());
        }
        let end = vaddr
            .checked_add(length)
            .ok_or(AddressSpaceError::AddressOverflow)?;
        for region in &space.regions {
            let region_end = region.vaddr + region.size;
            if vaddr >= region.vaddr && end <= region_end {
                let granted = match access {
                    MemoryAccess::Read => region.rights.read,
                    MemoryAccess::Write => region.rights.write,
                    MemoryAccess::Execute => region.rights.execute,
                };
                if granted {
                    space.last_access_ns = now_ns;
                    return Ok(());
                }
                return Err(AddressSpaceError::InsufficientRights);
            }
        }
        Err(AddressSpaceError::Unmapped)
    }

    /// Returns the page-size class of the mapping covering `vaddr`, if any.
    #[must_use]
    pub fn page_size_of(
        &self,
        domain: DomainId,
        vaddr: u64,
    ) -> Result<PageSize, AddressSpaceError> {
        let space = self
            .spaces
            .get(&domain)
            .ok_or(AddressSpaceError::NoAddressSpace)?;
        for region in &space.regions {
            if vaddr >= region.vaddr && vaddr < region.vaddr + region.size {
                return Ok(region.page_size);
            }
        }
        Err(AddressSpaceError::Unmapped)
    }

    /// Destroys `domain`'s address space, transitioning it to
    /// `Destroyed`.
    ///
    /// RFC-0009.15: `ASID_DESTROYED` is reachable only from `ASID_EMPTY`;
    /// destroying a domain without unmapping all regions is non-conformant.
    ///
    /// # Errors
    /// Returns `NoAddressSpace` if no address space exists for `domain`, or
    /// `InvalidLifecycleTransition` if the space is not `Empty`.
    pub fn destroy_address_space(
        &mut self,
        domain: DomainId,
        now_ns: u64,
    ) -> Result<(), AddressSpaceError> {
        let space = match self.spaces.get(&domain) {
            Some(space) => space,
            None => return Err(AddressSpaceError::NoAddressSpace),
        };
        if space.state != AddressSpaceState::Empty {
            return Err(AddressSpaceError::InvalidLifecycleTransition);
        }
        let asid = space.asid;
        self.spaces.remove(&domain);
        self.events.push(AddressSpaceEvent::AddressSpaceDestroyed {
            domain,
            asid,
            timestamp_ns: now_ns,
        });
        Ok(())
    }
}

impl Default for AddressSpaceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts `MappingRights` access rights to `aether_types::Rights`.
fn access_rights(rights: MappingRights) -> Rights {
    let mut bits = Rights::NONE;
    if rights.read {
        bits = bits.union(Rights::READ);
    }
    if rights.write {
        bits = bits.union(Rights::WRITE);
    }
    if rights.execute {
        bits = bits.union(Rights::EXECUTE);
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain(value: u16) -> DomainId {
        DomainId::new(value).unwrap()
    }

    fn bounds(start: u64, end: u64) -> Bounds {
        Bounds::new(start, end).unwrap()
    }

    fn registered_domains(values: &[u16]) -> DomainManager {
        let mut manager = DomainManager::new();
        for &value in values {
            manager
                .create_domain(domain(value), None, 1_000)
                .expect("test domain creation should succeed");
        }
        manager
    }

    fn map_cap(rights: Rights) -> Capability {
        Capability {
            object: ObjectId(42),
            object_type: ObjectType::Memory,
            rights,
            bounds: bounds(0, 4096),
            flags: CapabilityFlags::NONE,
            expires_at_ns: 0,
        }
    }

    fn mapping_request(vaddr: u64, size: u64, page: PageSize, rights: MappingRights) -> MapRequest {
        MapRequest {
            virtual_range: bounds(vaddr, vaddr + size),
            resource_range: bounds(0, size),
            page_size: page,
            rights,
        }
    }

    fn read_only() -> MappingRights {
        MappingRights {
            read: true,
            write: false,
            execute: false,
            copy_on_write: false,
            nocache: false,
        }
    }

    #[test]
    fn creation_derives_deterministic_unique_asids() {
        let manager = registered_domains(&[1, 2, 3]);
        let mut spaces = AddressSpaceManager::new();

        let asid_1 = spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space 1 creation should succeed");
        let asid_2 = spaces
            .create_address_space(domain(2), &manager, 1_000)
            .expect("space 2 creation should succeed");
        let asid_3 = spaces
            .create_address_space(domain(3), &manager, 1_000)
            .expect("space 3 creation should succeed");

        // ASIDs are immutable and distinct by construction (RFC-0009.1/.2).
        assert_ne!(asid_1, asid_2);
        assert_ne!(asid_1, asid_3);
        assert_ne!(asid_2, asid_3);
        assert_eq!(spaces.asid_of(domain(1)), Ok(asid_1));
        assert_eq!(spaces.asid_of(domain(2)), Ok(asid_2));
        assert_eq!(spaces.asid_of(domain(3)), Ok(asid_3));
        // Deterministic derivation: the same domain always yields the same ASID.
        let mut again = AddressSpaceManager::new();
        assert_eq!(
            again.create_address_space(domain(1), &manager, 1_000),
            Ok(asid_1)
        );
        assert_eq!(spaces.space_count(), 3);
    }

    #[test]
    fn unknown_domain_cannot_create_address_space() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        assert_eq!(
            spaces.create_address_space(domain(2), &manager, 1_000),
            Err(AddressSpaceError::UnknownDomain)
        );
    }

    #[test]
    fn duplicate_address_space_is_rejected() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("first space creation should succeed");
        assert_eq!(
            spaces.create_address_space(domain(1), &manager, 2_000),
            Err(AddressSpaceError::DuplicateAddressSpace)
        );
    }

    #[test]
    fn map_requires_map_capability() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        let request = mapping_request(0x1000, 4096, PageSize::Base4KiB, read_only());

        // No MAP right.
        let no_map = Capability {
            rights: Rights::READ,
            ..map_cap(Rights::READ)
        };
        assert_eq!(
            spaces.map(domain(1), &no_map, request, 1_000),
            Err(AddressSpaceError::MissingMapCapability)
        );
        // Not a memory capability.
        let not_memory = Capability {
            object_type: ObjectType::Endpoint,
            rights: Rights::MAP,
            ..map_cap(Rights::MAP)
        };
        assert_eq!(
            spaces.map(domain(1), &not_memory, request, 1_000),
            Err(AddressSpaceError::MissingMapCapability)
        );
        // Valid MAP capability is accepted.
        let map_cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            ..map_cap(Rights::READ)
        };
        assert!(spaces.map(domain(1), &map_cap, request, 1_000).is_ok());
    }

    #[test]
    fn map_cannot_elevate_rights_beyond_the_capability() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        // Capability grants only READ; a WRITE mapping request must be refused.
        let read_cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            ..map_cap(Rights::READ)
        };
        let write_request = mapping_request(
            0x1000,
            4096,
            PageSize::Base4KiB,
            MappingRights {
                read: false,
                write: true,
                execute: false,
                copy_on_write: false,
                nocache: false,
            },
        );
        assert_eq!(
            spaces.map(domain(1), &read_cap, write_request, 1_000),
            Err(AddressSpaceError::InsufficientRights)
        );
        // The rejected mapping left no trace: a conforming read mapping still applies.
        let read_request = mapping_request(0x1000, 4096, PageSize::Base4KiB, read_only());
        assert!(
            spaces
                .map(domain(1), &read_cap, read_request, 1_000)
                .is_ok()
        );
    }

    #[test]
    fn bounds_violation_is_rejected() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        let request = mapping_request(0x1000, 4096, PageSize::Base4KiB, read_only());
        let small_cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            bounds: bounds(0, 1024),
            ..map_cap(Rights::READ)
        };
        assert_eq!(
            spaces.map(domain(1), &small_cap, request, 1_000),
            Err(AddressSpaceError::BoundsViolation)
        );
    }

    #[test]
    fn unaligned_range_is_rejected() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            ..map_cap(Rights::READ)
        };
        // Unaligned vaddr.
        assert_eq!(
            spaces.map(
                domain(1),
                &cap,
                mapping_request(0x1001, 4096, PageSize::Base4KiB, read_only()),
                1_000
            ),
            Err(AddressSpaceError::NotPageAligned)
        );
        // Size not a multiple of the page size.
        assert_eq!(
            spaces.map(
                domain(1),
                &cap,
                mapping_request(0x2000, 100, PageSize::Base4KiB, read_only()),
                1_000
            ),
            Err(AddressSpaceError::NotPageAligned)
        );
    }

    #[test]
    fn overlapping_mapping_is_rejected() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            bounds: bounds(0, 8192),
            ..map_cap(Rights::READ)
        };
        let first = mapping_request(0x1000, 4096, PageSize::Base4KiB, read_only());
        assert!(spaces.map(domain(1), &cap, first, 1_000).is_ok());
        // Overlapping second mapping is rejected.
        let second = mapping_request(0x0, 0x2000, PageSize::Base4KiB, read_only());
        assert_eq!(
            spaces.map(domain(1), &cap, second, 1_000),
            Err(AddressSpaceError::RegionOverlap)
        );
        // Adjacent (non-overlapping) mapping is accepted.
        let third = mapping_request(0x2000, 4096, PageSize::Base4KiB, read_only());
        assert!(spaces.map(domain(1), &cap, third, 1_000).is_ok());
    }

    #[test]
    fn read_only_mapping_enforces_rights_at_every_access() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            ..map_cap(Rights::READ)
        };
        assert!(
            spaces
                .map(
                    domain(1),
                    &cap,
                    mapping_request(0x1000, 4096, PageSize::Base4KiB, read_only()),
                    1_000
                )
                .is_ok()
        );

        assert!(
            spaces
                .access(domain(1), 0x1000, 0x100, MemoryAccess::Read, 2_000)
                .is_ok()
        );
        // RFC-0009.7: a READ-only mapping never permits a store or execution.
        assert_eq!(
            spaces.access(domain(1), 0x1000, 0x100, MemoryAccess::Write, 2_000),
            Err(AddressSpaceError::InsufficientRights)
        );
        assert_eq!(
            spaces.access(domain(1), 0x1000, 0x100, MemoryAccess::Execute, 2_000),
            Err(AddressSpaceError::InsufficientRights)
        );
        // Outside the mapped range faults even though the capability is live.
        assert_eq!(
            spaces.access(domain(1), 0x5000, 0x100, MemoryAccess::Read, 2_000),
            Err(AddressSpaceError::Unmapped)
        );
    }

    #[test]
    fn read_write_mapping_grants_both() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ).union(Rights::WRITE),
            ..map_cap(Rights::READ)
        };
        let read_write = MappingRights {
            read: true,
            write: true,
            execute: false,
            copy_on_write: false,
            nocache: false,
        };
        assert!(
            spaces
                .map(
                    domain(1),
                    &cap,
                    mapping_request(0x1000, 4096, PageSize::Base4KiB, read_write),
                    1_000
                )
                .is_ok()
        );
        assert!(
            spaces
                .access(domain(1), 0x1000, 0x100, MemoryAccess::Write, 2_000)
                .is_ok()
        );
        assert!(
            spaces
                .access(domain(1), 0x1000, 0x100, MemoryAccess::Read, 2_000)
                .is_ok()
        );
    }

    #[test]
    fn page_size_selection_is_explicit_and_recorded() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            bounds: bounds(0, 2 * 1024 * 1024),
            ..map_cap(Rights::READ)
        };

        assert!(
            spaces
                .map(
                    domain(1),
                    &cap,
                    mapping_request(0x1000, 4096, PageSize::Base4KiB, read_only()),
                    1_000
                )
                .is_ok()
        );
        assert_eq!(
            spaces.page_size_of(domain(1), 0x1000),
            Ok(PageSize::Base4KiB)
        );

        let large = MappingRights {
            read: true,
            write: false,
            execute: false,
            copy_on_write: false,
            nocache: false,
        };
        assert!(
            spaces
                .map(
                    domain(1),
                    &cap,
                    mapping_request(0x200000, 2 * 1024 * 1024, PageSize::Large2MiB, large),
                    1_000
                )
                .is_ok()
        );
        assert_eq!(
            spaces.page_size_of(domain(1), 0x200000),
            Ok(PageSize::Large2MiB)
        );
    }

    #[test]
    fn map_and_unmap_are_atomic() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ).union(Rights::WRITE),
            ..map_cap(Rights::READ)
        };
        let read_write = MappingRights {
            read: true,
            write: true,
            execute: false,
            copy_on_write: false,
            nocache: false,
        };
        let request = mapping_request(0x3000, 0x1000, PageSize::Base4KiB, read_write);

        // A completed mapping is immediately and fully observable.
        assert!(spaces.map(domain(1), &cap, request, 1_000).is_ok());
        assert!(
            spaces
                .access(domain(1), 0x3000, 0x100, MemoryAccess::Write, 2_000)
                .is_ok()
        );
        assert_eq!(spaces.state_of(domain(1)), Ok(AddressSpaceState::Active));

        // A completed unmap removes the mapping entirely; no partially
        // unmapped state is observable.
        assert!(spaces.unmap(domain(1), 0x3000, 0x1000, 3_000).is_ok());
        assert!(
            matches!(
                spaces.access(domain(1), 0x3000, 0x100, MemoryAccess::Read, 4_000),
                Err(AddressSpaceError::Unmapped) | Err(AddressSpaceError::AddressSpaceEmpty)
            ),
            "access was possible after a completed unmap"
        );
        assert_eq!(spaces.state_of(domain(1)), Ok(AddressSpaceState::Empty));

        // Re-mapping after unmap restores access.
        assert!(spaces.map(domain(1), &cap, request, 5_000).is_ok());
        assert!(
            spaces
                .access(domain(1), 0x3000, 0x100, MemoryAccess::Write, 6_000)
                .is_ok()
        );
        assert_eq!(spaces.state_of(domain(1)), Ok(AddressSpaceState::Active));
    }

    #[test]
    fn failed_map_leaves_state_unchanged() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            bounds: bounds(0, 0x2000),
            ..map_cap(Rights::READ)
        };
        let first = mapping_request(0x1000, 4096, PageSize::Base4KiB, read_only());
        assert!(spaces.map(domain(1), &cap, first, 1_000).is_ok());
        // Overlapping mapping rejected: address space unchanged. Page-aligned
        // ranges that overlap first (0x1000..0x2000): e.g. 0x0..0x2000.
        let second = mapping_request(0x0, 0x2000, PageSize::Base4KiB, read_only());
        assert_eq!(
            spaces.map(domain(1), &cap, second, 2_000),
            Err(AddressSpaceError::RegionOverlap)
        );
        assert!(
            spaces
                .access(domain(1), 0x1000, 0x100, MemoryAccess::Read, 3_000)
                .is_ok()
        );
        // 0x2000..0x2100 is not covered by the first mapping and the overlapping
        // second mapping was rejected, so it remains unmapped.
        assert_eq!(
            spaces.access(domain(1), 0x2000, 0x100, MemoryAccess::Read, 3_000),
            Err(AddressSpaceError::Unmapped)
        );
    }

    #[test]
    fn unmap_flushes_and_invalidates_before_return() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            ..map_cap(Rights::READ)
        };
        assert!(
            spaces
                .map(
                    domain(1),
                    &cap,
                    mapping_request(0x4000, 4096, PageSize::Base4KiB, read_only()),
                    1_000
                )
                .is_ok()
        );
        assert_eq!(spaces.invalidation_count(), 0);

        assert!(spaces.unmap(domain(1), 0x4000, 4096, 2_000).is_ok());
        // RFC-0009.12: flush+invalidation completed before unmap returned.
        assert_eq!(spaces.invalidation_count(), 1);
        let record = &spaces.invalidations()[0];
        assert_eq!(record.vaddr, 0x4000);
        assert_eq!(record.size, 4096);
        assert_eq!(record.timestamp_ns, 2_000);
        // RFC-0009.13: RegionUnmapped event emitted.
        let last = spaces.events().last().expect("an event was recorded");
        assert!(matches!(
            last,
            AddressSpaceEvent::RegionUnmapped {
                vaddr: 0x4000,
                size: 4096,
                ..
            }
        ));
        assert_eq!(spaces.state_of(domain(1)), Ok(AddressSpaceState::Empty));
    }

    #[test]
    fn unmap_of_unknown_range_is_rejected() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        assert_eq!(
            spaces.unmap(domain(1), 0x1000, 4096, 1_000),
            Err(AddressSpaceError::AddressSpaceEmpty)
        );
        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            ..map_cap(Rights::READ)
        };
        assert!(
            spaces
                .map(
                    domain(1),
                    &cap,
                    mapping_request(0x1000, 4096, PageSize::Base4KiB, read_only()),
                    1_000
                )
                .is_ok()
        );
        // Inexact unmap range is rejected.
        assert_eq!(
            spaces.unmap(domain(1), 0x1000, 2048, 2_000),
            Err(AddressSpaceError::RegionNotMapped)
        );
    }

    #[test]
    fn asid_lifecycle_destroy_only_from_empty() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        let asid = spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        assert_eq!(spaces.state_of(domain(1)), Ok(AddressSpaceState::Created));

        // RFC-0009.15: ASID_DESTROYED is reachable only from ASID_EMPTY.
        assert_eq!(
            spaces.destroy_address_space(domain(1), 1_500),
            Err(AddressSpaceError::InvalidLifecycleTransition)
        );

        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            ..map_cap(Rights::READ)
        };
        assert!(
            spaces
                .map(
                    domain(1),
                    &cap,
                    mapping_request(0x5000, 4096, PageSize::Base4KiB, read_only()),
                    1_000
                )
                .is_ok()
        );
        assert_eq!(spaces.state_of(domain(1)), Ok(AddressSpaceState::Active));
        assert_eq!(
            spaces.destroy_address_space(domain(1), 2_000),
            Err(AddressSpaceError::InvalidLifecycleTransition)
        );

        assert!(spaces.unmap(domain(1), 0x5000, 4096, 3_000).is_ok());
        assert_eq!(spaces.state_of(domain(1)), Ok(AddressSpaceState::Empty));
        assert!(spaces.destroy_address_space(domain(1), 4_000).is_ok());
        assert_eq!(
            spaces.asid_of(domain(1)),
            Err(AddressSpaceError::NoAddressSpace)
        );

        // ASIDs are disjoint: a different domain derives a different ASID
        // (RFC-0009.2).
        let mut other = AddressSpaceManager::new();
        let other_domains = registered_domains(&[2]);
        let other_asid = other
            .create_address_space(domain(2), &other_domains, 1_000)
            .expect("other space creation should succeed");
        assert_ne!(other_asid, asid);
    }

    #[test]
    fn events_record_create_map_unmap_destroy() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            ..map_cap(Rights::READ)
        };
        assert!(
            spaces
                .map(
                    domain(1),
                    &cap,
                    mapping_request(0x6000, 4096, PageSize::Base4KiB, read_only()),
                    2_000
                )
                .is_ok()
        );
        assert!(spaces.unmap(domain(1), 0x6000, 4096, 3_000).is_ok());
        assert!(spaces.destroy_address_space(domain(1), 4_000).is_ok());

        assert_eq!(spaces.event_count(), 4);
        let events = spaces.events();
        assert!(matches!(
            events[0],
            AddressSpaceEvent::AddressSpaceCreated {
                domain: d,
                timestamp_ns: 1_000,
                ..
            } if d == domain(1)
        ));
        assert!(matches!(
            events[1],
            AddressSpaceEvent::RegionMapped {
                vaddr: 0x6000,
                size: 4096,
                page_size: PageSize::Base4KiB,
                timestamp_ns: 2_000,
                ..
            }
        ));
        assert!(matches!(
            events[2],
            AddressSpaceEvent::RegionUnmapped {
                vaddr: 0x6000,
                size: 4096,
                timestamp_ns: 3_000,
                ..
            }
        ));
        assert!(matches!(
            events[3],
            AddressSpaceEvent::AddressSpaceDestroyed {
                timestamp_ns: 4_000,
                ..
            }
        ));
    }

    #[test]
    fn access_requires_active_space() {
        let manager = registered_domains(&[1]);
        let mut spaces = AddressSpaceManager::new();
        spaces
            .create_address_space(domain(1), &manager, 1_000)
            .expect("space creation should succeed");
        // Created state has no mappings: access faults.
        assert_eq!(
            spaces.access(domain(1), 0x1000, 4096, MemoryAccess::Read, 2_000),
            Err(AddressSpaceError::AddressSpaceEmpty)
        );
        // After destruction the space is gone.
        let cap = Capability {
            rights: Rights::MAP.union(Rights::READ),
            ..map_cap(Rights::READ)
        };
        assert!(
            spaces
                .map(
                    domain(1),
                    &cap,
                    mapping_request(0x1000, 4096, PageSize::Base4KiB, read_only()),
                    1_000
                )
                .is_ok()
        );
        assert!(spaces.unmap(domain(1), 0x1000, 4096, 2_000).is_ok());
        assert!(spaces.destroy_address_space(domain(1), 3_000).is_ok());
        assert_eq!(
            spaces.access(domain(1), 0x1000, 4096, MemoryAccess::Read, 4_000),
            Err(AddressSpaceError::NoAddressSpace)
        );
    }
}
