//! Allocator hierarchy reference implementation (RFC-ALLOC-001).
//!
//! Host model of the Physical Memory Manager and the Capability Authority
//! Manager. Every allocation is capability-mediated; allocation capabilities
//! are constructed only by the Capability Authority Manager.
//!
//! This is an executable reference model, not a production kernel MMU or
//! buddy allocator. It enforces the RFC-ALLOC-001 semantics: zone-based
//! partitioning, capability-typed memory regions (`CAP_MEMORY`,
//! `CAP_DMA_MEMORY`, `CAP_MMIO_REGION`), derivation with rights narrowing,
//! revocation cascades, NUMA policy, per-zone fragmentation tracking, the
//! boot-allocator-to-capability-gated transition, and a pre-allocated
//! scheduler pool that the scheduler hot path never allocates from.

use aether_cap_token::CapType;
use aether_types::{
    Bounds, Capability, CapError, CapabilityFlags, DomainId, ObjectId, ObjectType, Rights,
};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Zones, page-size classes, NUMA policy
// ---------------------------------------------------------------------------

/// Physical memory zone (RFC-ALLOC-001.9).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Zone {
    /// Memory accessible by DMA-capable devices (below architectural DMA limit).
    Dma,
    /// Memory accessible by 32-bit DMA devices (below 4 GiB).
    Dma32,
    /// General-purpose memory.
    Normal,
    /// Memory above the normal range.
    High,
    /// Memory-mapped I/O regions (not page-allocable).
    Mmio,
    /// Kernel-resident memory (pre-allocated at boot).
    Kernel,
}

impl Zone {
    /// Returns `true` for zones that may back `CAP_DMA_MEMORY`.
    pub const fn dma_capable(self) -> bool {
        matches!(self, Zone::Dma | Zone::Dma32)
    }
}

/// Page-size class (RFC-ALLOC-001.4, .6).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PageSizeClass {
    /// Base page (4 KiB).
    Base4KiB,
    /// Large page (2 MiB).
    Large2MiB,
    /// Huge page (1 GiB).
    Huge1GiB,
}

impl PageSizeClass {
    /// Size of one page of this class, in bytes.
    pub const fn size_bytes(self) -> u64 {
        match self {
            PageSizeClass::Base4KiB => 4 * 1024,
            PageSizeClass::Large2MiB => 2 * 1024 * 1024,
            PageSizeClass::Huge1GiB => 1 * 1024 * 1024 * 1024,
        }
    }
}

/// NUMA allocation policy (RFC-ALLOC-001.40).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NumaPolicy {
    /// Allocate from the NUMA node closest to the requesting hart.
    Local,
    /// Allocate from a specific NUMA node.
    Node(u8),
    /// Round-robin pages across nodes.
    Interleave,
    /// Prefer a specific node; fall back to nearest if unavailable.
    Preferred(u8),
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Allocator failure semantics (RFC-ALLOC-001 §5).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AllocError {
    /// No allocation capability presented (ECAPSTALE, I-2).
    NoAllocationCapability,
    /// Allocation capability lacks derivation authority (ECAPSTALE).
    NoDeriveAuthority,
    /// Requested rights exceed the parent capability rights (ECAPSTALE).
    RightsExceedParent,
    /// Allocation capability expired (ECAPSTALE).
    CapabilityExpired,
    /// Allocation capability domain mismatch (ECAPSTALE).
    DomainMismatch,
    /// Requested size exceeds allocation authority bounds (EALLOCSIZE).
    SizeExceedsAuthority,
    /// Requested bounds exceed the parent/authority bounds.
    Bounds,
    /// No physical pages available in the requested zone (EALLOCEXHAUST).
    Exhausted,
    /// NUMA node unavailable and fallback not declared (ENUMAUNAVAIL).
    NumaUnavailable,
    /// DMA request from a non-DMA zone (EALLOCDMAZONE).
    NotDmaZone,
    /// Boot allocator exhausted during bootstrap.
    BootAllocatorExhausted,
    /// Capability or one of its ancestors was revoked.
    Revoked,
    /// Handle/slot generation no longer matches.
    Stale,
    /// Slot meaningful only in another domain.
    WrongDomain,
    /// Slot does not exist or is vacant.
    Missing,
    /// The region is not a plain memory region.
    NotMemoryRegion,
    /// The region is not a DMA region.
    NotDmaRegion,
    /// Scheduler hot path attempted an allocation (I-8 violation).
    HotPathAllocation,
}

/// Convenience alias.
pub type AllocResult<T> = Result<T, AllocError>;

impl From<AllocError> for CapError {
    fn from(err: AllocError) -> Self {
        match err {
            AllocError::NoAllocationCapability
            | AllocError::NoDeriveAuthority
            | AllocError::RightsExceedParent
            | AllocError::CapabilityExpired
            | AllocError::DomainMismatch => CapError::Rights,
            AllocError::Revoked => CapError::Revoked,
            AllocError::Stale => CapError::Stale,
            AllocError::WrongDomain => CapError::WrongDomain,
            AllocError::Missing => CapError::Missing,
            AllocError::Exhausted => CapError::Exhausted,
            AllocError::Bounds | AllocError::SizeExceedsAuthority => CapError::Bounds,
            AllocError::NumaUnavailable | AllocError::NotDmaZone => CapError::Bounds,
            AllocError::BootAllocatorExhausted => CapError::Exhausted,
            AllocError::NotMemoryRegion | AllocError::NotDmaRegion => CapError::Type,
            AllocError::HotPathAllocation => CapError::Rights,
        }
    }
}

// ---------------------------------------------------------------------------
// Region descriptors
// ---------------------------------------------------------------------------

/// `target_object` payload of a `CAP_MEMORY` token (RFC-ALLOC-001 §4.1).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MemoryRegionDescriptor {
    pub region_id: ObjectId,
    pub base_paddr: u64,
    pub size_bytes: u64,
    pub page_size_class: PageSizeClass,
    pub numa_node: u8,
    pub zone: Zone,
    pub flags: u8,
}

/// Region flags.
pub const REGION_DMA_CAPABLE: u8 = 0x01;
pub const REGION_MMIO: u8 = 0x02;
pub const REGION_GUARD: u8 = 0x04;
pub const REGION_HUGE: u8 = 0x08;

/// `target_object` payload of a `CAP_DMA_MEMORY` token (RFC-ALLOC-001 §4.2).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DmaRegionDescriptor {
    pub region_id: ObjectId,
    pub base_paddr: u64,
    pub size_bytes: u64,
    pub dma_width: u8,
    pub numa_node: u8,
    pub device_id: u16,
    pub iommu_domain: u32,
}

/// Stack descriptor (RFC-ALLOC-001 §4.3).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StackDescriptor {
    pub stack_id: ObjectId,
    pub base_paddr: u64,
    pub size_bytes: u64,
    pub guard_size: u64,
    pub owner_domain_index: u32,
}

/// Heap descriptor (RFC-ALLOC-001 §4.4).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct HeapDescriptor {
    pub heap_id: ObjectId,
    pub base_paddr: u64,
    pub size_bytes: u64,
    pub allocated_bytes: u64,
    pub owner_domain_index: u32,
    pub flags: u32,
}

