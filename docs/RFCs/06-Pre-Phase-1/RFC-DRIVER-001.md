# RFC-DRIVER-001: Driver Framework Architecture

**Status:** Draft.
**Implements invariants:** I-1, I-2, I-3, I-4, I-7, I-9, I-10, I-11, I-13.
**Depends on:** RFC-0039 (Capability Revocation), RFC-0018 (Lifecycle FSM), RFC-0031 (Event Model), RFC-0002 (Component Manifest), RFC-0004 (Dependency Resolution), RFC-0010 (Interrupt Routing), RFC-ALLOC-001 (Allocator Hierarchy), RFC-COMPILER-001 (Compiler and Runtime Boundary), INTF-000 (Capability Enforcement Substrate).
**Consumes:** INTF-001 (Memory Region Capability Type), INTF-002 (Capability Register Convention), INTF-003 (Component Entry Protocol), INTF-004 (DMA Capability Semantics), INTF-006 (Error Propagation at ABI).

---

## 1. Scope

This RFC specifies the driver framework: how drivers are discovered, loaded, instantiated, operated, faulted, and unloaded within the AetherOS capability system. Drivers are first-class components whose authority is mediated entirely through capability tokens. The driver framework is a consumer of the interfaces defined by `RFC-ALLOC-001`, `RFC-COMPILER-001`, and `INTF-000`.

This RFC implements I-1 (every driver operation authorised via capability), I-2 (no ambient driver authority), I-3 (least-privilege driver capability), I-4 (observable lifecycle transitions), I-7 (forward extensibility — drivers are replaceable), I-9 (replaceable mechanisms — drivers are replaceable components), I-10 (explicit failure semantics at every lifecycle transition), I-11 (declarable profiles — driver manifests declare capabilities), and I-13 (address–authority orthogonality — MMIO/DMA are addresses; capability grants authority).

This RFC does not re-define capability token format (`RFC-0037`), capability semantics (`RFC-0013`), the allocator hierarchy (`RFC-ALLOC-001`), or the compiler ABI (`RFC-COMPILER-001`). It specifies how drivers consume those interfaces.

---

## 2. Normative Requirements

### 2.1 Architecture Overview

`RFC-DRIVER-001.1`: The driver framework consists of:

1. **Driver Lifecycle Manager** — manages the complete lifecycle of a driver from discovery through unload.
2. **Driver Capability Broker** — grants, narrows, and revokes capabilities for driver components.
3. **Device Registry** — tracks discovered devices, their MMIO regions, interrupt assignments, and DMA requirements.
4. **Dependency Resolver** — orders driver initialisation based on declared dependencies.

`RFC-DRIVER-001.2`: A driver is a component (per `RFC-0002`) with a driver-specific manifest that declares: required capabilities, hardware device identifiers, dependency drivers, and lifecycle policy. A driver is instantiated, executed, and destroyed exactly like any other component, with additional driver-specific lifecycle constraints.

### 2.2 Driver Lifecycle FSM

`RFC-DRIVER-001.3`: Every driver traverses the following lifecycle states:

| State | Description |
|-------|-------------|
| `Discovered` | Hardware device detected; driver manifest matched; no capabilities granted |
| `Loaded` | Driver binary loaded into memory; dependencies resolved; capabilities not yet granted |
| `Initialised` | Driver's entry stub executed; initial capabilities granted; driver declared ready |
| `Operating` | Driver actively servicing device operations |
| `Faulted` | Driver encountered an error; capabilities suspended; recovery in progress |
| `Suspended` | Driver suspended (power management); capabilities preserved but inactive |
| `Unloaded` | Driver capabilities revoked; memory released; device handed to replacement or destroyed |

`RFC-DRIVER-001.4`: The mandatory lifecycle transition path is:

```
Discovered → Loaded → Initialised → Operating → Unloaded
                                                  ↑
                              Faulted ─────────────┘ (recovery or replacement)
                              Suspended → Operating (resume)
```

`RFC-DRIVER-001.5`: The `Discovered → Loaded` transition occurs when a driver manifest matches a discovered device and the Dependency Resolver confirms all prerequisite driver capabilities are available.

`RFC-DRIVER-001.6`: The `Loaded → Initialised` transition occurs when the driver's component entry stub (per RFC-COMPILER-001.19) executes successfully and the Driver Capability Broker grants the driver's declared capabilities.

`RFC-DRIVER-001.7`: The `Initialised → Operating` transition occurs when the driver declares readiness (emits `DriverReady` event per RFC-0031) and begins servicing device operations.

