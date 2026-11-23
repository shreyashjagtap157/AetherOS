Classification: Informative

> ARCHIVED HISTORICAL SOURCE — preserved to retain the complete earlier v1.0 convergence discussion and user-intent record. It is not the current constitutional specification. The repository's `docs/Platform-Architecture-Specification-v1.1.md`, governance process, accepted ADRs and active RFCs control. Any conflict must be handled through that governance process; this archive never silently reactivates superseded proposals.

---

# AetherOS — Comprehensive System Specification
## Version 1.0 — Canonical Architecture Foundation and Convergence Baseline

**Status:** Canonical architecture baseline — superseding earlier exploratory formulations while preserving the historical specification content below
**Scope:** Entire AetherOS platform, including desktop, laptop, tablet-class and mobile-computing variants
**Primary principle:** One coherent operating-system model with platform-specific implementations and policies where hardware, power, input, thermal, lifecycle, or UX requirements genuinely differ.

---

## 0. Purpose and provenance

This specification consolidates the design decisions, architectural direction, constraints, concerns, requirements, and architectural corrections established in the current AetherOS discussions, including the adversarial review of the initial baseline and the subsequent UI/HCI convergence work. It is intended to become the canonical conceptual baseline for future detailed design and implementation work.

This document is a design specification, not a claim that every item is already implemented. Existing AetherOS desktop/laptop development is treated as an existing implementation track that must be reconciled against this architecture rather than automatically defining the architecture for mobile. Likewise, mobile requirements SHALL be reconciled against the shared platform model rather than being allowed to redefine desktop/laptop assumptions.

The specification deliberately distinguishes:

1. **Semantics** — what AetherOS means and guarantees.
2. **Common services** — reusable operating-system functionality.
3. **Platform policies and adaptations** — implementation differences required by device class.
4. **Hardware/device implementations** — bus- and vendor-specific mechanisms.
5. **Experience shells** — desktop, laptop, tablet, foldable and mobile interaction models.

### 0.1 Architecture-convergence revisions

This revision incorporates the following major decisions and corrections from the architecture debate:

- Context is now the canonical human-facing unit of activity while applications remain deployment, runtime, and security entities.
- The UI architecture is consolidated into a Context-centric model with correlated Visual, Semantic, and Input Graphs.
- Semantic coverage is explicit; semantic information is authoritative when available, while visual/VLM interpretation is a controlled fallback.
- Context ownership and boundaries are explicitly separated from application ownership, Workspace organization, and security authority.
- Android compatibility is treated as an isolated compatibility environment, with hardware-assisted virtualization preferred where available, rather than a framework-translation project.
- Mobile update architecture uses redundant bootable generations and measured storage strategies rather than a blanket ban or blanket requirement for physical A/B duplication.
- Hardware support uses a layered driver model with a kernel hardware substrate, selectively in-kernel drivers, userspace device services, and high-level system APIs.
- Desktop/laptop/mobile hardware discovery is normalized through a common Device Graph without pretending ACPI, Device Tree, firmware, and bus discovery are identical mechanisms.
- UI, storage, hardware, AI, and platform behavior are explicitly treated as one cross-system architecture with device-specific policies rather than disconnected subsystems.

AetherOS SHALL NOT be treated as two independent operating systems (desktop and mobile). It SHALL be one platform with explicitly modeled operating regimes.


### 0.2 v1.0 convergence status and precedence

The architecture discussions following the original 0.2 baseline have now established a canonical v1.0 convergence layer covering the human-facing model, security boundaries, storage, resource governance, hardware/device architecture, runtime ecosystem, system composition, and boot/trust architecture.

The existing sections of this document are intentionally preserved. Where a later v1.0 convergence provision conflicts with an earlier exploratory or less precise formulation, the later v1.0 provision is authoritative unless an explicit Architecture Decision Record (ADR) states otherwise.

The v1.0 constitutional principle is:

> **AetherOS SHALL abstract semantic contracts and authority boundaries while keeping implementation mechanisms replaceable wherever no real architectural invariant requires them to be fixed.**

The architecture SHALL therefore distinguish, across the entire system:

- semantic contract;
- authority and ownership;
- lifecycle and failure semantics;
- resource contract and physical allocation;
- implementation mechanism;
- platform/profile specialization;
- observability and verification requirements.

The v1.0 convergence work establishes five foundational semantic separations:

1. **UI / HCI:** Semantic Meaning ≠ Visual Presentation ≠ Input Routing.
2. **Storage:** Logical Identity ≠ Physical Layout ≠ Compatibility View.
3. **Security:** Principal Identity ≠ Privilege Authority.
4. **Resources:** Context Meaning ≠ Execution Priority ≠ Physical Allocation.
5. **Hardware:** Device Semantics ≠ Physical Resources ≠ Driver Implementation.

These are semantic separations rather than mandates that a particular implementation must use a separate process, kernel, filesystem, scheduler, or hardware component for each side. An implementation MAY optimize across a boundary internally provided that the externally observable security, ownership, lifecycle, resource, compatibility, and semantic guarantees remain intact.

The v1.0 architecture also establishes the following major convergence decisions:

- Context is the canonical human-facing unit, but Context lifecycle SHALL NOT itself determine execution priority.
- Providers are application/runtime participants in Contexts, but a process boundary SHALL NOT automatically be treated as a security boundary.
- Capability authority SHALL be attenuating across all applicable policy layers.
- Accessibility, automation, and AI SHALL receive policy-derived semantic projections rather than unrestricted access.
- Persistent Context state SHALL be encrypted and independently attributable by Provider or other state owner; Providers SHALL NOT receive a Context root key.
- Storage SHALL separate logical object identity, transactional versioning, physical persistence, and compatibility filesystem presentation.
- Immutable system/package content SHALL support content-addressed storage and deduplication; mutable state SHALL use transactional append/version mechanisms; exact chunking, hashing, compaction and media-specific strategies remain implementation/benchmark decisions.
- Resource governance SHALL use Resource Contracts and Resource Leases, with user-space policy separated from latency-critical kernel/device enforcement.
- Memory pressure SHALL be managed proactively; kernel OOM SHALL be a final safety mechanism rather than ordinary application lifecycle management.
- Hardware shall use two complementary truths: a kernel-authoritative physical resource model and a user-space canonical semantic Device Graph.
- Shared multimedia buffers SHALL use capability-mediated zero-copy semantics and explicit synchronization where hardware permits, without requiring physical contiguity.
- Proprietary hardware stacks MAY execute inside narrowly bounded Vendor Compatibility Domains (VCDs), but compatibility protocols SHALL remain adapters below the canonical Aether semantic APIs.
- Boot SHALL combine hardware-rooted verified boot, measured boot, owner-controlled trust policy where hardware supports it, authenticated generation descriptors, anti-rollback state, and health-commit semantics.
- Certified Aether Hardware SHALL provide owner-controlled trust enrollment; Compatible Aether Hardware MAY lack that capability but SHALL NOT claim equivalent ownership guarantees.
- Normal boot SHALL be automated; manual generation selection SHALL be part of authenticated recovery/maintenance rather than an ordinary boot-menu workflow.

---

# 1. Vision

AetherOS is intended to be a state-of-the-art, high-performance, high-efficiency, secure, reliable, comprehensible, extensible, hardware-aware and AI-capable general-purpose operating system.

The platform SHALL target:

- desktop computers;
- laptops;
- conventional mobile devices;
- tablets;
- foldable devices;
- future device classes where the shared semantic model remains applicable.

The initial physical products may differ, but they SHALL share a common platform model wherever technically and economically practical.

## 1.1 Core thesis

AetherOS SHALL aim to combine:

- general-purpose computing freedom;
- consumer-grade usability;
- strong hardware/software integration;
- powerful file and storage semantics;
- strong security and privacy;
- transactional updates and recovery;
- broad application compatibility;
- deep external-device support;
- platform-native AI and automation;
- predictable resource behavior;
- architectural compactness and accountability.

## 1.2 The central design rule

> **Do not encode logical concepts as physical boundaries unless a real hardware, firmware, security, reliability, or compatibility invariant requires the boundary.**

This applies to:

- storage partitions;
- software packages;
- runtime boundaries;
- driver boundaries;
- permissions;
- device classes;
- update generations;
- platform variants.

---

# 2. Design principles

## 2.1 One platform, many operating regimes

AetherOS SHALL have one shared semantic model. Desktop, laptop and mobile SHALL be specialized through profiles, policies, hardware capability packages, optimized implementations and experience shells.

## 2.2 Capability over identity

Permissions SHALL be expressed primarily as capabilities rather than only as application identity-based privileges.

Examples:

- `camera.capture`
- `microphone.record`
- `location.precise`
- `storage.read`
- `storage.write`
- `network.internet`
- `bluetooth.control`
- `nfc.transact`
- `gpu.render`
- `npu.inference`
- `device.raw_access`

A process, application or AI agent receives capabilities under explicit policy.

## 2.3 Physical simplicity, logical sophistication

The physical storage layout SHALL be minimized. Logical domains SHALL provide separation, accounting, snapshots, rollback, encryption, quotas and policy.

## 2.4 Evidence and accountability

Every resource, persistent system object and privileged action SHALL be attributable to an owner, purpose, version, capability domain and lifecycle policy.

## 2.5 Resource awareness

CPU, GPU, NPU, RAM, storage, network, thermal capacity and battery energy SHALL be first-class resources with measurable allocation and accounting.

## 2.6 Graceful degradation

Subsystem failure SHOULD be isolated and recoverable where practical. A driver crash SHOULD NOT require a complete system restart unless hardware/firmware limitations make it unavoidable.

## 2.7 Standards first, extensions second

AetherOS SHALL prioritize stable industry standards and protocols before vendor-specific extensions.

## 2.8 Compatibility without architectural capture

Compatibility layers SHALL preserve access to existing software ecosystems without forcing the entire AetherOS architecture to become a clone of another platform.

## 2.9 Secure by construction

Security SHALL be architectural, not an application-layer feature added later.

## 2.10 Human control over consequential automation

AI and automated agents SHALL operate through the same capability and policy infrastructure as conventional applications and SHALL not receive unrestricted authority by default.

## 2.11 Compactness is a release requirement

Builds SHALL be measured for storage footprint, RAM footprint, boot time, background activity, energy use and update overhead. Unexplained regressions SHALL fail release gates.

---

# 3. Platform taxonomy

AetherOS SHALL define platform profiles rather than separate operating systems.

## 3.1 General Compute Profile

Base for systems optimized around sustained general-purpose compute.

Typical examples:

- desktop;
- workstation;
- performance laptop;
- conventional laptop.

## 3.2 Mobile Compute Profile

Base for devices optimized around mobility, thermal constraints, integrated hardware and battery energy.

Typical examples:

- phone;
- tablet;
- foldable;
- ultra-mobile computer.

## 3.3 Platform variants

The initial profile hierarchy SHOULD be:

```text
AetherOS
├── General Compute
│   ├── Desktop
│   └── Laptop
└── Mobile Compute
    ├── Phone
    ├── Tablet
    └── Foldable
```

Future profiles MAY include XR, automotive or specialized embedded systems only if the common semantics remain appropriate.

## 3.4 Platform Profile responsibilities

A platform profile MAY specialize:

- scheduler policy;
- memory reclamation policy;
- power policy;
- thermal policy;
- process lifecycle policy;
- networking/radio policy;
- input policy;
- display presentation;
- application background policy;
- hardware feature sets;
- driver selection;
- storage optimization;
- AI model routing;
- UX shell.

A profile SHALL NOT fork the semantic definition of core services without an explicit architectural decision.

---

# 4. High-level architecture

```text
                         AETHEROS
                            │
                    COMMON SEMANTICS
                            │
       ┌────────────────────┼────────────────────┐
       │                    │                    │
 Common Core Services   Capability Model    Resource Model
       │                    │                    │
       └────────────────────┼────────────────────┘
                            │
                  Platform Adaptation Layer
                            │
          ┌─────────────────┼─────────────────┐
          │                 │                 │
       Desktop            Laptop            Mobile
          │                 │                 │
     desktop shell     laptop shell      mobile shell
          │                 │                 │
          └─────────────────┼─────────────────┘
                            │
                      System Services
                            │
             ┌──────────────┼──────────────┐
             │              │              │
          Storage         Hardware       Security
             │              │              │
             └──────────────┼──────────────┘
                            │
                      Aether Kernel
                            │
                   Firmware / Hardware
```

## 4.1 Core layers

### Layer A — Hardware/Firmware

- CPU;
- GPU;
- NPU/AI accelerators;
- ISP/DSP;
- memory controllers;
- storage controllers;
- displays;
- cameras;
- audio hardware;
- buses;
- network interfaces;
- sensors;
- modem/radio;
- secure element/TEE/TPM-class hardware;
- boot firmware.

### Layer B — Kernel and low-level core

- scheduler;
- memory management;
- IPC;
- interrupt management;
- DMA/IOMMU;
- low-level security;
- device/resource management;
- low-level storage;
- low-level networking;
- power primitives;
- timekeeping.

### Layer C — Hardware/device architecture

- hardware discovery;
- device graph;
- bus managers;
- driver framework;
- device class interfaces;
- firmware management;
- resource arbitration.

### Layer D — System services

- storage service;
- package service;
- update service;
- network service;
- audio service;
- graphics service;
- camera service;
- sensor service;
- telephony/radio service on mobile;
- input service;
- identity/security service;
- notification service;
- power/thermal service;
- AI/intelligence service.

### Layer E — Runtime

- Aether Native Runtime;
- Android Compatibility Runtime;
- Web Runtime;
- optional Linux/other compatibility systems in future phases.

### Layer F — Experience

- desktop shell;
- laptop/hybrid shell;
- mobile shell;
- tablet shell;
- foldable shell;
- launcher;
- universal search;
- notifications;
- accessibility;
- voice/natural-language command interface.

### Layer G — Intelligence

- local inference;
- cloud/hybrid inference;
- context;
- memory;
- retrieval;
- planning;
- tool use;
- agents;
- policy enforcement;
- audit.

---

# 5. Kernel architecture

## 5.1 General direction

AetherOS SHOULD reuse a mature hardware foundation where practical rather than attempting to build every low-level hardware driver from zero. A Linux-derived foundation is a viable option for early production development, particularly for desktop/laptop and heterogeneous modern hardware, while the AetherOS semantic architecture remains independent of Linux-specific APIs.

The OS architecture SHALL NOT assume that the Linux kernel is the permanent conceptual boundary of the entire product. Kernel replacement or deeper custom kernel evolution may be considered later if measurable benefits justify it.

## 5.2 Kernel responsibilities

The kernel SHALL provide or coordinate:

- execution scheduling;
- virtual memory;
- physical memory management;
- process/thread lifecycle;
- IPC primitives;
- interrupt routing;
- DMA/IOMMU control;
- low-level device management;
- low-level security enforcement;
- timing;
- low-level power states;
- low-level storage and networking primitives where required.

## 5.3 Kernel non-responsibilities

The kernel SHOULD NOT directly own high-level policy such as:

- which notifications are grouped;
- application UI semantics;
- AI response generation;
- user-facing storage organization;
- high-level application permission UX.

These belong above the kernel.

---

# 6. Scheduler architecture

## 6.1 Resource-aware scheduling

Every workload SHOULD carry metadata such as:

```text
Latency sensitivity
CPU intensity
GPU intensity
NPU intensity
Memory pressure
I/O intensity
Network intensity
Thermal sensitivity
Power sensitivity
User importance
Deadline
Background tolerance
```

The scheduler SHALL use platform-specific objective functions.

## 6.2 Desktop scheduler priorities

Primary goals:

1. interactive latency;
2. throughput;
3. sustained performance;
4. fairness;
5. energy efficiency where applicable.

## 6.3 Laptop scheduler priorities

Primary goals:

1. responsiveness;
2. performance-per-watt;
3. thermal sustainability;
4. battery preservation;
5. workload completion.

## 6.4 Mobile scheduler priorities

Primary goals:

1. perceived responsiveness;
2. battery efficiency;
3. thermal sustainability;
4. radio/sensor wakeup minimization;
5. foreground workload correctness;
6. background containment.

## 6.5 Accelerator scheduling

CPU, GPU, NPU, DSP and other accelerators SHOULD be treated as schedulable compute resources.

---

# 7. Memory architecture

## 7.1 Memory categories

AetherOS SHALL track at least:

- active;
- interactive;
- warm;
- cold;
- reclaimable;
- compressible;
- pinned;
- shared;
- DMA-mapped;
- GPU-resident;
- NPU-resident;
- persistent snapshots;
- encrypted memory where applicable.

## 7.2 Application lifecycle and memory

The platform MAY support:

```text
RUNNING
  ↓
BACKGROUND
  ↓
SUSPENDED
  ↓
FROZEN
  ↓
SNAPSHOTTED
  ↓
EVICTED
```

Desktop applications SHOULD experience less aggressive eviction than mobile applications.

## 7.3 Process snapshotting

AetherOS SHOULD support snapshotting application state for supported workloads so that cold applications can be removed from RAM while preserving fast restoration.

Requirements:

- correct state semantics;
- crash consistency;
- privacy protection;
- encryption where required;
- compatibility with application-managed state;
- bounded storage cost.

## 7.4 Mobile memory priorities

Mobile memory policy SHOULD favor:

- aggressive cold-page reclaim;
- compression;
- application freezing;
- snapshotting;
- working-set prediction;
- avoidance of unnecessary CPU wakeups.

## 7.5 Desktop memory priorities

Desktop policy SHOULD favor:

- large filesystem caches;
- sustained working sets;
- application persistence;
- high-throughput I/O.

---

# 8. Storage architecture

## 8.1 Core philosophy

AetherOS SHALL distinguish **physical storage regions**, **logical storage domains**, and **storage objects**.

Physical partitions/regions SHALL exist only where required by firmware, bootability, hardware, security, recovery, or another explicit invariant.

Logical separation SHALL be preferred when it can provide equivalent isolation and reliability without consuming fixed physical capacity.

AetherOS SHALL NOT equate architectural simplicity with zero redundancy. Redundancy SHALL be introduced whenever it provides a demonstrated reliability or recoverability property, and SHALL be minimized when equivalent guarantees can be obtained with less duplicated data.

## 8.2 Canonical logical storage model

```text
Aether Storage Pool
├── System Domain
├── Hardware/Device-Support Domain
├── Application Domain
├── User Domain
├── Shared Domain
├── AI Domain
├── Generation/Snapshot Domain
├── Recovery Domain
└── Maintenance Domain
```

These are logical domains. They are not required to correspond one-to-one with physical partitions.

## 8.3 Physical layout — desktop/laptop

Preferred UEFI/GPT model:

```text
GPT
├── EFI System Partition
└── Aether Storage Pool
```

The exact ESP size SHALL be derived from actual boot-artifact requirements.

AetherOS SHOULD NOT create dedicated physical recovery, swap, hibernation, vendor, product, or update partitions unless a concrete platform invariant requires them.

## 8.4 Physical layout — mobile

Preferred Aether reference-device model:

```text
SoC secure-boot / firmware-required regions
                ↓
        Aether Main Storage Pool
```

Any SoC/firmware-required physical regions are hardware implementation constraints, not the canonical AetherOS storage model.

The mobile architecture SHALL explicitly permit physical redundancy for boot-critical components where required by SoC/firmware behavior.

## 8.5 Android-derived partition topology is not normative

AetherOS SHALL NOT require Android-equivalent conceptual partitions such as:

- `/system`;
- `/system_ext`;
- `/product`;
- `/vendor`;
- `/odm`;
- `/vendor_dlkm`;
- `/odm_dlkm`;
- `/recovery`;
- `/dtbo`;
- `/vbmeta`;
- fixed update slots;

merely because those boundaries exist in Android.

Equivalent functions MAY be represented by:

- signed boot artifacts;
- hardware packages;
- immutable system objects;
- logical domains;
- manifests;
- generation metadata;
- device-specific boot adapters.

A hardware platform MAY still require additional physical regions. Such requirements SHALL be documented as platform-specific constraints and SHALL NOT leak into the platform-wide conceptual architecture.

## 8.6 Unified storage pool

The main storage subsystem SHALL support dynamic allocation among logical domains.

A user SHALL NOT lose large amounts of usable capacity merely because an oversized fixed system partition was reserved in advance.

For example, a 1 TB device whose current system footprint is 7 GB SHOULD NOT need a 100 GB permanently inaccessible system partition.

## 8.7 Storage accounting

The OS SHALL maintain accountable categories including, where present:

```text
Core OS
Hardware support
System services/UI
Native runtime
Compatibility runtime
Applications
User data
AI runtime
AI models
Snapshots/generations
Recovery
Temporary/cache
```

The UI SHALL NOT present a large unexplained `System` bucket when the underlying allocation can be attributed.

## 8.8 Dynamic maintenance headroom

AetherOS SHALL prefer a logical maintenance-headroom policy rather than a large permanently inaccessible update partition.

As space becomes constrained, the system SHOULD reclaim, in a policy-controlled order:

```text
temporary data
      ↓
caches
      ↓
reclaimable AI assets
      ↓
old package artifacts
      ↓
obsolete snapshots/generations
      ↓
user notification / explicit action
```

Protected maintenance capacity SHALL be treated as a policy/resource constraint, not necessarily a physically reserved region.

The system SHALL avoid consuming all remaining space for ordinary user data when doing so would make safe recovery impossible.

## 8.9 Immutable system storage

The Aether System Domain SHOULD be immutable or transactionally versioned during normal operation.

The system SHOULD be optimized for read-mostly behavior and may use a compressed/read-optimized filesystem or object-store implementation where benchmarking demonstrates benefit.

The specification SHALL NOT mandate one filesystem prematurely.

## 8.10 Writable data storage

Applications, user data, mutable metadata, caches and other write-intensive workloads SHALL use a storage implementation optimized for the underlying medium.

Desktop/laptop storage MAY prioritize high throughput and parallel I/O.

Mobile storage MUST explicitly measure:

- write amplification;
- flash wear;
- garbage-collection behavior;
- latency under sustained writes;
- energy cost;
- thermal impact.

F2FS, ext4, a suitable copy-on-write filesystem, an Aether-native filesystem, or another implementation MAY be selected by profile after measurement.

## 8.11 Generation and snapshot semantics

AetherOS SHALL support versioned bootable system generations.

A generation is a cryptographically identified system state that can be selected by the boot manager.

Generations MAY share immutable objects/blocks.

The conceptual model is:

```text
Generation N
Generation N-1
        │
shared immutable objects
        │
mutable generation metadata
```

This provides rollback without requiring that every generation be a complete independent physical copy.

## 8.12 Mobile update storage policy

AetherOS SHALL NOT categorically prohibit copy-on-write/snapshot mechanisms on flash.

Instead, mobile update strategies SHALL be benchmarked against alternatives, including:

- physical firmware A/B;
- logical A/B system generations;
- shared immutable generations;
- snapshot/COW delta construction;
- hybrid approaches.

Measurements SHALL include:

- write amplification;
- update time;
- update energy consumption;
- UFS performance degradation;
- flash wear implications;
- rollback reliability;
- crash/power-loss resilience.

The final mobile storage implementation SHALL be selected from evidence on the reference hardware.

## 8.13 Content-addressed storage

Immutable objects SHOULD use content-addressable identifiers where beneficial.

Potentially deduplicable content includes:

- libraries;
- fonts;
- language resources;
- immutable system assets;
- AI model fragments;
- package components;
- generation objects.

Deduplication MUST preserve:

- security isolation;
- dependency correctness;
- upgrade/rollback correctness;
- data confidentiality;
- garbage-collection correctness.

## 8.14 Arbitrary file support

AetherOS SHALL permit storage of arbitrary files regardless of whether it has a native renderer/editor.

The Universal Content Framework SHOULD provide:

```text
Content Object
├── type/format
├── metadata
├── preview provider
├── renderer/converter
├── indexer
├── security classification
└── optional editor
```