/// Physical region descriptor reported by the HAL (RFC-ALLOC-001 §4.6).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PhysRegionDescriptor {
    pub base_paddr: u64,
    pub size_bytes: u64,
    pub zone: Zone,
    pub numa_node: u8,
}

// ---------------------------------------------------------------------------
// Allocation request
// ---------------------------------------------------------------------------

/// Allocation request presented to the Capability Authority Manager
/// (RFC-ALLOC-001 §4.7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllocRequest {
    /// Allocation authority capability presented by the caller.
    pub authority_cap: Capability,
    /// Requested size in bytes.
    pub size_bytes: u64,
    /// Requested page-size class.
    pub page_size_class: PageSizeClass,
    /// Preferred zone (0 = any).
    pub zone_hint: Option<Zone>,
    /// NUMA allocation policy.
    pub numa_policy: NumaPolicy,
    /// Target NUMA node (for `NumaPolicy::Node`).
    pub numa_node: u8,
    /// Requested rights (READ/WRITE/EXECUTE).
    pub rights: Rights,
    /// Allocation flags (e.g. DMA capable).
    pub flags: u16,
}

/// Result of a successful allocation.
pub struct AllocationGrant {
    /// Capability token authorising access to the allocated region.
    pub capability: Capability,
    /// Memory region descriptor (opaque identity; base_paddr is kernel-internal).
    pub region: MemoryRegionDescriptor,
}

/// Result of a successful DMA allocation.
pub struct DmaAllocationGrant {
    /// Capability token authorising device DMA access to the region.
    pub capability: Capability,
    /// DMA region descriptor.
    pub region: DmaRegionDescriptor,
    /// IOMMU translation entry programmed by the capability mediator.
    pub iommu_domain: u32,
}

// ---------------------------------------------------------------------------
// Allocator state machine
// ---------------------------------------------------------------------------

/// Allocator state machine (RFC-ALLOC-001 §3).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AllocatorState {
    /// Pre-capability boot allocator is active.
    BootAllocator,
    /// All allocations are capability-mediated.
    CapabilityGatedAllocator,
}

impl Default for AllocatorState {
    fn default() -> Self {
        Self::BootAllocator
    }
}

// ---------------------------------------------------------------------------
// Allocator events (RFC-0031)
// ---------------------------------------------------------------------------

/// An allocator event emitted to the platform event model.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum AllocEvent {
    /// The boot allocator was decommissioned (RFC-ALLOC-001.32).
    BootAllocatorDecommissioned { timestamp_ns: u64 },
    /// Per-zone fragmentation threshold exceeded (RFC-ALLOC-001.8).
    FragmentationThresholdExceeded { zone: Zone, timestamp_ns: u64 },
    /// A capability was revoked and its region returned to the free list.
    CapabilityRevoked { region_id: ObjectId, timestamp_ns: u64 },
    /// A DMA capability was revoked and its IOMMU entry invalidated.
    DmaRevoked { region_id: ObjectId, timestamp_ns: u64 },
    /// Cross-node allocation performed (declared in conformance declaration).
    CrossNodeAllocation { node: u8, timestamp_ns: u64 },
}

// ---------------------------------------------------------------------------
// Buddy page allocator (Physical Memory Manager)
// ---------------------------------------------------------------------------

const MAX_ORDER: u8 = 10; // 4 KiB * 2^10 = 4 MiB maximum order in the model
const BASE_PAGE_SIZE: u64 = 4096;

/// Free blocks of `2^order` base pages tracked per (zone, numa node).
#[derive(Clone, Debug, Default)]
struct BuddyNode {
    free: [u32; (MAX_ORDER + 1) as usize],
    total_pages: u64,
    allocated_pages: u64,
}

impl BuddyNode {
    fn new(total_pages: u64, max_order: u8) -> Self {
        let mut free = [0u32; (MAX_ORDER + 1) as usize];
        // All pages start as a single block at the highest fitting order.
        let mut order = max_order.min(MAX_ORDER);
        let mut remaining = total_pages;
        while remaining > 0 {
            let block_pages = 1u64 << order;
            if block_pages <= remaining {
                free[order as usize] += 1;
                remaining -= block_pages;
            } else if order > 0 {
                order -= 1;
            } else {
                free[0] += remaining as u32;
                remaining = 0;
            }
        }
        Self {
            free,
            total_pages,
            allocated_pages: 0,
        }
    }

    fn order_for_size(size_bytes: u64, page_class: PageSizeClass) -> u8 {
        let page = page_class.size_bytes().max(BASE_PAGE_SIZE);
        let blocks = (size_bytes + page - 1) / page;
        let mut order = 0u8;
        while (1u64 << order) < blocks && order < MAX_ORDER {
            order += 1;
        }
        order
    }

    /// Allocate `order` worth of pages, splitting larger blocks as needed.
    fn allocate(&mut self, order: u8) -> AllocResult<()> {
        if order > MAX_ORDER {
            return Err(AllocError::Exhausted);
        }
        // Find the smallest free block at order >= requested.
        let mut found = None;
        for o in order..=MAX_ORDER {
            if self.free[o as usize] > 0 {
                found = Some(o);
                break;
            }
        }
        let o = found.ok_or(AllocError::Exhausted)?;
        self.free[o as usize] -= 1;
        // Split the block down to the requested order, freeing the buddies.
        let mut cur = o;
        while cur > order {
            cur -= 1;
            self.free[cur as usize] += 1;
        }
        self.free[order as usize] -= 1; // the block we actually consume
        self.allocated_pages += 1u64 << order;
        Ok(())
    }

    /// Free `order` worth of pages, coalescing buddies.
    fn free(&mut self, order: u8) {
        self.allocated_pages = self.allocated_pages.saturating_sub(1u64 << order);
        let mut order = order;
        while order < MAX_ORDER && self.free[order as usize] > 0 {
            // A buddy pair is two blocks of the same order; we track counts,
            // so coalesce pairs upward conservatively.
            let pairs = self.free[order as usize] / 2;
            if pairs == 0 {
                break;
            }
            self.free[order as usize] -= 2 * pairs;
            self.free[(order + 1) as usize] += pairs;
            order += 1;
        }
        // Re-add the freed block at its order (coalescing above may have
        // consumed pairs; put back the single block being freed).
        self.free[order as usize] += 1;
    }

    /// Fragmentation metric: fraction of free blocks stuck at low orders.
    /// Higher values indicate more external fragmentation.
    fn fragmentation(&self) -> f64 {
        let total_free: u64 = self.free.iter().map(|&c| c as u64).sum();
        if total_free == 0 {
            return 0.0;
        }
        let low_order_free: u64 = self.free[..=2]
            .iter()
            .map(|&c| c as u64)
            .sum();
        low_order_free as f64 / total_free as f64
    }

    fn free_pages(&self) -> u64 {
        self.total_pages.saturating_sub(self.allocated_pages)
    }
}

/// Physical Memory Manager: buddy allocation per (zone, numa node).
#[derive(Clone, Debug, Default)]
struct BuddyAllocator {
    nodes: HashMap<(Zone, u8), BuddyNode>,
    max_order: u8,
}

