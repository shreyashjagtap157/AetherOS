# AetherOS — deployment profiles, device classes and experience-shell reconciliation

Classification: Informative

## 1. Why this mapping exists

The current AetherOS documentation uses more than one kind of “profile”:

- `docs/Profiles-Specification.md` defines architecture/deployment profiles (the documentation suite currently lists Embedded, Desktop, Cloud, HPC and Safety-Critical).
- The recovered v1.0 convergence source and v4.0 owner mandate define device/form-factor operating regimes and experience shells: desktop, laptop, tablet, foldable and mobile.
- Hardware/firmware targets add a further dimension: ISA, SoC, board, firmware, buses, accelerators and discovered devices.
- Security, assurance and resource contracts add additional applicability dimensions.

These are not necessarily conflicting lists. They describe different axes and must not be collapsed into a single enum or treated as interchangeable.

## 2. Required separation

| Axis | Question answered | Illustrative values |
|---|---|---|
| Deployment / computation profile | What execution, service, resource and lifecycle contract applies? | Embedded, desktop-class, cloud, HPC, safety-critical |
| Device / form-factor class | What physical interaction and power regime does the user have? | Desktop, laptop, tablet, foldable, mobile |
| Hardware / firmware target | What ISA, SoC, board, boot firmware, buses and devices exist? | RISC-V64 QEMU `virt`, named physical RISC-V board, x86-64/ARM64 target |
| Experience shell | How does the user interact with tasks and contexts on this device? | Desktop/windowed, touch-first tablet, foldable adaptive shell, mobile shell |
| Assurance / security profile | What trust, verification, safety and authority guarantees are required? | Capability policy, verified boot, isolation tier, declared safety/assurance target |
| Resource profile | What CPU/memory/I/O/energy/thermal budgets apply? | Real-time bounds, mobile power envelope, HPC throughput, embedded footprint |

A combination is supported only when the relevant contracts and hardware have been implemented and qualified. Do not infer that every deployment profile automatically supports every device class, or that every device supports every hardware feature.

## 3. One shared semantic platform

All supported device classes must remain one AetherOS platform:
- common identity, process, capability and authority semantics;
- common kernel/user ABI and system-service contracts where the architecture requires them;
- common storage/data lifecycle and update/recovery invariants;
- common application/runtime/package model;
- common Context/Workspace/Activity/Provider semantics;
- explicit per-profile policies and replaceable platform drivers;
- declared compatibility and migration behavior.

Different shells and platform adapters must not silently fork the OS semantics. Conversely, desktop assumptions must not be forced onto mobile when input, power, radio, suspend/resume, thermal, camera or lifecycle constraints differ.

## 4. Device classes and experience requirements

- **Desktop:** windowed multi-tasking, keyboard/pointer, multi-display, docking, graphics/performance, persistent workstation state.
- **Laptop:** desktop semantics with battery-aware scheduling, sleep/resume, thermal control, trackpad/input and mobile connectivity.
- **Tablet:** touch/pen-first navigation, adaptive layouts, orientation, sensors, battery and application lifecycle.
- **Foldable:** posture/hinge state, dynamic display geometry, continuity across folded/unfolded states and safe relayout.
- **Mobile:** touch-first UX, cellular/telephony where supported, radio/location/sensor privacy, aggressive power/thermal constraints, background lifecycle and secure device identity.

These examples guide requirement decomposition; precise support tiers and acceptance tests must come from current accepted architecture and profile decisions.

## 5. Hardware target and support matrix

For each supported target record:
- ISA, CPU/hart topology, MMU/privilege features and memory map;
- boot firmware, bootloader/firmware path and secure/measured-boot capability;
- buses and device-discovery mechanism (ACPI, Device Tree, UEFI/firmware tables, PCIe, USB, SoC buses);
- storage/network/display/input/audio/camera/sensor/GPU/NPU capabilities;
- IOMMU/DMA coherency, interrupt controller, timers, power and thermal interfaces;
- driver availability, trust tier, update path and failure isolation;
- tested firmware/image hashes, QEMU configuration or physical-board revision;
- unsupported functions and safe fallback behavior.

The initial target is RISC-V64 QEMU `virt`; its successful boot is not physical-board support. Future x86-64, ARM64, RISC-V boards, GPU/NPU, cloud/HPC or embedded targets require their own explicit support and evidence rows.

## 6. Profile cross-product is selective, not assumed universal

Maintain a matrix of supported combinations rather than an unbounded theoretical Cartesian product. Each row must state:
- device class and shell;
- deployment/computation profile;
- hardware/firmware target;
- assurance/security profile;
- resource and power contract;
- included features and compatibility runtimes;
- acceptance suite and qualification status;
- explicit exclusions and known limitations.

A platform milestone may support a declared subset, but the wider approved whole-platform goal remains tracked. Do not silently mark unsupported combinations as supported and do not force a feature onto hardware that cannot provide its required guarantees.

## 7. Governance actions required

1. Reconcile the current five deployment profiles with the device classes in the v4 product goal.
2. Make the orthogonal axes explicit in the architecture/profile specification and support matrix.
3. Define which profile combinations are required for the first usable release and which are whole-platform obligations.
4. Trace Context-centric UI, runtime, identity, storage, update/recovery, capabilities, AI policy and resource management to every supported shell/target combination.
5. Amend constitutional/profile/RFC text only via the current governance and ADR process; this informative mapping does not alter the current specification.