Unsupported formats SHALL remain storable, transferable, backup-able, and discoverable.

## 8.15 Swap

Swap SHOULD be dynamically managed rather than requiring a fixed partition.

Desktop/laptop and mobile policies MAY differ substantially.

## 8.16 Hibernate

Desktop/laptop hibernation SHOULD use dynamically allocated encrypted storage rather than a mandatory fixed-size hibernation partition.

Mobile devices MAY omit hibernation entirely or use platform-appropriate state persistence.

## 8.17 Recovery storage

Recovery SHOULD be represented as a signed bootable object/environment where hardware permits.

A permanent large recovery partition SHALL NOT be required without a concrete justification.

# 9. Boot architecture

## 9.1 Design goal

The boot architecture SHALL establish a hardware-rooted chain of trust while avoiding unnecessary physical storage boundaries.

Boot artifacts SHALL be represented according to what firmware and hardware actually require, not by copying another operating system's partition naming.

## 9.2 Conceptual boot chain

```text
Hardware Root of Trust
        ↓
Firmware / UEFI / SoC Boot ROM
        ↓
Aether Secure Boot Adapter
        ↓
Signed Aether Boot Manager
        ↓
Signed System Manifest
        ↓
Kernel + required early-boot data
        ↓
Hardware Profile
        ↓
Selected System Generation
        ↓
Core Services
```

## 9.3 Signed system manifest

The manifest SHOULD identify:

- generation ID;
- kernel ID/version;
- hardware profile;
- boot artifacts;
- trusted storage root;
- rollback counter/state;
- integrity/measurement metadata;
- recovery reference;
- boot policy;
- compatibility requirements.

## 9.4 Verified boot and rollback

AetherOS SHOULD support:

- hardware-rooted verification;
- signed boot components;
- signed system generations;
- integrity verification of immutable system objects;
- rollback protection;
- measured boot where available;
- device attestation where available.

## 9.5 Desktop/laptop boot

Preferred model:

```text
UEFI
 ↓
Aether Boot Manager
 ↓
Aether Storage
 ↓
Selected System Generation
```

The EFI System Partition is a firmware/bootability requirement rather than a general-purpose system-data partition.

## 9.6 Mobile boot

Preferred conceptual model:

```text
SoC secure boot / platform firmware
 ↓
Aether Boot Adapter
 ↓
Aether Boot Manager
 ↓
Selected System Generation
```

Exact boot-region layout SHALL be determined by the reference SoC and its secure-boot requirements.

## 9.7 Early hardware discovery

The kernel SHALL perform only the minimum hardware discovery required for safe bootstrap, including, where applicable:

- CPUs;
- memory;
- interrupt controllers;
- boot storage;
- essential buses;
- essential power infrastructure;
- security hardware.

Richer topology discovery and policy SHALL be available to userspace through the Aether Device Graph and Device Manager.

# 10. System generations and updates

## 10.1 Transactional generation model

AetherOS SHALL represent the installed system as a selectable, cryptographically identified generation.

```text
Generation N
      ↓
construct Candidate N+1
      ↓
verify
      ↓
boot candidate
      ↓
health verification
      ↓
commit
```

On failure:

```text
Candidate N+1 fails
      ↓
select Generation N
      ↓
recover
```

## 10.2 Shared immutable state

Generation implementations SHOULD reuse unchanged immutable objects/blocks wherever safe.

The system SHALL distinguish:

- generation identity;
- generation metadata;
- immutable objects;
- mutable system state.

## 10.3 Redundancy rule

AetherOS SHALL use the minimum redundancy necessary to meet its reliability requirements.

Neither of the following SHALL be a universal rule:

- “always keep two complete physical copies”; or
- “never keep redundant copies.”

Boot-critical firmware MAY require physical A/B or equivalent redundancy.

System generations SHOULD prefer logical redundancy/shared immutable data when it produces equivalent reliability with lower storage and update cost.
## 10.4 Copy-on-write and snapshots

Copy-on-write and snapshot mechanisms MAY be used where appropriate.

For mobile flash, the implementation SHALL be benchmarked for:

- write amplification;
- fragmentation;
- latency;
- energy;
- flash endurance;
- recovery reliability.

## 10.5 Update categories

Updates SHALL be distinguishable as:

- boot/firmware;
- kernel;
- hardware packages;
- drivers;
- system services;
- runtime;
- UI/shell;
- applications;
- AI runtime;
- AI models;
- security policy.

## 10.6 Independent updates

Components SHOULD be independently updateable where stable compatibility contracts permit.

## 10.7 Health verification

Post-update verification SHOULD cover:

- boot;
- storage integrity;
- core services;
- security state;
- display;
- input;
- networking;
- essential device health;
- recovery path.

## 10.8 Power-loss safety

The update process SHALL remain safe under unexpected interruption to the extent supported by the storage and boot hardware.

A partially constructed candidate SHALL NOT become the authoritative system generation.

# 11. Device and hardware architecture

# 11.1 Aether Hardware & Device Architecture (AHDA)

AHDA SHALL provide one coherent architecture for:

- hardware discovery;
- topology;
- device identity;
- device capabilities;
- resource requirements;
- power states;
- performance states;
- firmware association;
- drivers;
- device health;
- lifecycle;
- security policy.

AHDA SHALL support radically different platform discovery mechanisms without requiring the rest of AetherOS to understand their historical implementation differences.

## 11.2 Device Graph

All relevant hardware SHALL be represented in a normalized Device Graph.

Example:

```text
Host / SoC
├── CPU
├── GPU
├── NPU
├── Display Controller
│   └── Display
├── PCIe Controller
│   └── NVMe
├── USB4 Controller
│   └── Dock
│       ├── Display
│       ├── Ethernet
│       ├── Audio
│       └── USB devices
├── Camera/ISP
│   ├── Sensor 0
│   └── Sensor 1
└── Sensor/Control Bus
    ├── Accelerometer
    ├── Gyroscope
    └── Other sensors
```

Relationships SHALL be represented, not only a flat list of devices.

The graph SHOULD expose dependencies such as:

- device-to-bus;
- device-to-power-domain;
- device-to-clock;
- device-to-controller;
- device-to-firmware;
- device-to-driver;
- device-to-resource.

## 11.3 Discovery sources

Desktop/laptop discovery MAY use:

- UEFI;
- ACPI;
- PCI/PCIe enumeration;
- USB enumeration;
- SMBus/I²C/I³C;
- SPI;
- platform firmware.

Mobile discovery MAY use:

- SoC firmware;
- Device Tree;
- ACPI where applicable;
- platform hardware descriptions;
- bus enumeration;
- secure firmware interfaces.

These inputs SHALL be normalized into the Aether Device Graph.

## 11.4 Discovery boundary

AetherOS SHALL distinguish:

```text
Minimal kernel bootstrap discovery
              ↓
Kernel hardware topology
              ↓
Aether Device Manager
              ↓
Global Device Graph
```

Firmware-table parsing and rich policy SHOULD be kept out of the kernel where practical without preventing the kernel from obtaining information essential to bootstrap.

## 11.5 Device model

Every device SHOULD expose:

- stable identity;
- device class;
- capabilities;
- resources;
- dependencies;
- supported power states;
- supported performance states;
- firmware information;
- security requirements;
- driver state;
- health state;
- lifecycle state.

## 11.6 Capability-first hardware API

Applications SHOULD request capabilities such as:

```text
camera.capture
gpu.render
gpu.compute
npu.inference
storage.read
storage.write
audio.play
audio.record
network.internet
location.precise
bluetooth.control
nfc.transact
external-device.access
```

The physical bus, SoC, vendor, and driver implementation SHALL normally remain below this abstraction.

Advanced privileged software MAY access lower-level device interfaces under explicit capabilities.

# 12. Driver architecture

## 12.1 Driver philosophy

Drivers SHALL be capability providers with explicit lifecycle, resource, power, security, and firmware relationships.

AetherOS SHALL NOT require all driver functionality to reside in the kernel, nor SHALL it require all driver functionality to reside in userspace.

The boundary SHALL be selected according to:

- latency;
- throughput;
- DMA requirements;
- interrupt handling;
- security;
- hardware architecture;
- restartability;
- existing mature implementation availability;
- maintainability.

## 12.2 Layered driver architecture

```text
Layer 0 — Firmware / device firmware
            ↓
Layer 1 — Kernel hardware substrate
            ↓
Layer 2 — Kernel device driver, where required
            ↓
Layer 3 — User-space device service, where appropriate
            ↓
Layer 4 — High-level device/system API
```

Not every device SHALL require every layer.

## 12.3 Kernel hardware substrate

The kernel SHALL provide mechanisms for:

- MMIO;
- interrupts;
- DMA;
- IOMMU;
- bus enumeration;
- reset;
- low-level synchronization;
- memory protection;
- low-level power primitives;
- security boundaries.

## 12.4 Driver placement

Kernel drivers SHOULD be used where kernel residency is justified by:

- strict latency;
- high-bandwidth data paths;
- DMA/interrupt requirements;
- kernel-owned memory management;
- core storage paths;
- core networking paths;
- essential graphics/display control;
- hardware limitations.

User-space services SHOULD be used where fault containment and restartability provide material benefit.

Examples MAY include:

- camera services;
- audio services;
- Bluetooth services;
- modem services;
- sensor services;
- printer/scanner services;
- complex USB-class services.

A single device MAY have both kernel and userspace components.

## 12.5 GPU and high-performance hardware

AetherOS SHALL recognize that high-performance GPU stacks normally require kernel facilities while retaining significant userspace components.

Therefore the architecture SHOULD use:

```text
Application / graphics runtime
        ↓
Userspace GPU driver/runtime
        ↓
Aether graphics interface
        ↓
Kernel GPU/display substrate
        ↓
GPU hardware
```

Equivalent layered designs MAY apply to NVMe, high-speed networking and specialized accelerators.

## 12.6 Userspace driver isolation

Where userspace device services are used, they SHOULD operate in isolated security domains.

A recoverable device failure SHOULD follow:

```text
failure detected
      ↓
quiesce
      ↓
reset/reinitialize
      ↓
reload
      ↓
health-check
      ↓
resume
```

Recovery SHALL NOT be promised when hardware makes it impossible.

## 12.7 DMA/IOMMU

Device DMA SHALL be constrained by IOMMU/address-space policy wherever supported and practical.

Device services SHALL receive only the memory and device access needed for their declared function.

## 12.8 Driver eligibility for userspace execution

A device component MAY be moved to userspace when:

- device state can be safely mediated;
- required DMA can be isolated;
- latency is acceptable;
- the hardware does not require privileged kernel sequencing;
- the failure/restart model is useful;
- performance remains within target.

The classification SHALL be determined by engineering evidence rather than device-category folklore.

## 12.9 Driver package model

Driver packages SHOULD contain:

- identity;
- hardware identifiers;
- driver binaries;
- required firmware;
- Aether driver interface version;
- architecture;
- capability declarations;
- power behavior;
- security policy;
- diagnostics;
- recovery metadata.

## 12.10 Driver trust levels

Conceptual trust levels:

```text
built-in platform driver
Aether-certified driver
verified vendor driver
developer-signed driver
experimental driver
```

Trust level MAY affect available privileges and isolation.

## 12.11 Driver rollback

Driver activation SHOULD be transactional where practical and SHALL support rollback for update-critical drivers when the hardware permits safe fallback.

## 12.12 Vendor extensions

Vendor-specific functionality MAY exist through versioned capability extensions.

Vendor extensions SHALL NOT redefine or destabilize the common Aether device model.

## 12.13 Memory-safe implementation

New security-sensitive driver code SHOULD prefer memory-safe implementation languages where the hardware/software ecosystem permits.

Existing mature implementations MAY be reused when they materially improve hardware support, provided unsafe components are isolated, reviewed, instrumented and tested.

# 13. Resource management

## 13.1 Central resource manager

AetherOS SHALL centrally model:

- CPU;
- RAM;
- GPU;
- NPU;
- DSP;
- memory bandwidth;
- bus bandwidth;
- storage I/O;
- network;
- power;
- thermal capacity.

## 13.2 Resource contracts

Devices/services SHOULD declare resource requirements and limits.

Example:

```text
GPU Service
CPU budget
GPU budget
memory budget
latency target
thermal class
power sensitivity
```

## 13.3 Multi-resource arbitration

The system SHALL be capable of considering combined resource pressure.

Example mobile camera workload:

```text
ISP: high
Memory bandwidth: high
NPU: medium
GPU: medium
Power: high
Thermal: elevated
```

The resource manager SHALL coordinate these demands.

---

# 14. DMA and IOMMU

## 14.1 Requirement

DMA-capable hardware SHALL be isolated using an IOMMU where hardware supports one.

## 14.2 Goals

- device memory isolation;
- DMA attack containment;
- driver sandboxing;
- virtual machine device assignment;
- accelerator isolation;
- safer user-space drivers.

## 14.3 Device address spaces

Each device or trusted device group SHOULD receive an explicit DMA address space.

---

# 15. Bus and interconnect architecture

AetherOS SHALL support a broad set of interconnects through standard bus frameworks.

## 15.1 Desktop/laptop

Priority buses/interfaces:

- PCI/PCIe;
- USB 2/3/USB4;
- USB-C/Power Delivery;
- NVMe;
- SATA where applicable;
- SD;
- I²C/I³C;
- SPI;
- GPIO;
- DisplayPort;
- HDMI;
- Ethernet;
- platform firmware interfaces.

## 15.2 Mobile

Priority buses/interfaces:

- SoC interconnects;
- MIPI CSI-2;
- MIPI DSI-2;
- I²C;
- I³C;
- SPI;
- GPIO;
- UFS;
- USB-C/PD;
- cellular modem links;
- sensor buses;
- secure-element interfaces.

## 15.3 Bus abstraction

Applications SHALL NOT normally interact with buses directly. Bus-specific access SHALL be reserved for system/device development through privileged capabilities.

---

# 16. PCIe architecture

Desktop/laptop support SHALL include:

- enumeration;
- configuration space;
- BAR mapping;
- MSI/MSI-X;
- DMA;
- IOMMU integration;
- hotplug where supported;
- link power management;
- reset mechanisms;
- advanced error reporting where appropriate;
- SR-IOV/virtualization support where useful.

PCIe device identity SHALL be normalized into the Device Graph.

---

# 17. USB architecture

## 17.1 USB core

AetherOS SHALL support:

- USB 2;
- USB 3.x;
- USB4 where hardware supports it;
- USB-C;
- power negotiation;
- alternate modes where supported.

## 17.2 USB classes

Priority classes include:

- mass storage;
- HID;
- audio;
- video;
- serial;
- networking;
- MIDI;
- smart-card/class-standard peripherals;
- printers where standardized protocols exist.

## 17.3 USB-C port model

AetherOS SHALL represent each USB-C port with:

- orientation;
- power capability;
- negotiated power;
- data capability;
- display capability;
- cable/partner capabilities;
- active mode;
- connected topology;
- security state.

---

# 18. External displays

The graphics stack SHALL treat display destinations as abstract display targets.

Supported physical transports MAY include:

- internal display;
- DisplayPort;
- HDMI;
- USB-C DP Alt Mode;
- USB4 display transport;
- wireless display.

Requirements include:

- hotplug;
- multi-monitor;
- per-display scaling;
- DPI;
- HDR;
- variable refresh;
- color management;
- rotation;
- high-refresh modes;
- display topology.

Mobile internal displays SHALL use the same semantic Display API while allowing mobile-specific implementations.

---

# 19. Graphics architecture

## 19.1 Aether Graphics Model

The graphics system SHOULD be based around a scene/compositor model.

```text
Scene
├── Window/Surface
├── Transform
├── Visual properties
├── Effects
└── Accessibility semantics
```

## 19.2 APIs

AetherOS SHOULD expose:

- Aether 2D primitives;
- Aether 3D primitives;
- compute;
- video acceleration;
- standard graphics compatibility APIs where feasible.

## 19.3 GPU abstraction

GPU drivers SHALL expose:

- rendering capabilities;
- compute capabilities;
- memory capabilities;
- synchronization;
- power states;
- reset/recovery;
- supported features.

---

# 20. Camera architecture

## 20.1 Unified Camera API

Desktop webcams and mobile camera stacks SHALL ultimately expose a common CameraDevice semantic model.

## 20.2 Mobile pipeline

Potential mobile pipeline:

```text
Sensor
 ↓
MIPI CSI-2
 ↓
CSI receiver
 ↓
ISP
 ↓
Image processing
 ↓
GPU/NPU/DSP where appropriate
 ↓
Memory
 ↓
Display/Storage/Application
```

## 20.3 Camera capabilities

The Camera API SHOULD support:

- resolution;
- format;
- frame rate;
- exposure;
- autofocus;
- stabilization;
- HDR;
- RAW;
- multi-camera synchronization;
- computational photography;
- privacy state;
- power state.

---

# 21. Sensor architecture

Sensors SHALL be exposed through a unified SensorDevice model.

Capabilities include:

- sampling rate;
- accuracy;
- latency;
- timestamp source;
- calibration;
- power cost;
- privacy classification;
- background availability.

The system SHALL avoid wasteful high-frequency background sampling.

---

# 22. Audio architecture

## 22.1 Audio graph

Audio SHALL be modeled as a directed processing graph.

Example:

```text
Microphone
 ↓
DSP
 ↓
Noise suppression / echo cancellation
 ↓
Application
```

and:

```text
Application
 ↓
Mixer / Spatializer
 ↓
DSP
 ↓
Speaker
```

## 22.2 Interfaces

Desktop/laptop and mobile hardware SHALL be hidden behind the same Audio API, while permitting different lower-level transports.

Support MAY include:

- USB Audio;
- HDMI/DisplayPort audio;
- I²S;
- SoundWire;
- Bluetooth audio;
- low-latency paths;
- spatial audio.

---

# 23. Networking architecture

## 23.1 Unified NetworkDevice

Network interfaces SHALL include:

- Ethernet;
- Wi-Fi;
- cellular;
- Bluetooth PAN;
- USB networking;
- VPN interfaces;
- virtual NICs.

## 23.2 Application network intents

Applications SHOULD request intents such as:

- low latency;
- high throughput;
- background;
- metered-tolerant;
- offline-capable;
- privacy-sensitive;
- bulk transfer.

## 23.3 Mobile radio policy

Mobile networking SHALL coordinate:

- radio sleep/wake;
- connection batching;
- power consumption;
- roaming;
- thermal state;
- metering;
- foreground/background priority.

---

# 24. Cellular and telephony architecture

Mobile profile SHALL support a first-class telephony/radio service model for devices containing cellular hardware.

Capabilities include where hardware and carrier standards allow:

- physical SIM;
- eSIM;
- cellular data;
- voice calls;
- messaging;
- modem state;
- carrier configuration;
- voicemail;
- network registration;
- radio power states.

Telephony SHALL be absent from desktop system images unless hardware or policy requires it.

---

# 25. Bluetooth/NFC/UWB/GNSS

These capabilities SHALL use a common capability model while remaining optional by hardware profile.

## Bluetooth

Support SHOULD include:

- Classic;
- BLE;
- audio;
- HID;
- GATT;
- newer direction/channel capabilities as hardware and stack support mature.

## NFC

Capabilities:

- tag operations;
- peer/device operations;
- secure transactions subject to explicit security policy.

## UWB

The subsystem SHOULD treat UWB as a location/ranging/security capability rather than tying application semantics to a particular chipset.

## GNSS

The location framework SHOULD expose:

- position;
- precision estimate;
- timing;
- assisted positioning;
- privacy level;
- sensor fusion where permitted.

---

# 26. Input architecture

## 26.1 Unified input model

AetherOS SHALL represent:

- keyboard;
- pointer/mouse;
- touch;
- stylus;
- game controller;
- rotary/knob input;
- voice;
- accessibility input.

as input capabilities/events.

## 26.2 Desktop

Prioritize:

- keyboard;
- pointer;
- high-precision pointing;
- shortcuts;
- multi-window focus.

## 26.3 Mobile

Prioritize:

- touch;
- multi-touch;
- gestures;
- stylus where present;
- one-handed interaction;
- voice.

---

# 27. Device lifecycle

Every device SHOULD follow a common lifecycle state machine:

```text
DISCOVERED
   ↓
PROBED
   ↓
INITIALIZED
   ↓
ACTIVE
   ↓
IDLE
   ↓
SUSPENDED
   ↓
REMOVING
   ↓
REMOVED
```

Failure path:

```text
ACTIVE
  ↓
FAILED
  ↓
RECOVERING
  ↓
ACTIVE or REMOVED
```

Hotplug-capable devices SHALL support appropriate transitions without requiring a reboot.

---

# 28. Firmware architecture

Firmware SHALL be treated separately from the driver while remaining tightly coordinated.

Firmware objects SHALL include:

- hardware identity;
- firmware version;
- compatibility range;
- signature;
- rollback metadata;
- update state;
- health state.

Firmware updates SHOULD be transactional and rollback-capable where the hardware supports safe rollback.

---

# 29. Application architecture

AetherOS SHALL support multiple application runtimes without allowing compatibility runtimes to define the native platform semantics.

## 29.1 Native Aether Runtime

Primary runtime for first-class applications.

## 29.2 Android Compatibility Runtime

Provides Android application compatibility through an isolated compatibility environment, with hardware-assisted virtualization preferred where supported. APK/AAB-related installation workflows SHALL remain technically and legally supportable. Android compatibility SHALL remain separate from native AetherOS semantics.

## 29.3 Web Runtime

Provides PWAs/web applications with controlled system integration.

## 29.4 Future runtimes

Potential future compatibility environments MAY include Linux/desktop ecosystems where practical.

---

# 30. Android compatibility

## 30.1 Goal

AetherOS SHOULD provide broad Android application compatibility without making Android the semantic or architectural foundation of AetherOS.

Android support is a compatibility bridge, not platform identity.

## 30.2 Compatibility architecture

The preferred architecture is an isolated Android environment, with hardware-assisted virtualization strongly preferred where available.

Conceptually:

```text
AetherOS Host
      │
      ├── Native Aether Runtime
      ├── Web Runtime
      └── Android Compatibility Environment
                │
                ├── Android framework/runtime
                ├── Android system services
                ├── Android application environment
                └── Android application
```

A MicroVM/protected VM architecture MAY be used where hardware and platform facilities permit.

A containerized or lighter-weight compatibility mode MAY be used where it can demonstrably provide sufficient fidelity and isolation.

The implementation SHALL be selected from measurable compatibility, latency, memory, energy, graphics, security, and hardware-integration results.

## 30.3 Why framework translation is not the canonical design

AetherOS SHALL NOT attempt to translate arbitrary Android framework behavior into Aether APIs as its sole compatibility mechanism.
The Android environment may depend on:

- Android runtime behavior;
- Binder semantics;
- Bionic;
- Linux kernel behavior;
- vendor APIs;
- Google Play services;
- DRM;
- Play Integrity;
- hardware-backed services;
- proprietary media/graphics behavior.

Providing an appropriately isolated Android execution environment is therefore preferred to recreating every Android implementation detail in AetherOS itself.

## 30.4 Virtual hardware mediation

The Android environment SHOULD see a controlled virtual hardware model for:

- graphics;
- audio;
- camera;
- sensors;
- storage;
- networking;
- input;
- display;
- power state.

The host SHALL retain authority over the underlying physical hardware.

## 30.5 Security and capabilities

The Android compatibility environment SHALL be an explicit Aether security domain.

AetherOS SHALL mediate access to:

- camera;
- microphone;
- location;
- storage;
- network;
- Bluetooth;
- USB/external devices;
- biometrics;
- other privileged hardware.

Revoking a capability MAY result in:

- virtual-device removal;
- restricted behavior;
- synthetic response;
- denied operation;

according to the semantics required by Android compatibility.

The Android guest SHALL NOT bypass the host capability model.

## 30.6 Application UI integration

Android applications MAY be represented:

- as their own Context;
- as a Provider within a larger Context;
- beside native Aether Providers;
- through embedded presentation where technically feasible.