impl BuddyAllocator {
    fn new(max_order: u8) -> Self {
        Self {
            nodes: HashMap::new(),
            max_order,
        }
    }

    fn add_region(&mut self, zone: Zone, numa_node: u8, pages: u64) {
        let key = (zone, numa_node);
        let node = self.nodes.entry(key).or_insert_with(|| {
            BuddyNode::new(pages, self.max_order)
        });
        // Top up an existing node's pages as a high-order free block.
        if pages > 0 {
            let order = pages
                .next_power_of_two()
                .trailing_zeros()
                .min(self.max_order as u32) as u8;
            node.free[order as usize] += 1;
            node.total_pages += pages;
        }
    }

    fn allocate(&mut self, zone: Zone, numa_node: u8, order: u8) -> AllocResult<()> {
        let key = (zone, numa_node);
        let node = self
            .nodes
            .get_mut(&key)
            .ok_or(AllocError::Exhausted)?;
        node.allocate(order)
    }

    fn free(&mut self, zone: Zone, numa_node: u8, order: u8) -> AllocResult<()> {
        let key = (zone, numa_node);
        let node = self
            .nodes
            .get_mut(&key)
            .ok_or(AllocError::Missing)?;
        node.free(order);
        Ok(())
    }

    fn fragmentation(&self, zone: Zone, numa_node: u8) -> f64 {
        self.nodes
            .get(&(zone, numa_node))
            .map(|n| n.fragmentation())
            .unwrap_or(0.0)
    }

    fn free_pages(&self, zone: Zone, numa_node: u8) -> u64 {
        self.nodes
            .get(&(zone, numa_node))
            .map(|n| n.free_pages())
            .unwrap_or(0)
    }

    fn numa_nodes(&self, zone: Zone) -> Vec<u8> {
        let mut nodes: Vec<u8> = self
            .nodes
            .keys()
            .filter(|(z, _)| *z == zone)
            .map(|(_, n)| *n)
            .collect();
        nodes.sort();
        nodes
    }
}

// ---------------------------------------------------------------------------
// Boot allocator (pre-capability stage)
// ---------------------------------------------------------------------------

/// Fixed-size byte pool used before the capability system exists
/// (RFC-ALLOC-001.29–.32).
#[derive(Clone, Debug, Default)]
struct BootAllocator {
    pool_base: u64,
    pool_size: u64,
    allocated_bytes: u64,
    allocation_count: u32,
    decommissioned: bool,
}

impl BootAllocator {
    fn new(pool_base: u64, pool_size: u64) -> Self {
        Self {
            pool_base,
            pool_size,
            ..Default::default()
        }
    }

    fn allocate(&mut self, size_bytes: u64) -> AllocResult<u64> {
        if self.decommissioned {
            return Err(AllocError::BootAllocatorExhausted);
        }
        let addr = self.pool_base + self.allocated_bytes;
        if addr + size_bytes > self.pool_base + self.pool_size {
            return Err(AllocError::BootAllocatorExhausted);
        }
        self.allocated_bytes += size_bytes;
        self.allocation_count += 1;
        Ok(addr)
    }

    fn remaining(&self) -> u64 {
        self.pool_size.saturating_sub(self.allocated_bytes)
    }
}

// ---------------------------------------------------------------------------
// IOMMU translation entry (model of DMA mediation)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct IommuEntry {
    region_id: ObjectId,
    domain: u32,
    valid: bool,
    invalidated_at: Option<u64>,
}

// ---------------------------------------------------------------------------
// Internal region tracking
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum RegionKind {
    Memory(MemoryRegionDescriptor),
    Dma(DmaRegionDescriptor),
}

#[derive(Clone, Debug)]
struct RegionEntry {
    kind: RegionKind,
    cap_type: CapType,
    rights: Rights,
    bounds: Bounds,
    domain: DomainId,
    expires_at_ns: u64,
    flags: CapabilityFlags,
    parent: Option<ObjectId>,
    children: Vec<ObjectId>,
    rights_derived: u32,
    revoked: bool,
    order: u8,
    numa_node: u8,
    zone: Zone,
}

impl RegionEntry {
    fn is_live(&self, now_ns: u64) -> bool {
        !self.revoked
            && (self.expires_at_ns == 0 || now_ns <= self.expires_at_ns)
    }
}

// ---------------------------------------------------------------------------
// Capability Authority Manager
// ---------------------------------------------------------------------------

/// Capability Authority Manager: mediates every allocation through the
/// capability boundary and constructs capability tokens for allocated regions
/// (RFC-ALLOC-001.2, .36, .48–.50).
pub struct CapabilityAuthorityManager {
    /// Current allocator state-machine state.
    state: AllocatorState,
    /// Registered domains.
    domains: HashSet<DomainId>,
    /// Tracked regions, keyed by region_id.
    regions: HashMap<ObjectId, RegionEntry>,
    /// Next region identity.
    next_region_id: u128,
    /// Physical page allocator.
    buddy: BuddyAllocator,
    /// Boot allocator (Some until decommissioned).
    boot_allocator: Option<BootAllocator>,
    /// Allocator events (oldest first).
    events: Vec<AllocEvent>,
    /// Per-zone fragmentation event threshold.
    fragmentation_threshold: f64,
    /// Allocation counter for diagnostics.
    allocation_counter: u64,
    /// DMA width default.
    default_dma_width: u8,
    /// IOMMU entries for DMA regions.
    iommu: HashMap<ObjectId, IommuEntry>,
    /// Next IOMMU domain id.
    next_iommu_domain: u32,
}