`RFC-DRIVER-001.8`: The `Operating → Faulted` transition occurs when the driver encounters a fault that it cannot recover from internally. The Driver Capability Broker suspends the driver's capabilities (capability tokens remain valid but operations are blocked).

`RFC-DRIVER-001.9`: The `Faulted → Operating` transition (recovery) occurs when the fault is resolved (e.g., retry, device reset). The Driver Capability Broker restores the driver's capabilities.

`RFC-DRIVER-001.10`: The `Faulted → Unloaded` transition occurs when recovery is impossible or the driver is replaced. All capabilities are revoked and the driver is destroyed.

`RFC-DRIVER-001.11`: Every lifecycle transition MUST emit an event per RFC-0031 with: driver identity, old state, new state, transition timestamp, and trigger reason.

### 2.3 Driver Manifest

`RFC-DRIVER-001.12`: A driver manifest extends the component manifest (RFC-0002) with driver-specific fields:

```c
struct driver_manifest {
    // Component manifest fields (RFC-0002)
    struct component_manifest  base;

    // Driver-specific fields
    uint16                     device_class;       // device class identifier
    uint16                     device_vendor;      // device vendor identifier
    uint16                     device_product;     // device product identifier
    uint8                      capability_count;   // number of required capabilities
    uint8                      dependency_count;   // number of dependency drivers
    uint16                     flags;              // driver flags (see below)
    // Followed by: capability_count × capability declarations
    // Followed by: dependency_count × dependency declarations
};
```

Driver manifest flags:

| Flag | Name | Meaning |
|------|------|---------|
| 0x01 | `DRV_HOTPLUG` | Driver supports hot-plug add/remove |
| 0x02 | `DRV_SUSPEND` | Driver supports power management suspend/resume |
| 0x04 | `DRV_REPLACABLE` | Driver can be replaced without rebooting |
| 0x08 | `DRV_CRITICAL` | Driver failure is safety-critical |

### 2.4 Driver Capability Model

`RFC-DRIVER-001.13`: A driver receives capabilities through the Driver Capability Broker. The driver does NOT self-grant capabilities. The broker grants capabilities based on the driver manifest's declarations.

`RFC-DRIVER-001.14`: Three capability types are relevant to drivers:

| Capability Type | Grants | Held by | Origin |
|----------------|--------|---------|--------|
| `CAP_DEVICE_CONTROL` (type 12) | Device register operations (read/write/status) | Driver | Driver Capability Broker |
| `CAP_DMA_MEMORY` (type 8) | Device-originated DMA access to memory regions | Driver | Driver Capability Broker |
| `CAP_MMIO_REGION` (type 10) | Memory-mapped I/O register access | Driver | Driver Capability Broker |
| `CAP_IOMMU_CONFIG` (type 9) | IOMMU translation table programming | Capability-mediator ONLY | NOT granted to drivers |

`RFC-DRIVER-001.15`: A driver receives `CAP_DEVICE_CONTROL` and `CAP_DMA_MEMORY` for the device it services. The driver MAY also receive `CAP_MMIO_REGION` for the device's MMIO register space.

`RFC-DRIVER-001.16`: A driver does NOT receive `CAP_IOMMU_CONFIG`. The IOMMU is programmed by the capability-mediator as an internal technical effect when the driver presents a valid `CAP_DMA_MEMORY` token. The driver's DMA authority is translated into IOMMU translation entries by the mediator.

`RFC-DRIVER-001.17`: A driver's capabilities are bound to the driver's domain (`CAPF_BOUND` per RFC-0037.7). Capabilities are not transferable between drivers unless explicitly authorised by `RIGHT_TRANSFER` in the manifest.

`RFC-DRIVER-001.18`: The Driver Capability Broker MUST verify that every capability granted to a driver is declared in the driver's manifest. Granting a capability not declared in the manifest is non-conformant (I-2).

### 2.5 MMIO Capability Representation (REQ-DRIVER-05)

`RFC-DRIVER-001.19`: MMIO regions are represented by `CAP_MMIO_REGION` tokens (type 10). This is a DISTINCT capability type from `CAP_MEMORY` and `CAP_DMA_MEMORY`. An MMIO capability authorises access to device registers, not to backing physical pages as ordinary memory.

`RFC-DRIVER-001.20`: An MMIO capability's `target_object` contains:

```c
struct mmio_region_descriptor {
    struct object_id    region_id;          // MMIO region identity
    uint64              base_addr;          // MMIO base address (device-relative)
    uint64              size_bytes;         // MMIO region size
    uint8               access_width;       // 0=8-bit, 1=16-bit, 2=32-bit, 3=64-bit
    uint8               access_rights;      // bit 0=read, bit 1=write
    uint16              device_id;          // owning device
    uint32              reserved;
};
```

`RFC-DRIVER-001.21`: An MMIO capability's rights field declares which access widths and directions are permitted. A driver with a read-only MMIO capability MUST NOT write to the region. Violations produce `ECAPSTALE`.

`RFC-DRIVER-001.22`: MMIO capabilities MUST be domain-bound and MUST NOT be transferable unless the driver manifest declares `DRV_HOTPLUG` and the transfer is part of a device handoff sequence.

### 2.6 DMA Ownership and Memory Region Isolation (REQ-ALLOC-03)

`RFC-DRIVER-001.23`: A driver that requires DMA access receives `CAP_DMA_MEMORY` tokens from the Driver Capability Broker. The tokens are derived from the system-wide DMA pool (`ZONE_DMA` / `ZONE_DMA32` per RFC-ALLOC-001.9).

`RFC-DRIVER-001.24`: The driver MAY derive narrower `CAP_DMA_MEMORY` tokens from its received tokens (e.g., DMA-read-only for a device that only reads). Derivation follows RFC-ALLOC-001.21.

`RFC-DRIVER-001.25`: When a driver presents a `CAP_DMA_MEMORY` token to the capability-mediator, the mediator programmes the IOMMU to map the device's DMA window to the backing physical pages. The IOMMU programming is an internal effect of the mediator; the driver has no direct access to IOMMU translation tables.

`RFC-DRIVER-001.26`: Revocation of a `CAP_DMA_MEMORY` token (per RFC-0039) causes the mediator to invalidate the IOMMU translation entry within the declared revocation latency bound. The device faults on subsequent DMA access to the revoked region.

`RFC-DRIVER-001.27`: A driver's DMA authority is isolated from other drivers. DMA access by driver A to memory covered by driver B's `CAP_DMA_MEMORY` token is impossible unless driver B explicitly grants a derived token to driver A.

### 2.7 Hot-Plug Capability Renegotiation (REQ-DRIVER-02)

`RFC-DRIVER-001.28`: Device add is modelled as a capability grant event. When a new device is discovered:

1. The device registry creates a device record.
2. A matching driver manifest is located.
3. The Driver Capability Broker grants capabilities to the driver based on the new device's MMIO regions, interrupt assignments, and DMA requirements.
4. The driver enters the `Discovered → Loaded → Initialised → Operating` lifecycle.

`RFC-DRIVER-001.29`: Device remove is modelled as a capability revocation event. When a device is removed:

1. The Driver Capability Broker revokes all `CAP_DEVICE_CONTROL`, `CAP_MMIO_REGION`, and `CAP_DMA_MEMORY` tokens associated with the removed device.
2. The driver transitions to `Faulted` (device no longer available).
3. If the driver declares `DRV_HOTPLUG`, it MAY attempt recovery by re-discovering the device.
4. If recovery fails or the driver does not declare `DRV_HOTPLUG`, the driver transitions to `Unloaded`.

`RFC-DRIVER-001.30`: Hot-plug events MUST emit `DeviceAdded` and `DeviceRemoved` events per RFC-0031 with: device identity, driver identity, capability set changes, and timestamp.

`RFC-DRIVER-001.31`: A new device's capabilities MUST be isolated from existing drivers until explicitly granted. The capability broker does NOT automatically grant new-device capabilities to unrelated drivers.

### 2.8 Driver-to-Driver Dependency and Ordering (REQ-DRIVER-03)

`RFC-DRIVER-001.32`: Drivers may declare dependencies on other drivers in their manifest. A dependency is satisfied when the dependency driver reaches the `Initialised` state and grants the dependent driver a capability token.

`RFC-DRIVER-001.33`: The Dependency Resolver (per RFC-0004) orders driver initialisation based on declared dependencies. Cyclic dependencies MUST be detected at load time and prevented. A driver with cyclic dependencies cannot be loaded.

`RFC-DRIVER-001.34`: A driver MAY depend on a capability from another driver. The dependency is expressed as a capability type and device class in the manifest. The Dependency Resolver grants the dependent driver a derived capability from the dependency driver's capability.

`RFC-DRIVER-001.35`: If a dependency driver fails (enters `Faulted` or `Unloaded`), the dependent driver MUST be notified and MUST NOT continue operating. The dependent driver transitions to `Faulted`.