Embedding does not imply shared security authority.

## 30.7 Semantic integration

Where available, Android accessibility/semantic information SHOULD be bridged into the Aether Semantic Graph.

The resulting coverage SHALL be marked as `LEGACY` or another appropriate level when semantics are incomplete.

## 30.8 Compatibility claims

AetherOS SHALL NOT promise a fixed percentage of Android application compatibility without measured evidence.

Compatibility SHALL be reported through a maintained compatibility matrix covering at least:

- mainstream applications;
- banking/enterprise applications where legally testable;
- media/DRM applications;
- games;
- camera/audio applications;
- applications using Google services;
- applications using sensors;
- applications using background execution;
- applications using unusual graphics paths.

## 30.9 Host resource policy

The Android environment SHALL participate in AetherOS resource management.

Memory, CPU, GPU, NPU, storage, power and network usage SHALL be measurable and controllable by host policy.

## 30.10 Compatibility updates

The Android compatibility environment SHALL be independently versioned and updateable where stable contracts permit.

# 31. Package management

## 31.1 Aether package model

Conceptual native package format: **Aether Package (APX)**.

A package MAY contain:

- executable(s);
- libraries;
- resources;
- metadata;
- dependency declarations;
- capability declarations;
- architecture variants;
- signatures;
- sandbox policy;
- optional components;
- migrations;
- uninstall/recovery metadata.

## 31.2 Installation

Generic installation workflow:

```text
identify package
 ↓
verify provenance/signature
 ↓
resolve dependencies
 ↓
check capability policy
 ↓
stage package
 ↓
activate
 ↓
health-check
 ↓
commit
```

## 31.3 Universal installer

The UI SHOULD accept:

- Aether packages;
- Android packages;
- web applications;
- enterprise packages;
- developer packages.

The installer determines the appropriate runtime and security policy.

## 31.4 Driver packages

Drivers SHALL use a related package/signature mechanism but receive distinct trust and lifecycle treatment.

---

# 32. Application lifecycle

Desktop applications may remain active indefinitely.

Mobile applications SHALL support richer lifecycle states:

```text
foreground
background
suspended
frozen
snapshot
terminated
```

The application API SHALL expose lifecycle state changes clearly and deterministically.

---

# 33. Security architecture

## 33.1 Trust chain

```text
Hardware root
   ↓
Secure boot
   ↓
Kernel
   ↓
System services
   ↓
Runtime
   ↓
Application
   ↓
AI agent
```

## 33.2 Capability-based access

Every privileged action SHALL require an explicit capability or an authenticated trusted system context; a user-owned Context SHALL NOT by itself confer system privilege.

## 33.3 Permission scopes

Permissions SHOULD support:

- one-time;
- session;
- persistent;
- background;
- time-limited;
- context-limited;
- hardware-limited;
- data-domain-limited.

## 33.4 Sensitive hardware

Special controls SHALL apply to:

- camera;
- microphone;
- precise location;
- biometrics;
- storage raw access;
- external device access;
- networking control;
- payment/security hardware.

## 33.5 Driver security

Drivers SHALL receive the minimum required capabilities.

## 33.6 AI security

AI agents SHALL NOT receive unrestricted OS access by virtue of being integrated into the platform.

---

# 34. Privacy architecture

## 34.1 Data locality classifications

Data SHOULD be tagged as:

- device-only;
- device-encrypted;
- trusted-execution-only;
- private-cloud permitted;
- user-selected cloud;
- public.

## 34.2 AI routing

The AI router SHALL respect data locality constraints.

An object marked device-only SHALL NOT be transmitted to a cloud model without an explicit user/policy authorization.

## 34.3 Data lifecycle

Every long-lived data object SHOULD have:

- owner;
- scope;
- creation time;
- retention policy;
- deletion policy;
- export policy;
- access policy.

---

# 35. Identity and authentication

AetherOS SHALL support a platform identity model capable of representing:

- local user accounts;
- device identity;
- application identity;
- organization identity;
- hardware-backed credentials;
- passkeys/modern credentials where applicable;
- secure-element/TPM-backed keys.

The identity system SHALL be usable by both desktop and mobile shells.

---

# 36. Files, user data and ownership

## 36.1 User-owned files

User files SHALL remain independent from the system's executable generation model.

## 36.2 Application data

Applications SHALL have isolated data domains with explicit sharing mechanisms.

## 36.3 Shared data

Shared access SHALL be explicit and capability-mediated.

## 36.4 File provenance

The system SHOULD retain metadata describing source/provenance for downloaded or externally imported objects where privacy policy allows.

---

# 37. AI and intelligence architecture

## 37.1 Aether Intelligence Layer

The AI system SHALL be a platform service rather than a single application.

```text
Aether Intelligence
├── Perception
├── Context
├── Memory
├── Retrieval
├── Reasoning
├── Planning
├── Tool use
├── Execution
└── Audit
```

## 37.2 Local/hybrid/cloud routing

```text
User request
 ↓
Privacy / latency / cost / capability evaluation
 ↓
Local model / hybrid / cloud model
```

## 37.3 AI as a resource

AI workloads SHALL be schedulable over:

- CPU;
- GPU;
- NPU;
- DSP;
- cloud compute.

## 37.4 Model management

AI models SHALL be treated as managed system resources.

Conceptual AI storage domain:

```text
AI
├── installed
├── active
├── cached
├── cold
├── rollback
├── quantized variants
└── remote metadata
```

## 37.5 Model eviction

Unused optional models MAY be removed under policy if they can be restored safely.

## 37.6 AI memory

Memory domains MAY include:

- ephemeral;
- working;
- project;
- application;
- personal;
- device/system.

Every memory object SHOULD have scope and policy.

---

# 38. AI agent architecture

Agents SHALL use the same capability framework as applications.

Example:

```text
Travel Agent
├── calendar.read       allowed
├── contacts.read       denied
├── location.read       allowed
├── web.search          allowed
├── booking.execute     approval-required
├── payment.execute     denied
└── messaging.send      approval-required
```

## 38.1 Agent execution lifecycle

```text
Intent
 ↓
Context retrieval
 ↓
Plan
 ↓
Policy evaluation
 ↓
Tool/action execution
 ↓
Verification
 ↓
State update
 ↓
Audit
```

## 38.2 Human approval

Consequential actions SHOULD require explicit approval unless the user has created a policy that authorizes them.

Suggested action levels:

```text
L0 Observation
L1 Reversible local action
L2 External communication
L3 Financial/consequential action
L4 Restricted/forbidden action
```

---

# 39. AI model storage and resource accounting

AetherOS SHALL distinguish:

- AI runtime;
- system models;
- optional user models;
- cached weights;
- embeddings/indexes;
- model rollback state.

The Settings UI SHALL report these separately rather than hiding them under generic system data.

---

# 40. User Interface and Human-Computer Interaction architecture

## 40.1 Purpose

The AetherOS UI/HCI subsystem is a first-class operating-system subsystem, not a presentation layer placed above applications.

It SHALL integrate with:

- application lifecycle;
- storage/state persistence;
- security/capabilities;
- graphics/composition;
- input;
- accessibility;
- AI;
- resource management;
- device topology;
- cross-device continuity.

The primary human-facing abstraction SHALL be the **Context**.

## 40.2 Core interaction hierarchy

```text
System
  │
  └── Workspace
        │
        └── Context
              │
              ├── Activities
              ├── Application Providers
              ├── Context State
              ├── Capabilities
              └── Presentation
                    ├── Visual Graph
                    ├── Semantic Graph
                    └── Input Graph
```

These concepts are intentionally distinct.

## 40.3 Application

An Application is a software deployment/runtime/security identity.

It owns, subject to system policy:

- package identity;
- executable/runtime components;
- publisher identity;
- application data;
- application lifecycle;
- application capabilities.

Applications remain first-class technical entities.

They are not necessarily the user's fundamental unit of work.

## 40.4 Application Provider

A Provider is an application instance participating in a Context.

A Context MAY contain multiple Providers.

An Application MAY provide many Contexts.

Providers retain their application-level identity and security boundary even when collaborating within one Context.

Cross-provider communication SHALL use explicit Context-scoped IPC/capabilities.

## 40.5 Context definition

A Context is a durable, security-bounded representation of a coherent unit of user activity, intention, state, and interaction.

Examples:

- writing a document;
- planning a trip;
- developing software;
- conducting research;
- preparing taxes;
- editing a video;
- handling a project communication workflow.

A Context MAY contain one or many applications/providers and one or many presentation surfaces.

A Context MAY exist while no participating Provider process is running.

## 40.6 Context state

A Context SHOULD maintain, as applicable:

```text
Context identity
Metadata
Activity state
Provider bindings
Provider/context state references
Semantic state
Input state
Capability state
Continuity state
Resource metadata
Optional AI context
```

The system SHALL distinguish:

```text
Context-owned state
Provider-owned durable state
Application-owned data
Cached presentation state
AI memory
```

These categories SHALL NOT be treated as interchangeable.

## 40.7 Context ownership and authority

A Context SHALL have an explicit owner identity and security domain.

Ownership means **authority over the Context as a user/work object**, not ownership of every Provider or every file participating in it.

The Context owner MAY:

- rename the Context;
- archive/delete it subject to policy;
- determine continuity behavior;
- grant Context-level collaborative access where supported;
- manage Context preferences;
- authorize Context-scoped capabilities subject to higher-level policy.

The Context owner does NOT automatically gain:

- kernel privilege;
- unrestricted access to Provider-private data;
- unrestricted access to another user's data;
- authority to bypass application or system security policy.

System policy remains authoritative above Context ownership.

For multi-user systems, Context ownership SHALL be associated with an identity/account/security principal rather than an arbitrary application.

## 40.8 Context boundaries

A Context boundary exists when independent policy, state, lifecycle, or user intent requires separation.

A Context SHOULD NOT be created merely because:

- a new application process starts;
- a new window is opened;
- a different display is used;
- a different file is opened;
- an animation begins.

A new Context SHOULD be created when one or more of the following materially differ:

- user intent/task;
- security authority;
- data ownership;
- lifecycle independence;
- capability scope;
- continuity semantics;
- independent attention/notification semantics.

A Context MAY deliberately include multiple applications when they form one coherent activity.

A Context SHOULD remain separate when combining activities would create ambiguous ownership, excessive privilege, unrelated lifecycle behavior, or confusing user intent.

## 40.9 Context security boundary

The Context SHALL NOT be treated as a replacement for the Application sandbox.

The security structure is:

```text
System policy
    ↓
User identity
    ↓
Context policy
    ↓
Application/Provider policy
    ↓
Resource/device capability
```

The effective authority SHALL be the intersection of applicable policies and capabilities.

## 40.10 Context lifecycle

```text
CREATED
   ↓
ACTIVE
   ↓
IDLE
   ↓
SUSPENDED
   ↓
SNAPSHOTTED / DORMANT
   ↓
RESTORED
   ↓
ACTIVE
```

Additional states MAY include:

- migrating;
- capability-limited;
- degraded;
- faulted;
- archived;
- deleted.

A Context MAY outlive all participating processes.

## 40.11 Context suspension/resume

Suspending a Context SHOULD allow the operating system to release:

- CPU state;
- RAM;
- GPU resources;
- accelerator state;
- temporary buffers;

while retaining sufficient durable state for reconstruction.

Desktop/laptop policies MAY retain more live state than mobile policies.

Mobile MAY aggressively freeze, snapshot, evict, and later reconstruct Providers.

The Context semantics remain shared.

## 40.12 Context multiplicity and composition

AetherOS SHALL support:

```text
one application → many Contexts
one Context → many Providers
one Provider → multiple surfaces
one Context → multiple Activities
one Context → multiple displays
```

This enables applications such as browsers to participate in distinct Contexts without forcing every Context to become a separate installed application.

## 40.13 Workspace

A Workspace organizes one or more Contexts and their presentations.

A Workspace MAY represent:

- a desktop workspace;
- a mobile home/interaction environment;
- a project environment;
- a large-screen environment;
- a user-selected operating mode.

A Workspace SHALL NOT inherently equal a physical display.

## 40.14 Activity

An Activity is a more granular unit of interaction within a Context.

An Activity MAY involve one or more Providers.

Activities SHALL NOT be required to map one-to-one to:

- processes;
- windows;
- applications;
- files.

## 40.15 Context relationships

The system MAY represent relationships such as:

```text
related-to
derived-from
depends-on
blocked-by
continuation-of
child-of
parent-of
shared-with
```

These relationships SHALL NOT implicitly merge security domains.

## 40.16 Three-Graph UI architecture

Every native interactive presentation SHOULD expose three correlated graphs:

```text
                  UI Transaction
                       │
         ┌─────────────┼─────────────┐
         │             │             │
   Visual Graph   Semantic Graph   Input Graph
```

### Visual Graph

Represents actual presentation and composition, including:

- surfaces;
- buffers;
- transforms;
- clipping;
- effects;
- display targets;
- rendering metadata.

### Semantic Graph

Represents what the presentation means, including:

- text;
- controls;
- roles;
- labels;
- values;
- states;
- actions;
- relationships;
- application/document objects.

It SHALL be usable by:

- AI;
- accessibility;
- automation;
- search;
- testing;
- Context reconstruction.

### Input Graph

Represents how interaction is routed, including:

- hit regions;
- focus;
- pointer targets;
- touch targets;
- gesture regions;
- keyboard actions;
- stylus targets;
- modal regions;
- input capture;
- z-order/input masking.

## 40.17 UI transactions

Visual, Semantic, and Input Graph state SHALL be correlated by a logical UI transaction/version.

Example:

```text
UI Transaction #48192

Visual state     → version 48192
Semantic state   → version 48192
Input state      → version 48192
```

The transaction does NOT require all graphs to be regenerated for every display frame.

## 40.18 Incremental graph updates

Semantic and Input Graphs SHOULD be independently updated from visual frame submission.

For transforms such as scrolling:

```text
Initial:
Visual + Semantic + Input

Subsequent frames:
Visual transform/update
Semantic graph reused
Input graph transformed/reused
```

The compositor SHALL preserve visual/semantic/input correspondence.

## 40.19 Semantic coverage

Native and compatible surfaces SHALL advertise semantic coverage where practical:

```text
FULL
PARTIAL
OPAQUE
LEGACY
REMOTE
UNAVAILABLE
```

### FULL

Authoritative native semantics are available.

### PARTIAL

Some semantics are available while visual regions remain opaque.

### OPAQUE

Semantic interpretation is unavailable or prohibited.

Typical reasons include protected/DRM content or explicit privacy boundaries.

### LEGACY

The presentation originates from a compatibility environment.

### REMOTE

The presentation originates outside the local device/runtime.

### UNAVAILABLE

No reliable semantics are available.

The system SHALL NOT fabricate authoritative semantics.

## 40.20 Semantic interpretation hierarchy

AetherOS SHOULD prefer:

```text
Authoritative native Semantic Graph
            ↓
Application metadata
            ↓
System-known object state
            ↓
Compatibility/accessibility semantics
            ↓
Authorized visual interpretation
            ↓
No interpretation
```

VLM/pixel interpretation MAY be used when permitted and useful.

It SHALL be treated as an interpretation rather than silently promoted to authoritative state.

## 40.21 Accessibility

Accessibility SHALL consume the same semantic model used for native automation and AI where possible.

The Semantic Graph SHOULD expose:

- role;
- name;
- description;
- value;
- state;
- relationships;
- available actions;
- navigation order.

## 40.22 Context Router

The launcher SHALL evolve into a **Context Router** whose responsibilities include:

- Context discovery;
- Context resumption;
- Context creation;
- Context switching;
- Workspace switching;
- Activity discovery;
- universal retrieval;
- deterministic system actions;
- application discovery;
- AI interaction.

An application grid MAY remain available as an optional navigation surface.

It SHALL NOT be the only conceptual launcher model.

## 40.23 Intent Gateway

The Context Router SHALL expose an Intent Gateway supporting at least:

```text
retrieve
resume
create
navigate
execute
query
search
automate
```

Simple deterministic actions SHOULD bypass AI.

Complex interpretation/planning MAY invoke AI.

## 40.24 Mobile shell

Mobile UX SHALL prioritize:

- touch;
- gestures;
- one-handed interaction;
- limited screen area;
- rapid Context switching;
- interruption control;
- glanceability;
- battery efficiency;
- high-refresh responsiveness.

A Context Stream MAY be the primary launcher presentation.

A conceptual mobile structure is:

```text
┌────────────────────────────┐
│ System / Status            │
├────────────────────────────┤
│ Intent Gateway             │
├────────────────────────────┤
│ Context Stream             │
│                            │
│ Context                    │
│ Context                    │
│ Context                    │
│                            │
├────────────────────────────┤
│ Deterministic Actions      │
└────────────────────────────┘
```

Exact pixel dimensions SHALL be derived from physical display characteristics rather than frozen in the specification.

## 40.25 Desktop/laptop shell
Desktop/laptop UX SHALL support:

- windows;
- multiple surfaces;
- multiple displays;
- pointer;
- keyboard;
- dense information presentation;
- tiling;
- floating surfaces;
- desktop-class application workflows.

Windows remain valid presentation surfaces but are not the fundamental human abstraction.

## 40.26 Tablet/foldable shell

Tablet/foldable UX SHALL adapt Context presentation to:

- display area;
- posture;
- hinge state;
- orientation;
- stylus;
- touch reachability;
- available multitasking space.

Context identity SHALL survive presentation changes.

## 40.27 Multi-display model

A Context SHALL NOT inherently belong to one physical display.

It MAY:

- occupy one display;
- occupy several displays;
- provide different surfaces on different displays;
- migrate between displays;
- change layout as display topology changes.

Connecting/disconnecting displays SHALL modify presentation state rather than necessarily destroying or recreating Context state.

## 40.28 Context splitting

A Context MAY simultaneously present multiple surfaces.

For example:

```text
Project Context

┌──────────────┬─────────────────────┐
│ Notes        │ Browser             │
├──────────────┼─────────────────────┤
│ Terminal     │ Document            │
└──────────────┴─────────────────────┘
```

The surfaces may come from different Providers.

A user MAY still create separate Contexts when security, lifecycle, attention, or intent warrant independent boundaries.

## 40.29 Notifications

Notifications SHALL be modeled as events associated with semantic entities, including:

- Context;
- Activity;
- Application;
- Provider;
- Person;
- Device;
- System.

Where meaningful Context association exists, noncritical notifications SHOULD update the Context rather than unnecessarily interrupting the user.

Application provenance SHALL remain visible.

Minimum conceptual priorities:

```text
LIFE_SAFETY
CRITICAL
ACTION_REQUIRED
IMPORTANT
INFORMATION
BACKGROUND
```

## 40.30 Deterministic system actions

AetherOS SHALL provide direct, low-latency actions for operations such as:

- flashlight;
- camera;
- volume;
- display brightness;
- connectivity;
- media controls;
- emergency functions;
- selected system settings.

AI MAY provide natural-language access to such operations but SHALL NOT be required for their basic execution.

## 40.31 AI interaction with Contexts

AI SHOULD interact with:

```text
Context state
Semantic Graph
Application metadata
Device Graph
System state
Capabilities
Stored knowledge
```

before resorting to screenshot/VLM interpretation.

AI MAY:

- search Contexts;
- summarize Contexts;
- create Contexts;
- resume Contexts;
- navigate Contexts;
- invoke authorized Provider actions;
- execute Context-scoped workflows.

AI SHALL remain subject to the capability and security model.

## 40.32 Context-aware AI memory

The existence of a Context or Semantic Graph SHALL NOT grant AI unrestricted access to all Context information.

Access SHALL consider:

- identity;
- Context ownership;
- security policy;
- data classification;
- capability;
- purpose;
- retention.

AI memory SHALL distinguish authoritative state from inferred or cached information.

## 40.33 Cross-device Context continuity

A compatible Context SHALL be migratable between compatible AetherOS devices.

The Context identity remains stable while the presentation is reconstructed according to the destination device profile.

Unavailable destination capabilities SHALL cause controlled adaptation rather than silent semantic corruption.

## 40.34 Animation architecture

Animations SHALL be primarily declarative.

An animation SHOULD specify:

- initial state;
- target state;
- duration;
- interpolation;
- synchronization;
- constraints;
- cancellation behavior.

The Aether Animation Scheduler SHALL select among:

- display/controller hardware;
- compositor hardware;
- GPU;
- specialized accelerators;
- CPU.

Selection SHOULD minimize energy and latency while satisfying visual requirements.

## 40.35 UI resource management

UI work SHALL participate in the platform resource model.

Relevant resources include:

- CPU;
- GPU;
- NPU;
- memory;
- display bandwidth;
- power;
- thermal capacity.

Desktop/laptop and mobile policies MAY use different optimization priorities.

## 40.36 UI security

Security-sensitive system UI SHALL have authenticated origin and protected input routing.

Untrusted applications SHALL NOT be able to impersonate:

- authentication prompts;
- permission dialogs;
- trusted system indicators;
- security-critical controls;
- recovery UI.

## 40.37 UI failure/degraded modes

The primary UI path SHALL have a degraded rendering strategy.

Conceptually:

```text
Normal compositor
      ↓
failure
Emergency compositor/path
      ↓
minimal rendering
      ↓
recovery UI
```

The fallback path SHALL not depend on the same failing subsystem.

Failure of:

- AI;
- network;
- an application;
- a Provider;

SHALL NOT unnecessarily terminate the UI system.

GPU/compositor failures SHOULD permit degraded interaction where hardware permits.

## 40.38 UI observability

The system SHALL measure:

- input-to-photon latency;
- frame latency;
- frame pacing;
- dropped frames;
- animation jitter;
- compositor stalls;
- semantic update latency;
- input-routing latency;
- Context-switch latency;
- Context-resume latency;
- display-topology transition latency;
- UI energy consumption.

Peak FPS SHALL NOT be the sole UI quality metric.

## 40.39 UI platform invariants

AetherOS SHALL maintain these invariants:

1. Context identity is independent of process identity.
2. Application identity remains independent of Context identity.
3. Context ownership does not bypass system/application security boundaries.
4. Windows/surfaces are presentation mechanisms, not the sole human abstraction.
5. Visual, Semantic, and Input Graphs remain conceptually distinct.
6. The three graphs may be updated at different frequencies while remaining transactionally correlated.
7. Native semantics are authoritative when available.
8. The system SHALL identify incomplete/unavailable semantic coverage.
9. AI SHALL prefer structured semantics to pixel reconstruction.
10. Platform shells MAY differ substantially while consuming the same Context semantics.
11. A Context SHALL survive display topology changes.
12. A Context SHOULD survive provider/process failure.
13. UI security boundaries SHALL be independent of visual composition boundaries.
14. Deterministic system actions SHALL remain available without AI.
15. UI resource consumption SHALL be measurable.
16. The semantic model SHALL not depend on rectangular displays or a single input modality.
17. Failure of one UI subsystem SHALL not unnecessarily destroy unrelated state.

## 40.40 Normative summary

> **AetherOS SHALL implement a Context-centric, semantic, multi-modal UI architecture in which Applications are technical deployment/security entities, Providers supply functionality to Contexts, Contexts represent coherent human activities, Workspaces organize Contexts, and Visual, Semantic, and Input Graphs provide correlated representations of presentation, meaning, and interaction. The semantic model SHALL remain shared across desktop, laptop, mobile, tablet, foldable, accessibility, automation, and AI workloads, while presentation, input policy, rendering strategy, lifecycle pressure, and resource policy MAY be specialized for each platform.**

> **AetherOS UI SHALL optimize for clarity, accessibility, responsiveness, semantic integrity, security, resource efficiency, recoverability, and extensibility. Visual sophistication SHALL never be treated as a substitute for interaction quality or system reliability.**

# 48. Power architecture

## 48.1 Unified power model

Every device SHOULD expose:

- active;
- idle;
- sleep;
- retention;
- deep sleep;
- off;
- performance states.

## 48.2 Power budget