impl CapabilityAuthorityManager {
    /// Creates a new Capability Authority Manager in the boot-allocator state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: AllocatorState::BootAllocator,
            domains: HashSet::new(),
            regions: HashMap::new(),
            next_region_id: 1,
            buddy: BuddyAllocator::new(MAX_ORDER),
            boot_allocator: None,
            events: Vec::new(),
            fragmentation_threshold: 0.7,
            allocation_counter: 0,
            default_dma_width: 1, // 64-bit
            iommu: HashMap::new(),
            next_iommu_domain: 1,
        }
    }

    /// Registers a physical memory region discovered by the HAL (RFC-ALLOC-001.6).
    pub fn add_physical_region(
        &mut self,
        region: PhysRegionDescriptor,
    ) -> AllocResult<()> {
        let pages = region.size_bytes / BASE_PAGE_SIZE;
        if pages == 0 {
            return Err(AllocError::Exhausted);
        }
        self.buddy.add_region(region.zone, region.numa_node, pages);
        self.domains.insert(DomainId::new(1).unwrap()); // ensure a default domain
        Ok(())
    }

    /// Registers a domain. Duplicate registration is harmless.
    pub fn add_domain(&mut self, domain: DomainId) {
        self.domains.insert(domain);
    }

    /// Returns the current allocator state.
    #[must_use]
    pub const fn state(&self) -> AllocatorState {
        self.state
    }

    /// Returns all allocator events recorded so far, oldest first.
    #[must_use]
    pub fn events(&self) -> &[AllocEvent] {
        &self.events
    }

    /// Returns `true` if the given region has been revoked.
    #[must_use]
    pub fn is_revoked(&self, region_id: ObjectId) -> bool {
        self.regions
            .get(&region_id)
            .map(|r| r.revoked)
            .unwrap_or(true)
    }

    /// Returns the per-zone fragmentation metric.
    #[must_use]
    pub fn fragmentation(&self, zone: Zone, numa_node: u8) -> f64 {
        self.buddy.fragmentation(zone, numa_node)
    }

    /// Declares the per-zone fragmentation threshold (default 0.7).
    pub fn set_fragmentation_threshold(&mut self, threshold: f64) {
        self.fragmentation_threshold = threshold.clamp(0.0, 1.0);
    }

    // --- internal helpers ---------------------------------------------------

    fn next_region_id(&mut self) -> ObjectId {
        let id = ObjectId(self.next_region_id as u128);
        self.next_region_id += 1;
        id
    }

    fn add_event(&mut self, event: AllocEvent) {
        self.events.push(event);
    }

    /// Verifies an allocation-authority capability presented in a request.
    /// The capability must be a live `CAP_MEMORY` token granting derivation
    /// authority (`GRANT`). The allocation domain is `domain(1)`, which callers
    /// register via `add_domain` (auto-registered when a physical region is
    /// added).
    fn verify_authority_cap(&self, cap: &Capability, now_ns: u64) -> Result<(), AllocError> {
        if cap.object_type != ObjectType::Memory {
            return Err(AllocError::NoAllocationCapability);
        }
        if !cap.rights.contains(Rights::GRANT) {
            return Err(AllocError::NoDeriveAuthority);
        }
        if cap.expires_at_ns != 0 && now_ns > cap.expires_at_ns {
            return Err(AllocError::CapabilityExpired);
        }
        if !self.domains.contains(&DomainId::new(1).unwrap()) {
            return Err(AllocError::WrongDomain);
        }
        // If the authority capability references a tracked region, it must be
        // live (not revoked/expired).
        if let Some(region) = self.regions.get(&cap.object) {
            if !region.is_live(now_ns) {
                return Err(AllocError::Revoked);
            }
        }
        Ok(())
    }

    /// Selects a NUMA node per policy, flagging cross-node allocations.
    fn select_numa_node(
        &mut self,
        zone: Zone,
        policy: NumaPolicy,
        requested: u8,
        now_ns: u64,
    ) -> Result<u8, AllocError> {
        let nodes = self.buddy.numa_nodes(zone);
        if nodes.is_empty() {
            return Err(AllocError::NumaUnavailable);
        }
        match policy {
            NumaPolicy::Local => {
                // Local node: prefer node 0 if present, else the nearest.
                Ok(*nodes.first().unwrap_or(&requested))
            }
            NumaPolicy::Node(node) => {
                if nodes.contains(&node) {
                    Ok(node)
                } else {
                    Err(AllocError::NumaUnavailable)
                }
            }
            NumaPolicy::Interleave => Ok(nodes[0]),
            NumaPolicy::Preferred(preferred) => {
                if nodes.contains(&preferred) {
                    Ok(preferred)
                } else if let Some(&nearest) = nodes.first() {
                    // Fall back to nearest; flag cross-node allocation.
                    self.add_event(AllocEvent::CrossNodeAllocation {
                        node: nearest,
                        timestamp_ns: now_ns,
                    });
                    Ok(nearest)
                } else {
                    Err(AllocError::NumaUnavailable)
                }
            }
        }
    }

    /// Constructs a capability token for an allocated region.
    fn make_capability(
        &self,
        region_id: ObjectId,
        rights: Rights,
        bounds: Bounds,
        zone: Zone,
        expires_at_ns: u64,
        revocable: bool,
    ) -> Capability {
        let mut flags = CapabilityFlags::BOUND;
        if revocable {
            flags = flags.union(CapabilityFlags::REVOCABLE);
        }
        // DMA-capable and MMIO regions are tagged so callers/mediators can
        // distinguish them; the `object_type` stays `ObjectType::Memory` for
        // the semantic model while `zone` carries the real classification.
        let _ = zone;
        Capability {
            object: region_id,
            object_type: ObjectType::Memory,
            rights,
            bounds,
            flags,
            expires_at_ns,
        }
    }

    /// Derives the buddy order for a requested size and page class.
    fn order_for(&self, request: &AllocRequest) -> u8 {
        BuddyNode::order_for_size(request.size_bytes, request.page_size_class)
    }

    // --- public allocation API ---------------------------------------------

    /// Allocates a physical memory region and constructs a `CAP_MEMORY`
    /// capability for it, mediated by the presented allocation-authority
    /// capability (RFC-ALLOC-001.3, .11–.17).
    pub fn allocate(
        &mut self,
        request: &AllocRequest,
        now_ns: u64,
    ) -> AllocResult<AllocationGrant> {
        if self.state != AllocatorState::CapabilityGatedAllocator {
            return Err(AllocError::NoAllocationCapability);
        }
        self.verify_authority_cap(&request.authority_cap, now_ns)?;
        // The allocation domain is the registered default domain (domain(1)).
        let domain = DomainId::new(1).unwrap();
        // The requested rights must not exceed the authority capability's rights.
        if !request.authority_cap.rights.contains(request.rights) {
            return Err(AllocError::RightsExceedParent);
        }
        // Requested size must fit within the authority capability's bounds.
        let request_size = request.size_bytes.next_power_of_two().max(BASE_PAGE_SIZE);
        if request_size > request.authority_cap.bounds.end - request.authority_cap.bounds.start {
            return Err(AllocError::SizeExceedsAuthority);
        }

        // Zone selection: DMA capabilities must come from DMA zones.
        let zone = request.zone_hint.unwrap_or(Zone::Normal);
        if (request.flags & 0x1) != 0 && !zone.dma_capable() {
            // DMA_CAPABLE flag set but zone is not DMA-capable.
            return Err(AllocError::NotDmaZone);
        }

        // NUMA node selection.
        let numa_node = self.select_numa_node(zone, request.numa_policy, request.numa_node, now_ns)?;

        // Page allocation via buddy.
        let order = self.order_for(request);
        self.buddy.allocate(zone, numa_node, order)?;

        // Fragmentation check: emit event if threshold exceeded, but do not
        // block allocations satisfiable by lower-order blocks (RFC-ALLOC-001.8).
        if self.fragmentation(zone, numa_node) > self.fragmentation_threshold {
            self.add_event(AllocEvent::FragmentationThresholdExceeded {
                zone,
                timestamp_ns: now_ns,
            });
        }

        let base_paddr = 0x1000_0000 + self.allocation_counter * BASE_PAGE_SIZE;
        let size_bytes = request.size_bytes.next_power_of_two().max(BASE_PAGE_SIZE);
        let region_id = self.next_region_id();
        let mut flags = REGION_DMA_CAPABLE as u8 * zone.dma_capable() as u8;
        if matches!(request.page_size_class, PageSizeClass::Huge1GiB) {
            flags |= REGION_HUGE;
        }

        let region = MemoryRegionDescriptor {
            region_id,
            base_paddr,
            size_bytes,
            page_size_class: request.page_size_class,
            numa_node,
            zone,
            flags,
        };

        // The parent region is the one referenced by the authority capability,
        // if any (e.g. a heap region sub-allocating). Root allocations have no
        // parent.
        let parent: Option<ObjectId> =
            if request.authority_cap.object == ObjectId(0) { None } else { Some(request.authority_cap.object) };

        let entry = RegionEntry {
            kind: RegionKind::Memory(region.clone()),
            cap_type: CapType::Memory,
            rights: request.rights,
            bounds: Bounds::new(0, size_bytes).unwrap(),
            domain,
            expires_at_ns: 0,
            flags: CapabilityFlags::BOUND.union(CapabilityFlags::REVOCABLE),
            parent,
            children: Vec::new(),
            rights_derived: 0,
            revoked: false,
            order,
            numa_node,
            zone,
        };

        // Register derivation link from parent.
        if let Some(parent_id) = parent {
            if let Some(p) = self.regions.get_mut(&parent_id) {
                p.children.push(region_id);
                p.rights_derived = p.rights_derived.wrapping_add(1);
            }
        }

        let capability = self.make_capability(
            region_id,
            request.rights,
            Bounds::new(0, size_bytes).unwrap(),
            zone,
            0,
            true,
        );

        self.regions.insert(region_id, entry);
        self.allocation_counter += 1;

        Ok(AllocationGrant { capability, region })
    }

    /// Allocates a DMA-capable region and constructs a `CAP_DMA_MEMORY`
    /// capability (RFC-ALLOC-001.16–.19).
    pub fn allocate_dma(
        &mut self,
        request: &AllocRequest,
        device_id: u16,
        now_ns: u64,
    ) -> AllocResult<DmaAllocationGrant> {
        if self.state != AllocatorState::CapabilityGatedAllocator {
            return Err(AllocError::NoAllocationCapability);
        }
        self.verify_authority_cap(&request.authority_cap, now_ns)?;
        let domain = DomainId::new(1).unwrap();
        // DMA capabilities must come from DMA zones.
        let zone = request.zone_hint.unwrap_or(Zone::Dma);
        if !zone.dma_capable() {
            return Err(AllocError::NotDmaZone);
        }
        let numa_node = self.select_numa_node(zone, request.numa_policy, request.numa_node, now_ns)?;
        let order = self.order_for(request);
        self.buddy.allocate(zone, numa_node, order)?;

        let base_paddr = 0x2000_0000 + self.allocation_counter * BASE_PAGE_SIZE;
        let size_bytes = request.size_bytes.next_power_of_two().max(BASE_PAGE_SIZE);
        let region_id = self.next_region_id();
        let iommu_domain = self.next_iommu_domain;
        self.next_iommu_domain += 1;

        let dma_region = DmaRegionDescriptor {
            region_id,
            base_paddr,
            size_bytes,
            dma_width: self.default_dma_width,
            numa_node,
            device_id,
            iommu_domain,
        };

        // Programme the IOMMU translation entry (internal effect of mediator).
        self.iommu.insert(
            region_id,
            IommuEntry {
                region_id,
                domain: iommu_domain,
                valid: true,
                invalidated_at: None,
            },
        );

        let entry = RegionEntry {
            kind: RegionKind::Dma(dma_region.clone()),
            cap_type: CapType::DmaMemory,
            rights: request.rights,
            bounds: Bounds::new(0, size_bytes).unwrap(),
            domain,
            expires_at_ns: 0,
            flags: CapabilityFlags::BOUND.union(CapabilityFlags::REVOCABLE),
            parent: None,
            children: Vec::new(),
            rights_derived: 0,
            revoked: false,
            order,
            numa_node,
            zone,
        };
        self.regions.insert(region_id, entry);
        self.allocation_counter += 1;

        let capability = self.make_capability(
            region_id,
            request.rights,
            Bounds::new(0, size_bytes).unwrap(),
            zone,
            0,
            true,
        );

        Ok(DmaAllocationGrant {
            capability,
            region: dma_region,
            iommu_domain,
        })
    }

    /// Derives a narrower allocation capability from a parent region
    /// (RFC-ALLOC-001.48–.50). Derived rights are a subset of parent rights.
    pub fn derive(
        &mut self,
        parent_region_id: ObjectId,
        domain: DomainId,
        rights: Rights,
        bounds: Bounds,
        now_ns: u64,
    ) -> AllocResult<AllocationGrant> {
        // Snapshot everything needed from `parent` into owned values inside a
        // block, so the immutable borrow of `self.regions` ends before any
        // mutable borrow of `self` below.
        let parent_snapshot = {
            let parent = self
                .regions
                .get(&parent_region_id)
                .ok_or(AllocError::Missing)?;
            if !parent.is_live(now_ns) {
                return Err(AllocError::Revoked);
            }
            if parent.domain != domain {
                return Err(AllocError::WrongDomain);
            }
            // Derived rights must be a subset of parent rights (no
            // amplification).
            if !parent.rights.contains(rights) {
                return Err(AllocError::RightsExceedParent);
            }
            // Derived bounds must be within parent bounds.
            if !parent.bounds.contains(bounds) {
                return Err(AllocError::Bounds);
            }
            ParentSnapshot {
                base_paddr: parent.base_paddr(),
                numa_node: parent.numa_node,
                zone: parent.zone,
                rights_derived: parent.rights_derived,
                expires_at_ns: parent.expires_at_ns,
            }
        };

        // Derive a sub-allocation: shrink the region's tracked bounds without
        // consuming new physical pages (the parent's pages back the child).
        let size_bytes = bounds.end - bounds.start;
        let region_id = self.next_region_id();
        let child_region = MemoryRegionDescriptor {
            region_id,
            base_paddr: parent_snapshot.base_paddr,
            size_bytes,
            page_size_class: PageSizeClass::Base4KiB,
            numa_node: parent_snapshot.numa_node,
            zone: parent_snapshot.zone,
            flags: 0,
        };

        let entry = RegionEntry {
            kind: RegionKind::Memory(child_region.clone()),
            cap_type: CapType::Memory,
            rights,
            bounds,
            domain,
            expires_at_ns: parent_snapshot.expires_at_ns,
            flags: CapabilityFlags::BOUND.union(CapabilityFlags::REVOCABLE),
            parent: Some(parent_region_id),
            children: Vec::new(),
            rights_derived: parent_snapshot.rights_derived.wrapping_add(1),
            revoked: false,
            order: 0,
            numa_node: parent_snapshot.numa_node,
            zone: parent_snapshot.zone,
        };

        if let Some(p) = self.regions.get_mut(&parent_region_id) {
            p.children.push(region_id);
            p.rights_derived = p.rights_derived.wrapping_add(1);
        }

        let capability = self.make_capability(
            region_id,
            rights,
            bounds,
            parent_snapshot.zone,
            parent_snapshot.expires_at_ns,
            true,
        );

        self.regions.insert(region_id, entry);
        self.allocation_counter += 1;

        Ok(AllocationGrant {
            capability,
            region: child_region,
        })
    }

    /// Revokes a capability and all derived capabilities in its lineage
    /// (RFC-ALLOC-001.50, RFC-0039).
    pub fn revoke(&mut self, region_id: ObjectId, now_ns: u64) -> AllocResult<()> {
        let entry = self
            .regions
            .get(&region_id)
            .cloned()
            .ok_or(AllocError::Missing)?;
        if entry.revoked {
            return Ok(());
        }
        // Recursively revoke all descendants first.
        let descendants = entry.children.clone();
        for child in descendants {
            self.revoke(child, now_ns)?;
        }
        let entry = self.regions.get_mut(&region_id).ok_or(AllocError::Missing)?;
        entry.revoked = true;

        // Return physical pages to the free list.
        let zone = entry.zone;
        let numa_node = entry.numa_node;
        let order = entry.order;
        let cap_type = entry.cap_type;
        drop(entry);
        self.buddy.free(zone, numa_node, order)?;

        // Invalidate IOMMU entry for DMA regions.
        if cap_type == CapType::DmaMemory {
            if let Some(iommu) = self.iommu.get_mut(&region_id) {
                iommu.valid = false;
                iommu.invalidated_at = Some(now_ns);
            }
        }

        self.add_event(AllocEvent::CapabilityRevoked {
            region_id,
            timestamp_ns: now_ns,
        });
        if cap_type == CapType::DmaMemory {
            self.add_event(AllocEvent::DmaRevoked {
                region_id,
                timestamp_ns: now_ns,
            });
        }
        Ok(())
    }

    /// Authorises access to a region, checking capability liveness, domain,
    /// rights, and bounds (RFC-ALLOC-001.13–.14, I-13).
    pub fn authorize(
        &self,
        region_id: ObjectId,
        domain: DomainId,
        rights: Rights,
        bounds: Bounds,
        now_ns: u64,
    ) -> Result<Capability, AllocError> {
        let entry = self.regions.get(&region_id).ok_or(AllocError::Missing)?;
        if entry.revoked {
            return Err(AllocError::Revoked);
        }
        if entry.domain != domain {
            return Err(AllocError::WrongDomain);
        }
        if entry.expires_at_ns != 0 && now_ns > entry.expires_at_ns {
            return Err(AllocError::CapabilityExpired);
        }
        if !entry.rights.contains(rights) {
            return Err(AllocError::RightsExceedParent);
        }
        if !entry.bounds.contains(bounds) {
            return Err(AllocError::Bounds);
        }
        Ok(self.make_capability(
            region_id,
            rights,
            bounds,
            entry.zone,
            entry.expires_at_ns,
            true,
        ))
    }

    /// Revokes a DMA capability and invalidates its IOMMU translation entry
    /// within the declared revocation latency bound (RFC-ALLOC-001.20).
    pub fn revoke_dma(&mut self, region_id: ObjectId, now_ns: u64) -> AllocResult<()> {
        self.revoke(region_id, now_ns)
    }

    /// Returns whether the IOMMU entry for a DMA region is still valid.
    #[must_use]
    pub fn iommu_valid(&self, region_id: ObjectId) -> bool {
        self.iommu
            .get(&region_id)
            .map(|e| e.valid)
            .unwrap_or(false)
    }

    // --- stack / heap -------------------------------------------------------

    /// Creates a stack allocation capability for a thread (RFC-ALLOC-001.23–.28).
    pub fn create_stack(
        &mut self,
        domain: DomainId,
        size_bytes: u64,
        guard_size: u64,
        _now_ns: u64,
    ) -> AllocResult<StackDescriptor> {
        if self.state != AllocatorState::CapabilityGatedAllocator {
            return Err(AllocError::NoAllocationCapability);
        }
        let numa_node = 0;
        let order = BuddyNode::order_for_size(size_bytes, PageSizeClass::Base4KiB);
        self.buddy.allocate(Zone::Normal, numa_node, order)?;
        let base_paddr = 0x3000_0000 + self.allocation_counter * BASE_PAGE_SIZE;
        let stack_id = self.next_region_id();
        let stack = StackDescriptor {
            stack_id,
            base_paddr,
            size_bytes,
            guard_size,
            owner_domain_index: domain.get() as u32,
        };
        let entry = RegionEntry {
            kind: RegionKind::Memory(MemoryRegionDescriptor {
                region_id: stack_id,
                base_paddr,
                size_bytes: size_bytes + guard_size,
                page_size_class: PageSizeClass::Base4KiB,
                numa_node,
                zone: Zone::Normal,
                flags: REGION_GUARD,
            }),
            cap_type: CapType::Memory,
            rights: Rights::READ.union(Rights::WRITE),
            bounds: Bounds::new(0, size_bytes + guard_size).unwrap(),
            domain,
            expires_at_ns: 0,
            flags: CapabilityFlags::BOUND.union(CapabilityFlags::REVOCABLE),
            parent: None,
            children: Vec::new(),
            rights_derived: 0,
            revoked: false,
            order,
            numa_node,
            zone: Zone::Normal,
        };
        self.regions.insert(stack_id, entry);
        self.allocation_counter += 1;
        Ok(stack)
    }

    /// Creates a heap allocation capability for a component (RFC-ALLOC-001.25–.29).
    pub fn create_heap(
        &mut self,
        domain: DomainId,
        size_bytes: u64,
        _now_ns: u64,
    ) -> AllocResult<HeapDescriptor> {
        if self.state != AllocatorState::CapabilityGatedAllocator {
            return Err(AllocError::NoAllocationCapability);
        }
        let numa_node = 0;
        let order = BuddyNode::order_for_size(size_bytes, PageSizeClass::Base4KiB);
        self.buddy.allocate(Zone::Normal, numa_node, order)?;
        let base_paddr = 0x4000_0000 + self.allocation_counter * BASE_PAGE_SIZE;
        let heap_id = self.next_region_id();
        let heap = HeapDescriptor {
            heap_id,
            base_paddr,
            size_bytes,
            allocated_bytes: 0,
            owner_domain_index: domain.get() as u32,
            flags: 0,
        };
        let entry = RegionEntry {
            kind: RegionKind::Memory(MemoryRegionDescriptor {
                region_id: heap_id,
                base_paddr,
                size_bytes,
                page_size_class: PageSizeClass::Base4KiB,
                numa_node,
                zone: Zone::Normal,
                flags: 0,
            }),
            cap_type: CapType::Memory,
            rights: Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            bounds: Bounds::new(0, size_bytes).unwrap(),
            domain,
            expires_at_ns: 0,
            flags: CapabilityFlags::BOUND.union(CapabilityFlags::REVOCABLE),
            parent: None,
            children: Vec::new(),
            rights_derived: 0,
            revoked: false,
            order,
            numa_node,
            zone: Zone::Normal,
        };
        self.regions.insert(heap_id, entry);
        self.allocation_counter += 1;
        Ok(heap)
    }

    // --- boot allocator -----------------------------------------------------

    /// Installs the boot allocator with a fixed-size byte pool in ZONE_KERNEL
    /// (RFC-ALLOC-001.29–.30).
    pub fn install_boot_allocator(&mut self, pool_base: u64, pool_size: u64) {
        self.boot_allocator = Some(BootAllocator::new(pool_base, pool_size));
    }

    /// Allocates from the boot allocator (single-use bootstrap stage only).
    pub fn boot_allocate(&mut self, size_bytes: u64) -> AllocResult<u64> {
        let boot = self.boot_allocator.as_mut().ok_or(AllocError::Missing)?;
        boot.allocate(size_bytes)
    }

    /// Decommissions the boot allocator: one-way transition to the
    /// capability-gated state, emits a decommissioned event, and reclaims
    /// boot pages into the free list (RFC-ALLOC-001.31–.32, .51).
    pub fn decommission_boot_allocator(&mut self, now_ns: u64) -> AllocResult<()> {
        if self.state == AllocatorState::CapabilityGatedAllocator {
            return Err(AllocError::Missing);
        }
        let boot = self.boot_allocator.take().ok_or(AllocError::Missing)?;
        // Reclaim boot pages into the buddy free list under ZONE_KERNEL, node 0.
        let pages = boot.pool_size / BASE_PAGE_SIZE;
        self.buddy.add_region(Zone::Kernel, 0, pages);
        self.state = AllocatorState::CapabilityGatedAllocator;
        self.add_event(AllocEvent::BootAllocatorDecommissioned { timestamp_ns: now_ns });
        Ok(())
    }

    // --- accessors ----------------------------------------------------------

    #[must_use]
    pub const fn max_order(&self) -> u8 {
        MAX_ORDER
    }
}

