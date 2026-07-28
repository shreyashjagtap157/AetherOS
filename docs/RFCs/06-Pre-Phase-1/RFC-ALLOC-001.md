# RFC-ALLOC-001: Allocator Hierarchy

Classification: Normative
Authoritative Source: RFC-ALLOC-001.md
Requirement-ID: RFC-ALLOC-001-000
Status: Draft

**Lifecycle note:** Draft.
**Implements invariants:** I-1, I-2, I-3, I-8, I-13.
**Depends on:** RFC-0037 (Capability Token Format), RFC-0009 (Address Space), RFC-0011 (Scheduler), INTF-000 (Capability Enforcement Substrate).
**Resolves:** INTF-001 (Memory Region Capability Type), INTF-004 (DMA Capability Semantics), INTF-005 (Stack/Heap Allocation Capability).

---

## 1. Scope

This RFC specifies the allocator hierarchy: how physical memory is discovered, organised, partitioned, and allocated; how allocation is mediated by capability tokens; and how three fundamental capability types — CPU memory, DMA memory, and stack/heap memory — are constructed and governed.

The allocator hierarchy is the lowest layer in the AetherOS capability-mediated resource model. Every subsequent resource allocation (MMIO regions, device memory, driver DMA windows, component stacks and heaps) ultimately depends on the primitives defined here.

This RFC implements I-1 (authority mediation at every allocation), I-2 (no ambient allocation authority), I-3 (least-privilege allocation), I-8 (deterministic core allocation for the scheduler), and I-13 (address–authority orthogonality in allocation: the allocation produces an address region; the capability grants authority over it).

This RFC does not specify the compiler ABI (`RFC-COMPILER-001`), the driver lifecycle (`RFC-DRIVER-001`), or the HAL implementation (`Hardware-Support-RFC.md`). It defines the interfaces that those RFCs consume.

---

## 2. Normative Requirements

### 2.1 Architecture Overview

`RFC-ALLOC-001.1`: The allocator hierarchy consists of two logically distinct layers:

1. **Physical Memory Manager** — discovers, tracks, and allocates physical page frames. Manages buddy allocation, NUMA topology, huge pages, fragmentation control, and zone policies.
2. **Capability Authority Manager** — wraps allocation operations with capability-mediated authority. Constructs capability tokens representing authority over allocated regions. Enforces derivation, narrowing, and revocation of allocation capabilities.

`RFC-ALLOC-001.2`: The Physical Memory Manager is a privileged kernel service with direct access to physical page frames. The Capability Authority Manager mediates every allocation operation through the capability-mediator boundary (INTF-000). No allocation capability may be created except by the Capability Authority Manager.

`RFC-ALLOC-001.3`: The two layers are composed as:

```
Allocation Request (with capability token)
        │
        ▼
Capability Authority Manager
    ├── verify capability (seal + rights + domain)
    ├── check allocation rights on presented capability
    │
        ▼
Physical Memory Manager
    ├── select physical pages (buddy, zone, NUMA)
    ├── allocate pages
    │
        ▼
Capability Authority Manager
    ├── construct new capability token for allocated region
    ├── seal and return
```

### 2.2 Physical Page Allocator

`RFC-ALLOC-001.4`: The physical page allocator uses a buddy allocation algorithm. Pages are organised into power-of-two order classes: order 0 (base page, 4 KiB), order 1 (8 KiB), ..., order 9 (2 MiB large page), order 18 (1 GiB huge page). The maximum order depends on hardware support declared by the HAL.

`RFC-ALLOC-001.5`: The allocator MUST track free pages per NUMA node. A page allocated from node N MUST be freeable to node N. Cross-node allocation is permitted only when explicitly requested via a NUMA-constrained capability and is flagged in the conformance declaration per RFC-0009.10.

`RFC-ALLOC-001.6`: The allocator MUST support at minimum two page-size classes: base page (4 KiB or the nearest hardware-supported base) and large page (2 MiB or the nearest hardware-supported large). Huge page (1 GiB) is RECOMMENDED and MUST be supported when hardware declares it.