AetherOS SHALL track an estimated energy budget across:

- display;
- CPU;
- GPU;
- NPU;
- modem;
- Wi-Fi;
- Bluetooth;
- camera;
- sensors;
- storage;
- external USB power.

## 48.3 Mobile optimization

The mobile platform SHOULD minimize:

- radio wakeups;
- sensor sampling when unnecessary;
- display refresh when not useful;
- CPU residency disruption;
- redundant storage writes;
- background work.

## 48.4 Desktop optimization

Desktop systems SHOULD prioritize performance and responsiveness while still supporting meaningful idle/sleep efficiency.

---

# 49. Thermal architecture

AetherOS SHALL treat thermal capacity as a shared resource.

Thermal state SHALL influence:

- scheduler;
- GPU frequency;
- NPU workload;
- camera processing;
- charging;
- display refresh;
- background work.

## 49.1 Mobile thermal modes

Conceptually:

```text
normal
warm
hot
critical
```

At increasingly severe states, the system SHALL reduce or relocate workloads while preserving critical functionality.

---

# 50. External device ecosystem

The OS SHALL provide a standardized framework for:

- keyboards;
- mice;
- game controllers;
- printers;
- scanners;
- cameras;
- microphones;
- audio interfaces;
- storage;
- docks;
- external displays;
- networking hardware;
- accelerators;
- USB-C accessories;
- specialized industrial hardware where drivers exist.

No OS can guarantee support for every proprietary device ever created. The realistic requirement is an extensible signed-driver architecture capable of supporting any technically and legally accessible device interface.

---

# 51. Docking and device topology

AetherOS SHOULD expose an end-user device topology view.

Example:

```text
Laptop
├── Internal Display
├── GPU
├── NVMe
├── Wi-Fi
└── USB4 Controller
    └── Dock
        ├── Display 1
        ├── Display 2
        ├── Ethernet
        ├── Audio Interface
        └── Storage
```

The same topology SHALL be queryable by developer tools and AI services subject to permission policy.

---

# 52. Health and diagnostics

AetherOS SHALL have a Device Health API capable of reporting where the hardware supports it:

- temperature;
- wear;
- errors;
- firmware version;
- performance state;
- power state;
- health/fault state;
- reset count;
- device availability.

The system SHOULD explain meaningful resource problems to users.

Example:

> “Battery drain is primarily associated with camera + modem + high display refresh over the last hour.”

---

# 53. Observability

The system SHALL provide developer-grade tracing for:

- CPU scheduling;
- GPU scheduling;
- memory pressure;
- I/O latency;
- network activity;
- wakeups;
- thermals;
- battery drain;
- process lifecycle;
- driver events;
- device resets;
- frame timing;
- update operations.

Privacy-sensitive user content SHOULD NOT be collected merely to support observability.

---

# 54. Crash containment and reliability

## 54.1 Service isolation

High-level system services SHOULD be restartable independently.

## 54.2 Fault domains

Failure of:

- camera;
- Bluetooth;
- Wi-Fi;
- audio;
- GPU service;
- a driver;
- Android compatibility runtime;
- AI service

SHOULD not inherently bring down unrelated parts of the OS.

## 54.3 Self-healing

Where hardware supports safe reset/reinitialization, the system SHOULD attempt recovery automatically.

## 54.4 Diagnostic state

Before reset/recovery, the system SHOULD capture minimal diagnostic information needed to identify the failure.

---

# 55. Virtualization

Desktop/laptop AetherOS SHALL support virtual machines as a first-class concern.

The device model SHOULD support virtualized:

- CPU;
- memory;
- storage;
- network;
- display;
- USB;
- GPU where supported;
- virtual TPM/security devices.

AetherOS SHOULD allow controlled device assignment using IOMMU capabilities.

Mobile may use virtualization for isolated compatibility/security workloads rather than general-purpose desktop virtualization by default.

---

# 56. Development and driver testing

AetherOS SHOULD provide simulated hardware devices:

```text
Virtual GPU
Virtual Camera
Virtual Sensor
Virtual Modem
Virtual Battery
Virtual USB devices
Virtual Storage
```

This allows CI to test driver and resource behavior before real hardware is available.

## 56.1 Fault injection

Tests SHOULD simulate:

- driver crash;
- DMA fault;
- device timeout;
- storage corruption;
- firmware mismatch;
- hotplug/unplug;
- thermal excursion;
- low battery;
- memory pressure;
- network disappearance;
- failed update.

---

# 57. Developer platform

## 57.1 Aether SDK

The SDK SHOULD expose:

- UI;
- storage;
- networking;
- camera;
- audio;
- sensors;
- GPU;
- NPU;
- AI;
- notifications;
- background execution;
- automation;
- security;
- package management;
- inter-application communication.

## 57.2 Native application portability

A native Aether application SHOULD be able to target multiple platform shells while sharing application logic and system semantics.

## 57.3 Driver SDK

Driver SDK SHALL provide:

- Device Graph APIs;
- bus APIs;
- resource APIs;
- DMA/IOMMU APIs;
- interrupt APIs;
- power APIs;
- firmware APIs;
- diagnostics;
- testing;
- packaging/signing;
- certification tools.

---

# 58. Application-to-hardware architecture

Applications SHALL normally follow:

```text
Application
 ↓
Capability
 ↓
Device Class API
 ↓
Driver
 ↓
Bus
 ↓
Hardware
```

They SHALL NOT normally depend on vendor-specific bus mechanics.

Privileged developer/system software MAY access lower levels through explicit capabilities.

---

# 59. IPC architecture

AetherOS SHALL provide a coherent IPC mechanism supporting:

- asynchronous messaging;
- request/reply;
- event streams;
- capability transfer;
- resource handles;
- zero-copy/shared-memory paths where safe;
- service discovery.

The IPC architecture SHALL support efficient fast paths for graphics, storage, networking and multimedia while retaining strong isolation.

---

# 60. System service discovery

Services SHOULD be discoverable through typed interfaces.

Example:

```text
CameraService
StorageService
DisplayService
AudioService
NetworkService
AIService
PowerService
DeviceManager
```

Interfaces SHALL be versioned.

---

# 61. API and ABI versioning

AetherOS interfaces SHALL have explicit version contracts.

A driver/application can declare:

```text
API major
API minor
minimum supported OS
maximum tested OS
required capabilities
```

The OS SHALL reject incompatible modules before activation whenever possible.

---

# 62. Hardware profile and certification system

AetherOS SHALL define a hardware profile model:

```text
Aether Hardware Profile
├── device identity
├── architecture
├── bus topology
├── hardware capabilities
├── firmware requirements
├── driver packages
├── power characteristics
├── thermal characteristics
└── certification metadata
```

A device can be:

- reference;
- certified;
- community-supported;
- experimental.

---

# 63. Reference hardware strategy

AetherOS SHOULD have at least one official reference device for each major class.

## 63.1 Desktop reference

Representative x86-64 or equivalent general-purpose system.

## 63.2 Laptop reference

Modern mobile processor platform, integrated GPU, NVMe, modern wireless, USB-C/USB4 and power management.

## 63.3 Mobile reference

ARM64 SoC, modern GPU, NPU, ISP, UFS, display, touch, multiple cameras, modem, Wi-Fi, Bluetooth and secure boot facilities.

Exact vendors/chips are implementation choices to be determined by hardware availability, licensing, documentation and driver access.

---

# 64. Source-tree organization

The source tree SHALL reflect architectural responsibilities.

Suggested structure:

```text
aether/
├── boot/
├── kernel/
├── core/
│   ├── process/
│   ├── memory/
│   ├── scheduler/
│   ├── ipc/
│   └── security/
├── storage/
│   ├── filesystem/
│   ├── object_store/
│   ├── snapshots/
│   ├── packages/
│   └── encryption/
├── hardware/
│   ├── discovery/
│   ├── graph/
│   ├── resources/
│   ├── buses/
│   └── devices/
├── drivers/
│   ├── storage/
│   ├── graphics/
│   ├── camera/
│   ├── audio/
│   ├── network/
│   ├── sensors/
│   ├── input/
│   ├── power/
│   └── accelerators/
├── services/
├── runtime/
│   ├── native/
│   ├── android/
│   └── web/
├── intelligence/
│   ├── inference/
│   ├── memory/
│   ├── agents/
│   └── policy/
├── ui/
├── shell/
│   ├── desktop/
│   ├── laptop/
│   ├── mobile/
│   ├── tablet/
│   └── foldable/
├── applications/
├── sdk/
├── tools/
├── tests/
├── profiles/
│   ├── desktop/
│   ├── laptop/
│   ├── mobile/
│   └── tablet/
└── devices/
    ├── desktop/
    ├── laptop/
    └── mobile/
```

The source tree SHALL avoid arbitrary historical “dumping grounds.” Every major boundary SHALL have an architectural justification.

---

# 65. Build system and target composition

The build system SHALL support target-specific composition.

Conceptual commands:

```text
aether build --platform desktop --hardware <profile>
aether build --platform laptop  --hardware <profile>
aether build --platform mobile  --hardware <profile>
```

The build system SHALL include only components relevant to the target unless explicitly selected.

## 65.1 Target-specific exclusion

A desktop build SHOULD NOT permanently include mobile-only components such as:

- cellular framework;
- telephony services;
- GNSS stack;
- NFC stack;
- mobile-only sensor services;

unless required by the chosen hardware profile.

Likewise, a mobile build SHOULD NOT include unnecessary desktop-specific infrastructure.

---

# 66. System footprint governance

This is a first-class requirement.

## 66.1 Footprint dimensions

Every release SHOULD report:

- compressed system size;
- installed system size;
- application runtime size;
- hardware package size;
- AI runtime/model size;
- recovery footprint;
- update workspace requirement;
- peak RAM;
- idle RAM;
- boot time;
- cold launch time;
- resume time;
- CPU background usage;
- wakeups;
- energy consumption.

## 66.2 Footprint regression detection

CI SHALL identify unexplained increases.

Example:

```text
System generation N
System: 4.81 GB

System generation N+1
System: 5.03 GB

Regression: +220 MB
```

The build pipeline SHOULD break down the change by component and require an explanation or corrective action.

## 66.3 No unexplained storage category

Release artifacts SHALL have zero unexplained persistent content.

---

# 67. System storage targets

Illustrative design targets, not hard implementation limits:

### Core OS

Approximately 3–5 GB target on a flagship/reference installation.

### Full first-party system

Approximately 5–8 GB target before optional user applications/data and large optional AI model collections.

### Android compatibility

Separately accounted and minimized; target roughly 1–3 GB where implementation permits.

### AI models

Managed separately and user/configuration dependent.

These targets SHALL be validated through actual engineering measurements and are subject to revision.

---

# 68. Resource accounting UI

AetherOS SHALL expose transparent accounting.

Example:

```text
STORAGE
────────────────────────
Core OS                4.1 GB
Hardware support      1.2 GB
System applications   1.1 GB
Compatibility         0.9 GB
AI models             1.6 GB
Snapshots             0.7 GB
Recovery              0.1 GB
Temporary             0.1 GB
────────────────────────
System total          9.8 GB
```

Every category SHALL be drillable.

---

# 69. System self-auditing

AetherOS SHOULD provide an OS Storage Auditor capable of identifying:

- unused optional components;
- old rollback snapshots;
- stale AI models;
- duplicate objects;
- temporary data;
- obsolete packages;
- unusual footprint growth.

The auditor SHALL distinguish safe automatic cleanup from user-confirmed cleanup.

---

# 70. Recovery and reset

## 70.1 Recovery modes

AetherOS SHOULD provide:

- normal boot;
- previous system generation;
- recovery;
- diagnostics;
- factory reset;
- external recovery;
- network recovery where hardware/platform permits.

## 70.2 Factory reset

Factory reset SHALL distinguish:

- user-data reset;
- application-data reset;
- settings reset;
- complete secure erase where supported.

The OS SHALL avoid confusing recovery operations with deletion of trusted system generations unless explicitly chosen.

---

# 71. Backup and restore

AetherOS SHOULD back up state rather than only files.

Potential state categories:

- user files;
- application state;
- application preferences;
- system settings;
- launcher layout;
- device configuration;
- AI memory where enabled;
- workflow definitions;
- package inventory.

Restore SHALL respect compatibility, hardware differences, privacy policies and permissions.

---

# 72. Cross-device continuity

AetherOS SHOULD eventually support:

```text
Phone ↔ Tablet ↔ Laptop ↔ Desktop
```

with shared identity and transferable application/session state.

The underlying semantic state SHALL remain device-independent where possible, while UI presentation and available hardware capabilities vary.

---

# 73. Desktop-mobile convergence

A phone connected to a keyboard, mouse and external display MAY transition from mobile shell to a desktop-class shell without changing the underlying OS/runtime model.

The same application can adapt through:

- responsive UI;
- multiple-window capability;
- external input capability;
- display capabilities.

---
# 74. Common application state model

Aether applications SHOULD be modeled around durable state rather than presentation alone.

Example:

```text
Application State
├── documents
├── task state
├── user preferences
├── workflow state
└── session context
```

The same state can feed mobile and desktop shells.

---

# 75. System command interface

AetherOS SHALL support a universal command interface exposed by:

- keyboard shortcut;
- launcher/search;
- voice;
- optional gesture;
- accessibility controls.

Examples:

> Find the contract from last week.

> Show all external storage devices.

> Enable maximum camera quality.

> Put this application into low-power mode.

> Summarize this file.

Natural-language commands SHALL resolve to deterministic system capabilities wherever possible.

AI SHALL not be required for simple deterministic operations.

---

# 76. Deterministic-first system design

AetherOS SHALL not use an LLM where a deterministic subsystem can perform the same action more reliably and efficiently.

Examples:

- flashlight control → deterministic hardware API;
- file enumeration → filesystem/index;
- device inventory → Device Graph;
- battery percentage → power service;
- complex planning → AI if valuable.

AI is a reasoning/interface layer, not a replacement for deterministic operating-system primitives.

---

# 77. AI and hardware integration

AI SHALL query structured system state where possible rather than requiring screenshots or heuristic reconstruction.

Examples:

```text
AI asks Resource Service:
GPU load
NPU load
camera active?
modem active?
refresh rate?
thermal state?
```

This produces more reliable and auditable behavior.

---

# 78. Application and AI audit ledger

Every privileged AI action SHOULD produce an audit trail:

```text
Intent
 ↓
Context
 ↓
Plan
 ↓
Policy evaluation
 ↓
Tools/capabilities used
 ↓
Action
 ↓
Result
 ↓
Verification
```

The same infrastructure MAY be used for non-AI automated workflows.

---

# 79. Performance philosophy

AetherOS SHALL optimize for real-world latency and efficiency rather than synthetic benchmark scores alone.

Key metrics:

- touch/input-to-visible-response;
- frame pacing;
- cold launch;
- warm launch;
- resume;
- storage latency;
- sustained compute throughput;
- thermal performance after long workloads;
- network responsiveness;
- power consumption.

---

# 80. Mobile-specific performance priorities

Measure:

- application resume latency;
- camera startup;
- camera capture-to-preview latency;
- biometric response;
- modem wakeup latency;
- notification delivery latency;
- background battery drain;
- idle drain;
- sustained gaming/AI performance.

---

# 81. Desktop/laptop-specific performance priorities

Measure:

- multi-monitor frame timing;
- application launch;
- compilation/build performance;
- storage throughput;
- virtualization overhead;
- GPU compute/rendering;
- suspend/resume;
- external-device attachment latency;
- sustained thermal performance.

---

# 82. Testing architecture

Testing SHALL be multi-layered.

## 82.1 Unit tests

Individual subsystem correctness.

## 82.2 Integration tests

Cross-service behavior.

## 82.3 Hardware tests

Real devices and reference boards.

## 82.4 Simulation tests

Virtual hardware and fault injection.

## 82.5 Performance tests

Regression benchmarks.

## 82.6 Power tests

Energy and wakeup profiling.

## 82.7 Security tests

Privilege boundaries, isolation, boot-chain integrity, driver trust and package verification.

## 82.8 Compatibility tests

Native, Android and web application behavior.

---

# 83. Formal verification priorities

Formal methods SHOULD focus first on critical invariants rather than every line of the OS.

Priority targets:

- boot state machine;
- storage transaction semantics;
- snapshot/rollback correctness;
- capability transitions;
- package installation transaction;
- update state machine;
- driver lifecycle;
- DMA/IOMMU security boundaries;
- permission state machines;
- power-state transitions.

---

# 84. Architectural invariants

## AOS-ARCH-001
AetherOS SHALL maintain one semantic platform model across supported device classes.

## AOS-ARCH-002
Platform-specific behavior SHALL be isolated behind explicit profiles, policies, hardware capabilities or experience shells.

## AOS-ARCH-003
No subsystem SHALL assume mobile or desktop characteristics unless that assumption is explicit and testable.

## AOS-ARCH-004
Physical storage partitions SHALL exist only for justified hardware, firmware, security or reliability requirements.

## AOS-ARCH-005
Logical storage separation SHALL NOT automatically imply physical partitioning.

## AOS-ARCH-006
Every persistent system object SHALL have a known owner, purpose and lifecycle.

## AOS-ARCH-007
System storage SHALL be attributable and user-visible.

## AOS-ARCH-008
System updates SHALL be transactional and rollback-capable.

## AOS-ARCH-009
Drivers SHALL have explicit lifecycle and compatibility contracts.

## AOS-ARCH-010
AI SHALL operate through the capability and policy framework.

---

# 85. Storage invariants

## AOS-STO-001
The system SHALL avoid large fixed reserves for hypothetical update requirements whenever dynamic allocation can provide equivalent reliability.

## AOS-STO-002
Identical immutable content MAY be deduplicated when safe.

## AOS-STO-003
System and user data SHALL have independent logical ownership.

## AOS-STO-004
Snapshots SHALL be crash-consistent.

## AOS-STO-005
Rollback SHALL not require duplicating unchanged data unnecessarily.

## AOS-STO-006
Unsupported file formats SHALL remain storable.

## AOS-STO-007
The storage engine SHALL distinguish permanent content from temporary/cache content.

## AOS-STO-008
Builds SHALL have zero unexplained persistent release artifacts.

---

# 86. Hardware invariants

## AOS-HW-001
All discoverable hardware SHALL be represented in the Device Graph.

## AOS-HW-002
Normal application APIs SHALL not require knowledge of physical buses.

## AOS-HW-003
Every device SHALL expose capabilities and resource requirements.

## AOS-HW-004
DMA SHALL be isolated by IOMMU when possible.

## AOS-HW-005
Driver failure SHALL be isolated where possible.

## AOS-HW-006
Firmware SHALL be versioned and verified.

## AOS-HW-007
Hardware-specific extensions SHALL not corrupt the common platform interface.

## AOS-HW-008
External devices SHALL be dynamically discoverable where hardware permits.

---

# 87. Security invariants

## AOS-SEC-001
Secure boot SHALL establish a verifiable chain of trust.

## AOS-SEC-002
Applications SHALL not receive arbitrary hardware access by default.

## AOS-SEC-003
AI agents SHALL not receive implicit unrestricted privilege.

## AOS-SEC-004
Sensitive permissions SHALL support explicit scope and lifecycle.

## AOS-SEC-005
Drivers SHALL have least-privilege access consistent with functionality.

---

# 88. Resource invariants

## AOS-RES-001
CPU, memory, storage, network, accelerator and power usage SHOULD be measurable by responsible component.

## AOS-RES-002
Platform profiles SHALL be allowed to use different resource policies without changing resource semantics.

## AOS-RES-003
Thermal constraints SHALL be treated as resource constraints.

## AOS-RES-004
Energy-sensitive device classes SHALL minimize unnecessary wakeups.

---

# 89. Mobile-specific architectural invariants

## AOS-MOB-001
Mobile SHALL share core OS semantics with desktop/laptop.

## AOS-MOB-002
Mobile SHALL implement a distinct low-power resource policy.

## AOS-MOB-003
Mobile application lifecycle SHALL support aggressive background suspension/eviction.

## AOS-MOB-004
Cellular, camera, sensors and battery SHALL be first-class system resources.

## AOS-MOB-005
Mobile hardware-specific storage topology SHALL not dictate the conceptual AetherOS storage architecture.

## AOS-MOB-006
Mobile UI SHALL not be a reduced desktop UI; it SHALL be a purpose-designed experience shell.

---

# 90. Desktop/laptop-specific architectural invariants

## AOS-DESK-001
Desktop/laptop SHALL provide general-purpose external-device support.

## AOS-DESK-002
Desktop/laptop SHALL support multi-window and multi-display experiences.

## AOS-DESK-003
Desktop/laptop SHALL support full developer/administrative workflows subject to security policy.

## AOS-DESK-004
Laptop SHALL incorporate battery and thermal policies without changing the general-purpose semantic model.

---

# 91. API abstraction philosophy

AetherOS SHALL expose hardware and services at the highest useful semantic level.

Examples:

```text
Camera.capture()
Storage.open()
Display.present()
Network.connect()
Sensor.subscribe()
AI.execute()
```

Advanced APIs MAY expose lower-level functionality.

The implementation SHALL be able to map these APIs to different buses or silicon.

---

# 92. Raw hardware escape hatches

For systems developers and approved hardware developers, AetherOS SHOULD permit privileged:

- raw device access;
- MMIO;
- diagnostic registers;
- DMA control;
- bus-specific operations;
- firmware operations.

Such access SHALL be explicitly privileged and audited where practical.

---

# 93. Vendor ecosystem

The OS SHALL permit hardware vendors to publish:

- drivers;
- firmware;
- hardware profiles;
- optional extensions;
- acceleration backends;
- diagnostics.

Vendor packages SHALL not be allowed to redefine common Aether semantics.

---

# 94. Device certification

Aether-certified devices SHOULD pass:

- boot;
- storage;
- security;
- update;
- display;
- graphics;
- audio;
- camera where present;
- networking;
- power;
- thermal;
- driver recovery;
- external device;
- AI acceleration where advertised.

---

# 95. Reference device support levels

Suggested levels:

### Tier 1 — Official reference

Fully optimized and validated.

### Tier 2 — Certified

Meets defined platform requirements.

### Tier 3 — Community

Core functions supported, varying optimization.

### Tier 4 — Experimental

Bring-up/test support with known limitations.

---

# 96. Distribution and installation strategy

AetherOS distribution SHOULD include:

- signed system generations;
- hardware profile packages;
- application repository/index;
- driver repository;
- recovery artifacts;
- SDK;
- developer tools.

Users SHOULD be able to install software from:

- official store/repository;
- verified developer packages;
- enterprise repositories;
- local packages;
- compatible Android packages;
- web apps.

---

# 97. Enterprise considerations

Future enterprise support SHOULD include:

- SSO;
- device management;
- policy enforcement;
- remote wipe;
- hardware attestation;
- audit logs;
- organizational repositories;
- fleet deployment;
- certificate management;
- controlled AI policies.

These features SHALL not unnecessarily burden consumer builds.

---

# 98. Consumer usability principles

AetherOS SHALL avoid exposing implementation complexity unnecessarily.

The user SHOULD think in terms of:

- apps;
- files;
- devices;
- capabilities;
- storage;
- battery;
- tasks;
- permissions.

The user SHOULD NOT need to understand:

- physical partition tables;
- vendor HAL layers;
- driver ABI internals;
- boot metadata formats;
- bus enumeration details

to use normal functionality.

---

# 99. Expert usability principles

Developers and administrators SHOULD have access to:

- device graph;
- driver state;
- hardware health;
- resource traces;
- package provenance;
- storage topology;
- system generations;
- permission graph;
- AI audit history;
- recovery state.

AetherOS SHALL have both a simple user experience and a powerful observability/debugging plane.

---

# 100. What AetherOS explicitly learns from current mobile OS families

## From Android

Adopt:

- enormous application ecosystem compatibility;
- broad hardware support principles;
- flexible software distribution;
- sophisticated device abstraction;
- mature sandboxing/security concepts;
- strong compatibility discipline.