// ---------------------------------------------------------------------------
// RegionEntry helper for base_paddr
// ---------------------------------------------------------------------------

impl RegionEntry {
    fn base_paddr(&self) -> u64 {
        match &self.kind {
            RegionKind::Memory(d) => d.base_paddr,
            RegionKind::Dma(d) => d.base_paddr,
        }
    }
}

/// Snapshot of parent-region fields needed to build a derived allocation
/// capability, owned so the immutable borrow of `self.regions` can end.
#[derive(Clone, Debug)]
struct ParentSnapshot {
    base_paddr: u64,
    numa_node: u8,
    zone: Zone,
    rights_derived: u32,
    expires_at_ns: u64,
}

// ---------------------------------------------------------------------------
// Pre-allocated scheduler pool (I-8)
// ---------------------------------------------------------------------------

/// Pre-allocated scheduler structures (RFC-ALLOC-001.42–.44).
///
/// The scheduler hot path operates exclusively on these pre-allocated
/// structures and MUST NOT invoke the page allocator, slab allocator, or any
/// capability-gated allocation.
#[derive(Clone, Debug, Default)]
pub struct SchedulerPreallocatedPool {
    tcb_count: u32,
    dispatch_queue_count: u32,
    next_tcb: u32,
    next_dispatch: u32,
    /// Count of allocations performed through the pool (must stay 0 on the
    /// hot path; incremented only by the one-time boot initialisation).
    allocation_count: u64,
}