`RFC-DRIVER-001.36`: Dependency cycle detection MUST be performed at load time, not at runtime. Cycles are a load-time error that prevents all drivers in the cycle from being loaded.

### 2.9 Power Management Integration (REQ-DRIVER-04)

`RFC-DRIVER-001.37`: Device suspend is modelled as capability suspension. When a device is suspended:

1. All in-flight operations MUST complete or be aborted.
2. The Driver Capability Broker marks the driver's capabilities as `SUSPENDED` (a flag in the capability's `flags` field).
3. The driver transitions to `Suspended`.
4. Suspended capabilities CANNOT be used for new operations.

`RFC-DRIVER-001.38`: Device resume is modelled as capability restoration. When a device is resumed:

1. The Driver Capability Broker verifies that the device is operational.
2. The `SUSPENDED` flag is cleared from the driver's capabilities.
3. The driver transitions to `Operating`.
4. The driver MAY re-verify device state after resume.

`RFC-DRIVER-001.39`: If a device fails to resume, the driver transitions to `Faulted`. The capability broker MAY revoke the driver's capabilities if the device is unrecoverable.

`RFC-DRIVER-001.40`: Power management events MUST emit `DeviceSuspended` and `DeviceResumed` events per RFC-0031.

### 2.10 Driver Fault Containment

`RFC-DRIVER-001.41`: A driver runs in an isolated address space (per RFC-0009). A driver fault (exception, capability violation, memory error) CANNOT corrupt another driver's address space, capability namespace, or memory.

`RFC-DRIVER-001.42`: Fault containment is enforced by two mechanisms:
1. **Address space isolation** (Execution Architecture) — each driver has its own virtual address space.
2. **Capability isolation** (Capability Mediation) — each driver holds only its declared capabilities; capability faults do not extend scope.

`RFC-DRIVER-001.43`: The maximum driver fault blast radius is the driver's own address space and capability set. A driver CANNOT corrupt another driver's capabilities, another driver's memory, or the kernel's memory.

`RFC-DRIVER-001.44`: When a driver faults, the Driver Capability Broker:
1. Suspends the driver's capabilities (preventing further operations).
2. Emits a `DriverFaulted` event per RFC-0031 with the fault details.
3. Notifies dependent drivers (per RFC-DRIVER-001.35).
4. Initiates recovery or replacement.

### 2.11 Driver Unload Protocol

`RFC-DRIVER-001.45`: Driver unload is the reverse of the load sequence:

1. **Capability revocation.** The Driver Capability Broker revokes ALL capabilities held by the driver: `CAP_DEVICE_CONTROL`, `CAP_MMIO_REGION`, `CAP_DMA_MEMORY`. Revocation follows RFC-0039.
2. **IOMMU cleanup.** Revocation of `CAP_DMA_MEMORY` triggers IOMMU translation entry invalidation (per RFC-ALLOC-001.20).
3. **Interrupt deregistration.** The driver's interrupt bindings (per RFC-0010) are removed.
4. **Memory release.** The driver's heap, stack, and TLS capabilities are revoked. Memory is returned to the allocator.
5. **Event emission.** A `DriverUnloaded` event per RFC-0031 is emitted.
6. **Address space destruction.** The driver's address space is unmapped and released.

`RFC-DRIVER-001.46`: After unload, the driver's device MAY be assigned to a replacement driver. The replacement driver goes through the standard `Discovered → Loaded → Initialised → Operating` lifecycle.

### 2.12 Device Discovery

`RFC-DRIVER-001.47`: Device discovery is performed by the HAL-BOOT stage (per `Hardware-Support-RFC.md`) before the driver framework is active. The HAL discovers devices and populates the device registry with device records.

`RFC-DRIVER-001.48`: The driver framework is responsible for MATCHING discovered devices to driver manifests, not for discovering devices. The device registry is populated by the HAL; the driver framework queries it.

`RFC-DRIVER-001.49`: Device discovery events MUST emit `DeviceDiscovered` events per RFC-0031 with: device identity, device class/vendor/product, MMIO regions, interrupt assignments, and DMA requirements.

---

## 3. Driver Lifecycle State Machine

```
DISCOVERED — device found, manifest matched
    │ dependencies satisfied
    ▼
LOADED — binary loaded, not yet initialised
    │ entry stub executed, capabilities granted
    ▼
INITIALISED — driver ready, capabilities active
    │ driver declares readiness
    ▼
OPERATING — servicing device operations
    │ fault occurs                │ suspend requested
    ▼                             ▼
FAULTED — capabilities suspended    SUSPENDED — capabilities preserved
    │ recovery                     │ resume
    ├──→ OPERATING                 └──→ OPERATING
    │
    └──→ UNLOADED — all capabilities revoked
              ↑
         Device removed or driver replaced
```