`RFC-ALLOC-001.7`: Page promotion from base to large or huge MUST be explicit in the allocation request. Implicit promotion is non-conformant unless the promotion produces a verifiable performance improvement and does not change the accessible address range, per RFC-0009.9.

`RFC-ALLOC-001.8`: The allocator MUST maintain a fragmentation metric per zone. When fragmentation exceeds a declared threshold, the allocator MUST trigger defragmentation or emit a fragmentation event per RFC-0031. Defragmentation MUST NOT block any allocation that could be satisfied by a lower-order block.

### 2.3 Allocator Zones

`RFC-ALLOC-001.9`: Physical memory is partitioned into zones:

| Zone | Description | Capabilities |
|------|-------------|-------------|
| `ZONE_DMA` | Memory accessible by DMA-capable devices (below architectural DMA limit) | CAP_DMA_MEMORY only |
| `ZONE_DMA32` | Memory accessible by 32-bit DMA devices (below 4 GiB) | CAP_DMA_MEMORY, CAP_MEMORY |
| `ZONE_NORMAL` | General-purpose memory | CAP_MEMORY |
| `ZONE_HIGH` | Memory above normal range (if applicable) | CAP_MEMORY with explicit mapping |
| `ZONE_MMIO` | Memory-mapped I/O regions (not page-allocable; managed as fixed regions) | CAP_MMIO_REGION |
| `ZONE_KERNEL` | Kernel-resident memory (pre-allocated at boot, not capability-gated for kernel) | No capability (kernel-internal) |

`RFC-ALLOC-001.10`: Zone membership is a property of the physical address, not of the allocation capability. A capability over a DMA zone page MUST authorise DMA access; a capability over a NORMAL zone page MUST NOT authorise DMA access.

### 2.4 Capability-Addressable Memory Regions (INTF-001)

`RFC-ALLOC-001.11`: Every allocated memory region is represented by a standard capability token (`RFC-0037`) with `type = CAP_MEMORY` (type 1). The `target_object` field contains a memory region descriptor (Section 4.1). The `rights` field declares which of `RIGHT_READ`, `RIGHT_WRITE`, `RIGHT_EXECUTE` are granted.

`RFC-ALLOC-001.12`: A `CAP_MEMORY` token is a base capability type instantiated directly by the Capability Authority Manager from `RFC-0037`. It is NOT a derived sub-type. The token is sealed with the platform master seal key per RFC-0037.8.

`RFC-ALLOC-001.13`: A memory region capability MAY be narrowed (rights reduced) by derivation per RFC-0037.12. Narrowing a `CAP_MEMORY` token produces a new token with a subset of the original rights (e.g., READ-only from READ+WRITE). The derived token shares the same `target_object` but has an incremented `rights_derived` counter.

`RFC-ALLOC-001.14`: A narrowed capability MUST NOT grant rights not present in the parent capability. Derivation that attempts to add rights is rejected by the Capability Authority Manager.

`RFC-ALLOC-001.15`: The `target_object` in a `CAP_MEMORY` token is an opaque memory region identifier, NOT a raw physical address. The address of the backing physical pages is an implementation detail of the Physical Memory Manager and is NOT exposed through the capability token. This maintains I-13: the token grants authority; the mapping operation (RFC-0009) resolves the address.

### 2.5 DMA Capability Semantics (INTF-004)

`RFC-ALLOC-001.16`: DMA authority is represented by a distinct capability type: `CAP_DMA_MEMORY` (type 8). This is a SEPARATE capability type from `CAP_MEMORY`. A device that requires DMA access to a memory region MUST hold a valid `CAP_DMA_MEMORY` token for that region.

`RFC-ALLOC-001.17`: Three authorities are distinguishable:

| Authority | Capability Type | Grants | Held by |
|-----------|----------------|--------|---------|
| CPU memory access | `CAP_MEMORY` | CPU read/write/execute of physical pages | Component, driver, kernel |
| Device DMA access | `CAP_DMA_MEMORY` | Device-originated access to physical pages via DMA | Driver (via explicit grant) |
| IOMMU translation programming | `CAP_IOMMU_CONFIG` (type 9) | Programming IOMMU translation tables | Capability-mediator ONLY |