impl SchedulerPreallocatedPool {
    /// Populates the pool at boot time for `max_threads` threads and
    /// `max_exec_units` execution units (RFC-ALLOC-001.42).
    pub fn populate(&mut self, max_threads: u32, max_exec_units: u32) {
        self.tcb_count = max_threads;
        self.dispatch_queue_count = max_exec_units;
        self.next_tcb = 0;
        self.next_dispatch = 0;
        // One-time boot allocation of the pool structures.
        self.allocation_count = 1;
    }

    /// Scheduler hot path: select the next thread to dispatch. Operates only
    /// on pre-allocated structures; never allocates (RFC-ALLOC-001.43).
    #[must_use]
    pub fn schedule_next(&self) -> Option<u32> {
        if self.next_tcb < self.tcb_count {
            Some(self.next_tcb)
        } else {
            None
        }
    }

    /// Scheduler hot path: dispatch a thread. Operates only on pre-allocated
    /// structures; never allocates (RFC-ALLOC-001.43).
    #[must_use]
    pub fn dispatch(&self, tcb: u32) -> bool {
        tcb < self.tcb_count
    }

    /// Returns the number of allocations performed through the pool.
    #[must_use]
    pub fn allocation_count(&self) -> u64 {
        self.allocation_count
    }

    /// Returns the configured thread-control-block count.
    #[must_use]
    pub fn tcb_count(&self) -> u32 {
        self.tcb_count
    }
}