---

## 4. Data Structures

### 4.1 Device Record

```c
struct device_record {
    struct object_id    device_id;              // device identity
    uint16              device_class;            // device class
    uint16              device_vendor;           // vendor ID
    uint16              device_product;          // product ID
    uint8               mmio_region_count;       // number of MMIO regions
    uint8               interrupt_count;         // number of interrupt lines
    uint8               dma_region_count;        // number of DMA-capable regions
    uint8               flags;                   // device flags
    // Followed by: mmio_region_count × mmio_region_descriptor
    // Followed by: interrupt_count × interrupt_assignment
    // Followed by: dma_region_count × dma_region_descriptor
};
```

### 4.2 Driver State Record

```c
struct driver_state {
    struct object_id        driver_id;           // driver identity
    uint8                   lifecycle_state;     // enum driver_lifecycle_state
    uint8                   capability_count;    // number of active capabilities
    uint8                   dependency_count;    // number of satisfied dependencies
    uint8                   flags;               // driver flags
    uint32                  device_id_index;     // index into device registry
    uint64                  state_timestamp_ns;  // last state transition timestamp
    // Followed by: capability_count × capability_token (active capabilities)
};
```

### 4.3 Driver Capability Grant

```c
struct driver_capability_grant {
    struct capability_token  capability;        // granted capability
    uint8                    grant_type;         // 0=MMIO, 1=DMA, 2=device-control
    uint8                    derived;            // 1 if derived from parent
    uint16                   device_id;          // associated device
    uint32                   reserved;
};
```

### 4.4 Driver Dependency Declaration

```c
struct driver_dependency {
    uint16              dependency_device_class; // device class of dependency
    uint8               required_capability_type; // capability type needed
    uint8               flags;                   // dependency flags (required/optional)
};
```

---

## 5. Failure Semantics

| Failure | Consequence |
|---------|-------------|
| Driver manifest does not match device | Load rejected; `DeviceNotMatched` event |
| Cyclic dependency detected | Load rejected; all drivers in cycle fail to load |
| Driver entry stub fails | Driver terminated; `DriverFaulted` event; capabilities not granted |
| Driver requests capability not in manifest | `ECAPSTALE`; capability grant denied; driver faulted |
| Driver fault in operating state | Capabilities suspended; `DriverFaulted` event; dependent drivers notified |
| Device removed while driver operating | Capabilities revoked; driver transitions to `Faulted` |
| Driver unload with active DMA | DMA capabilities revoked; IOMMU entries invalidated; driver transitions to `Unloaded` |
| Suspend with in-flight DMA | DMA completed or aborted before suspend; driver transitions to `Suspended` |
| Resume fails (device unresponsive) | Driver transitions to `Faulted`; capabilities revoked |
| Driver corrupts another driver's memory | Non-conformant; address space isolation violated (I-10) |
| Driver accesses IOMMU translation tables | `ECAPSTALE`; driver does not hold `CAP_IOMMU_CONFIG` |

---

## 6. Conformance Tests

- `CTX-DRIVER-001`: DMA access fails without a valid `CAP_DMA_MEMORY` token. A driver without DMA capability cannot perform device DMA.
- `CTX-DRIVER-002`: Revocation of a `CAP_DMA_MEMORY` token revokes the IOMMU translation entry within the declared latency bound. Subsequent device DMA to the revoked region faults.
- `CTX-DRIVER-003`: Driver cannot operate without its declared capabilities. A driver in `Initialised` state with fewer capabilities than declared in its manifest cannot reach `Operating`.
- `CTX-DRIVER-004`: Driver fault does not leak capability to other drivers. A faulted driver's capabilities are suspended, not transferred.
- `CTX-DRIVER-005`: Driver unload fully revokes all driver capabilities. After `Unloaded`, the driver holds zero capabilities and its address space is destroyed.
- `CTX-DRIVER-006`: Device removal revokes all capabilities held by the removed device's driver. The driver transitions to `Faulted`.
- `CTX-DRIVER-007`: New device capabilities are isolated from existing drivers until explicitly granted. A newly discovered device's capabilities are not accessible to unrelated drivers.
- `CTX-DRIVER-008`: Cyclic driver dependencies are detected at load time. A driver with a cyclic dependency cannot be loaded.
- `CTX-DRIVER-009`: Driver initialises only after all dependency capabilities are granted. A driver cannot reach `Initialised` if any dependency is unsatisfied.
- `CTX-DRIVER-010`: Suspended device capability cannot be used for new operations. A `SUSPENDED` capability is rejected by the capability-mediator.
- `CTX-DRIVER-011`: Resume restores exactly the capability set present before suspend. No capabilities are lost or gained during a suspend/resume cycle.
- `CTX-DRIVER-012`: MMIO access without `CAP_MMIO_REGION` triggers `ECAPSTALE`. A driver without MMIO capability cannot access device registers.
- `CTX-DRIVER-013`: MMIO capability grants only declared access rights. A read-only MMIO capability cannot be used for write operations.