`RFC-ALLOC-001.18`: A driver receives `CAP_DMA_MEMORY` for the regions it needs the device to access. The driver does NOT receive `CAP_IOMMU_CONFIG`. The capability-mediator programmes the IOMMU on behalf of the driver when the driver presents a valid `CAP_DMA_MEMORY` token, converting driver-level DMA authority into IOMMU-level translation entries as an internal technical effect.

`RFC-ALLOC-001.19`: `CAP_DMA_MEMORY` tokens are issued from the `ZONE_DMA` or `ZONE_DMA32` zones. An allocation request for a DMA capability from `ZONE_NORMAL` is rejected unless the zone constraint is explicitly waived in the conformance declaration and the platform guarantees cross-zone DMA accessibility.

`RFC-ALLOC-001.20`: Revocation of a `CAP_DMA_MEMORY` token MUST cause the capability-mediator to invalidate the corresponding IOMMU translation entry within the declared revocation latency bound (per RFC-0039). The device MUST fault on subsequent DMA access to the revoked region.

`RFC-ALLOC-001.21`: A `CAP_DMA_MEMORY` token MAY be derived from a parent `CAP_DMA_MEMORY` token with narrowed rights (e.g., DMA-read-only). The derivation rules mirror `CAP_MEMORY` derivation: derived rights are a subset of parent rights (RFC-0037.12).

`RFC-ALLOC-001.22`: A single physical page MAY simultaneously be covered by a `CAP_MEMORY` token (for CPU access) and a `CAP_DMA_MEMORY` token (for device access). These are independent capabilities. Revocation of one does not affect the other unless the underlying physical page is reclaimed.

### 2.6 Stack and Heap Allocation Capability (INTF-005)

`RFC-ALLOC-001.23`: Every thread has a distinguished stack allocation capability: a `CAP_MEMORY` token granting the stack region's address range with READ+WRITE rights and a fixed size. The stack capability is created by the Capability Authority Manager at thread instantiation.

`RFC-ALLOC-001.24`: The stack capability's `target_object` identifies a contiguous physical region of at least the thread's declared stack size. The stack region MAY be backed by guard pages (unmapped) at the low end for stack overflow detection, per RFC-0009.

`RFC-ALLOC-001.25`: Heap allocation is accessed through a capability-gated system call. A component holds a heap allocation capability: a `CAP_MEMORY` token authorising the component to request sub-allocations from a managed heap region. The heap region itself is a larger `CAP_MEMORY` allocation; the heap allocation capability grants `RIGHT_DERIVE` and `RIGHT_WRITE` on that region.

`RFC-ALLOC-001.26`: A component MAY derive a narrower heap allocation capability for a subsystem. The derived capability grants a subset of the parent heap region and a subset of allocation rights. Sub-allocations from the narrower capability cannot exceed the derived region's bounds.

`RFC-ALLOC-001.27`: The heap allocator MUST NOT allocate from the scheduler hot path. Layer 1 scheduler structures (`RFC-0011`) MUST use pre-allocated control blocks. The heap allocator is invoked only for component-level allocations, not for scheduler dispatch.

`RFC-ALLOC-001.28`: Stack and heap capabilities MUST be bound to a specific domain (`CAPF_BOUND` per RFC-0037.7). A stack capability from domain A is invalid in domain B.

### 2.7 Boot Allocator Transition

`RFC-ALLOC-001.29`: Before the capability system exists (Stage II of the bootstrap, RFC-0007), the Minimum Executor uses a static, pre-capability boot allocator. The boot allocator is a fixed-size byte pool placed in `ZONE_KERNEL` at a known physical address.

`RFC-ALLOC-001.30`: The boot allocator is single-use: it serves only the Minimum Executor and the Capability Root during bootstrap. Its allocated regions are register-traced (every allocation is logged) and are NOT exposed to any user-level component.

`RFC-ALLOC-001.31`: After the Capability Root constructs the first `CAP_MEMORY` token (the origin allocation capability) and seals it, the boot allocator is decommissioned. All subsequent allocations go through the Capability Authority Manager.