Avoid inheriting unnecessary architectural complexity by default.

## From iOS

Adopt:

- vertical hardware/software coherence;
- strong performance predictability;
- power efficiency;
- polished interaction;
- deep system integration;
- strong security defaults.

Avoid excessive restrictions on legitimate general-purpose computing.

## From GrapheneOS

Adopt:

- aggressive security hardening;
- least privilege;
- strong sandboxing;
- privacy-first defaults;
- careful compatibility isolation.

## From HarmonyOS

Adopt:

- cross-device computing;
- hardware/software co-design;
- distributed services;
- system-level AI integration.

## From Linux mobile ecosystems

Adopt:

- user ownership;
- transparency;
- Unix-like openness;
- desktop convergence;
- flexible software environments.

Avoid assuming Linux freedom alone solves mobile hardware integration and consumer app compatibility.

---

# 101. Explicit non-goals

AetherOS SHALL NOT initially attempt to:

- support every hardware device without a usable driver/interface;
- reproduce every vendor-specific Android behavior exactly;
- force every platform into one identical UI;
- keep every optional feature permanently installed;
- use AI for deterministic tasks where conventional code is superior;
- make every system component a kernel module;
- make every system component user-space if performance/security requirements contradict it;
- guarantee desktop-level background freedom on mobile;
- guarantee mobile-level power constraints on desktop.

---

# 102. Key architectural risks

## 102.1 Hardware driver availability

The primary external risk is obtaining sufficient hardware documentation, firmware interfaces and legally distributable drivers.

## 102.2 Application compatibility

Android compatibility may be technically achievable but still encounter:

- proprietary services;
- DRM requirements;
- device integrity assumptions;
- vendor-specific APIs;
- banking/security software;
- licensing restrictions.

## 102.3 Over-abstraction

The platform SHALL avoid abstractions that impose unacceptable performance or latency costs.

## 102.4 Under-abstraction

Platform-specific code SHALL not leak into common semantics merely because it is faster to implement initially.

## 102.5 Storage deduplication complexity

Deduplication SHALL not be adopted where it introduces unacceptable CPU, metadata, integrity or security costs.

## 102.6 AI resource growth

AI models can become a major storage/memory/energy consumer. AI resources MUST remain independently managed.

---

# 103. Design decision: physical partitions

### Decision

AetherOS SHALL minimize physical partitions.

### Rationale

Physical partitions are legitimate when required by boot firmware, security, hardware isolation or implementation constraints. They are not intrinsically required for logical software separation.

### Preferred design

Desktop/laptop:

```text
ESP + Aether Storage
```

Mobile:

```text
minimal hardware/secure-boot storage + Aether Storage
```

### Exception

Specific SoC/firmware/reference-platform constraints MAY force extra physical regions. These SHALL be documented as hardware constraints and not elevated into AetherOS's conceptual storage model without justification.

---

# 104. Design decision: update reserve

### Decision

No large permanent update reserve by default.

### Replacement

Dynamic maintenance headroom + transactional snapshots + reclaimable caches + rollback metadata.

### Constraint

The system SHALL prevent updates from reducing free space below safe operational limits.

---

# 105. Design decision: recovery

### Decision

Recovery SHALL be a signed capability/environment, not necessarily a large dedicated partition.

### Preferred mechanisms

- embedded boot-recovery object;
- external USB recovery;
- network recovery where feasible;
- previous system generation.

---

# 106. Design decision: vendor separation

### Decision

AetherOS SHALL separate hardware implementation concerns without recreating Android's exact `/vendor`, `/odm`, `vendor_dlkm`, `odm_dlkm` topology as the normative design.

### Replacement

Versioned hardware profile + driver + firmware packages.

---

# 107. Design decision: mobile vs desktop reconciliation

### Decision

The existing desktop/laptop development is not the definition of the complete platform architecture.

### Required approach

For every subsystem, classify implementation as:

```text
COMMON
DESKTOP-SPECIFIC
LAPTOP-SPECIFIC
MOBILE-SPECIFIC
BUS-SPECIFIC
SOC-SPECIFIC
VENDOR-SPECIFIC
NEEDS REFACTORING BEHIND ABSTRACTION
```

This classification SHALL be revisited whenever architecture changes.

---

# 108. Reconciliation matrix

| Subsystem | Common semantic layer | Desktop/Laptop specialization | Mobile specialization |
|---|---|---|---|
| Scheduler | workload/resource model | throughput/interactive | energy/thermal/latency |
| Memory | VM + lifecycle model | large cache/persistence | aggressive reclaim/freeze |
| Storage | logical domains/snapshots | NVMe/SATA optimized | UFS/flash optimized |
| Boot | signed generation model | UEFI integration | SoC secure boot |
| Graphics | surfaces/compositor | multi-window/multi-display | touch/mobile compositor |
| Input | unified events | keyboard/pointer | touch/gesture/stylus |
| Camera | CameraDevice | webcams/external cameras | ISP/MIPI/multi-camera |
| Audio | audio graph | USB/HDMI/desktop audio | DSP/mobile audio |
| Network | NetworkDevice | Ethernet/Wi-Fi | cellular/Wi-Fi/radio power |
| Sensors | sensor model | limited external | extensive integrated sensors |
| Power | power-state semantics | AC/battery/idle | battery/deep idle |
| Thermal | thermal resource | sustained cooling | strict thermal envelope |
| AI | intelligence API | CPU/GPU/NPU/cloud | NPU/DSP/local-first |
| Applications | runtime/package semantics | native + web + compatibility | native + Android + web |
| UI | semantic toolkit | windows/keyboard | mobile surfaces/gestures |
| External devices | device graph | PCIe/USB/docks | USB-C/Bluetooth/NFC/UWB |

---

# 109. Implementation sequencing

The detailed implementation plan SHOULD proceed approximately as follows, but milestones SHALL be changed when actual repository state and dependencies demonstrate a better order.

## Stage 0 — Architecture reconciliation

- audit existing AetherOS desktop/laptop implementation;
- identify assumptions that leak into common layers;
- establish common/mobile/desktop boundaries;
- define canonical interfaces;
- freeze terminology.

## Stage 1 — Core kernel and platform model

- scheduler;
- memory;
- IPC;
- capability primitives;
- resource model.

## Stage 2 — Hardware/device model

- Device Graph;
- discovery;
- bus abstraction;
- resource manager;
- device lifecycle.

## Stage 3 — Storage

- unified storage pool;
- logical domains;
- snapshots;
- transactional generations;
- package object store;
- integrity/encryption.

## Stage 4 — Boot/update/recovery

- secure boot;
- signed generations;
- rollback;
- recovery;
- update orchestration.

## Stage 5 — Core hardware

- display;
- graphics;
- input;
- storage devices;
- networking;
- audio.

## Stage 6 — External devices

- USB;
- USB-C/PD;
- PCIe;
- docks;
- external displays;
- removable storage.

## Stage 7 — Desktop/laptop shell

- windowing;
- launcher;
- settings;
- file manager;
- diagnostics.

## Stage 8 — Mobile hardware foundation

- SoC integration;
- UFS;
- display/touch;
- camera/ISP;
- modem;
- sensors;
- power/thermal.

## Stage 9 — Native runtime and package ecosystem

- SDK;
- package manager;
- application sandbox;
- native UI framework.

## Stage 10 — Android compatibility

- compatibility environment;
- APK installation;
- lifecycle mapping;
- permission mapping;
- graphics/media integration.

## Stage 11 — Intelligence

- local models;
- AI router;
- memory;
- system command interface;
- agents;
- audit.

## Stage 12 — Cross-device continuity

- shared identity;
- state migration;
- phone/tablet/laptop continuity.

---

# 110. First production-grade reference target

The first fully credible AetherOS release SHOULD target a single reference hardware family rather than attempting universal hardware support immediately.

## Desktop/laptop reference requirements

- modern 64-bit CPU;
- UEFI;
- secure boot capability;
- NVMe;
- modern GPU;
- USB-C/USB4 class connectivity where available;
- Wi-Fi/Bluetooth;
- multiple displays;
- external USB devices;
- reliable sleep/wake;
- hardware health telemetry;
- virtualization support.

## Mobile reference requirements

- ARM64;
- secure boot/root of trust;
- modern SoC;
- GPU;
- NPU;
- ISP;
- UFS;
- high-refresh display;
- touch;
- primary/rear/front cameras;
- cellular modem;
- Wi-Fi/Bluetooth;
- GNSS;
- sensors;
- USB-C;
- battery/charging telemetry;
- secure biometric capability where available.

---

# 111. Acceptance criteria for the architecture

AetherOS architecture SHALL not be considered coherent until all of the following are demonstrably true:

1. The same storage semantics work on NVMe and UFS without duplicate conceptual APIs.
2. The same device model can represent PCIe, USB, MIPI, I²C/I³C and SoC-integrated hardware.
3. Desktop and mobile scheduler policies can differ without forking scheduler semantics.
4. Desktop and mobile application lifecycle policies can differ without duplicating application identity semantics.
5. Native applications can target different shells without rewriting core business logic.
6. Android applications can run through a compatibility layer without becoming the AetherOS native application model.
7. AI operates through explicit capabilities on every platform.
8. System updates do not require a large permanent update partition.
9. Recovery does not inherently require a large permanent recovery partition.
10. System storage is fully attributable and auditable.
11. External hardware can be dynamically discovered and managed.
12. Driver failures can be contained and recovered where hardware permits.
13. Hardware-specific drivers and firmware can update transactionally.
14. Mobile-specific power policy does not contaminate desktop semantics.
15. Desktop-specific flexibility does not create unacceptable mobile power/thermal behavior.

---

# 112. Unresolved design questions

The following are intentionally left open and SHALL require evidence-based design decisions rather than assumptions:

## Kernel foundation

- Linux-derived kernel versus more custom kernel evolution;
- long-term kernel strategy;
- exact IPC implementation;
- exact scheduler design.

## Storage

- exact filesystem implementation;
- whether to build a new Aether filesystem versus adapt an existing mature filesystem;
- exact content-addressing mechanism;
- deduplication policy;
- exact snapshot implementation;
- wear-aware UFS optimization.

## Boot

- exact boot manager implementation;
- secure boot key hierarchy;
- manifest format;
- mobile SoC integration;
- firmware update mechanism.

## Drivers
- exact driver ABI;
- kernel/user-space split per device class;
- vendor SDK architecture;
- long-term compatibility strategy.

## Graphics

- compositor architecture;
- native graphics API design;
- standard graphics compatibility strategy;
- GPU reset model.

## Android compatibility

- exact runtime/container/VM model;
- Android framework compatibility depth;
- graphics integration;
- Play-dependent software strategy;
- security boundary between Android runtime and Aether core.

## AI

- model format;
- local inference runtime;
- NPU abstraction;
- AI memory representation;
- cloud model routing;
- agent protocol.

## UX

- precise launcher design;
- mobile gesture language;
- desktop/mobile convergence rules;
- cross-device continuity model.

## Ecosystem

- app-store model;
- signed external packages;
- vendor-driver distribution;
- certification program.

---

# 113. Terminology

### AetherOS
The complete operating-system platform.

### Aether Kernel
Low-level execution/resource/security substrate.

### AHDA
Aether Hardware & Device Architecture.

### Aether Device Graph
Normalized graph representing physical and virtual devices and relationships.

### Aether Storage
Unified storage architecture spanning logical system/user/application/AI domains.

### Aether System Generation
Immutable/transactionally staged version of the OS selected by the boot manager.

### Aether Hardware Profile
Description of device hardware, drivers, firmware, resources and capabilities.

### Aether Platform Profile
Device-class policy layer such as Desktop, Laptop or Mobile.

### Aether Native Runtime
First-class native application environment.

### Android Compatibility Runtime
Compatibility environment for Android applications.

### Aether Intelligence Layer
Platform-wide AI, memory, reasoning, retrieval and agent services.

### Capability
Explicit permission to access a protected system or hardware operation.

### Resource Contract
Declared expectations/limits for CPU, memory, power, thermal, I/O or other resources.

---

# 114. Canonical architectural summary

The intended AetherOS architecture is:

```text
                               AETHEROS
                                  │
                   ┌──────────────┴──────────────┐
                   │       COMMON SEMANTICS      │
                   │                              │
                   │ Applications                │
                   │ Storage                     │
                   │ Security                    │
                   │ Capabilities                │
                   │ IPC                         │
                   │ Updates                     │
                   │ AI                          │
                   │ Device model                │
                   │ Resource model              │
                   └──────────────┬──────────────┘
                                  │
                       COMMON SYSTEM SERVICES
                                  │
           ┌──────────────────────┼──────────────────────┐
           │                      │                      │
        STORAGE                HARDWARE               SECURITY
           │                      │                      │
           │                Device Graph               │
           │                Bus Managers                │
           │                Driver Framework            │
           │                Resource Manager            │
           │                      │                      │
           └──────────────────────┼──────────────────────┘
                                  │
                           AETHER KERNEL
                                  │
              ┌───────────────────┼───────────────────┐
              │                   │                   │
           DESKTOP              LAPTOP              MOBILE
           PROFILE              PROFILE              PROFILE
              │                   │                   │
        Desktop shell       Hybrid shell        Mobile shell
              │                   │                   │
              └───────────────────┼───────────────────┘
                                  │
                         APPLICATION RUNTIMES
              ┌───────────────────┼───────────────────┐
              │                   │                   │
           Native             Android               Web
           Runtime          Compatibility           Runtime
                                  │
                              APK/AAB
                                  │
                         AETHER INTELLIGENCE
                                  │
              ┌──────────────────┼──────────────────┐
              │                  │                  │
           Perception         Reasoning          Execution
              │                  │                  │
           Local NPU          Models             Capabilities
           /GPU/CPU        Memory/Retrieval      Policies
              │                  │                  │
              └──────────────────┼──────────────────┘
                                  │
                       HARDWARE / FIRMWARE
```

---

# 115. Final architectural position

AetherOS SHALL NOT attempt to win merely by being a smaller Android fork, a prettier Linux distribution, or a desktop OS with a mobile UI.

Its intended distinction is architectural:

> **One coherent operating system that treats hardware, software, storage, applications, resources, security, AI and user experience as parts of one explicit computational model, while adapting execution policy and presentation to the device on which it runs.**

The platform SHALL seek:

- Android-level application reach;
- iOS-level hardware/software coherence;
- hardened security comparable in philosophy to the strongest Android security distributions;
- Linux-class openness and general-purpose computing;
- cross-device capabilities;
- first-class external hardware support;
- transparent and compact resource management;
- system-integrated AI with explicit human control.

The strongest architectural differentiator SHALL be **coherence with accountability**:

```text
Every component has a purpose.
Every resource has an owner.
Every capability has a policy.
Every update has a generation.
Every driver has a lifecycle.
Every hardware device has a graph representation.
Every AI action has explicit authority.
Every platform difference has a declared reason.
Every storage byte is accountable.
```

That is the intended foundation for the AetherOS family across desktop, laptop and mobile computing devices.

---

# Appendix A — Platform reconciliation checklist

For every subsystem already implemented or newly changed, evaluate:

```text
[ ] Common semantic contract identified
[ ] Desktop-specific assumptions identified
[ ] Laptop-specific assumptions identified
[ ] Mobile-specific assumptions identified
[ ] Bus-specific assumptions isolated
[ ] SoC-specific assumptions isolated
[ ] Vendor-specific assumptions isolated
[ ] Resource model compatible across profiles
[ ] Security model compatible across profiles
[ ] Update/recovery behavior compatible across profiles
[ ] Storage semantics compatible across profiles
[ ] AI access compatible across profiles
[ ] UX semantic model remains shared
[ ] Tests exist for each relevant platform profile
```

---

# Appendix B — System footprint checklist

```text
[ ] Core OS size measured
[ ] System services size measured
[ ] Hardware package size measured
[ ] Driver size measured
[ ] Native runtime measured
[ ] Android runtime measured
[ ] AI runtime measured
[ ] AI model storage measured
[ ] Recovery measured
[ ] Snapshot/update workspace measured
[ ] Temporary/cache storage measured
[ ] No unexplained content
[ ] No accidental duplicate resources
[ ] Unsupported hardware omitted from target build
[ ] Optional components are actually optional
```

---

# Appendix C — Hardware support checklist

```text
[ ] Hardware discovered
[ ] Device Graph entry created
[ ] Driver selected
[ ] Firmware verified
[ ] Resources allocated
[ ] IOMMU/DMA policy applied
[ ] Power states implemented
[ ] Thermal behavior implemented
[ ] Health telemetry implemented
[ ] Hotplug behavior implemented where applicable
[ ] Driver recovery implemented where applicable
[ ] Package/rollback path implemented
[ ] Standard API exposed
[ ] Vendor extension isolated
[ ] CI/simulation test coverage exists
```

---

# Appendix D — AI safety/execution checklist

```text
[ ] Intent identified
[ ] Context source identified
[ ] Required capabilities identified
[ ] Data locality evaluated
[ ] Model route selected
[ ] Policy evaluated
[ ] User approval required where appropriate
[ ] Action executed through typed API
[ ] Result verified
[ ] Audit record written
[ ] State/memory updated
```

---

# Appendix E — Architectural anti-patterns

AetherOS design/review SHALL flag these patterns:

1. Creating a physical partition solely because a logical separation is convenient.
2. Copying desktop assumptions directly into mobile code.
3. Copying mobile lifecycle assumptions directly into desktop code.
4. Creating separate APIs for identical semantic operations simply because buses differ.
5. Letting vendor extensions redefine the common API.
6. Giving AI unrestricted system access.
7. Using AI where deterministic services are simpler and more reliable.
8. Hiding large storage consumers under a generic “System” category.
9. Shipping optional hardware/software components in builds that cannot use them.
10. Installing drivers without transactional verification/rollback.
11. Treating driver crashes as inherently unrecoverable.
12. Freezing interfaces without versioning.
13. Optimizing only for benchmark speed while ignoring energy, thermals and latency.
14. Building mobile as “desktop with a smaller screen.”
15. Building desktop as “mobile with more windows.”

---

# Appendix G — Intended evolution

The architecture is intentionally designed to permit future expansion into:

- tablets;
- foldables;
- XR/spatial devices;
- advanced local AI computers;
- automotive computing;
- other specialized devices.

Such additions SHALL be accepted only when they can reuse the common semantic model without excessive conditional complexity.

---

# Appendix F — Architecture convergence decision record

This appendix records the current architectural decisions that supersede earlier exploratory formulations. It is intended to prevent the specification from reintroducing previously debated alternatives without an explicit new decision.

| Area | Current baseline | Status |
|---|---|---|
| Human interaction abstraction | Context-centric | Adopted as normative direction |
| Application role | Technical runtime/deployment/security entity | Adopted |
| UI representation | Visual + Semantic + Input Graphs | Adopted |
| Semantic fallback | Structured semantics first; authorized visual interpretation second | Adopted |
| UI launcher | Context Router + Intent Gateway; app grid optional | Adopted as target architecture |
| UI ownership | Context owner controls Context-level organization/policy but cannot bypass app/system security | Adopted |
| Android compatibility | Isolated compatibility environment; VM preferred when useful | Adopted, implementation subject to measurement |
| Android compatibility percentage | No fixed promise | Normative |
| Mobile update redundancy | Minimum required redundancy; hybrid/logical/physical mechanisms benchmarked | Adopted |
| Mobile COW | Not prohibited; must be measured for flash effects | Adopted |
| Driver architecture | Layered kernel + userspace model | Adopted |
| Driver language | Memory-safe implementation preferred for new security-sensitive work | Adopted |
| Hardware discovery | Minimal kernel bootstrap + normalized Device Graph + userspace policy | Adopted |
| Physical partitions | Minimum required by actual platform invariants | Adopted |
| Platform model | Shared semantics + platform-specific policies/implementations/shells | Adopted |

## F.1 Decisions that remain deliberately open

The following SHALL remain subject to prototype/measurement rather than being prematurely frozen:

- exact kernel architecture and degree of Linux reuse;
- exact filesystem implementation for each storage profile;
- exact mobile update generation/storage mechanism;
- exact Android VM/container backend;
- exact compositor protocol/implementation;
- exact UI scene/semantic graph wire format;
- exact driver/userspace split by device class;
- exact AI model-memory strategy;
- exact reference hardware architecture.

An implementation choice SHALL be promoted to normative specification only after its required properties, trade-offs, and cross-system consequences have been demonstrated.

---

# 116. Master Architecture v1.0 — Extended Convergence Baseline

This section records the full architecture convergence established after the original 0.2 baseline. It is an additive canonical layer. Earlier material remains valuable as design history and detailed subsystem background; this section provides the current cross-system authority when terminology or mechanisms were refined during later debate.

## 116.1 Constitutional semantic separations

AetherOS SHALL preserve five foundational semantic separations:

### UI / HCI

```text
Semantic Meaning
      ≠
Visual Presentation
      ≠
Input Routing
```

Semantic state, visual state, and input routing MAY be correlated and updated transactionally, but SHALL remain conceptually distinct. The Semantic Graph SHALL be the primary machine-readable representation for accessibility, automation, AI, search and system interaction when semantic information is available.

### Storage

```text
Logical Identity
      ≠
Physical Layout
      ≠
Compatibility View
```

Logical object identity SHALL remain independent of the physical CAS/segment layout and any POSIX or other filesystem projection.

### Security

```text
Principal Identity
      ≠
Privilege Authority
```

An identity does not itself confer authority. Authority SHALL emerge from explicit capabilities and applicable policy.

### Resources

```text
Context Meaning
      ≠
Execution Priority
      ≠
Physical Allocation
```

A Context describes human activity and organization. Activities, Providers and system workloads carry resource contracts and leases. Schedulers and device arbiters map those contracts onto hardware.

### Hardware

```text
Device Semantics
      ≠
Physical Resources
      ≠
Driver Implementation
```

An Aether device class SHALL not be defined by a particular firmware table, bus, driver, or vendor stack.

## 116.2 Context, Provider and security-domain clarification

A Context SHALL be owned by a User Principal and identified by a durable ContextIdentity. ApplicationIdentity and ProviderInstanceIdentity SHALL remain distinct. Authentication and security-sensitive resource attribution SHALL bind to the ProviderInstanceIdentity or other explicit subject identity appropriate to the operation.

A Provider is an application/runtime participant in a Context. Context membership SHALL NOT grant system privilege and SHALL NOT collapse Provider security boundaries.

A single process MAY host multiple Providers only when those Providers share an intentional trust/security domain or the execution environment provides enforceable intra-process isolation sufficient for the required boundary. ProviderInstanceIdentity alone SHALL NOT turn one ordinary address space into multiple mutually isolated security domains.

Provider-owned durable data remains Provider/application-owned unless an explicit Context-state contract transfers ownership. Context-owned state, Provider-owned state, application persistent data, cached presentation state, and AI-associated state SHALL remain distinguishable.

## 116.3 Capability and authority model

Effective authority SHALL be attenuating across applicable policy layers. The generalized authority model is:

```text
Effective Authority
=
System Policy
≠ User Policy
≠ Context Policy
≠ Application Policy
≠ Provider Policy
≠ Activity Policy
≠ Device Policy
≠ Resource/Data Policy
```

No higher-level Context grant SHALL manufacture authority absent from the underlying Application, Provider, Activity, Device or Resource/Data policy.

A Context SHALL not be a super-principal.

Cross-Context communication SHALL occur only through explicitly authorized transfer or IPC mechanisms. Context boundaries SHALL therefore be treated as security boundaries for semantic organization, while capabilities determine actual authority.

## 116.4 Cryptographic Context state envelope

Context state SHALL be stored using independently attributable encryption domains.

A conceptual model is:

```text
KMS / TEE
   │
   ├── Context Key Policy
   │
   ├── Provider State DEK wrapper A
   ├── Provider State DEK wrapper B
   └── other independently owned state
```

The Context Root Key, when one exists, SHALL remain inside an authorized KMS/TEE boundary and SHALL not be delivered to ordinary Providers. Providers SHALL request OS-mediated access to their authorized state slices.