---

## 7. Verification Expectations

| Component | Tier | Proof Obligation |
|-----------|------|-----------------|
| Driver Capability Broker | VT-A | Driver cannot acquire capability not declared in manifest (I-2). Driver fault cannot extend capability scope (I-2). |
| Driver lifecycle FSM | VT-B | All drivers follow the declared lifecycle. No driver reaches `Operating` without passing through `Initialised`. |
| DMA capability mediation | VT-A | DMA access requires valid `CAP_DMA_MEMORY`. IOMMU invalidation on revocation (I-1, I-2). |
| Hot-plug capability revocation | VT-A | Device removal revokes all device capabilities within declared latency bound. |
| Driver fault containment | VT-B | Driver fault does not corrupt another driver's address space or capability namespace. |
| Dependency ordering | VT-B | No cyclic dependencies. All dependencies satisfied before initialisation. |
| Power management | VT-B | Suspended capabilities cannot be used for new operations. Resume restores capability set. |

---

## 8. Cross-References

- `RFC-0037` (Capability Token Format — device-control, DMA, MMIO, IOMMU-config capability types).
- `RFC-0039` (Capability Revocation — revocation cascade for driver capabilities).
- `RFC-0018` (Lifecycle FSM — driver as managed component with defined states).
- `RFC-0031` (Event Model — driver lifecycle events, device add/remove events).
- `RFC-0002` (Component Manifest — driver manifest extends component manifest).
- `RFC-0004` (Dependency Resolution — driver dependency ordering).
- `RFC-0010` (Interrupt Routing — interrupt binding to driver components).
- `RFC-0009` (Address Space — driver address space isolation).
- `RFC-0013` (Capability Mediation — enforcement at driver boundary).
- `RFC-ALLOC-001` (Allocator Hierarchy — DMA memory pools, MMIO region management, IOMMU config capability).
- `RFC-COMPILER-001` (Compiler and Runtime Boundary — component entry/exit, ABI, error propagation).
- `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md` (Software Capability Provider active in Phase 1).
- `Hardware-Support-RFC.md` (HAL-BOOT device discovery, execution-provider contract).
- `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md` (REQ-DRIVER-01 through REQ-DRIVER-05, INTF-001 through INTF-006).

---

## 9. Open Questions

1. **Driver cohort update.** Can a running driver be replaced with a newer version without rebooting? The `DRV_REPLACABLE` flag suggests yes, but the protocol for in-place replacement is not specified here. Deferred to RFC-0042 if needed.
2. **Driver fault recovery depth.** How many times may a driver transition `Operating → Faulted → Operating` before the system declares it permanently failed? Phase 1 uses a configurable maximum (default: 3 retries).
3. **Multi-device drivers.** A single driver may service multiple instances of the same device class. The driver manifest MUST declare the maximum number of device instances. Each instance gets its own capability set. This is an extension of the basic single-device model.
4. **IOMMU granularity.** The capability-mediator programmes IOMMU entries at page granularity (4 KiB). finer-grained IOMMU mapping (sub-page) is deferred to Phase 3 when hardware supports it.
5. **Driver security sandboxing.** Beyond address space isolation, should drivers run in a further-restricted sandbox (e.g., no file system access, no network access)? Phase 1 relies on capability-only access; additional sandboxing is deferred to profile-specific extensions.

---

## Revision History

| Date | Change |
|------|--------|
| this cycle | Initial draft. Driver lifecycle FSM, capability model (device-control, DMA-memory, MMIO-region, IOMMU-config), hot-plug, dependency ordering, power management, and fault containment specified. INTF-001 through INTF-006 consumed. I-1, I-2, I-3, I-4, I-7, I-9, I-10, I-11, I-13 implemented. |