`RFC-ALLOC-001.32`: The transition from boot allocator to capability-gated allocator MUST emit a `BootAllocatorDecommissioned` event per RFC-0031. The boot allocator's physical pages are reclaimed into the page allocator's free list after decommissioning.

### 2.8 MMIO Region Management

`RFC-ALLOC-001.33`: Memory-mapped I/O regions are NOT page-allocable. They are managed as fixed regions discovered by the HAL at boot and tracked by the Physical Memory Manager as `ZONE_MMIO` descriptors.

`RFC-ALLOC-001.34`: MMIO regions are represented by a distinct capability type: `CAP_MMIO_REGION` (type 10). This is SEPARATE from `CAP_MEMORY` and `CAP_DMA_MEMORY`. An MMIO capability grants authority to read/write device registers, not to access backing physical pages as if they were ordinary memory.

`RFC-ALLOC-001.35`: MMIO region capabilities are created by the Capability Authority Manager when a device is discovered and its MMIO regions are mapped into a driver's address space (per RFC-DRIVER-001). The MMIO capability inherits the device's access constraints (e.g., read-only for status registers, write-only for command registers).

`RFC-ALLOC-001.36`: MMIO capabilities MUST be domain-bound (`CAPF_BOUND`) and MUST NOT be transferable unless explicitly authorised by the driver manifest (per RFC-DRIVER-001).

### 2.9 Slab Allocator

`RFC-ALLOC-001.37`: The kernel uses a slab allocator for fixed-size kernel objects (capability tokens, domain descriptors, thread control blocks, event records). The slab allocator allocates from `ZONE_KERNEL` pages obtained from the Physical Memory Manager.

`RFC-ALLOC-001.38`: The slab allocator MUST NOT allocate from the scheduler hot path. Pre-allocated slab caches for scheduler objects MUST be populated at boot time and sized to the declared maximum concurrent thread count.

`RFC-ALLOC-001.39`: Slab allocator operations are kernel-internal and are NOT capability-gated. The slab allocator runs in the kernel's privileged context. Capability gating applies at the boundary between the kernel and user-level components, not within the kernel itself.

### 2.10 NUMA Policy

`RFC-ALLOC-001.40`: The allocator MUST support the following NUMA allocation policies, selectable per allocation request:

| Policy | Description |
|--------|-------------|
| `NUMA_LOCAL` | Allocate from the NUMA node closest to the requesting hart |
| `NUMA_NODE(N)` | Allocate from a specific NUMA node |
| `NUMA_INTERLEAVE` | Round-robin pages across nodes (for bandwidth workloads) |
| `NUMA_PREFERRED` | Prefer a specific node; fall back to nearest if unavailable |

`RFC-ALLOC-001.41`: NUMA policy selection is encoded in the allocation request's flags. The allocator MUST NOT silently select a non-requested NUMA node without flagging the cross-node allocation in the conformance declaration (per RFC-0009.10).

### 2.11 Deterministic Allocation for Scheduler (I-8)

`RFC-ALLOC-001.42`: Layer 1 scheduler structures (`RFC-0011`) are allocated from a dedicated pre-allocated pool at boot time. The pool size is the maximum number of concurrent threads multiplied by the thread control block size, plus the maximum number of execution units multiplied by the dispatch queue size.

`RFC-ALLOC-001.43`: The scheduler hot path — the code path executed at every scheduler invocation — MUST NOT invoke the page allocator, slab allocator, or any capability-gated allocation. The hot path operates exclusively on pre-allocated data structures.

`RFC-ALLOC-001.44`: The pre-allocated scheduler pool MUST be allocated during the boot allocator phase (before capability-gating is active) or by a one-time capability-gated allocation during the Capability Root initialisation. After initialisation, the pool is fixed and immutable.

### 2.12 Address-Authority Orthogonality in Allocation (I-13)

`RFC-ALLOC-001.45`: Allocation produces a memory region (an address range backed by physical pages). A capability token authorises operations on that region. These are orthogonal: the region exists as a physical resource independent of any capability, and the capability exists as an authority token independent of any particular address mapping.