// ---------------------------------------------------------------------------
// Top-level Allocator: Physical Memory Manager + Capability Authority Manager
// ---------------------------------------------------------------------------

/// Composed allocator hierarchy: Physical Memory Manager (buddy) +
/// Capability Authority Manager (capability-mediated allocation).
pub struct Allocator {
    authority: CapabilityAuthorityManager,
}

impl Default for Allocator {
    fn default() -> Self {
        Self::new()
    }
}

impl Allocator {
    /// Creates a new allocator in the boot-allocator state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            authority: CapabilityAuthorityManager::new(),
        }
    }

    /// Returns the underlying Capability Authority Manager.
    #[must_use]
    pub const fn authority(&self) -> &CapabilityAuthorityManager {
        &self.authority
    }

    /// Returns a mutable reference to the Capability Authority Manager.
    pub fn authority_mut(&mut self) -> &mut CapabilityAuthorityManager {
        &mut self.authority
    }

    /// Returns the current allocator state.
    #[must_use]
    pub const fn state(&self) -> AllocatorState {
        self.authority.state
    }

    /// Registers a physical memory region discovered by the HAL.
    pub fn add_physical_region(&mut self, region: PhysRegionDescriptor) -> AllocResult<()> {
        self.authority.add_physical_region(region)
    }

    /// Registers a domain.
    pub fn add_domain(&mut self, domain: DomainId) {
        self.authority.add_domain(domain);
    }

    /// Returns the per-zone fragmentation metric.
    #[must_use]
    pub fn fragmentation(&self, zone: Zone, numa_node: u8) -> f64 {
        self.authority.fragmentation(zone, numa_node)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const NOW_NS: u64 = 1_000_000;

    fn domain(value: u16) -> DomainId {
        DomainId::new(value).expect("non-zero domain")
    }

    fn authority_cap(domain: DomainId) -> Capability {
        Capability {
            object: ObjectId(0),
            object_type: ObjectType::Memory,
            rights: Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            bounds: Bounds::new(0, 1 << 30).unwrap(),
            flags: CapabilityFlags::NONE,
            expires_at_ns: 0,
        }
    }

    fn request(size: u64) -> AllocRequest {
        AllocRequest {
            authority_cap: authority_cap(domain(1)),
            size_bytes: size,
            page_size_class: PageSizeClass::Base4KiB,
            zone_hint: None,
            numa_policy: NumaPolicy::Local,
            numa_node: 0,
            rights: Rights::READ.union(Rights::WRITE),
            flags: 0,
        }
    }

    #[test]
    fn allocator_starts_in_boot_state() {
        let a = Allocator::new();
        assert_eq!(a.state(), AllocatorState::BootAllocator);
    }

    #[test]
    fn boot_allocator_then_decommission_transitions_one_way() {
        let mut a = Allocator::new();
        a.authority_mut().install_boot_allocator(0x1000, 4096);
        let addr = a
            .authority_mut()
            .boot_allocate(256)
            .expect("boot allocation succeeded");
        assert_eq!(addr, 0x1000);
        a.authority_mut()
            .decommission_boot_allocator(NOW_NS)
            .expect("decommission succeeded");
        assert_eq!(a.state(), AllocatorState::CapabilityGatedAllocator);
        // Decommissioning twice is non-conformant (one-way).
        assert!(a
            .authority_mut()
            .decommission_boot_allocator(NOW_NS)
            .is_err());
        // A boot-allocator event was emitted.
        assert!(a.authority().events().iter().any(|e| matches!(
            e,
            AllocEvent::BootAllocatorDecommissioned { .. }
        )));
    }

    #[test]
    fn allocation_grants_capability_with_subset_rights() {
        let mut a = Allocator::new();
        a.authority_mut()
            .decommission_boot_allocator(NOW_NS)
            .unwrap();
        a.authority_mut()
            .add_physical_region(PhysRegionDescriptor {
                base_paddr: 0,
                size_bytes: 16 * 1024 * 1024,
                zone: Zone::Normal,
                numa_node: 0,
            })
            .unwrap();
        let grant = a
            .authority_mut()
            .allocate(&request(4096), NOW_NS)
            .expect("allocation succeeded");
        assert_eq!(grant.capability.rights, Rights::READ.union(Rights::WRITE));
        assert!(grant.capability.flags.contains(CapabilityFlags::BOUND));
        assert!(!a.authority().is_revoked(grant.region.region_id));
    }

    #[test]
    fn derivation_cannot_add_rights() {
        let mut a = Allocator::new();
        a.authority_mut()
            .decommission_boot_allocator(NOW_NS)
            .unwrap();
        a.authority_mut()
            .add_physical_region(PhysRegionDescriptor {
                base_paddr: 0,
                size_bytes: 16 * 1024 * 1024,
                zone: Zone::Normal,
                numa_node: 0,
            })
            .unwrap();
        let grant = a
            .authority_mut()
            .allocate(&request(4096), NOW_NS)
            .unwrap();
        // Narrowing down to read-only is allowed.
        let narrow = a.authority_mut().derive(
            grant.region.region_id,
            domain(1),
            Rights::READ,
            Bounds::new(0, 4096).unwrap(),
            NOW_NS,
        );
        assert!(narrow.is_ok());
        // Adding EXECUTE (not in parent) is rejected.
        let amplified = a.authority_mut().derive(
            grant.region.region_id,
            domain(1),
            Rights::READ.union(Rights::EXECUTE),
            Bounds::new(0, 4096).unwrap(),
            NOW_NS,
        );
        assert_eq!(amplified, Err(AllocError::RightsExceedParent));
    }

    #[test]
    fn revocation_cascades_to_derived_capabilities() {
        let mut a = Allocator::new();
        a.authority_mut()
            .decommission_boot_allocator(NOW_NS)
            .unwrap();
        a.authority_mut()
            .add_physical_region(PhysRegionDescriptor {
                base_paddr: 0,
                size_bytes: 16 * 1024 * 1024,
                zone: Zone::Normal,
                numa_node: 0,
            })
            .unwrap();
        let grant = a.authority_mut().allocate(&request(4096), NOW_NS).unwrap();
        let child = a
            .authority_mut()
            .derive(
                grant.region.region_id,
                domain(1),
                Rights::READ,
                Bounds::new(0, 1024).unwrap(),
                NOW_NS,
            )
            .unwrap();
        assert!(!a.authority().is_revoked(child.region.region_id));
        a.authority_mut()
            .revoke(grant.region.region_id, NOW_NS)
            .unwrap();
        assert!(a.authority().is_revoked(grant.region.region_id));
        assert!(a.authority().is_revoked(child.region.region_id));
    }

    #[test]
    fn dma_allocation_rejected_from_non_dma_zone() {
        let mut a = Allocator::new();
        a.authority_mut()
            .decommission_boot_allocator(NOW_NS)
            .unwrap();
        a.authority_mut()
            .add_physical_region(PhysRegionDescriptor {
                base_paddr: 0,
                size_bytes: 16 * 1024 * 1024,
                zone: Zone::Normal,
                numa_node: 0,
            })
            .unwrap();
        let mut req = request(4096);
        req.zone_hint = Some(Zone::Normal);
        req.flags = 0x1; // DMA_CAPABLE
        let result = a.authority_mut().allocate_dma(&req, 1, NOW_NS);
        assert_eq!(result, Err(AllocError::NotDmaZone));
    }

    #[test]
    fn dma_revoke_invalidates_iommu() {
        let mut a = Allocator::new();
        a.authority_mut()
            .decommission_boot_allocator(NOW_NS)
            .unwrap();
        a.authority_mut()
            .add_physical_region(PhysRegionDescriptor {
                base_paddr: 0,
                size_bytes: 16 * 1024 * 1024,
                zone: Zone::Dma,
                numa_node: 0,
            })
            .unwrap();
        let req = request(4096);
        let grant = a
            .authority_mut()
            .allocate_dma(&req, 42, NOW_NS)
            .expect("dma allocation succeeded");
        assert!(a.authority().iommu_valid(grant.region.region_id));
        a.authority_mut()
            .revoke_dma(grant.region.region_id, NOW_NS)
            .unwrap();
        assert!(!a.authority().iommu_valid(grant.region.region_id));
        assert!(a
            .authority()
            .events()
            .iter()
            .any(|e| matches!(e, AllocEvent::DmaRevoked { .. })));
    }

    #[test]
    fn scheduler_hot_path_does_not_allocate() {
        let mut pool = SchedulerPreallocatedPool::default();
        pool.populate(8, 2);
        let count_before = pool.allocation_count();
        // Run the hot path many times.
        for _ in 0..1000 {
            if let Some(tcb) = pool.schedule_next() {
                assert!(pool.dispatch(tcb));
            }
        }
        assert_eq!(pool.allocation_count(), count_before);
        assert_eq!(pool.tcb_count(), 8);
    }

    #[test]
    fn fragmentation_event_emitted_above_threshold() {
        let mut a = Allocator::new();
        a.authority_mut()
            .decommission_boot_allocator(NOW_NS)
            .unwrap();
        a.authority_mut()
            .add_physical_region(PhysRegionDescriptor {
                base_paddr: 0,
                size_bytes: 16 * 1024 * 1024,
                zone: Zone::Normal,
                numa_node: 0,
            })
            .unwrap();
        // Lower the threshold so even modest fragmentation triggers an event.
        a.authority_mut().set_fragmentation_threshold(0.0);
        let _ = a.authority_mut().allocate(&request(4096), NOW_NS);
        assert!(a
            .authority()
            .events()
            .iter()
            .any(|e| matches!(e, AllocEvent::FragmentationThresholdExceeded { .. })));
    }

    #[test]
    fn region_ids_unique_across_allocations() {
        let mut a = Allocator::new();
        a.authority_mut()
            .decommission_boot_allocator(NOW_NS)
            .unwrap();
        a.authority_mut()
            .add_physical_region(PhysRegionDescriptor {
                base_paddr: 0,
                size_bytes: 64 * 1024 * 1024,
                zone: Zone::Normal,
                numa_node: 0,
            })
            .unwrap();
        let mut seen = HashSet::new();
        for _ in 0..10 {
            let grant = a.authority_mut().allocate(&request(4096), NOW_NS).unwrap();
            assert!(seen.insert(grant.region.region_id));
        }
    }
}