Cryptographic erasure SHALL be defined as destruction/revocation of AetherOS-controlled encryption keys for the affected encrypted domains. It SHALL NOT be represented as an absolute guarantee against copies outside AetherOS control, including external backups, cloud replicas, removable media or unrelated plaintext remnants.

Key lifecycle SHALL account for creation, rotation, backup, migration, recovery, re-wrapping, revocation and deletion.

## 116.5 Semantic projections and privileged readers

Accessibility, automation, search, diagnostics and AI MAY receive derived semantic projections.

No privileged projection is inherently omniscient. A projection SHALL be generated from:

- requesting principal;
- application/provider metadata;
- data classification;
- capability policy;
- user policy;
- system policy;
- Context scope;
- resource/purpose constraints.

`AI_EXCLUDE` SHALL be treated as policy input to projection generation rather than an unconditional unilateral bypass of system policy by a Provider.

`cross_context.read` SHALL be scoped at minimum by source Context, target/requesting Context, resource/object scope, purpose, authority, and duration or lifetime.

Cross-Context data movement SHALL be mediated by an explicit **Intent/Transfer Broker** or semantically equivalent authority. The compositor, Visual Graph, or Input Graph SHALL not silently become a privileged data-transfer mechanism merely because a drag-and-drop or other UI gesture occurs. A transfer SHALL carry an explicit source, destination, object/resource, purpose, authorization and applicable data-classification policy.

Opaque or protected content SHALL not be semantically fabricated. Where semantic coverage is `OPAQUE`, AI and automation SHALL not invent structured meaning merely from unavailable information.

## 116.5a Context isolation modes and side-channel policy

AetherOS SHALL distinguish semantic/OS-resource isolation from optional hardware-level side-channel minimization.

### Standard Isolation

Standard Context isolation SHALL protect against ordinary OS-level cross-Context disclosure through mechanisms including:

- clipboard and data-transfer policy;
- notifications and previews;
- shared semantic projections;
- memory and object access controls;
- IPC capabilities;
- persistent-state ownership;
- device/resource authority.

Standard Isolation MAY share CPU cores, caches, memory hierarchy and other microarchitectural resources for performance and efficiency. AetherOS SHALL not claim universal immunity to microarchitectural side channels under Standard Isolation.

### Strict Isolation

Certified platforms MAY expose an optional **Strict Isolation** mode for workloads with unusually strong side-channel requirements. Where hardware supports it, Strict Isolation MAY employ dedicated physical cores, disabled SMT, cache-management/flush measures, restricted co-scheduling, dedicated memory/accelerator resources, or other hardware mechanisms.

Strict Isolation SHALL be an explicit resource/security mode rather than a normal Context requirement. Its guarantees SHALL be stated only to the extent actually supported by the platform hardware.

A Context requesting Strict Isolation SHALL receive an explicit admission decision. Resource cost, reduced capacity and thermal/power implications SHALL be visible to the Resource Governance system.

## 116.6 Canonical storage architecture

AetherOS SHALL use a hybrid semantic storage model:

```text
Aether Storage Pool
      │
      └── Aether Storage Engine
             ├── Immutable CAS Store
             ├── Mutable Transactional Segment Store
             ├── Transaction/Commit Manager
             ├── Object/Version Indexes
             └── Compatibility Views
                    └── POSIX VFS and other projections
```

### Immutable content

System binaries, libraries, firmware, signed package content, static resources and historical generation content SHOULD be represented as immutable content-addressed objects where appropriate.

CAS SHALL provide logical content identity/sharing and deduplication opportunities. Deduplication SHALL remain optional where CPU, metadata, integrity, security or endurance costs outweigh the benefit.

### Mutable state

Context metadata, Provider state, user documents, configuration and other mutable objects SHALL use stable logical identifiers plus transactional versions. Append-oriented segments, MVCC, LSM-like indexing and compaction MAY be used; exact mechanisms are implementation decisions subject to evidence.

A complete Context SHALL not be rewritten for every mutation. Mutable changes SHOULD update only the affected logical object/version records and root references.

### Ephemeral state

Caches, temporary data, GPU intermediates, AI scratch data and similar non-authoritative state SHALL remain separately identifiable and reclaimable.

### Logical files

Files SHALL remain first-class user concepts. A file MAY be represented internally as one or more typed objects with stable logical identifiers and namespace bindings. POSIX SHALL therefore be a compatibility/projection interface rather than the sole semantic authority for persistent storage.

## 116.7 Generation sharing and garbage collection

System generations SHOULD share unchanged immutable objects. A generation switch SHOULD therefore change authoritative roots/pointers rather than duplicate unchanged content.

Reachability and reclamation SHALL account for roots from:

- active system generation;
- rollback/known-good generations;
- installed packages;
- live Contexts;
- snapshots;
- backups;
- recovery roots;
- user-pinned content.

The storage system SHALL never reclaim an immutable object still reachable from an authoritative root.

## 116.8 Resource governance

AetherOS Resource Governance SHALL contain five conceptual layers:

1. **Resource Semantics** — common definitions of resources, contracts, leases, criticality, pressure, thermal capacity and budgets.
2. **Policy and Admission** — user-space Resource Governor / Context-Aware Resource Broker.
3. **Kernel Enforcement** — scheduler, virtual memory, cgroup-like control, reclaim and low-level resource enforcement.
4. **Accelerator/Device Arbitration** — GPU/NPU/DSP/display/audio/storage/network-specific scheduling and admission.
5. **Emergency Safety Plane** — kernel/hardware fallbacks independent of the normal broker.

The Resource Broker SHALL not be a required synchronous dependency of latency-critical input or presentation paths.

## 116.9 Resource Contracts

A Resource Contract SHOULD express:

```text
subject
context
provider
activity
criticality
latency target
jitter tolerance
deadline
preemptibility
checkpointability
CPU requirement
memory requirement
GPU/NPU/DSP requirement
bandwidth requirement
I/O requirement
network requirement
power sensitivity
thermal sensitivity
reclaimability
lifetime
```

Contracts MAY be requested by applications, Providers, Activities, compatibility runtimes or system workloads. AetherOS SHALL validate, attenuate, admit and enforce them rather than allowing workloads to self-promote unconditionally.

## 116.10 Resource Leases

Transient latency-sensitive obligations SHALL be represented as explicit Resource Leases.

Examples include:

- realtime audio lease;
- media pipeline lease;
- display frame deadline lease;
- latency-sensitive network lease;
- hardware sensor acquisition lease.

A Context marked `BACKGROUND` MAY contain an Activity holding an active realtime lease. Context lifecycle therefore SHALL NOT cancel such execution merely because presentation priority changed.

Hard temporal guarantees SHALL be admitted only when the system can provide a schedulable execution budget and appropriate resource capacity. A requested latency target SHALL not be represented as an unconditional promise when the necessary capacity cannot be admitted.

## 116.11 CPU scheduling policy

AetherOS SHALL specify workload properties rather than core-pinning rules.

`ACTIVE` workloads SHOULD receive interactive scheduling treatment appropriate to the target platform, but AetherOS SHALL NOT universally define “active” as pinning to a particular CPU cluster or frequency state.

The implementation MAY use mechanisms such as EEVDF, real-time scheduling, deadline scheduling, capacity-aware scheduling, utilization clamps, scheduler extensions or platform-specific vendor mechanisms while preserving the common Aether contract model.

## 116.12 Memory pressure ladder

The canonical memory-pressure ladder is:

```text
P0 NORMAL
  ↓
P1 PRESSURE
  ↓
P2 CONSTRAIN
  ↓
P3 SUSPEND
  ↓
P4 EVICT
  ↓
P5 EMERGENCY
```

### P1 PRESSURE

Reclaim cold pages, reduce cache growth, compress memory, reduce speculative work and suppress unnecessary wakeups.

### P2 CONSTRAIN

Throttle over-budget domains, shrink background working sets, reduce accelerator residency and enforce resource limits.

### P3 SUSPEND

Select suspendable workloads, request cooperative quiescence, checkpoint required state and release execution resources.

### P4 EVICT

Terminate or release the lowest-criticality evictable Providers while preserving Context identity and durable state where possible.

### P5 EMERGENCY

Use emergency reserves, mandatory containment and kernel/hardware recovery mechanisms. Kernel OOM MAY occur as a final safety mechanism but SHALL NOT be treated as ordinary Context lifecycle management.

No normal lifecycle path SHALL depend on a Provider successfully responding to a checkpoint request forever. Forced suspension/termination mechanisms MAY exist with explicitly weaker state-preservation guarantees.

## 116.13 Hardware and Device Architecture — two hardware truths

AHDA SHALL maintain two complementary authoritative models:

### Kernel Hardware Resource Model

Authoritative for:

- physical memory mappings;
- MMIO ownership;
- interrupt routing;
- DMA domains;
- IOMMU mappings;
- reset authority;
- low-level power/clock access;
- trusted hardware resource ownership.

### Aether Device Graph

Authoritative for the semantic model of:

- device identity;
- topology;
- relationships;
- capabilities;
- resource requirements;
- power/performance states;
- firmware association;
- driver association;
- health;
- lifecycle;
- policy metadata.

The Device Manager SHALL construct and reconcile the semantic Device Graph from firmware descriptions, enumeration, driver information and runtime discovery. It SHALL NOT unilaterally establish physical authority.

The kernel SHALL perform the minimum safe bootstrap discovery required for execution. Firmware parsing and rich policy SHOULD remain above the kernel where practical, consistent with the existing discovery boundary.

## 116.14 Discovery authority model

The canonical discovery flow is:

```text
Firmware / enumeration sources
        ↓
Aether Device Manager
        ↓
Global Aether Device Graph
        ↓
Driver/service binding requests
        ↓
Kernel physical-resource validation/enforcement
```

Firmware SHALL be treated as configuration/discovery evidence, not unconditional physical authority.

The Device Manager MAY request:

```text
bind device → driver/service
```

but the kernel SHALL independently validate addresses, interrupts, IOMMU mappings, ownership and other physical security constraints before granting access.

## 116.15 Zero-copy and shared-buffer architecture

AetherOS SHALL provide capability-mediated zero-copy or shared-buffer semantics wherever compatible hardware permits them.

The canonical semantic objects are:

```text
BufferIdentity
BufferCapability
BufferAttachment
BufferMapping
BufferLease
BufferFence
BufferPool
```

A buffer SHALL NOT require physical contiguity unless a concrete hardware constraint requires it. The backing MAY be contiguous, scatter-gather, device-local, protected, reserved, or otherwise implementation-defined.

A Buffer Capability SHALL encode authority through an opaque kernel-managed handle rather than by treating the handle value itself as a cryptographic proof.

Rights MAY include:

```text
READ
WRITE
MAP_CPU
MAP_DEVICE
IMPORT
EXPORT
SHARE
REVOKE
```

Sharing SHALL be attenuating. A consumer SHALL receive only the rights explicitly granted to it.

High-frequency multimedia paths SHOULD use persistent Buffer Pools and amortized device mappings rather than remapping every frame.

## 116.16 Buffer synchronization

Zero-copy SHALL be accompanied by explicit or kernel-managed synchronization semantics sufficient to prevent races and stale data.

The semantic model SHOULD support:

```text
Acquire Fence
Producer Completion
Consumer Fence(s)
Release Fence
```

Frame metadata, buffer capabilities and synchronization handles SHALL be transferable through IPC without copying the multimedia payload.

## 116.17 Driver architecture convergence

AetherOS SHALL retain the layered driver model:

```text
Layer 0 — Firmware / device firmware
Layer 1 — Kernel hardware substrate
Layer 2 — Kernel driver where required
Layer 3 — Userspace device service where appropriate
Layer 4 — Aether device/system API
```

This is a placement model, not a fixed implementation requirement for every device. A device class MAY be kernel-heavy, user-space-heavy, split, virtualized or compatibility-backed according to latency, throughput, DMA, security, restartability, maintainability and hardware evidence.

Driver failures SHOULD be isolated and recovered where hardware permits.

## 116.18 Vendor Compatibility Domains

A proprietary vendor stack MAY execute inside a **Vendor Compatibility Domain (VCD)** when native Aether integration is unavailable or economically impractical.

A VCD SHALL be a subordinate compatibility environment, not an alternate source of Aether semantic authority.

Conceptually:

```text
Aether Device API
       ↓
Aether Vendor Adapter
       ↓
Vendor Compatibility Domain
       ↓
Vendor HAL/runtime/protocol stack
       ↓
Vendor kernel/firmware interface
```

A VCD MAY contain the runtime and protocols required by the specific vendor stack, including Android-derived interfaces such as Binder/AIDL/HIDL or a compatible libc/runtime where necessary. Those mechanisms SHALL remain implementation details rather than normative Aether semantics.

The VCD SHALL receive only explicitly authorized hardware, DMA, filesystem, IPC and resource capabilities.

A vendor HAL SHALL not be treated as an arbitrary RPC interceptor. The Aether Vendor Adapter SHALL implement an explicit translation boundary between the vendor interface and canonical Aether device semantics.

## 116.19 Application identity and execution-domain separation

AetherOS SHALL distinguish:

```text
ApplicationIdentity
ProviderInstanceIdentity
ExecutionDomain
Security Domain
ContextIdentity
```

A shared process is not inherently a shared Context, and a shared Context is not inherently a shared process.

The OS SHALL be able to host multiple Providers from one application when their required boundaries permit it, while allowing stronger execution-domain separation where required by security, trust or compatibility requirements.

## 116.20 Native application architecture

Native applications SHALL target high-level Aether semantic APIs through an SDK that provides:

- capability declarations;
- static capability analysis where practical;
- runtime capability enforcement;
- Context and Provider APIs;
- typed storage transactions;
- device APIs;
- resource contracts;
- UI semantic/visual/input APIs;
- explicit IPC.

The SDK SHALL not imply that compile-time analysis alone determines runtime authority.

## 116.21 Web Runtime

The Web Runtime SHALL provide controlled capability-mapped integration for PWAs and other web applications.

Web APIs such as graphics, USB, sensors or geolocation SHALL terminate at Aether capability checks rather than acquiring ambient authority through the browser/runtime.

## 116.22 Android Compatibility Environment

Android SHALL remain a compatibility environment rather than the native Aether semantic model.

Where hardware permits, a protected MicroVM or equivalent isolated environment is preferred. A lighter compatibility implementation MAY exist where it demonstrably provides sufficient fidelity and isolation.

The implementation SHALL be chosen using measured compatibility, latency, memory, energy, graphics, security and hardware-integration results.

## 116.23 Android host/guest boundary

The Aether host owns the MicroVM resource envelope:

```text
CPU
RAM
GPU
NPU/DSP
I/O
network
power
thermal capacity
```

Android retains its internal lifecycle and memory-management semantics inside that envelope.

A host suspension or termination of the Android MicroVM SHALL be treated as a VM lifecycle transition and SHALL NOT be disguised as a guaranteed Android lifecycle callback such as `onStop()` or `onDestroy()`.

The host SHALL provide explicit coordination through an AndroidGuestResourceContract or semantically equivalent interface.

## 116.24 Android hardware mediation

The Android environment SHOULD see controlled virtual or bridged device interfaces for graphics, audio, camera, sensors, storage, network, input, display and power state.

Direct hardware passthrough SHALL remain exceptional, explicit, capability-mediated and auditable.

Compatibility environments SHALL not bypass the Aether capability model.

## 116.25 Package transaction semantics

Package installation and updates SHALL follow:

```text
identify
  ↓
verify provenance/signature
  ↓
resolve dependencies
  ↓
validate capabilities/resources
  ↓
stage
  ↓
prepare/migrate where required
  ↓
activate
  ↓
health-check
  ↓
commit
```

CAS-backed package content MAY be shared across generations and packages without implying that every package update is semantically equivalent to simply changing one pointer. Migrations, persistent state, permissions, dependencies and runtime compatibility SHALL be accounted for.

## 116.26 Generation Health Manager

A Generation Health Manager SHALL distinguish component failure from generation failure.

A compositor crash, Provider crash, isolated driver failure or restartable service failure SHALL not automatically mark the whole system generation defective.

Generation health SHOULD evaluate:

- boot success;
- kernel stability;
- storage integrity;
- core service availability;
- security state;
- device initialization;
- display/input availability;
- networking where required;
- power/thermal stability;
- watchdog or recovery events;
- critical service health.

A newly installed generation SHALL become `KNOWN_GOOD` only after completing the required health-commit protocol.

## 116.27 Generation lifecycle

The canonical generation state machine is:

```text
STAGED
   ↓
CANDIDATE
   ↓
BOOTING
   ↓
TRIAL
   ↓
HEALTHY
   ↓
COMMITTED
   ↓
KNOWN GOOD
```

Failure paths MAY lead to:

```text
BOOT FAILURE
RUNTIME FAILURE
QUARANTINED
REJECTED
RECOVERY
```

Generation health state SHALL not itself constitute cryptographic authorization to execute a generation.

## 116.28 Boot trust architecture

AetherOS SHALL define a platform-independent trust abstraction over platform-specific mechanisms.
Conceptually:

```text
Immutable Hardware Root of Trust
              ↓
      Platform Trust Policy
              ↓
      Owner / Vendor Trust Authorities
              ↓
        Aether Boot Adapter
              ↓
        Aether Boot Manager
              ↓
   Aether System Generation Descriptor
              ↓
        Kernel Bootstrap
              ↓
       Aether System Services
```

The immutable hardware root remains the ultimate anchor. An owner key SHALL not replace the hardware root; it SHALL operate through an authenticated owner trust policy.

## 116.29 Certified and Compatible Aether Hardware

### Certified Aether Hardware

Certified hardware SHOULD provide:

- hardware-rooted verified boot;
- measured boot;
- owner-controlled trust enrollment;
- secure trust-policy changes;
- rollback/monotonic security state;
- protected key binding to boot state where supported;
- trusted recovery.

### Compatible Aether Hardware

Compatible hardware MAY omit owner-controlled trust enrollment or other advanced capabilities.

A Compatible platform SHALL expose its reduced trust properties honestly and SHALL not claim the same ownership/security guarantees as Certified Aether Hardware.

## 116.30 Owner trust and User Owner Key

A **User Owner Key (UOK)** MAY be used on Certified Aether Hardware as an owner-controlled signing authority.

The canonical hierarchy is:

```text
Hardware Root of Trust
        ↓
Aether Ownership Trust Policy
        ↓
Accepted signing authorities
        ├── Owner Key(s)
        ├── Vendor Key(s), when explicitly trusted
        └── Recovery/Maintenance Key(s), when explicitly trusted
```

Owner enrollment SHALL require authenticated user intent and, where available, physical presence.

Owner-controlled trust SHALL not inherently disable measured boot, verified boot, or hardware-backed key policy.

AetherOS SHALL not promise that owner-installed software retains guarantees beyond the actual certified hardware capabilities.

## 116.31 Generation Descriptor

The canonical authenticated boot object SHALL be an **Aether System Generation Descriptor (ASGD)** or semantically equivalent structure.

It SHALL identify at least:

- format version;
- generation identity;
- target platform/hardware profile;
- required boot environment;
- security version information;
- boot-critical artifacts;
- system/object roots;
- relevant security-policy identity;
- measurement policy/requirements where applicable;
- recovery metadata;
- signatures and trust-authority information.

The ASGD MAY reference Merkle roots or equivalent authenticated content roots. It SHALL not be required to contain the entire storage object graph itself.

## 116.32 Acyclic bootstrapping

Boot-critical verification SHALL not require initialization of the full Aether Storage Engine.

AetherOS MAY maintain a small boot metadata area or equivalent authenticated metadata channel containing generation descriptors, boot state and recovery metadata.

The canonical trust flow is:

```text
Hardware Root
   ↓
Boot Adapter
   ↓
ASGD verification
   ↓
Boot-critical object verification
   ↓
Kernel bootstrap
   ↓
Full Storage Engine
   ↓
Progressive verification of remaining immutable objects
```

This prevents a circular dependency between boot trust and the complete storage implementation.

## 116.33 Verified boot versus measured boot

AetherOS SHALL distinguish:

```text
Verified Boot = authorization for execution
Measured Boot = record of what executed
```

Where supported, the boot chain SHALL produce authenticated measurements that can be consumed by KMS/security policy and attestation mechanisms.

## 116.34 Anti-rollback security state

Anti-rollback SHALL use hardware-backed monotonic state where supported.

AetherOS SHALL prefer a **security version vector** or equivalent rollback-domain model over assuming one universal generation counter.

Conceptually:

```text
Boot Security Version
TEE/Security Version
Kernel Security Version
System Security Version
Firmware Security Version
Compatibility Security Version
```

A generation SHALL not execute in Secure Owner Mode when its applicable security version is below the hardware-backed security floor.

## 116.35 Exceptional downgrade

UOK SHALL not, merely by existing, bypass the hardware-backed security floor.

An explicitly authorized development or recovery transition MAY permit an exceptional downgrade only by entering a distinguishable maintenance/development/recovery security state and applying appropriate key invalidation, rekeying, data-protection and attestation semantics.

## 116.36 Boot selection

Normal boot SHALL be automated and policy-driven.

Manual generation selection SHALL be available through authenticated recovery/maintenance facilities rather than a routine boot menu.

The selector SHALL filter candidates by:

- signature/trust policy;
- hardware/profile compatibility;
- security version floor;
- generation state;
- recovery policy;
- required resources.

## 116.37 Recovery behavior

When a candidate generation fails before becoming known-good:

```text
Candidate failure
      ↓
select previous eligible known-good generation
      ↓
boot/recover
```

If no eligible generation remains, the system SHALL enter a trusted recovery path where platform capability permits it.

Recovery MAY be implemented as a standalone minimal boot object, embedded diagnostic generation, external recovery source or network recovery according to hardware constraints.

## 116.38 Power, thermal and energy governance

Power and thermal resources SHALL remain shared and first-class.

The Resource Governor SHALL coordinate CPU, GPU, NPU, DSP, ISP, modem, display, storage, network, memory bandwidth and other coupled resource demands.

Thermal conditions SHOULD be modeled separately from response policy:

```text
Thermal Condition
    ↓
Resource Admission
    ↓
Workload Budgets
    ↓
Scheduler / Device Enforcement
```

Conceptual thermal states MAY include:

```text
NORMAL
ELEVATED
SEVERE
CRITICAL
```

As thermal severity rises, the system SHOULD reduce or relocate lower-criticality work before compromising critical functionality.

Energy accounting SHALL remain multidimensional. “Energy tokens” MAY be used as an implementation/accounting technique but SHALL not be the sole semantic resource primitive because CPU, modem, display, storage, accelerator and wakeup costs are not directly fungible.

## 116.39 Observability and accountability

Every major resource and privileged action SHALL be attributable sufficiently for engineering analysis without silently collecting protected user content.

Required observability domains include, where relevant:

- CPU scheduling;
- GPU scheduling;
- memory pressure;
- accelerator queues;
- I/O latency;
- network activity;
- wakeups;
- thermals;
- battery/energy consumption;
- Provider/process lifecycle;
- driver events;
- device resets;
- frame timing;
- package/update operations;
- boot and generation transitions.

## 116.40 Application ecosystem and compatibility separation

AetherOS SHALL provide at least:

```text
Native Aether Runtime
Web Runtime
Android Compatibility Environment
```

Future compatibility runtimes MAY be added without redefining native semantics.

Native applications SHOULD share application logic across device shells while presentation adapts through the platform profile.

Compatibility environments SHALL remain subordinate to host security, resource and device authority.

## 116.41 Build and target composition

The build system SHALL support declarative target-specific composition.

A target build SHOULD include only components relevant to the selected platform and hardware profile.

Examples:

```text
aether build --platform desktop --hardware <profile>
aether build --platform laptop  --hardware <profile>
aether build --platform mobile  --hardware <profile>
```

Target composition SHALL remain auditable and explainable.

## 116.42 Footprint governance