`RFC-ALLOC-001.46`: A physical page MAY exist without any capability covering it (e.g., during boot before the first capability is constructed, or after all capabilities over a page are revoked and the page is returned to the free list). An uncapability-covered page is NOT accessible by any component — access requires a valid capability.

`RFC-ALLOC-001.47`: A capability MAY exist over a region that is not currently mapped into any component's address space (per RFC-0009). The capability grants authority to map; the mapping operation resolves the address. This separation is mandatory: capability existence does not imply addressability, and addressability does not imply capability.

### 2.13 Allocation Authority Derivation

`RFC-ALLOC-001.48`: A component holding a `CAP_MEMORY` token with `RIGHT_DERIVE` MAY derive a narrower allocation capability for a subsystem. The derived capability covers a sub-region of the parent and grants a subset of rights.

`RFC-ALLOC-001.49`: A derived allocation capability MUST be domain-bound to the creating component's domain unless `RIGHT_TRANSFER` is also granted. Unbounded derivation is non-conformant.

`RFC-ALLOC-001.50`: The Capability Authority Manager tracks the derivation tree for every allocation capability. Revocation of a parent capability (per RFC-0039) MUST revoke all derived capabilities in its lineage.

---

## 3. Allocator State Machine

```
BOOT_ALLOCATOR
    │ Capability Root created
    ▼
CAPABILITY_GATED_ALLOCATOR — all allocations mediated
    │ component requests allocation
    ▼
ALLOCATION_REQUESTED — capability presented and verified
    │ physical pages selected
    ▼
PAGES_ALLOCATED — physical pages assigned to region
    │ capability token constructed
    ▼
CAPABILITY_GRANTED — token sealed and returned
    │ component uses capability
    ▼
ACTIVE — region accessible via capability
    │ capability narrowed or revoked
    ▼
CAPABILITY_NARROWED or CAPABILITY_REVOKED
    │ all capabilities over region revoked
    ▼
PAGES_FREE — physical pages returned to free list
```

`RFC-ALLOC-001.51`: The `BOOT_ALLOCATOR → CAPABILITY_GATED_ALLOCATOR` transition is one-way and irreversible. Decommissioning the boot allocator is mandatory before any user-level component is instantiated.

`RFC-ALLOC-001.52`: The `PAGES_FREE` state is reachable only when all capability tokens covering the physical pages have been revoked. Premature page reclamation is non-conformant.

---

## 4. Data Structures

### 4.1 Memory Region Descriptor

The `target_object` field of a `CAP_MEMORY` token contains:

```c
struct memory_region_descriptor {
    struct object_id        region_id;           // RFC-0015: content-resolved identity
    uint64                  base_paddr;          // physical base address (kernel-internal)
    uint64                  size_bytes;          // region size, page-aligned
    uint8                   page_size_class;     // 0=base, 1=large, 2=huge
    uint8                   numa_node;           // NUMA node identity
    uint8                   zone;                // ZONE_DMA, ZONE_NORMAL, etc.
    uint8                   flags;               // region flags (see below)
    uint32                  reserved;
};
```

Region flags:

| Flag | Name | Meaning |
|------|------|---------|
| 0x01 | `REGION_DMA_CAPABLE` | Region is in a DMA zone |
| 0x02 | `REGION_MMIO` | Region is MMIO (not page-allocable) |
| 0x04 | `REGION_GUARD` | Region is a guard page (unmapped) |
| 0x08 | `REGION_HUGE` | Region backed by huge pages |

### 4.2 DMA Region Descriptor

For `CAP_DMA_MEMORY` tokens, the `target_object` contains:

```c
struct dma_region_descriptor {
    struct object_id        region_id;           // same identity as backing CAP_MEMORY
    uint64                  base_paddr;          // physical base address
    uint64                  size_bytes;          // region size
    uint8                   dma_width;           // 0=32-bit, 1=64-bit
    uint8                   numa_node;           // NUMA node
    uint16                  device_id;           // requesting device (for IOMMU mapping)
    uint32                  iommu_domain;        // IOMMU domain (set by mediator)
};
```

### 4.3 Stack Descriptor

```c
struct stack_descriptor {
    struct object_id        stack_id;            // stack region identity
    uint64                  base_paddr;          // physical base
    uint64                  size_bytes;          // total stack size
    uint64                  guard_size;          // guard page size at low end
    uint32                  owner_domain_index;  // domain that owns this stack
    uint32                  reserved;
};
```

### 4.4 Heap Descriptor

```c
struct heap_descriptor {
    struct object_id        heap_id;             // heap region identity
    uint64                  base_paddr;          // physical base
    uint64                  size_bytes;          // total heap size
    uint64                  allocated_bytes;     // currently allocated (for diagnostics)
    uint32                  owner_domain_index;  // owning domain
    uint32                  flags;               // heap flags
};
```

### 4.5 Boot Allocator Record

```c
struct boot_allocator_record {
    uint64                  pool_base;           // physical base of boot pool
    uint64                  pool_size;           // total pool size
    uint64                  allocated_bytes;     // bytes allocated during boot
    uint32                  allocation_count;    // number of boot allocations
    uint32                  reserved;
    struct capability_token origin_capability;   // first capability constructed from this pool
};
```

### 4.6 Physical Region Descriptor (from HAL)

```c
struct phys_region_descriptor {
    uint64                  base_paddr;
    uint64                  size_bytes;
    uint8                   zone;                // zone classification
    uint8                   numa_node;           // NUMA affinity
    uint16                  flags;               // reserved
    uint32                  reserved;
};
```

### 4.7 Allocation Request

```c
struct alloc_request {
    struct capability_token authority_cap;       // allocation authority capability
    uint64                  size_bytes;          // requested size
    uint8                   page_size_class;     // 0=base, 1=large, 2=huge
    uint8                   zone_hint;           // preferred zone (0=any)
    uint8                   numa_policy;         // NUMA_LOCAL, NUMA_NODE(N), etc.
    uint8                   numa_node;           // target node (for NUMA_NODE policy)
    uint8                   rights;              // requested rights (READ/WRITE/EXEC)
    uint16                  flags;               // DMA_CAPABLE, etc.
    uint32                  reserved;
};
```

---

## 5. Failure Semantics

| Failure | Consequence |
|---------|-------------|
| No allocation capability presented | `ECAPSTALE`; allocation denied (I-2) |
| Allocation capability lacks `RIGHT_DERIVE` | `ECAPSTALE`; derivation denied |
| Allocation capability lacks sufficient rights | `ECAPSTALE`; rights narrowing attempted beyond parent |
| Allocation capability expired | `ECAPSTALE`; allocation denied |
| Allocation capability domain mismatch | `ECAPSTALE`; cross-domain allocation denied |
| Requested size exceeds allocation authority bounds | `EALLOCSIZE`; allocation denied |
| No physical pages available in requested zone | `EALLOCEXHAUST`; fragmentation event emitted |
| NUMA node unavailable and fallback not declared | `ENUMAUNAVAIL`; allocation denied |
| DMA request from non-DMA zone | `EALLOCDMAZONE`; allocation denied |
| Boot allocator exhausted during bootstrap | Bootstrap halted; non-conformant |
| Capability revocation during active allocation | Allocation revoked; region returned to free list after all derived capabilities are revoked |
| Scheduler hot path triggers allocation | Non-conformant (I-8 violation) |

---

## 6. Conformance Tests