The existing illustrative Core OS target of approximately 3–5 GB on a flagship/reference installation is retained as an engineering target rather than a hard invariant. The current specification explicitly requires these targets to be validated by actual measurements and revised when evidence warrants. 

Every release SHOULD report:

- compressed system size;
- installed system size;
- runtime size;
- hardware package size;
- AI runtime/model size;
- recovery footprint;
- update workspace requirement;
- peak/idle RAM;
- boot time;
- cold launch time;
- Context resume time;
- background CPU/wakeup activity;
- energy consumption.

Unexplained regressions SHALL be investigated before release.

## 116.43 CI and fault-injection architecture

CI SHALL test semantic boundaries through a combination of:

- static/dependency checks;
- API/ABI conformance;
- capability tests;
- security tests;
- integration tests;
- runtime tracing;
- simulation;
- hardware testing;
- fault injection;
- performance tests;
- power/thermal tests;
- compatibility tests;
- formal verification where applicable.

CI SHALL not rely on source scanning alone to prove semantic separation.

Fault injection SHOULD cover:

- memory pressure;
- forced suspension;
- device timeout;
- driver failure;
- DMA/IOMMU fault;
- storage corruption;
- power loss during commit;
- thermal excursion;
- low battery;
- network loss;
- generation boot failure;
- AI latency spike;
- compatibility-runtime failure.

## 116.44 Release gates

A release SHALL be blocked by unresolved:

- constitutional invariant violations;
- security-boundary regressions;
- unacceptable latency regressions;
- unexplained footprint growth;
- broken rollback/recovery semantics;
- loss of required resource containment;
- unexplained compatibility regressions against declared contracts;
- critical fault-containment failures.

Single-run noise SHALL not automatically constitute a regression; release thresholds SHALL use statistically meaningful measurements appropriate to the target.

## 116.45 Architecture governance

The AetherOS architecture SHALL use the following change classes:

### Class A — Constitutional

Semantic separations, ownership, security boundaries, lifecycle rules or foundational authority changes.

Requires an ADR, compatibility analysis, security analysis, affected-subsystem analysis and explicit specification-version impact.

### Class B — Normative Interface

API, ABI, capability, resource, storage, device or runtime contract changes.

Requires versioning, compatibility analysis, migration/deprecation planning and conformance tests.

### Class C — Implementation

Changes preserving normative contracts.

These MAY vary freely subject to conformance, security, reliability and performance requirements.

### Class D — Profile-Specific

Changes isolated to Desktop, Laptop, Mobile, Tablet, Foldable or another hardware profile.

Profile-specific changes SHALL not redefine common semantics.

## 116.46 Architecture Decision Records

Every decision that promotes an implementation mechanism from open design space to a normative architectural requirement SHALL have an ADR containing:

- problem;
- alternatives considered;
- constraints;
- decision;
- rationale;
- evidence;
- rejected alternatives;
- affected interfaces;
- affected profiles;
- security implications;
- performance implications;
- migration implications;
- reversal conditions.

## 116.47 Specification authority hierarchy

```text
AetherOS Master Architecture Specification
        ↓
Normative subsystem specifications
        ↓
Platform / Hardware Profile specifications
        ↓
Architecture Decision Records
        ↓
API / ABI / Capability / Resource contracts
        ↓
Implementation documentation
        ↓
Source code
```

A lower-level artifact SHALL not silently redefine a higher-level architectural contract.

## 116.48 Subsystem completeness

A subsystem SHALL not be considered architecturally complete merely because an implementation works.

Completion requires a coherent set of:

```text
semantic contract
security model
resource model
failure model
compatibility model
observability
verification/tests
reference implementation evidence
```

The architecture SHALL favor:

> **coherence before convenience, evidence before assumption, isolation before implicit authority, and semantic stability before implementation preference.**

---

# 117. Canonical Application Ecosystem and Runtime Baseline

This section consolidates the v1.0 application/runtime decisions without replacing the more detailed earlier application, Android and package sections.

## 117.1 Native Aether applications

Native applications are first-class Aether runtime entities and SHALL integrate with Contexts through Providers, typed capabilities, semantic UI state, storage transactions and Resource Contracts.

The native runtime SHALL remain independent of POSIX, Android framework semantics and vendor-specific hardware protocols at the application API boundary.

## 117.2 Application state and Provider recovery

Application execution state SHALL be separable from durable application/Context state. The OS MAY terminate and recreate processes without implying that the user-facing Context is destroyed.

Provider checkpointability SHALL be explicit. Providers that cannot serialize complete execution state SHALL still expose the durable state required for correct Context reconstruction where practical.

## 117.3 Package identity and provenance

Every production package SHALL carry attributable publisher/provenance identity, dependency information, capability declarations and required runtime/hardware information. Signed packages SHALL be validated before activation.

## 117.4 Enterprise policy

Enterprise policies MAY further attenuate capabilities, Context sharing, data transfers, buffer sharing, network access, AI use and application distribution. Enterprise policy SHALL never expand authority beyond the base Aether security model.

---

# 118. Canonical System Composition and Engineering Baseline

## 118.1 Core System versus TCB

The term **Core System** SHALL describe the minimally required set of trusted/foundational components needed for boot, security, execution, resource management, core persistence, hardware access and canonical system semantics.

The Core System is not a fixed list of binaries. Its concrete composition MAY differ by profile and implementation.

The **Trusted Computing Base (TCB)** SHALL be modeled separately and SHALL include every component whose compromise could violate a stated security invariant, regardless of whether that component is classified as “core” for product packaging purposes.

## 118.2 Optionality

Desktop, laptop, mobile, tablet and foldable builds SHOULD exclude components that cannot be used by the target hardware/profile unless they are needed for recovery, compatibility, development, certification or another explicit reason.

## 118.3 Recovery and reset

Factory reset SHALL distinguish:

- user-data reset;
- application-data reset;
- Context-state reset;
- settings reset;
- AI-memory reset where enabled;
- complete secure erase where supported.

Factory reset SHALL not destroy system trust roots merely because user state is being reset.

Cryptographic destruction SHOULD target the encryption domains associated with the reset scope.

---

# 119. Canonical Resource Invariants — RES-001 through RES-015

## RES-001 — Context/resource separation

Context lifecycle SHALL not itself determine execution priority or physical resource allocation.

## RES-002 — Validated resource contracts

Resource contracts SHALL be validated, attenuated, admitted and enforced by AetherOS policy.

## RES-003 — Resource leases

Transient latency-sensitive workloads MAY acquire explicit resource leases independent of Context presentation state.

## RES-004 — Control/enforcement separation

User-space resource policy SHALL not be a mandatory synchronous dependency of latency-critical input or presentation enforcement.

## RES-005 — Kernel/device enforcement

The trusted kernel/device layer SHALL provide authoritative physical resource enforcement and emergency fallback independent of the Resource Broker.

## RES-006 — Predictive pressure handling

Normal memory-pressure management SHOULD act before emergency kernel OOM conditions are reached.

## RES-007 — OOM containment

Kernel OOM SHALL not constitute normal Provider/Context lifecycle management.

## RES-008 — Graceful degradation

A resource request that cannot be satisfied SHALL produce a defined degraded/rejected state rather than silent unbounded starvation.

## RES-009 — Real-time admission

Hard temporal guarantees SHALL require explicit admission control and schedulable execution budgets.

## RES-010 — Multi-resource arbitration

Resource decisions SHALL consider coupled CPU, memory, accelerator, bandwidth, storage, network, power and thermal effects.

## RES-011 — Compatibility hierarchy

Compatibility environments SHALL remain subordinate to host resource governance while retaining authority over their internal application lifecycle.

## RES-012 — Context persistence

Provider termination SHALL not inherently destroy Context identity or Context-owned durable state.

## RES-013 — Thermal supremacy

Thermal safety policy MAY constrain lower-criticality contracts and SHALL preserve critical functionality where the hardware permits.

## RES-014 — Resource attribution

Significant resource consumption SHALL be attributable to the responsible Context, Provider, Activity, system service, compatibility runtime or other explicit subject.

## RES-015 — Platform-independent semantics

Resource semantics SHALL remain common across supported platform profiles even when scheduling implementations and objective functions differ.

---

# 120. Canonical Hardware and Device Invariants — AHDA-001 through AHDA-020

## AHDA-001 — Capability-mediated buffer sharing

Compatible producer/consumer domains SHALL support zero-copy/shared-buffer transfer where hardware permits, using explicit capability authority and synchronization.

## AHDA-002 — Physical-layout independence

Buffer sharing SHALL not require physical contiguity unless a concrete hardware requirement makes it necessary.

## AHDA-003 — Attenuating buffer rights

Buffer capabilities SHALL transfer only explicitly authorized rights.

## AHDA-004 — Vendor quarantine

Untrusted/proprietary hardware software SHALL execute only within an appropriately bounded compatibility domain unless independently integrated and reviewed.

## AHDA-005 — Compatibility below semantics

Vendor protocols and HALs SHALL not redefine canonical Aether device semantics.

## AHDA-006 — Kernel physical authority

Only the trusted kernel/hardware substrate SHALL authorize and enforce physical resource mappings, interrupt ownership, DMA domains, IOMMU mappings and privileged hardware access.

## AHDA-007 — Device Manager semantic authority

The Aether Device Manager SHALL maintain the canonical semantic Device Graph.

## AHDA-008 — Firmware as evidence

Firmware descriptions SHALL be treated as discovery/configuration inputs subject to validation.

## AHDA-009 — Discovery encapsulation

ACPI, Device Tree, UEFI, SoC firmware and other discovery mechanisms SHALL terminate at the AHDA normalization boundary.

## AHDA-010 — Driver failure isolation

Where hardware permits, driver/service failure SHALL be isolated and recoverable without unrelated system failure.

## AHDA-011 — Device semantics are stable

Aether device APIs SHALL remain stable across bus and silicon implementations.

## AHDA-012 — Resource attachment is explicit

Device access SHALL be associated with explicit resource and capability authority.

## AHDA-013 — IOMMU-first DMA isolation

DMA-capable hardware SHALL use explicit IOMMU/address-space isolation when supported.

## AHDA-014 — Persistent multimedia resources

High-frequency pipelines SHOULD amortize buffer allocation and attachment through reusable pools.

## AHDA-015 — Synchronization correctness

Shared buffers SHALL have explicit or correctly managed synchronization semantics.

## AHDA-016 — VCD subordinate authority

Vendor Compatibility Domains SHALL remain subordinate to Aether capabilities and host resource governance.

## AHDA-017 — Driver placement is evidence-based

Driver placement SHALL not be determined solely by historical OS conventions.

## AHDA-018 — Physical ownership is kernel-validated

Userspace binding requests SHALL be independently checked by the kernel for physical correctness and security.

## AHDA-019 — Semantic graph is portable

The Device Graph SHALL be independent of the historical representation used to discover a device.

## AHDA-020 — Hardware profile accountability

Certified hardware profiles SHALL identify the supported hardware, firmware, driver, capability, power, thermal and certification properties required for conformance.

---

# 121. Canonical Boot and Trust Invariants — BOOT-001 through BOOT-020

## BOOT-001 — Immutable root

Verified boot SHALL originate from an immutable or equivalently hardware-protected root of trust.

## BOOT-002 — Verified and measured execution

Where hardware supports it, boot SHALL provide both execution verification and measurement.

## BOOT-003 — Owner-controlled trust

Certified Aether Hardware SHALL provide secure owner-controlled trust enrollment without disabling the verified/measured boot chain.

## BOOT-004 — Authenticated ownership transition

Trust-state changes SHALL require authenticated owner intent and, where available, physical presence.

## BOOT-005 — Hardware-qualified guarantees

AetherOS SHALL not claim security guarantees exceeding the certified hardware's trust capabilities.

## BOOT-006 — Authenticated generation descriptor

Every bootable Aether generation SHALL be identified by an authenticated Generation Descriptor.

## BOOT-007 — Acyclic bootstrap

Boot-critical verification SHALL not depend upon initialization of the full runtime/storage engine.

## BOOT-008 — Layered verification

Boot stages SHALL verify the artifacts needed at that stage, with higher layers continuing verification as appropriate.

## BOOT-009 — Rollback domains

Anti-rollback SHALL support independently updateable security domains where required.

## BOOT-010 — Security floor

Secure Owner Mode SHALL reject generations below the hardware-backed applicable security floor.

## BOOT-011 — Explicit exceptional downgrade

Development/recovery downgrade paths SHALL be distinguishable and SHALL apply appropriate protected-state/key semantics.

## BOOT-012 — Candidate isolation

A candidate generation SHALL not replace the known-good generation until health commitment succeeds.

## BOOT-013 — Health/verification separation

Health state SHALL not itself constitute cryptographic authorization to execute.

## BOOT-014 — Automatic recovery

Failure of the selected generation SHOULD cause selection of the previous eligible known-good generation where available.

## BOOT-015 — Trusted recovery fallback

If no eligible generation remains, the system SHALL enter a trusted recovery path where platform capability permits.

## BOOT-016 — Protected generation metadata

Generation and boot-health metadata SHALL be integrity-protected and resistant to unauthorized rollback.

## BOOT-017 — No routine generation menu

Normal boot SHALL not require an interactive generation-selection menu.

## BOOT-018 — Authenticated owner recovery

Manual generation selection and exceptional maintenance SHALL require an authenticated recovery path.

## BOOT-019 — Attestable state

Boot and trust state SHALL be measurable and, where supported, attestable.

## BOOT-020 — Platform abstraction

UEFI, SoC firmware, secure monitors, TPMs, TEEs, RPMB and other platform mechanisms SHALL terminate at the Aether boot/trust abstraction boundary.

---

# 122. Canonical Governance Charter — Appendix H

## H.1 Specification authority

The Master Architecture Specification is the canonical semantic authority. Lower-level documents, ADRs and source code SHALL not silently redefine it.

## H.2 Semantic-contract-first design

Specifications SHOULD define semantics, authority, lifecycle, failure, security, resource and observability contracts before freezing implementations.

## H.3 Five-separation review

Every major design review SHALL evaluate whether the proposed implementation accidentally conflates any of the Five Semantic Separations.

## H.4 Change classes

Constitutional, Normative Interface, Implementation and Profile-Specific changes SHALL be reviewed according to their impact class.

## H.5 Evidence requirement

Implementation mechanisms SHALL become normative only when supported by the architectural reason and evidence required by the affected subsystem.

## H.6 Reference-hardware discipline

Performance, security, thermal, power, storage, boot, compatibility and recovery claims SHALL be measured on designated reference hardware and maintained as regression baselines.

## H.7 Conformance before optimization

Platform-specific optimization SHALL preserve the common semantic contract and SHALL be accompanied by appropriate regression tests.

## H.8 Compatibility is measured

Compatibility claims SHALL be supported by maintained fidelity matrices and test evidence.

## H.9 Fault semantics are explicit

Subsystem specifications SHALL distinguish component failure, service failure, Provider failure, Context degradation, generation failure, system failure and hardware failure.

## H.10 Security threat modeling

Privileged subsystems SHALL maintain threat models covering authority boundaries, confused deputies, cross-Context access, DMA/IOMMU, compromised drivers, compatibility runtimes, supply chain, recovery and relevant side channels.

## H.11 Supply-chain provenance

Production components SHOULD carry source/package/build/firmware provenance and appropriate signatures or verification metadata.

## H.12 Interface stability

Stable interfaces SHALL have explicit version semantics, compatibility expectations, deprecation rules and conformance tests.

## H.13 Deprecation discipline

AetherOS SHOULD prefer additive evolution and explicit migration over silent incompatible removal.

## H.14 Observability privacy

Debugging and observability mechanisms SHALL not become implicit privilege or data-exfiltration paths.

## H.15 Formal verification priorities

Formal verification SHALL prioritize properties whose failure can compromise system-wide guarantees, including boot state, capability attenuation, Context isolation, IPC authority transfer, buffer sharing, DMA/IOMMU, storage transactions, generation selection, package updates, driver lifecycle, permission transitions and recovery.

## H.16 Release gates

Constitutional violations, critical security regressions, unacceptable latency/resource regressions, unexplained footprint growth and failed recovery/fault-containment properties SHALL block release until resolved or explicitly reclassified by architecture governance.

## H.17 ADR requirements

Any previously open implementation choice promoted to normative architecture SHALL have an ADR documenting alternatives, evidence, impact and reversal conditions.

## H.18 Versioning semantics

Major versions indicate incompatible architectural semantics. Minor versions add compatible capabilities or clarify behavior. Errata correct ambiguities without intentionally changing semantics.

## H.19 Final constitutional rule

A subsystem is complete only when its semantics, security, resources, failure behavior, compatibility behavior, observability, tests, verification and reference implementation evidence form one coherent system.

---

# 123. Canonical Dependency Graph for Full-System Engineering

The following dependency order is the canonical engineering sequence for future debates and implementation planning. It is a dependency guide rather than a prohibition on parallel work.

```text
1. Boot + Trust Root
        ↓
2. Kernel Execution Model
        ↓
3. IPC + Capability Transfer
        ↓
4. Memory + Virtual Memory
        ↓
5. CPU / Accelerator Scheduling
        ↓
6. Device Model + Driver Framework
        ↓
7. Graphics + Display + Input
        ↓
8. Audio + Media
        ↓
9. Networking + Radio
        ↓
10. Power / Thermal / Battery
        ↓
11. Storage Implementation + Recovery
        ↓
12. Native ABI + Runtime
        ↓
13. Package / Distribution / Updates
        ↓
14. Android Compatibility
        ↓
15. Web Runtime
        ↓
16. AI Runtime + NPU + Model Management
        ↓
17. Identity / Accounts / Enterprise
        ↓
18. Backup / Restore / Continuity
        ↓
19. Developer Platform
        ↓
20. Certification / Manufacturing / Release
```

Subsystems MAY be explored in parallel when their interfaces are sufficiently specified, but a downstream implementation SHALL not be allowed to redefine unresolved upstream semantics merely for implementation convenience.

---

# 124. Current v1.0 architectural decision register

The following decisions are considered converged as of this version unless a future ADR explicitly reopens them:

| Area | Converged decision | Implementation still open? |
|---|---|---|
| Human-facing abstraction | Context-centric | Yes |
| UI semantics | Visual + Semantic + Input Graphs | Yes |
| Context security | Zero-trust Provider boundaries | Yes |
| Capability authority | Attenuating policy intersection | Yes |
| Accessibility/AI projection | Policy-derived semantic projections | Yes |
| Cross-Context transfer | Intent/Transfer Broker; compositor is not a data-authority bypass | Yes |
| Context isolation modes | Standard Isolation plus optional hardware-enforced Strict Isolation | Yes |
| Persistent Context state | KMS-mediated independently encrypted slices | Yes |
| Storage model | Object semantics + transactional mutable store + immutable CAS + views | Yes |
| POSIX | Compatibility/projection interface, not sole authority | Yes |
| Resource model | Contracts + Leases + policy/enforcement separation | Yes |
| Memory pressure | P0–P5 predictive management ladder | Yes |
| Hardware truth | Kernel physical model + userspace semantic Device Graph | Yes |
| DMA sharing | Capability-mediated shared buffers + synchronization | Yes |
| Buffer physical layout | Implementation-defined unless hardware requires otherwise | Yes |
| Driver placement | Evidence-based layered model | Yes |
| Vendor hardware | VCD + Aether Vendor Adapter | Yes |
| Android | Isolated compatibility environment; VM preferred when useful | Yes |
| Generation identity | Authenticated ASGD | Yes |
| Boot | Verified + measured + owner trust where supported | Yes |
| Anti-rollback | Hardware-backed security floor / rollback domains | Yes |
| Generation health | Separate Generation Health Manager | Yes |
| Manual rollback | Authenticated recovery path, not routine boot menu | Yes |
| Core OS | Semantic category, not fixed binary list | Yes |
| Footprint | Measured target, not hard size law | Yes |
| Governance | ADR/change-class based | No, unless governance itself is constitutionally amended |

---

# 125. Convergence status and remaining open areas

The foundational architecture is now considered **constitutionally converged**, but the operating system as a whole is intentionally not considered implementation-complete.

The next debates SHALL continue through the dependency graph and SHALL address, at minimum:

- precise Aether kernel execution-domain semantics;
- process/thread/address-space model;
- kernel/user boundary;
- capability representation and revocation;
- IPC transport and fast paths;
- interrupt/timer architecture;
- memory allocators and page lifecycle;
- scheduler objective functions and admission;
- accelerator scheduling;
- device reset and recovery semantics;
- graphics/compositor architecture;
- display synchronization;
- audio timing and isolation;
- networking and radio policy;
- exact storage implementation;
- native ABI and language/runtime contracts;
- identity and account model;
- package repository/distribution trust;
- Android graphics/media integration;
- Web Runtime isolation;
- AI runtime and model-management architecture;
- backup/restore and cross-device continuity;
- developer and enterprise interfaces;
- certification and production engineering.

Each future AetherOS architecture discussion SHALL update this specification, preserve useful prior content, and add newly converged semantics rather than repeatedly creating independent competing specifications.

This specification SHALL therefore be treated as a continuously expanded canonical architecture record rather than a document that is rewritten from scratch for every design phase.



---

# 126. Phase 2 Step 2 — Kernel Execution Model Convergence

This section records the architectural outcome of the Phase 2 Step 2 debate concerning the bridge between AetherOS semantic execution and the underlying CPU/MMU/kernel substrate.

The purpose is not to require a specific kernel implementation. The purpose is to freeze the semantic boundaries that every future kernel implementation SHALL preserve.

## 126.1 Core architectural conclusion

AetherOS SHALL distinguish:

```text
Aether Execution Semantics
        ≠
Kernel Execution Mechanisms
        ≠
Legacy POSIX Compatibility Semantics
```

Native Aether applications SHALL execute under an explicit, capability-mediated execution model. POSIX process, namespace and ambient-authority semantics MAY remain available inside compatibility environments and through carefully bounded native compatibility libraries, but SHALL NOT define the authority model of native Aether execution.

The kernel remains the authoritative enforcement layer for:

- virtual address spaces;
- physical memory ownership;
- thread execution;
- scheduling enforcement;
- capability checks for kernel-mediated operations;
- IPC primitives;
- interrupt delivery;
- DMA/IOMMU mappings;
- timers;
- low-level resource limits;
- low-level isolation and security state.

Higher-level Aether services remain responsible for semantic policy rather than delegating semantic meaning to kernel implementation details.

## 126.2 Capsule execution domain

The canonical native execution boundary SHALL be the **Aether Capsule**.

A Capsule is an explicit execution/security domain containing, as applicable:

- one or more virtual address spaces or one primary protected address space;
- execution threads;
- a capability namespace;
- explicit IPC endpoints/rings;
- resource bindings and budgets;
- namespace/resource views;
- lifecycle state;
- fault and recovery policy;
- Provider association where applicable.

A Capsule SHALL begin with no ambient authority except explicitly provisioned bootstrap capabilities.

A Capsule SHALL NOT inherit filesystem, network, device, IPC, process-control, or other privileged authority merely because it was created by another Capsule or application.

Native Provider creation SHALL use an Aether semantic spawn operation rather than requiring POSIX `fork()`/`execve()` semantics.

The semantic operation MAY be implemented on a Linux-derived kernel using an appropriate combination of existing kernel process/address-space primitives, but those mechanisms SHALL remain below the Aether abstraction boundary.

## 126.3 Capsule and Provider relationship

A Capsule and ProviderInstanceIdentity are distinct concepts.

A Provider normally receives one dedicated Capsule/security execution domain when strong isolation is required.

Multiple Providers MAY share an execution environment only when the runtime provides an enforceable internal isolation mechanism sufficient for all applicable Provider, Context and capability boundaries.

ProviderInstanceIdentity SHALL NOT by itself create memory isolation inside one ordinary address space.

A process boundary SHALL therefore not automatically be treated as equivalent to a security boundary.

## 126.4 Capability namespace