- `CTX-ALLOC-001`: Allocator grants no capability without explicit allocation request and valid allocation authority capability (I-2).
- `CTX-ALLOC-002`: Derived allocation capability grants rights that are a subset of the parent capability rights. Attempting to derive a capability with additional rights is rejected.
- `CTX-ALLOC-003`: NUMA allocation respects the capability-specified node constraint. Cross-node allocation without explicit NUMA policy is flagged.
- `CTX-ALLOC-004`: Fragmentation metric is tracked per zone. When the declared threshold is exceeded, a fragmentation event is emitted and the allocator does not block allocations satisfiable by lower-order blocks.
- `CTX-ALLOC-005`: Allocated region identity (`region_id` in `memory_region_descriptor`) is content-resolved, immutable, and unique across all live allocations.
- `CTX-ALLOC-006`: Revocation of a `CAP_MEMORY` capability removes the allocation's reachability. All derived capabilities in the lineage are also revoked.
- `CTX-ALLOC-007`: Scheduler hot path completes without invoking the page allocator, slab allocator, or any capability-gated allocation. Pre-allocated structures are used exclusively.
- `CTX-ALLOC-008`: `CAP_DMA_MEMORY` access is rejected without a valid DMA capability. DMA access to a non-DMA zone is rejected.
- `CTX-ALLOC-009`: Revocation of a `CAP_DMA_MEMORY` token causes the capability-mediator to invalidate the IOMMU translation entry within the declared latency bound.

---

## 7. Verification Expectations

| Component | Tier | Proof Obligation |
|-----------|------|-----------------|
| Capability Authority Manager | VT-A | Allocation capabilities cannot be created without explicit grant (I-2). Right-narrowing holds across derivation (I-3). |
| Physical Memory Manager (buddy algorithm) | VT-C | Correctness of buddy splitting and coalescing. Fragmentation bound. |
| Scheduler pre-allocated pool | VT-A | No heap allocation on dispatch path (I-8). Pool size is sufficient for declared maximum. |
| DMA capability mediation | VT-A | `CAP_DMA_MEMORY` access requires valid token. IOMMU invalidation on revocation (I-1, I-2). |
| NUMA policy enforcement | VT-D | Cross-node allocation declared in conformance declaration (HPC profile). |
| Boot allocator transition | VT-A | Boot allocator is decommissioned before user components are instantiated. No capability-free allocations survive beyond bootstrap (I-12). |

---

## 8. Cross-References

- `RFC-0037` (Capability Token Format — capability token structure, types, rights, seal).
- `RFC-0009` (Address Space — page-size classes, NUMA, mapping rights).
- `RFC-0011` (Scheduler — deterministic allocation, pre-allocated structures).
- `RFC-0013` (Capability Mediation — enforcement boundary at allocation).
- `RFC-0007` (Execution Domain — bootstrap stages, domain binding of stack/heap capabilities).
- `RFC-0039` (Capability Revocation — revocation propagation for allocation capabilities).
- `RFC-0031` (Event Model — fragmentation events, boot allocator decommission event).
- `RFC-0015` (Object Identity — content-resolved region identity).
- `Hardware-Support-RFC.md` (Execution-Provider Contract — physical region descriptors, HAL-BOOT).
- `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md` (Capability Enforcement Substrate — SCP active in Phase 1).
- `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md` (REQ-ALLOC-01 through REQ-ALLOC-04, INTF-001, INTF-004, INTF-005).

---

## 9. Open Questions

1. **Boot allocator pool size.** What is the minimum boot allocator pool size required to bootstrap the Capability Root? This depends on the size of the origin capability and the initial capability namespace. Must be validated during Phase 1 implementation.
2. **Component heap derivation depth.** Is there a maximum derivation depth for heap capabilities? Infinite derivation could cause unbounded memory consumption for capability metadata. Consider a maximum depth of 16 for Phase 1.
3. **DMA coherency.** Does `CAP_DMA_MEMORY` require cache-coherent DMA, or does the allocator track dirty cache lines? Phase 1 assumes coherent DMA via hardware (cache flush/invalidate on DMA mapping). Non-coherent DMA deferred to Phase 3.
4. **Guard page mechanism.** Stack guard pages are unmapped per RFC-0009. The allocator must ensure guard pages are not accidentally included in a derived capability's address range. The derivation operation MUST exclude guard pages from the `target_object` size.

---

## Revision History

| Date | Change |
|------|--------|
| this cycle | Initial draft. Physical Memory Manager and Capability Authority Manager specified. INTF-001 (memory region capability type), INTF-004 (DMA capability semantics), and INTF-005 (stack/heap allocation capability) resolved. I-1, I-2, I-3, I-8, I-13 implemented. |