Native Capsules SHALL possess an explicit capability namespace.

Capabilities MAY refer to:

- memory objects;
- channels/endpoints;
- storage objects;- device resources;
- buffer objects;
- events;
- timers;
- resource leases;
- services;
- Context-scoped communication endpoints;
- other protected kernel or system-service objects.

Capability authority SHALL be attenuating. A derived or delegated capability SHALL NOT grant more authority than its source capability.

Capabilities SHALL be revocable where the protected resource and security model require revocation.

Capabilities SHALL be unforgeable at the semantic level, although the physical representation of a capability handle remains implementation-defined.

This direction is compatible with capability-oriented kernels such as seL4, where kernel-managed capabilities explicitly control access to kernel objects and may be selectively delegated or attenuated. citeturn679147search0turn679147search37

## 126.5 Native syscall model

AetherOS SHALL NOT require native applications to use the complete ambient POSIX syscall surface as their architectural authority model.

Instead, the native kernel ABI SHALL be divided conceptually into:

```text
Control / Primitive Syscalls
        +
Capability-mediated IPC and Resource Operations
        +
Asynchronous Data-plane Rings
```

Primitive synchronous operations MAY include, where required:

- thread creation/termination;
- address-space and memory mapping operations;
- capability operations;
- wait/wakeup operations;
- event and timer operations;
- IPC/ring setup;
- fault handling;
- process/Capsule lifecycle operations;
- clock/time queries;
- scheduler/resource binding;
- controlled yield or park operations.

The exact syscall set SHALL remain implementation-dependent.

AetherOS SHALL NOT mandate that every operation cross a userspace broker before reaching the kernel. Security-critical authority checks that must be performed by the kernel SHALL remain directly enforceable by the kernel.

## 126.6 Capability Ring architecture

AetherOS SHALL support asynchronous shared-memory request/completion mechanisms for high-throughput and high-concurrency operations.

Conceptually:

```text
Provider
   │
   ├── Submission Ring
   │
   ▼
Kernel / Aether Service
   │
   ├── Completion Ring
   │
   ▼
Provider
```

Requests SHALL carry explicit capability handles and operation metadata.

Completion records SHALL be attributable to the originating request and security domain.

The mechanism MAY be implemented using a ring architecture analogous to `io_uring`, shared-memory queues, channels, or another transport. Linux documentation demonstrates that ring-based request/completion mechanisms can support efficient userspace/kernel and userspace/daemon communication, including FUSE-over-io-uring. citeturn731502search4

However, AetherOS SHALL NOT define `io_uring` itself as the native application ABI.

## 126.7 Native POSIX compatibility without ambient authority

AetherOS SHALL support practical reuse of portable C/C++ and other libraries without requiring the ecosystem to rewrite every library around raw Aether capability operations.

A native compatibility library layer MAY provide familiar APIs such as:

- `open`-like object access;
- `read`/`write`-like operations;
- socket-like networking interfaces;
- file-descriptor-like opaque references;
- pthread-like synchronization;
- standard streams where explicitly provided.

These APIs SHALL be implemented as userspace semantics over Aether capabilities and services rather than as ambient authority.

For example:

```text
Portable Library
      ↓
Aether Libc / Compatibility Facade
      ↓
Capability Handle / Aether Request
      ↓
Aether Service or Kernel Primitive
```

A library's `open()`-like call therefore does not imply a globally authorized filesystem path lookup. The compatibility layer requests or resolves an authorized storage capability and presents a library-local handle.

Blocking APIs MAY be implemented synchronously over asynchronous rings so that portable libraries retain conventional programming semantics without sacrificing the underlying asynchronous transport.

This preserves ecosystem portability while keeping the native authority model capability-based.

## 126.8 No native ambient global namespace

Native Aether execution SHALL NOT assume a globally visible filesystem namespace, process namespace, device-node namespace, environment namespace, or network namespace.

A Capsule MAY receive explicit namespace projections such as:

- a storage object namespace;
- a POSIX compatibility filesystem view;
- a network service view;
- a device-class service view;
- a Context service namespace.

Such projections SHALL be capability-mediated and policy-controlled.

This preserves the Storage and Security architectural separations while allowing practical compatibility.

## 126.9 Thread semantics

The Aether Thread is the fundamental CPU execution entity.

A Thread SHALL have:

- a stable execution identity;
- an address-space association;
- an owning Capsule;
- a baseline resource policy;
- scheduling state;
- wait/block state;
- fault state;
- optional active scheduling contract context.

Threads SHALL NOT derive authority merely from being scheduled on a CPU.

Threads SHALL NOT inherit unrestricted capabilities from arbitrary parent threads.

Thread creation SHALL receive an explicit attenuated capability set and resource context.

## 126.10 Resource Lease and scheduling semantics

A Resource Lease SHALL be an externally granted, revocable scheduling/resource authority associated with a workload, Activity, resource contract, or scheduling context.

The Lease SHALL NOT redefine the semantic identity of the Context or Provider.

The scheduler SHALL therefore conceptually evaluate:

```text
Effective Execution Policy
    = Baseline Thread Policy
      ∩ Capsule Policy
      ∩ Activity/Workload Policy
      ∩ Active Resource Lease
      ∩ System/Platform Policy
      ∩ Current Resource Availability
      ∩ Thermal/Safety Constraints
```

A Resource Lease MAY affect:

- CPU scheduling class or weight;
- deadline parameters;
- utilization constraints;
- CPU placement preferences;
- accelerator priority;
- memory residency protection;
- I/O priority;
- wakeup constraints;
- other resource dimensions explicitly covered by the lease.

A Lease SHALL never grant more security authority merely by granting more execution resources.

## 126.11 Lease scope and mixed-purpose threads

A Resource Lease SHALL NOT be treated as a permanent intrinsic priority attached to the semantic identity of a thread.

A thread MAY operate under an active scheduling context for a particular workload phase, subject to policy.

A thread executing two logically distinct workloads simultaneously SHALL NOT be assumed to receive independent priorities merely because both workloads reference the same thread.

For genuinely independent criticality levels, the native runtime SHOULD use separate scheduling entities/threads or equivalent kernel-supported execution contexts.

If multiple activities are deliberately multiplexed through one scheduling entity, the effective execution policy SHALL be the policy actually applicable to the current work, and the runtime SHALL prevent a low-integrity workload from inheriting a high-criticality lease without authorization.

This preserves the separation:

```text
Context Meaning
    ≠
Execution Criticality
    ≠
Physical CPU Allocation
```

The scheduler is therefore a consumer of resource contracts and leases, not an interpreter of Context semantics.

## 126.12 Deadline and realtime admission

A hard timing guarantee SHALL require explicit admission control.

A workload SHALL NOT obtain a hard realtime/deadline guarantee merely by requesting elevated priority.

The Resource Governor SHALL determine whether sufficient CPU/resource capacity exists to admit the requested contract.

Linux's `SCHED_DEADLINE` provides an existing implementation precedent for runtime/deadline/period-based scheduling, while EEVDF provides latency-aware fair scheduling for ordinary workloads. citeturn731502search0turn731502search2

AetherOS SHALL define the semantic distinction between:

- best effort;
- latency-sensitive;
- bounded-latency;
- soft deadline;
- hard deadline;
- safety-critical execution.

The exact mapping to kernel scheduler classes SHALL remain implementation-specific.

## 126.13 sched_ext and kernel scheduler integration

A Linux-derived AetherOS kernel MAY use `sched_ext`, EEVDF, deadline scheduling, real-time scheduling, utilization clamping, cpusets, or combinations thereof.

`sched_ext` MAY provide an implementation route for an Aether-specific scheduler policy because it permits a BPF-defined scheduler and can dynamically fall back to the fair-class scheduler when the extension terminates or encounters an error. citeturn731502search1turn731502search3

Nevertheless:

```text
Aether Scheduler Semantics
        ≠
Linux scheduler implementation
```

No Aether application or service SHALL depend directly on a specific Linux scheduler implementation as the stable semantic contract.

## 126.14 Asynchronous IPC is a capability, not a dogma

AetherOS SHALL provide fast asynchronous communication paths but SHALL NOT require every native operation to use an asynchronous ring.

The canonical communication model is:

```text
Small / control operation
    → direct capability-mediated syscall or IPC

High-throughput / streaming operation
    → shared ring / shared memory / resource handle

Large buffer / multimedia operation
    → zero-copy capability + synchronization
```

This preserves low latency for primitive operations while allowing high-throughput asynchronous execution where it materially improves performance.

The existing IPC architecture already requires asynchronous messaging, request/reply, event streams, capability transfer, resource handles and zero-copy paths where safe. fileciteturn17file0

## 126.15 Page faults and memory authority

Page faults SHALL remain kernel-enforced events because the kernel controls virtual memory and page-table authority.

A native runtime MAY use userspace-managed memory allocators and object abstractions, but it SHALL NOT obtain arbitrary physical mappings through an application-level storage or buffer handle.

Memory mapping operations SHALL distinguish at least:

- anonymous/private memory;
- shared memory capability;
- file/object-backed memory capability;
- device buffer capability;
- protected/secure memory capability.

The mapping operation SHALL enforce the capability's rights and applicable resource policies.

## 126.16 Interrupt architecture

Physical hardware interrupts SHALL remain under kernel/hardware authority.

The kernel SHALL perform the minimum trusted interrupt processing required to:

- acknowledge or mask the interrupt;
- maintain hardware correctness;
- establish the event identity;
- preserve security;
- queue or coalesce the resulting event;
- recover or contain hardware faults where required.

The kernel MAY then export an authenticated **Event Capability** to a userspace service.

Conceptually:

```text
Hardware IRQ
    ↓
Kernel interrupt path
    ↓
Minimal trusted handling
    ↓
Event Capability
    ↓
Service Event Queue
    ↓
Scheduler wakeup
    ↓
Userspace device service
```

A userspace device driver SHALL NOT receive arbitrary raw IRQ authority merely because it owns a device service.

## 126.17 Interrupt wakeup and scheduler authority

An interrupt fast path MAY directly mark an eligible wait target runnable and SHOULD minimize unnecessary scheduler/bookkeeping overhead.

However, an interrupt SHALL NOT bypass scheduler authority or force arbitrary user execution independently of scheduler policy.

The kernel remains responsible for deciding when and on which CPU the awakened thread executes.

AetherOS SHALL therefore use the semantic rule:

```text
IRQ delivery latency
    ≠
User-thread execution latency
```

Both SHALL be measured independently.

Linux futexes provide an instructive implementation precedent: uncontended synchronization can remain in userspace, while the kernel participates when contention requires blocking/wakeup. citeturn731502search40turn731502search7

## 126.18 Event capabilities and wait sets

AetherOS SHALL support capability-scoped event waiting for userspace services.

A service MAY wait on multiple events through a wait-set or equivalent mechanism where supported, including:

- device interrupts/events;
- buffer completion fences;
- IPC messages;
- timers;
- cancellation;
- resource-pressure events;
- lifecycle events.

This SHOULD allow userspace device services to avoid polling.

The exact primitive MAY use futex-like waits, kernel notifications, channels, event queues, or another mechanism.

## 126.19 Device service fault model

A userspace device service SHALL be restartable where the hardware and device protocol permit safe restart.

The system SHALL distinguish:

```text
interrupt lost
request timeout
service crash
hardware fault
firmware fault
controller fault
unrecoverable device failure
```

A service crash SHALL not automatically imply a Kernel or System failure.

This continues the existing AetherOS requirement for fault-domain isolation and recovery of camera, audio, GPU, driver, Android and AI components where hardware permits. fileciteturn12file1

## 126.20 Kernel role boundary

The Aether Kernel SHALL remain intentionally small in semantic scope even when the underlying implementation is a mature monolithic kernel.

The kernel SHALL own mechanisms that require privileged enforcement, including:

- scheduling enforcement;
- address-space and memory protection;
- low-level IPC;
- capability enforcement where kernel objects are involved;
- interrupts;
- timers;
- DMA/IOMMU;
- low-level device access;
- low-level power/safety mechanisms.

The kernel SHOULD NOT absorb high-level Aether semantics such as:

- Context ranking;
- application UI semantics;
- notification grouping;
- natural-language intent interpretation;
- user-facing storage organization;
- high-level capability consent UX;
- AI reasoning.

This preserves the existing kernel boundary already established by AetherOS. fileciteturn14file0

## 126.21 Native execution versus compatibility execution

AetherOS SHALL support two intentionally different execution philosophies:

```text
Native Aether
    explicit capabilities
    Capsules
    Aether IPC
    Aether resource contracts
    semantic APIs

Compatibility Environment
    POSIX/Android/Linux semantics as required
    isolated runtime boundary
    host-controlled resources
    translated/bridged capabilities
```

The existence of the second SHALL NOT redefine the first.

The Native Runtime MAY provide compatibility libraries for practical software reuse, but those libraries remain projections over Aether semantics rather than an authority escape hatch.

## 126.22 Kernel escape prevention

Native applications SHALL NOT be able to obtain ambient authority by:

- using a legacy syscall that bypasses the capability model;
- manufacturing or guessing handles;
- reinterpreting arbitrary memory as a capability;
- accessing global device nodes;
- reaching unrestricted namespace paths;
- bypassing the Resource Governor;
- requesting privileged scheduler state without an authorized lease;
- using compatibility libraries as privilege escalators.

All such paths SHALL terminate in explicit capability and policy checks.

## 126.23 Formal execution model

The minimum formally reasoned execution state of a native workload SHOULD be representable as:

```text
ExecutionDomain {
    domain_id
    address_space
    capability_space
    threads
    resource_contracts
    active_leases
    namespace_views
    ipc_endpoints
    event_sources
    lifecycle_state
    fault_state
}
```

The formal model SHALL distinguish:

```text
identity
authority
execution
resource policy
physical allocation
```

No transition SHALL implicitly manufacture authority.

## 126.24 Execution lifecycle

A native Capsule MAY progress through:

```text
CREATED
   ↓
INITIALIZING
   ↓
RUNNABLE
   ↓
RUNNING
   ↓
BLOCKED / WAITING
   ↓
RUNNING
   ↓
QUIESCING
   ↓
SUSPENDED
   ↓
RESTORING
   ↓
RUNNABLE
```

Failure transitions MAY include:

```text
FAULTED
QUARANTINED
TERMINATED
CHECKPOINTED
RECOVERING
```

Provider, Capsule and Context lifecycles SHALL remain separately represented.

In particular:

```text
Provider termination
    ≠ Capsule identity destruction
    ≠ Context destruction
```

## 126.25 Native subprocess model

Native applications SHALL use explicit spawn semantics for child execution.

The canonical model is conceptually:

```text
aether_spawn(
    executable/object,
    capabilities,
    resource_contract,
    namespace_views,
    environment
)
```

The exact syscall/API name remains open.

Copy-on-write process inheritance MAY be used internally by a Linux-derived implementation, but it SHALL NOT imply automatic inheritance of Aether capabilities or ambient resource authority.

## 126.26 Development ergonomics

Native application development SHALL not require developers to manipulate raw kernel capabilities manually for ordinary operations.

The Aether SDK and runtime SHOULD provide:

- typed capability acquisition;
- scoped capability lifetimes;
- typed resource contracts;
- asynchronous and synchronous wrappers;
- cancellation;
- structured error reporting;
- portable library adapters;
- testing/simulation facilities.

The architecture therefore aims for:

```text
Low-level substrate:
    capability-oriented

Application programming model:
    high-level and ergonomic
```

## 126.27 Performance principle

AetherOS SHALL distinguish semantic indirection from physical data movement.

An abstraction SHALL not inherently imply a data copy, context switch, kernel round trip, device remapping, or serialization unless required by the security or hardware contract.

Equivalent operations SHOULD be optimized through:

- shared memory;
- capability handles;
- zero-copy buffers;
- batching;
- rings;
- persistent mappings;
- event-driven waits;
- direct service channels;
- hardware offload.

This continues the same principle used by the Storage and Hardware architectures.

## 126.28 Security and performance must not be traded implicitly

An optimization SHALL NOT silently weaken capability isolation, Context isolation, memory protection, DMA protection, resource accounting, or observability.

Any optimization that deliberately changes a security or isolation guarantee SHALL require explicit architectural classification and an ADR.

## 126.29 Converged decisions for Phase 2 Step 2

The following decisions are considered converged:

1. Native Aether execution uses explicit Capsules rather than ambient POSIX processes as the canonical semantic model.
2. Native Capsules begin with explicitly provisioned capabilities and no ambient privileged namespaces.
3. Process boundaries and security boundaries are distinct concepts.
4. Native Aether does not require the full POSIX syscall model to define its authority semantics.
5. Aether provides both primitive synchronous syscalls and asynchronous data-plane rings.
6. Asynchronous rings are a fast-path mechanism, not a universal mandate for every operation.
7. Userspace compatibility libraries SHALL provide practical portability without granting ambient authority.
8. Blocking APIs MAY be implemented over asynchronous Aether transports.
9. Threads remain the physical CPU scheduling entities.
10. Resource Leases are revocable execution/resource policy objects rather than intrinsic thread authority.
11. Hard realtime guarantees require admission control.
12. Mixed-purpose workloads SHOULD use separate scheduling entities when distinct criticality is required.
13. Kernel interrupts remain authoritative; userspace drivers receive capability-scoped events rather than raw IRQs.
14. Interrupts MAY trigger efficient scheduler wakeups, but SHALL NOT bypass scheduler authority.
15. Aether Device Services remain restartable fault domains where hardware permits.
16. Native execution semantics remain independent of the Linux implementation substrate.

## 126.30 Remaining Step 2 questions intentionally left open

The semantic model is converged, but the following implementation questions remain open for subsequent kernel design work and SHALL require evidence before freezing:

- exact Aether capability-table representation;
- exact capability revocation mechanism;
- exact IPC object model;
- exact channel/ring memory ordering rules;
- exact syscall numbering and ABI encoding;
- kernel scheduler integration mechanism;
- exact deadline/resource-admission algorithm;
- exact user/kernel page-fault contract;
- exact kernel allocator architecture;
- exact timer-wheel/high-resolution timer implementation;
- exact interrupt routing strategy;
- exact kernel preemption configuration;
- exact SMP topology and CPU hotplug semantics;
- exact kernel crash/recovery behavior;
- exact implementation of native POSIX compatibility libraries;
- exact Capsule checkpoint implementation;
- exact sandbox/runtime isolation technology for co-hosted Providers;
- exact formal model and proof decomposition.

These are implementation/design questions, not unresolved contradictions in the semantic architecture.

## 126.31 Phase 2 Step 2 invariant set

### KERN-001 — Explicit execution domains

Native execution SHALL occur within explicit Aether execution domains.

### KERN-002 — No ambient native authority

Native execution domains SHALL NOT acquire privileged authority merely through process creation, parentage, namespace inheritance or conventional POSIX defaults.

### KERN-003 — Explicit capability provisioning

Native execution domains SHALL receive capabilities explicitly and with attenuation.

### KERN-004 — Capability non-forgeability

Capability references SHALL be unforgeable at the semantic security boundary.

### KERN-005 — Process/security distinction

A process, thread group, or address space SHALL NOT automatically constitute an Aether security domain unless the required isolation properties are actually enforced.

### KERN-006 — Kernel physical authority

The kernel SHALL remain authoritative for privileged physical execution mechanisms.

### KERN-007 — Aether semantic independence

Native Aether semantics SHALL not depend on Linux/POSIX implementation details.

### KERN-008 — Primitive syscall minimality

The native kernel ABI SHALL expose a bounded set of primitive operations sufficient to manage execution, memory, synchronization, capabilities, IPC, events and faults.

### KERN-009 — Asynchronous fast paths

AetherOS SHALL support asynchronous shared-memory/ring fast paths for appropriate high-throughput operations.

### KERN-010 — No universal async mandate

AetherOS SHALL NOT require every kernel or system operation to use an asynchronous ring when a direct primitive is more appropriate.

### KERN-011 — Compatibility without authority leakage

Native compatibility libraries MAY emulate POSIX-style APIs but SHALL translate them into explicit Aether authority rather than provide ambient privilege.

### KERN-012 — Thread execution identity

Every schedulable Aether Thread SHALL belong to an explicit execution domain and resource baseline.

### KERN-013 — Lease revocability

Resource Leases SHALL be revocable and SHALL be bounded by higher-level system, security and safety policy.

### KERN-014 — Lease non-authority

A Resource Lease SHALL affect resource execution policy but SHALL NOT itself grant unrelated security privileges.

### KERN-015 — Deadline admission

Hard deadline guarantees SHALL require explicit resource admission.

### KERN-016 — Mixed-criticality separation

Independent criticality workloads SHOULD use separable scheduling entities whenever distinct timing guarantees are required.

### KERN-017 — Interrupt authority

Raw hardware interrupts SHALL remain under kernel/hardware authority.

### KERN-018 — Event capability delivery

Userspace device services SHALL receive capability-scoped events rather than arbitrary hardware interrupt authority.

### KERN-019 — Scheduler integrity

Fast interrupt wakeups SHALL not bypass kernel scheduling authority.

### KERN-020 — Fault isolation

Failure of a userspace execution domain or device service SHALL not inherently imply kernel or unrelated system failure.

### KERN-021 — Memory authority

All privileged memory mappings SHALL be explicitly authorized and kernel-enforced.

### KERN-022 — Namespace projection

Native namespace access SHALL be explicit and capability-mediated.

### KERN-023 — No implicit child authority

Spawning a child execution domain SHALL not implicitly grant it the parent's full capability set.

### KERN-024 — Semantic/physical separation

Aether execution semantics SHALL remain distinct from CPU, MMU, interrupt-controller and kernel implementation details.

### KERN-025 — Observable execution

Scheduling, lifecycle, resource, fault and IPC behavior SHALL be observable through privacy-preserving system tracing sufficient for diagnosis and conformance testing.

## 126.32 Architectural rationale

The final Aether Kernel Execution Model is therefore:

```text
                    AETHER SEMANTICS
                           │
             ┌─────────────┼─────────────┐
             │             │             │
          Capsule       Capability    Resource
          Identity         Space       Contracts
             │             │             │
             └─────────────┼─────────────┘
                           ▼
                    AETHER EXECUTION
                           │
             ┌─────────────┼─────────────┐
             ▼             ▼             ▼
        Sync Primitives  Async Rings   Events
             │             │             │
             └─────────────┼─────────────┘
                           ▼
                    KERNEL ENFORCEMENT
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
       CPU/Threads       MMU/Memory      IRQ/DMA
          │                │                │
          └────────────────┼────────────────┘
                           ▼
                         SILICON
```

The operating principle is:

> **AetherOS schedules and isolates explicit execution domains using capability-mediated authority and resource policy; Linux/POSIX mechanisms may implement those semantics on Tier-1 hardware, but they SHALL never become the semantic source of truth.**

This completes the semantic convergence of Phase 2 Step 2 while deliberately leaving low-level kernel implementation choices open for subsequent evidence-based design.


## 126.33 Decision-register update

Phase 2 Step 2 adds the following canonical entry to the architecture decision record:

| Area | Converged decision | Implementation still open? |
|---|---|---|
| Native execution domains | Aether Capsules with explicit capability provisioning and no ambient native authority | Yes |
| Native kernel ABI | Primitive synchronous operations plus asynchronous capability/ring fast paths | Yes |
| Native POSIX reuse | Userspace compatibility facade over Aether capabilities/services; POSIX is not the native authority model | Yes |
| Scheduling identity | Threads are physical scheduling entities; resource leases are explicit revocable execution-policy objects | Yes |
| Realtime execution | Hard deadlines require admission control; mixed-criticality workloads SHOULD be separable | Yes |
| Interrupts | Kernel-owned physical IRQs with capability-scoped userspace event delivery | Yes |
| Userspace driver wakeup | Event capability + scheduler wakeup; no raw IRQ delivery and no bypass of scheduler authority | Yes |
| Kernel boundary | Kernel owns privileged enforcement; high-level semantic policy stays above it | Yes |