Classification: Operational
Authoritative Source: docs/Platform-Architecture-Specification-v1.1.md

> Active project execution mandate. The constitutional architecture and RFC/ADR governance remain controlled by the repository's current authority hierarchy; this operational mandate defines the intended product outcome and execution scope.

---

# AetherOS — Master Goal and Execution Mandate

## Version 4.0 — Complete AetherOS platform, fully implemented and integrated as one operating system

**Date:** 21 November 2026  
**Status:** ACTIVE — ultimate completion NOT MET; this mandate remains in force until the full approved AetherOS platform is implemented, integrated, verified and qualified  
**Primary execution environment:** The user's existing local AetherOS workspace, operated through OpenCode  
**Initial reference architecture:** RISC-V 64-bit  
**Initial qualification platform:** QEMU `virt`  
**Preferred first product delivery:** One bootable removable-media image that starts AetherOS in a safe live environment and also provides an installer for a separate target disk; a separately installable disk image is also required.  
**Long-term platform scope:** One coherent AetherOS platform with desktop, laptop, tablet, foldable and mobile operating profiles.

---

# 1. Mission and final outcome

You are the accountable Chief Operating Systems Architect, Principal Systems Engineer and implementation lead for AetherOS. Your task is to take the current integrated RISC-V64 kernel and turn it into a real operating system that a person can boot, interact with, install, update, recover, develop applications for and use for ordinary computing tasks.

The project has crossed an important kernel-integration milestone. The objective is to complete the entire operating-system platform—not merely a kernel, bootable image, first usable release or collection of subsystem crates. Every subsystem must have a real implementation, a defined authority and lifecycle model, all required consumers wired to it, end-to-end behavior, failure handling, tests and documentation. The complete platform must be assembled into a coherent, bootable, installable, updatable and recoverable system with unified services, APIs, policy, UI, runtime, packaging and release artifacts.

**The final product must be a working and usable AetherOS system that supports both of these user journeys:**

1. **Live use:** The user writes an AetherOS image to removable media, boots a compatible RISC-V64 system from it, reaches a usable AetherOS environment, explores and runs applications without changing the host's internal storage, and can shut down or reboot safely.
2. **Installation:** From the live environment, the user launches the AetherOS installer, selects a target drive, reviews the proposed disk changes, explicitly confirms the operation, installs AetherOS, reboots, and reaches the installed system. The installed system retains accounts, settings, applications and user files across shutdowns, reboots and normal updates.

Both journeys must be demonstrated end to end for the first usable milestone and remain supported as the platform evolves. They are necessary but not sufficient for ultimate AetherOS completion. A kernel that prints a boot message, a shell that only runs inside the kernel, an application VM with no protected process model, a mock desktop, a disk image that only boots through a special direct-kernel QEMU command, a network driver whose send/receive methods are stubs, or an installer that merely copies files is not a completed OS capability.

The first usable RISC-V64 release is an intermediate product milestone, NOT the final project goal. The ultimate goal is completion of the entire approved AetherOS platform: every mandatory and accepted SHOULD-level subsystem, service, runtime, security boundary, platform profile, device framework, user experience, developer facility, update/recovery mechanism and ecosystem capability defined by the canonical specifications and accepted decisions must be implemented, integrated into the same coherent system, tested and qualified for its declared targets. Features may be delivered in stages, but work is not complete when the first reference release ships. Any capability not yet implemented remains OPEN in the whole-platform inventory and must never disappear into an untracked “future” category.

Do not stop after writing an audit or roadmap. Research, design, implement, integrate, test, debug, document and continue through dependency-ready work. Continue autonomously for routine engineering decisions. Escalate only genuine owner decisions, unavailable hardware/credentials, paid dependencies, destructive real-device operations or changes to fundamental approved contracts.

# 2. Current milestone context — preserve and verify, do not restart

The user reports that the following state is already complete, committed, pushed and verified:

- Git working tree clean and up to date with `origin/main`.
- Recent CI runs pass, including a QEMU kernel boot.
- An earlier local report recorded 171 tests passing across 12 crates; the latest user-supplied report now records 174 tests passing across the same 12 crates. Treat 174 as the latest reported count, not independently reverified evidence until it is reproduced on the current exact commit.
- The project's check gate passes formatting, checking, tests, Clippy and documentation checks.
- The six items previously listed under `AGENTS.md` “Next Move” have been completed:
  1. Storage and IPC are wired into the kernel main loop.
  2. Shell command processing operates through IPC.
  3. The runtime VM / userspace execution engine is integrated.
  4. A/B partition update and recovery are integrated.
  5. Hardware/release qualification includes CI-verified QEMU boot.
  6. The check gate script exists and passes.

Treat those as **owner-reported baseline claims** until they are reproduced against the exact current local `HEAD`. They are not permission to rewrite, replace or duplicate the completed work. Begin by inspecting the live repository and validating the claims. If they are accurate, record the evidence and move on. If a result is stale, incomplete, misleading or from a different revision, report the discrepancy precisely and repair the root cause without discarding completed work.

In particular, inspect the exact gate-script filename. The user has referred to both `check.sh` and `.check.sh` in the milestone description. Determine which file is canonical from the repository and `AGENTS.md`; do not create duplicate scripts just to satisfy the wording.

The 174-test count is a reported baseline, not a permanent acceptance threshold. The earlier 171 count is superseded by the later report. Add tests as requirements demand. Count and report tests by crate, integration target and test kind; do not use raw test count as a proxy for quality.

The A/B update implementation is an existing milestone. Do not reimplement it blindly. Audit its actual storage layout, generation metadata, boot selection, health confirmation, commit, interrupted-update behavior, rollback and recovery path. Reuse and complete it where correct.

## 2.1 Immediate baseline verification

Before modifying files:

- Identify the exact workspace path, Git root, repository identity, remote URLs, branch, commit SHA and recent history.
- Confirm the current branch policy and all applicable `AGENTS.md` instructions.
- Capture staged, unstaged, untracked, ignored and generated-file state. Inspect any unexpected content before touching it.
- Verify that the local checkout is the actual AetherOS operating-system implementation, not a research directory or unrelated project.
- Read the canonical system specification, ADRs, RFCs, implementation inventory, roadmap, security model, boot documents, hardware notes and current handoff files.
- Run the established verification gate and record exact commands, tool versions, exit codes and test totals.
- Run the QEMU boot workflow and retain the complete serial/firmware/guest log and exit result.
- Inspect the actual `AGENTS.md` Next Move list and mark the six stated items complete only to the extent supported by current source, call paths and tests.
- Produce a short, timestamped baseline report and then immediately select the next dependency-ready implementation task.
- Inspect `AGENTS.md` “Next Move” and all task lists. The six historical Next Move items being complete means only that that list has been exhausted. Preserve its historical completion record, then replace the active Next Move with the next dependency-ready incomplete requirement from the full-platform inventory. Do not let an exhausted short task list mark AetherOS complete. Keep the project-wide goal status ACTIVE until Section 23.4 is satisfied.

Do not assume the repository is clean merely because the previous report said so. Check it. Do not assume all commits on the remote are present in the open local workspace. Check it. Do not pull, reset, rebase, switch branches or otherwise alter repository history without understanding the local state and following repository policy.

## 2.2 Latest reported network-driver work — NOT a completed network feature

The user has supplied a later implementation report stating that the repository now has 174 tests passing across 12 crates and that `cargo build --release --target riscv64gc-unknown-none-elf` succeeds. The report describes new VirtIO network scaffolding in `reference/crates/aether-arch-riscv64/src/main.rs`, including a presumed MMIO base, configuration constants, descriptor/header structs, initialization, and public network functions. It explicitly states that:

- `virtio_net_send()` always returns `false` and falls back;
- `virtio_net_recv()` always returns `0`;
- full device emulation and real packet transfer remain pending.

Classify this result as **network-driver scaffold / partial implementation**, not a working driver, not a working network interface, not a network stack, not an integrated network service and not completion of Phase 3. The successful release build and 174 tests are valuable evidence for the code and tests they actually exercise, but do not prove packet transmission, packet reception, link state, queue completion, IP connectivity or Internet access. Do not repeat the phrase “Phase 3 features complete” for this state.

Before promoting this work, inspect the actual current source and the applicable VirtIO specification. Specifically audit: transport discovery; Device Tree `virtio,mmio` node parsing and interrupt mapping; magic/version/device/vendor IDs; reset and status sequencing; device/driver feature negotiation and `FEATURES_OK` validation; queue size and alignment; queue addresses; descriptor, available and used ring ownership; memory-ordering and cache/DMA rules; queue notifications; interrupts or polling; RX buffer provisioning and replenishment; TX buffer lifetime and reclamation; link state and MAC address; device reset/recovery; malformed device indices/lengths; and safe shutdown.

Do not assume that a hard-coded `0x1000_2000` address is the universal VirtIO network location. It may correspond to a particular QEMU transport slot, but the active platform description is authoritative. Discover and validate the device from the actual firmware-provided Device Tree (or other approved discovery interface) and verify the address and interrupt on each supported profile.

Do not assume an unconditional 16-byte `VirtioNetHdr` is correct. Validate the exact header size, field offsets, endian rules and optional fields against the selected VirtIO version and the features actually negotiated. The retrieved OASIS VirtIO 1.2 specification describes a base network header and feature-dependent extensions; mergeable receive buffers and hash reporting affect the data layout. A fixed layout must be justified by the negotiated feature set and tests.

A real VirtIO-net driver milestone requires working RX and TX virtqueues with correct descriptor/ring ownership, packet buffers, notifications/completions, link/MAC handling, error/reset handling and tests that exchange real Ethernet frames with the QEMU backend. A real network-stack milestone additionally requires Ethernet, address configuration (for example DHCP), ARP/neighbor handling, DNS, routing, transport protocols (TCP/UDP as needed), socket/API integration, user-visible network state and security policy. Prove bidirectional communication with a test service and capture/assert the packets at the host side. A successful `net_init()` alone is not enough.

Candidate Rust network stacks such as `smoltcp` and `embassy-net` may be evaluated rather than automatically adopted. Compare no-std/alloc requirements, async/executor assumptions, RISC-V support, protocol coverage, licensing, maintenance, memory footprint, security posture and integration cost against AetherOS IPC, runtime, service lifecycle and security contracts. Select through evidence and an implementation ADR where the choice affects public architecture.

# 3. Canonical AetherOS vision — preserve the architecture

The canonical AetherOS architecture specification describes **one coherent operating-system platform with multiple operating regimes**, not separate desktop and mobile operating systems.

AetherOS is intended to support, over time:

- Desktop computers.
- Laptops and convertible/hybrid computers.
- Tablets.
- Foldable devices.
- Conventional mobile devices.
- Future specialized profiles only where the shared semantic model remains appropriate.

Common semantics and services should be shared. Platform profiles may specialize scheduler policy, memory reclamation, power and thermal management, process lifecycle, radio/network policy, display and input, hardware drivers, storage optimization, AI routing and the experience shell. A profile must not silently fork the meaning of core services.

The initial RISC-V64 product is the first reference implementation and qualification target. It does not erase the long-term architecture. Conversely, the future mobile and multi-device vision must not prevent the first reference system from becoming a real usable OS.

Preserve the canonical architectural principle:

> Abstract semantic contracts and authority boundaries while keeping implementation mechanisms replaceable wherever no real architectural invariant requires them to be fixed.

Preserve these semantic separations:

1. **UI/HCI:** semantic meaning is distinct from visual presentation and input routing.
2. **Storage:** logical identity is distinct from physical layout and compatibility filesystem presentation.
3. **Security:** principal identity is distinct from privilege authority.
4. **Resources:** Context meaning is distinct from execution priority and physical allocation.
5. **Hardware:** device semantics are distinct from physical resources and driver implementation.

These are semantic boundaries, not automatic requirements for separate processes or separate kernels. Optimization may combine internal implementations only when observable security, ownership, lifecycle, resource, compatibility and semantic guarantees remain intact.

## 3.1 Context-centric experience model

Context is the canonical human-facing unit of activity. Applications remain deployment, execution, packaging and security entities. Do not conflate Context ownership with application ownership, Workspace organization, process identity or security authority.

The UI architecture should converge on correlated:

- **Visual Graph:** what is rendered and where.
- **Semantic Graph:** what controls and content mean, including accessible names, roles, relationships, state and actions.
- **Input Graph:** how pointer, keyboard, touch, pen, controller and automation input is routed and authorized.

Maintain synchronization, provenance, freshness, consistency and privacy boundaries. Structured semantic information is authoritative when available; visual-model interpretation is a controlled fallback, never an excuse to expose unrestricted screen or system access. Accessibility, automation and AI receive policy-derived semantic projections. Input routing and action execution are separately authorized and auditable.

A Context lifecycle must not determine scheduler priority. Providers may participate in a Context, but a process boundary alone is not proof of isolation. Persistent Context state must be encrypted and attributable to its owner; Providers must not receive a Context root key.

## 3.2 Capability-based security

Security must be enforced across the system, not merely represented by types or UI prompts. Preserve least privilege, authority attenuation and non-amplification across kernel, processes, IPC, drivers, services, Contexts, applications, automation and AI.

The security model must define and test capability creation, derivation, transfer, delegation, revocation, expiry, domain binding, scheduling-time authorization, provenance, audit, concurrency and failure behavior. Identity does not confer authority. A process, driver, Provider, Context participant, model or agent must not amplify its authority by crossing a boundary. Human approval is required for consequential operations according to the approved security policy.

## 3.3 Storage and generations

Keep logical storage domains separate from physical partitions. Use physical partitions only where firmware, bootability, hardware, security, recovery or another explicit invariant requires them. Prefer logical separation when it provides equivalent guarantees.

The architecture anticipates system, hardware, application, user, shared, AI, generation/snapshot, recovery and maintenance domains. The exact filesystem, object representation, content-addressing, chunking, copy-on-write, append/version strategy, garbage collection, compaction and media-specific policy must be selected through source inspection, research and benchmarks—not assumed by this mandate.

Immutable system/package data should support integrity and, where useful, content-addressed deduplication. Mutable state needs explicit transactional and crash-consistency semantics. Storage accounting must be attributable and user-understandable. Retain only the amount of physical redundancy required by the reliability objective.

## 3.4 Hardware and driver model

Use a layered hardware model:

- Kernel-authoritative physical resource and protection substrate.
- Hardware discovery and normalized Device Graph with provenance.
- Selectively in-kernel drivers where latency or trust requires them.
- Userspace device services where isolation and restart/recovery are practical.
- Stable high-level system APIs above bus- and vendor-specific mechanisms.

Do not pretend Device Tree, ACPI, UEFI tables, PCIe, USB, firmware and SoC discovery are identical. Preserve their distinct semantics and normalize the results. Model resource ownership, interrupts, DMA/IOMMU, power, reset, hotplug, firmware, compatibility and failure recovery. Do not claim that a driver is isolated or restartable unless the actual hardware ownership model permits it.

## 3.5 Runtime and ecosystem direction

The long-term platform includes:

- Aether Native Runtime and SDK.
- A Web Runtime.
- An isolated Android Compatibility Runtime, preferably using hardware-assisted virtualization where available.
- Other compatibility environments only if research justifies them.

Native APIs remain the primary AetherOS application model. Android compatibility is not the system architecture and must not require cloning the whole Android framework into the OS. Web and Android runtimes must have explicit ABI, filesystem, graphics, input, networking, security, lifecycle and capability boundaries.

## 3.6 AI and automation

AI and automation are platform participants, not privileged exceptions. Inference, perception, retrieval, planning, memory and tool use must operate through declared services and attenuated capabilities. Model outputs are observations or suggestions, not trusted system facts. Provide user consent, bounded authority, revocation, audit, provenance, privacy controls, offline behavior and resource/thermal governance. AI is an important long-term AetherOS capability, but it must not delay the first usable OS or weaken its security model.

# 4. Product scope and release definitions

Maintain explicit separation between the reference release, the complete platform architecture and future research.

## 4.1 First Usable Release — mandatory product outcome

The first usable release is a coherent RISC-V64 AetherOS environment that:

- Boots through a documented firmware/boot path without requiring the developer to inject a kernel through an ad hoc command for every run.
- Provides a live removable-media mode that does not write to attached internal drives by default.
- Provides an installer that can install to a selected target disk in a disposable VM and, after separate hardware qualification, to a supported real RISC-V device.
- Boots successfully from the installed system disk after removing the installer/live medium.
- Provides protected user-mode execution and a usable native application ABI, rather than running all applications inside the privileged kernel.
- Starts essential system services reliably.
- Provides a functional graphical or, where the hardware profile cannot yet provide graphics, a fully usable documented text environment. The declared reference release must include a graphical desktop if it is called a desktop/general-purpose release.
- Supports keyboard and pointer input on the reference platform.
- Provides persistent user files and settings on the installed system.
- Provides working storage, networking, timekeeping, process management, user/session management, logging and shutdown/reboot.
- Includes an integrated file manager, terminal, text editor, settings utility, application launcher/task switching and a useful set of basic applications.
- Provides a native application installation/package workflow with integrity checks and clean removal.
- Provides an accessible and understandable first-run/setup path.
- Updates safely using the existing generation/A-B architecture, verifies the candidate, confirms health and recovers to a known-good generation on failure.
- Has reproducible builds, downloadable artifacts, checksums, build provenance, a known hardware/VM support matrix and complete user/developer installation instructions.

A web browser or functional Web Runtime is a required capability for a full general-purpose release. Research the viable RISC-V-compatible engines and the cost of porting/integrating them with the selected native process, graphics, network and sandbox architecture. Do not claim this requirement met by a static HTML mockup or a web view that cannot load real pages. If the browser engine is a major external dependency, document its licensing, security-update strategy, ABI integration and minimum supported feature set.

The first release does not need Android application compatibility, mobile telephony, every GPU, every peripheral class, cloud AI, all office applications or universal board support. Those remain visible in the long-term scope and may be staged with explicit evidence and release labeling.

## 4.2 Ultimate AetherOS platform completion — the actual final goal

This is the project's final target, not an optional future wishlist. The canonical AetherOS System Specification v1.0, its later convergence sections, approved ADRs, RFCs, normative requirement registries and subsequent owner decisions define the authoritative scope. The local agent MUST locate the live canonical documents and trace every applicable requirement to implementation and verification. The material below is a system-level coverage map to prevent omissions; it does not replace the normative source or authorize changing its MUST/SHOULD/MAY force.

The following platform capability groups remain part of the ultimate goal until each is implemented and integrated, or formally classified according to the canonical specification and governance. A feature being outside the first usable release does not remove it from the ultimate goal.

### A. One coherent multi-profile operating system

Complete one shared AetherOS platform with coherent semantics and explicit profiles for:

- General compute: desktop, workstation where approved, performance laptop and conventional laptop.
- Mobile compute: phone, tablet, foldable and ultra-mobile computer.
- Additional XR, automotive or embedded profiles only when approved and when the shared semantics genuinely fit.

Each profile must have a real hardware support matrix, firmware path, driver set, resource/power policy, application lifecycle policy, shell/input model, supported features, limitations and qualification evidence. Common APIs, capability semantics, storage semantics, package identity, Context model and application contracts remain shared. Profiles may specialize implementation and policy without silently forking core meaning. The final product must not consist of disconnected OS forks or demos sharing only a name.

For each declared profile, complete at least one real reference-hardware implementation and a measurable conformance matrix. The general-compute reference must cover the applicable modern 64-bit CPU/ISA, firmware/boot, memory, persistent storage, graphics/display, keyboard/pointer, network, USB/PCIe, audio, external display and suspend/resume capabilities in the canonical spec. The mobile-compute reference must cover the selected SoC and boot chain, ARM64 or other approved ISA, GPU/NPU where present, UFS/eMMC storage, touchscreen/display, camera/ISP where present, Wi-Fi/Bluetooth, cellular/modem and telephony where present, GNSS, sensors, USB-C/power delivery, battery/charging, thermal and lifecycle behavior. Do not claim a profile complete because a generic profile enum or shell compiles; it requires working hardware support and end-to-end use cases on the named reference device.

### B. Kernel, execution and protection foundation

Complete the selected kernel strategy and all normative execution contracts, including CPU/hart discovery and lifecycle; privilege levels; exception/trap handling; interrupt routing; timers and timekeeping; SMP and inter-processor coordination; physical and virtual memory; address-space isolation; page faults and memory authority; scheduling and thread semantics; process/capsule execution domains; native executable loading; syscall ABI; IPC; event capabilities and wait sets; capability namespace and delegation; resource leases; synchronization; kernel escape prevention; fault containment; and controlled virtualization.

The existing RISC-V64 kernel is the current implementation track and must be preserved and evolved based on evidence. The local agent must reconcile its execution model with the canonical kernel convergence specification rather than assume the present model is already the final architecture. Resolve remaining kernel questions through research and ADRs; do not silently rewrite the kernel or treat a successful VM boot as proof of the entire execution model.

### C. Hardware discovery, device model and driver ecosystem

Complete AHDA and the two complementary hardware truths: kernel-authoritative physical resources and the user-space semantic Device Graph. Implement discovery adapters for the firmware/tables and buses relevant to each target, including Device Tree, ACPI, UEFI, SMBIOS, PCI/PCIe, USB, I2C/I3C, SPI, MIPI, SoC-integrated and vendor mechanisms where applicable. Keep discovery provenance, confidence, resource ownership and validation explicit.

Complete the layered driver architecture: kernel hardware substrate; selective in-kernel drivers; isolated user-space device services; canonical class APIs; versioned driver ABI; driver packaging, signing and trust; firmware management; reset and recovery; capability/resource arbitration; DMA/IOMMU isolation; shared-buffer authority and synchronization; Vendor Compatibility Domains; device hotplug and lifecycle; driver update/rollback; diagnostics; fault injection; and certification tooling. Cover all hardware classes required by the approved profiles—not just the current QEMU VirtIO subset—including CPU/platform, storage, network, USB, input, display, GPU, audio, camera, sensors, radios, power, thermal and security hardware.

### D. Storage, filesystem and data lifecycle

Complete the unified Aether Storage model: logical domains and object identity; user files and namespaces; application state; system generations; immutable content; mutable transactional state; ephemeral state; encryption and keys; ownership and provenance; access control/capabilities; quota/resource accounting; snapshots and generation sharing; content addressing and deduplication where approved; garbage collection; compaction; media-aware write behavior; corruption detection; atomicity and crash consistency; mount/unmount; hot removal; backup/restore; migration; recovery; swap and hibernation policy where supported; arbitrary file preservation; compatibility filesystem views; and transparent, understandable user storage accounting.

All semantics must work through appropriate backends without encoding NVMe, UFS, eMMC, SD, USB or virtual disks as separate incompatible user-facing storage systems. Benchmark media-specific mechanisms and preserve data across updates, migration, failure and recovery according to the declared contract.

### E. Boot, trust, release generations, update and recovery

Complete the boot architecture for all supported profiles: firmware/boot ROM adapters; UEFI/U-Boot/SoC paths as applicable; authenticated boot artifacts and descriptors; acyclic trust bootstrap; owner-controlled trust enrollment where the hardware supports it; verified and measured boot; signing and key lifecycle; secure/measured boot distinction; anti-rollback state; version/compatibility checks; generation construction and sharing; independent firmware/driver/system/application update domains; health manager; first-boot confirmation; automatic fallback; exceptional downgrade policy; recovery environment; reset; backup/export; factory recovery; boot diagnostics; and attestation where available.

The existing A/B or generation mechanism must be audited and completed, not treated as complete merely because two partitions or slots exist. Exercise power-loss and corruption at every transaction boundary. Distinguish VM/test signing from production trust. Never claim hardware-rooted or owner-controlled security on a profile that lacks the required hardware and qualification evidence.

### F. Complete network and communications platform

Complete device drivers and services for wired Ethernet and the approved wireless/radio hardware. Provide the protocol stack and user APIs for Ethernet, ARP/neighbor discovery, IPv4, IPv6, DHCP, DNS, routing, TCP, UDP, TLS, multicast and required application protocols; network interface configuration and diagnostics; capability-based network intents and filtering; connection lifecycle and failure recovery; secure trust-store updates; and network observability. Implement Wi-Fi, Bluetooth, NFC, UWB, GNSS, cellular modem and telephony/radio control for profiles whose hardware includes them. Support offline operation and explicitly report what is unavailable.

The current `virtio_net_send() -> false` and `virtio_net_recv() -> 0` implementation is not a completed network path. The driver, network stack, network service, system UI, application sockets, TLS trust model and end-to-end communications must operate together.

### G. Graphics, multimedia, external display and input

Complete the Aether Graphics Model, 2D/3D APIs, compositor/scene model, display-target abstraction, GPU capability discovery, GPU memory/synchronization/reset/recovery, software fallback, accelerated path where supported, multi-display and docking topology, color management, scaling, rotation, HDR/high refresh/variable refresh where supported, and safe shared multimedia buffers.

Complete the audio graph, device APIs, routing, mixing, capture/playback, synchronization, latency/power policy, spatial/audio effects where supported, and privacy controls. Complete the camera model and mobile pipeline where relevant (sensor, MIPI CSI-2, ISP, imaging, processing, privacy, capture lifecycle). Complete sensor APIs, calibration, timestamps, rate/power governance and privacy. Complete unified input for keyboard, pointer, touch, pen, controllers, haptics, biometrics and accessibility input where present.

A framebuffer demo is not a graphics stack; a displayed screenshot is not a functioning shell; a declared audio/camera API is not a working device. Hardware-accelerated capabilities are claimed only on profiles with working drivers and measured behavior.

### H. Full system service layer

Complete service manager/init, service discovery and typed versioned IPC, service lifecycle and health, independent restart/fault domains, logging, tracing, notifications, clock/time service, device manager, resource manager, storage, package manager, network, display/compositor, input, audio, camera/sensor, identity/authentication, power/thermal, update/recovery, virtualization, application lifecycle and AI services. Services must be integrated through actual boot orchestration and policy, not invoked only by tests or hard-coded in the kernel main loop.

### I. Application runtimes, packages and developer ecosystem

Complete the Aether Native Runtime, stable native ABI and SDK, executable loader, dynamic-linking/runtime model where selected, application manifest/identity, capability/permission declarations, package repository and local install paths, dependency resolution, signature/provenance validation, atomic package transactions, install/upgrade/rollback/removal, app lifecycle, inter-application communication, notifications, background execution policy, developer tools, debugger, profiler, crash diagnostics, SDK docs, sample projects and app certification.

Complete the Web Runtime and real browser capability with supported standards, secure process model, real network page loading, downloads/uploads, storage/privacy policy, sandboxing, web permissions, certificate/trust handling and a security-update path. Complete the isolated Android Compatibility Environment to the compatibility depth defined by the canonical requirements, using hardware-assisted virtualization where available and an explicit host/guest security boundary. Implement the required graphics/input/media/filesystem/network/clipboard/notification and application lifecycle mediation. Provide native/compatibility API separation and, where required, POSIX-like compatibility without introducing ambient authority or leaking guest privilege. Include Linux/other compatibility environments only as specified/approved. Compatibility must be measured by a real application matrix; a window appearing is not a compatibility claim.

### J. Context-centric UI/HCI and all experience shells

Complete the Context-centric interaction architecture. Applications, Providers, Contexts, Workspaces and Activities have distinct identity, ownership, lifecycle, authority and composition semantics. Implement correlated Visual, Semantic and Input Graphs, graph transactions/incremental updates, semantic coverage, freshness and provenance, Context Router, Intent Gateway, focus/input routing, safe automation, accessibility, screen readers, keyboard navigation, notifications, deterministic system actions, UI failure/degraded modes, UI security, UI observability and platform-invariant tests.

Deliver cohesive profile-appropriate shells: desktop/laptop windowing and multitasking; tablet and foldable posture/window transitions; mobile navigation and gestures; multi-display and docking; display/input adaptation; launcher, search, settings, notifications, status, user/session, lock and power UX. UI semantics remain shared; shells specialize presentation. Every control performs an authorized operation against a real system service.

### K. AI, intelligence and automation platform

Complete the Aether Intelligence Layer as a policy-controlled system capability: model registry/management; local, hybrid and cloud inference routing; CPU/GPU/NPU/DSP acceleration; model integrity and provenance; model storage/accounting/eviction; AI memory and data lifecycle; retrieval; Context projections; planning and agents; tool APIs; deterministic system command interface; resource/thermal budgeting; offline and degraded modes; user consent; least-privilege capability grants; revocation; approval for consequential operations; provenance/audit; privacy and data-locality controls; model/provider isolation; and secure integration with every profile. Models remain untrusted perception/reasoning participants, never a root authority or source of truth over structured system state.

### L. Identity, security, privacy and owner control

Complete the trust chain and security architecture: identities/principals; authentication; capability issuance/attenuation/transfer/delegation/revocation/expiry/domain binding; no ambient authority; syscall and IPC validation; kernel/device enforcement; driver isolation; DMA/IOMMU; app/runtime sandboxes; secure storage and Context envelopes; credential/key management; user/owner keys and trust transitions; sensitive hardware permissions; network permissions; privacy classifications and data lifecycle; audit; secure updates; boot trust; anti-rollback; vulnerability response; dependency provenance; and threat models for confused deputies, stale authority, TOCTOU, replay, cross-domain access, compromised providers and hostile devices.

Security claims must match the hardware profile. Do not use empty labels such as “military-grade” or “space-grade”; translate them into testable invariants, formal models, assurance cases and applicable qualifications.

### M. Resource, performance, power and thermal governance

Complete the Resource Manager, Resource Contracts and Resource Leases; CPU/GPU/NPU/DSP/RAM/storage/network/display/radio/battery/thermal accounting; multi-resource arbitration; fairness and starvation prevention; real-time/deadline admission where specified; background execution; pressure prediction; memory pressure ladder; compression/reclaim/suspend/evict semantics; OOM containment; workload priorities; thermal supremacy; power budgets and operating states; wakeup minimization; performance telemetry; adaptive display/radio policy; sustained-performance testing; footprint budgets; and profile-specific policies that preserve common semantics. Measure boot time, latency, throughput, memory, storage, energy and thermal behavior.

### N. Reliability, health, backup, virtualization and recovery operations

Complete service/device fault domains, watchdogs, crash containment, self-healing where safe, health APIs, system self-auditing, diagnostics, event tracing, support bundles, privacy controls, backup/restore, reset/reprovision, virtual-machine management, snapshots, virtual devices, IOMMU-mediated assignment where available, and deterministic hardware/failure simulation. The OS must recover from supported failures without corrupting unrelated services or user data.

### O. Build, system composition, distribution and governance

Complete reproducible system builds, cross-target configuration, generated artifacts from authoritative sources, dependency pinning and auditing, source-tree governance, target-specific composition, image creation, signing, SBOM, provenance, package repositories, live images, installable images, update channels, release channels, hardware profiles/certification, conformance suites, documentation, contribution/driver policies, enterprise policy controls where specified, and long-term maintenance/upgrade/deprecation processes. The kernel, services, runtimes, shells, applications and recovery environment must be assembled by the supported build/release system into coherent artifacts and a single integrated product experience.

### P. Full-system integration is mandatory

AetherOS is not complete when each category has a standalone crate or isolated demo. Every subsystem must be integrated into the dependency graph, service lifecycle, capability model, resource governance, user experience, package/update process, diagnostics and boot/recovery path required by its contract. The integrated system must be able to discover hardware, allocate resources, enforce authority, expose stable APIs, launch applications, handle failure, update and recover as one platform. Cross-layer interactions and profile differences must be tested as system behavior. “Single integrated system” means one coherent OS architecture, common contracts, compatible services/runtimes, unified product composition and release artifacts—not one giant kernel binary. Kernel, system services, drivers, runtimes and applications may remain separate components/processes when that is required for isolation, reliability or modularity, but they must actually interoperate through the approved interfaces and be delivered as one supported platform. A collection of independently passing crates or several unrelated experimental OS images is not integration.

The canonical specification contains detailed architecture sections, invariants and dependency graphs. The local agent must build a bidirectional inventory against the *entire* current canonical spec, including its later convergence additions (not just the initial chapters or an old stage list). Preserve normative strength: MUST requirements are mandatory; SHOULD requirements need evidence-based implementation or an explicit approved deviation; MAY features remain recorded and are implemented where approved. A summary or a hand-picked subset cannot replace this inventory.

# 5. Reference platform and boot strategy

## 5.1 First target: QEMU RISC-V64 `virt`

Use QEMU `virt` as the deterministic reference virtual platform for development, CI, install/live testing and release qualification. It is a virtual platform, not a physical-board certification. Keep that distinction explicit.

Use a controlled machine profile with a recorded:

- QEMU version and machine type.
- CPU model and ISA extensions.
- Hart count.
- RAM size.
- Firmware and bootloader versions/hashes.
- Device Tree or ACPI configuration.
- Storage controllers and disk topology.
- Network device and network backend.
- Display/GPU device.
- USB controller and attached removable-media device.
- RNG, RTC and other virtual devices used by the system.

Exercise realistic multihart operation rather than relying exclusively on a one-hart boot.

## 5.2 Firmware and privilege model

Research and implement the correct RISC-V boot chain for the selected QEMU profile. The practical baseline to evaluate is:

1. QEMU machine model.
2. Machine-mode firmware, normally OpenSBI or another explicitly qualified SBI implementation.
3. UEFI firmware (EDK2 is the reference implementation to evaluate on QEMU `virt`) or a documented board-specific U-Boot/firmware adapter.
4. AetherOS boot manager/loader.
5. Authenticated boot manifest and selected system generation.
6. AetherOS supervisor-mode kernel.
7. Userspace runtime, services and shell.

AetherOS kernel code should execute in the intended supervisor privilege level; ordinary applications and user services must execute in user mode with protected address spaces. Do not accidentally keep the entire operating system in machine mode just because it makes initial bring-up easier. If the existing kernel uses a different privilege strategy, audit its rationale, security boundary and migration cost before changing it.

Use SBI feature discovery and version/extension checks. Inspect the current RISC-V SBI and Boot and Runtime Services specifications. At a minimum, account for hart state management and multi-hart bring-up, timer service when the relevant hardware timer extension is absent, inter-processor interrupt and remote fence mechanisms when hardware interrupt extensions are absent, reset/shutdown, console/debug facilities and error behavior. Do not blindly assume every SBI extension exists. Keep firmware calls behind a small, versioned, testable platform interface.

Parse and validate the actual Device Tree or ACPI data supplied by the selected firmware. The kernel must not rely on hard-coded QEMU RAM sizes, interrupt IDs, peripheral addresses or device counts when those can be discovered. Validate table bounds, addresses, overlap, malformed properties, interrupt domains and reserved memory before use.

If UEFI is used on RISC-V, inspect and honor the RISC-V UEFI protocol requirements, including the boot-hart identity handoff. Handle UEFI memory map ownership and `ExitBootServices` correctly if the boot design crosses that boundary. Never assume the RISC-V boot register convention for a direct firmware payload is identical to UEFI's application calling convention.

## 5.3 Removable-media boot

For the UEFI reference path, create a standards-conforming removable-media layout with an EFI System Partition readable by firmware and the architecture-specific fallback application path:

`\EFI\BOOT\BOOTRISCV64.EFI`

The EFI image must be a genuine RISC-V64 PE/COFF EFI application with the correct machine type and calling convention. It must locate and verify the selected AetherOS manifest, kernel, init data and system payload; validate sizes and addresses; pass required firmware tables and boot metadata; and fail safely with useful diagnostics.

Do not assume every RISC-V board implements UEFI, supports USB mass-storage enumeration in firmware, or supports the same boot order. For each physical board profile, document whether boot is through UEFI, U-Boot, ROM/SPL, SD, eMMC, USB or another vendor mechanism. Board-specific adapters may be necessary. The live-media product must support removable USB boot on at least one explicitly qualified physical reference board before claiming physical USB boot. If the available hardware cannot boot USB, document and test the nearest supported removable-media route (for example SD) without relabeling it USB support.

The standard UEFI removable-media file path and FAT filesystem requirements are interoperability mechanisms, not proof that a particular firmware has working USB host drivers. Verify both the firmware and media path.

## 5.4 Boot artifacts and formats

Produce a versioned, reproducible artifact set appropriate to the actual boot firmware:

- A raw removable-media disk image (`.img`) suitable for writing to a USB flash drive on qualified hardware.
- A compressed distribution of the raw image (`.img.xz`, `.img.zst` or another selected format) with checksum and verification instructions.
- A QEMU boot artifact or disk image that can be run with the documented reference firmware and device topology.
- An installed-system image or installer target image for automated VM testing.
- An optional optical/ISO image only if the selected RISC-V firmware/boot flow supports it; do not assume that a generic x86 hybrid ISO recipe is correct for RISC-V.
- A manifest describing image layout, firmware assumptions, supported ISA/profile, versions, hashes and build provenance.

A raw USB image must have a deterministic partition table and a well-documented layout. Prefer GPT where supported by the reference firmware. Use a firmware-readable EFI System Partition where UEFI is the selected boot route. The system/live payload and persistent state must be represented according to the existing AetherOS storage and generation design, not forced into arbitrary fixed partitions without an architectural reason.

# 6. Live mode requirements

Live mode is not simply an installer image that happens to launch a kernel. It is a safe, usable operating environment.

Required behavior:

- Detect that the OS is running from removable media and enter an explicit Live session.
- Keep the live system base read-only or otherwise integrity-protected.
- Put transient writable state in RAM or a clearly identified temporary overlay by default.
- Do not automatically mount internal disks writable.
- Do not automatically create, resize, format, encrypt or repartition any attached disk.
- Do not modify host boot entries, EFI System Partitions, firmware settings or existing operating systems.
- Show a clear Live Mode indicator and explain that unsaved volatile data is lost at shutdown unless an explicitly designed persistence option is enabled.
- Make the desktop, core applications, terminal, diagnostics, networking and help available in live mode.
- Allow users to inspect detected hardware and storage without granting write access implicitly.
- Provide a deliberate, authenticated path from the live session to the installer.
- Keep system logs useful but bounded; offer export to removable media only through an explicit user action.
- Shut down and reboot cleanly, including correct firmware/SBI reset handling.

Test the live environment with multiple attached disks, unusual partition tables, empty disks, corrupted filesystems, read-only media, full media, missing network, unplug/replug events and unexpected power interruption. The default live boot must never write to the internal installation target.

# 7. Installer and installed-system requirements

## 7.1 Installer as a real product component

Provide a functional installer reachable from the live desktop. Provide a robust text/serial installer path for recovery and headless environments if the graphical stack is unavailable.

The installer must:

- Identify disks using stable device identity where available, not only transient enumeration order.
- Display model, capacity, transport, current partition information and the exact target selected.
- Distinguish removable source media from installation targets.
- Validate target capacity, firmware boot requirements, partition layout, filesystem support and bad-device errors.
- Offer a safe guided install and a documented advanced/manual path appropriate to the first release.
- Explain every destructive change before execution.
- Require explicit confirmation immediately before writing to the target.
- Never infer the target merely because it is the only disk visible.
- Prevent the user from selecting the live source medium as the target unless a supported, fully understood workflow explicitly permits it.
- Avoid overwriting unrelated disks or EFI partitions.
- Verify the target is not mounted or actively used by another system operation before destructive steps.
- Create the required boot metadata, system generation and persistent user-data layout.
- Install the boot manager/EFI application in the correct location without destroying unrelated boot entries.
- Verify every written artifact and filesystem after installation.
- Record a transaction log and provide useful errors and recovery instructions.
- Handle cancellation, failure, low space, I/O errors and interruption without leaving the target falsely marked as installed.
- Reboot only after explicit user action.

Never run installer tests against the developer's real disk. Use disposable QEMU images and verify the exact output paths before any image-writing command. Physical installation testing requires a separate test device, an explicit device-specific plan and owner approval.

## 7.2 Installed-system first boot

After installation and removal of the live medium, the system must:

- Boot from the installed target by its documented firmware path.
- Select a valid committed system generation.
- Verify boot-critical integrity according to the current trust model.
- Mount persistent system and user storage correctly.
- Start the service manager and essential services in dependency order.
- Present first-run setup if needed.
- Create or configure the initial user securely.
- Establish a session and launch the graphical shell.
- Retain user files and settings across shutdown/reboot.
- Report boot failures and enter a tested recovery path rather than silently looping.

## 7.3 Installer safety and recovery

Test the installer using disposable target images with:

- A blank disk.
- A previously installed AetherOS disk.
- A disk with another OS and EFI System Partition.
- Multiple disks attached.
- A nearly full target.
- Read-only target.
- I/O errors and simulated disconnection.
- Corrupted partition metadata.
- Repeated installation and repair.
- Cancellation at each destructive step.
- Power loss or forced QEMU termination at defined transaction boundaries.

Document exactly which install modes are supported. Do not claim safe dual-boot, preservation of another OS, filesystem migration or disk encryption until tested end to end.

# 8. Kernel-to-userspace completion

The integrated runtime VM is an important existing component. Audit its real semantics and preserve it where correct. Do not assume a VM execution engine by itself provides a protected userspace operating system.

The reference release must have a clearly defined and enforced execution model:

- Kernel privilege and userspace privilege are distinct.
- User processes have independent address spaces and memory permissions.
- System services and applications execute outside the kernel's privileged address space unless a formally justified exception exists.
- The kernel validates syscall arguments, pointers, lengths, identifiers and state transitions.
- IPC transfers data and capabilities according to explicit ownership and authorization rules.
- Process and thread lifecycle includes creation, scheduling, blocking, wakeup, exit, wait, crash handling and cleanup.
- Resource exhaustion and service failure have bounded, diagnosable behavior.
- Debugging and tracing do not expose secrets or bypass the security boundary.

## 8.1 Native executable format and ABI

Research and define a stable AetherOS native executable contract. Prefer interoperability with the standard RISC-V ELF64 and ratified RISC-V psABI conventions where technically appropriate. Confirm the exact ABI variant, code model, floating-point convention, relocation model, TLS, stack alignment, calling convention, ELF loader requirements, dynamic-linking decision and syscall ABI from the live toolchain and current normative specifications.

Do not make a draft or experimental ABI the platform contract merely because it appears in a recent document. Pin the chosen ABI and toolchain versions. Keep the AetherOS syscall interface separately versioned from the RISC-V hardware ABI.

Implement and test:

- ELF64 validation and loading.
- Program header and segment validation, including overflow, overlap, alignment and permission checks.
- Relocations required by the selected linking model.
- Entry-point and stack setup.
- User/supervisor transition and trap return.
- Argument/environment delivery.
- System call entry and error conventions.
- Shared-library/dynamic-loader support if required by the selected application model.
- Process termination, signals/events or their approved Aether equivalents.
- Debug symbols and crash diagnostics.

## 8.2 Runtime VM and native process relationship

Document whether the current VM runs Aether bytecode, managed applications, sandboxed tasks, command programs or another defined artifact. Make it an explicit runtime with versioned bytecode, verifier, resource accounting, capability boundary and host API. Ensure malformed programs cannot corrupt the kernel or other processes.

The VM must complement—not conceal the absence of—native U-mode isolation and a process/syscall model. Define which system components use native ELF processes and which use the VM. Provide a small end-to-end demonstration in which a separately loaded userspace program communicates with a system service over IPC and performs authorized filesystem or display operations without executing as kernel code.

## 8.3 Core system services

Establish a minimal service manager/init system and a declared dependency graph. At minimum, the installed reference environment needs:

- Storage and filesystem service.
- Device discovery/device manager.
- Process/session and user identity service.
- IPC/name or service-discovery mechanism.
- Input service.
- Display/compositor service.
- Network service.
- Time service.
- Audio service where supported by the declared profile.
- Logging/diagnostics service.
- Update and recovery service.
- Package/application service.

Every service needs a versioned interface, lifecycle, authority set, resource budget, restart/failure behavior and meaningful health signal. No service should be considered integrated merely because its crate compiles or is manually invoked by a test harness.

# 9. Core hardware and device support

The first QEMU profile should exercise actual virtual devices rather than hard-coded test substitutes.

Prioritize and qualify:

- UART/serial console for early boot and recovery.
- Device Tree discovery and validation.
- Interrupt controller and timer services.
- Multi-hart start/stop/synchronization.
- Physical and virtual memory management.
- VirtIO block storage.
- VirtIO network.
- VirtIO input or USB HID keyboard and pointer.
- VirtIO GPU or the selected QEMU display path.
- USB xHCI and USB mass storage for live-media boot tests.
- RTC/timekeeping and random-number source.
- Audio device if available and required by the chosen profile.

Use a layered VirtIO transport/device implementation. Validate device features and negotiate only supported features. Implement descriptor/ring ownership, DMA/cache coherency assumptions, interrupt/poll behavior, bounds, reset, queue recovery and malicious/malformed device behavior. The driver must not trust device-produced lengths or indices.

After QEMU, qualify one physical RISC-V board as a distinct profile. Select that board from evidence: available hardware, boot firmware, upstream kernel/device-tree/driver status, graphics/display availability, USB mass-storage boot, network, storage, RAM, CPU extensions, firmware maintenance and documentation. Do not choose a board solely from advertised GPU specifications. Record whether every driver is upstream, vendor-maintained, proprietary, custom, incomplete or unsupported.

The user's currently reported QEMU boot does not prove physical USB boot, GPU acceleration, HDMI, Wi-Fi, Bluetooth, NVMe, USB storage, audio or suspend/resume on any board.

# 10. Graphics, desktop and human interaction

A release called a desktop/general-purpose operating system must provide a functioning desktop, not merely a console or screenshot.

## 10.1 Graphics substrate

Research and choose the least complex viable graphics path for the reference target. Evaluate:

- UEFI GOP framebuffer handoff where supported.
- VirtIO-GPU on QEMU.
- Kernel framebuffer/DRM-like device boundary and modesetting policy.
- A software rasterizer fallback.
- GPU acceleration only when a real driver and userspace stack exist.
- Buffer ownership, synchronization, clipping, damage tracking and safe presentation.
- Multi-resolution, scaling and display hotplug as supported.

A framebuffer may be a valid first display backend; it is not proof of 3D acceleration. Label software-rendered and hardware-accelerated modes accurately.

## 10.2 Compositor/windowing/shell

Deliver a real shell composed of functioning components:

- Desktop/workspace surface.
- Window and surface management.
- Compositor and event loop.
- Launcher/application menu.
- Task switcher and taskbar/dock equivalent.
- Window focus, movement, resizing and stacking.
- Keyboard shortcuts and system navigation.
- Pointer and keyboard input.
- Display settings and resolution selection where supported.
- Notification/status region with time, network and system state.
- Application launch, close and crash handling.
- Accessibility semantics and keyboard navigation.
- User/session and lock/screen policy appropriate to the first release.

The desktop must launch actual userspace applications. Controls must invoke real system services, not merely change local UI state.

The UI design should have a distinctive, coherent AetherOS visual language and accessible contrast, typography, spacing, focus indication and scaling. Do not copy a generic desktop literally, but do not spend time polishing nonfunctional mock screens. First establish the complete interaction model and behavior, then refine visual design.

## 10.3 Essential applications

The first usable graphical release should include or provide verified installation of:

- File manager with safe file operations.
- Terminal/console application.
- Text editor.
- System settings.
- System monitor/diagnostics.
- Application launcher and package manager UI or CLI.
- Basic image/document viewing or a useful content viewer.
- Network configuration and status.
- Help/documentation viewer.
- Web browser/Web Runtime for a full general-purpose release.

Each application must execute as a proper userspace entity under the relevant process/runtime and capability model.

# 11. Networking, storage and persistence

## 11.1 Networking

Implement a complete minimal network path for the reference profile:

- Device discovery and network interface lifecycle.
- Link state and device reset.
- Ethernet framing.
- IPv4 and, according to release scope, IPv6.
- DHCP or equivalent supported configuration.
- DNS resolver.
- Routing and local interfaces.
- TCP and UDP as needed by the chosen applications.
- TLS using maintained cryptographic implementations and a secure trust-store update strategy.
- User-visible connection state and diagnostics.

Prove real communication with an external test endpoint and local network services. A mocked packet exchange or successful interface initialization does not prove network usability. Document what works offline and what requires a network.

A VirtIO-net implementation is not complete until all of the following are real and wired into the platform: (1) the firmware-described transport is discovered and validated; (2) reset, status, negotiated features and queue setup follow the exact supported VirtIO transport/specification; (3) RX descriptors are populated with safe buffers before the device is enabled; (4) TX descriptors carry actual Ethernet frames and are notified, completed and reclaimed; (5) received buffers are validated, surfaced and replenished; (6) link/MAC/status and reset conditions are handled; (7) real Ethernet traffic is exchanged in both directions with QEMU or qualified hardware; (8) the network interface is connected to the network stack/service; (9) DHCP/static configuration, ARP/neighbor discovery, DNS, routing and TCP/UDP behave correctly; (10) a userspace application uses the socket/network API; and (11) packet capture, counters, fault tests and user-visible diagnostics confirm the behavior.

Tests must include empty/full queues, malformed descriptors, invalid lengths/indices, descriptor chains, queue wraparound, concurrent enqueue/dequeue, notification suppression, interrupt/poll races, RX buffer exhaustion/replenishment, link down/up, device reset, queue reset, timeouts, cancellation, memory pressure and hostile device behavior. Use QEMU's actual VirtIO network device/backend, not a hand-written mock alone. Keep host-side networking isolated and reproducible.

## 11.2 Persistent filesystem

Audit the existing storage implementation and select the filesystem/object-store strategy based on correctness, testability, tooling and target requirements. Do not replace an existing implementation merely because another filesystem is fashionable.

Required semantics include:

- Stable file/object identity.
- Directory and namespace operations.
- Ownership and access control.
- Capability-mediated access.
- Atomic create/replace/rename behavior as defined by the API.
- Crash consistency and recovery.
- Data and metadata integrity checking.
- Disk-full and I/O-error handling.
- Quotas/accounting where specified.
- Persistent user settings and application data.
- Safe mount/unmount and device removal behavior.
- Backup/export and recovery affordances.

Run repeated write/read/rename/delete/reboot tests, corruption tests, bounded fault injection and interrupted-operation tests on disposable disk images. Verify the installed system retains files after QEMU process termination and a full restart.

## 11.3 Live and installed storage semantics

Live media: immutable or integrity-protected base plus volatile writable state by default. Optional persistent live storage may be added only with explicit user choice and a clearly separate persistence area.

Installed media: persistent AetherOS system generations and user/application state using the existing A/B design where its guarantees are verified. Preserve user data across normal OS generation updates. Make rollback semantics explicit: identify which system state rolls back and which user data is deliberately retained. Never conflate rollback of the OS with restoring arbitrary user data.

# 12. Updates, boot trust and recovery

The existing A/B update/recovery implementation is a foundation to verify and complete.

The release update lifecycle must be:

1. Acquire or construct an update candidate.
2. Authenticate the update source and validate metadata, compatibility, size and dependencies.
3. Verify all required artifacts before they become bootable.
4. Stage the candidate without corrupting the active generation.
5. Record an unambiguous pending generation and boot attempt state.
6. Boot the candidate.
7. Run meaningful boot and service health checks.
8. Commit only after successful health confirmation.
9. On failure or interrupted update, select the last known-good generation and report the failure.
10. Protect rollback and anti-rollback state according to the actual platform trust capabilities.

Test interruption during download, verification, staging, metadata update, first boot, health checks, commit and rollback. Test low storage, corrupt payload, wrong signature, wrong target, downgraded version, missing firmware feature and failed service. Use deterministic fault injection on disposable images.

Development signing and production trust are separate. Development keys must never be represented as production identities. Do not create, publish or request private signing keys in chat or the repository. Production secure boot, measured boot, TPM/TEE, key enrollment and anti-rollback guarantees must be claimed only when the reference hardware and implementation support them and tests prove the stated boundary.

Recovery must be independently bootable or otherwise demonstrably reachable when the primary generation fails. It must provide logs, hardware/storage diagnostics, generation repair/selection and safe data export. Normal boot should be automatic; manual generation selection belongs in authenticated recovery/maintenance rather than an ordinary boot menu unless the approved UX decision states otherwise.

# 13. Application packaging and developer usability

A usable OS needs a path for users and developers to install and remove software without manually copying binaries into privileged locations.

Create a small, secure package model with:

- Stable package/application identity.
- Version and compatibility metadata.
- Executable/runtime declaration.
- Capability/permission declaration.
- Dependency metadata.
- Integrity hashes and signature verification.
- Atomic installation and rollback on failure.
- Clean uninstall and ownership tracking.
- Update and repository metadata.
- Clear distinction between system packages and user applications.
- Offline package installation for live/demo environments.
- Reproducible package creation and verification.

Do not build a complex app store or plugin framework before the secure local package path works.

Provide an AetherOS SDK/developer workflow with documented compiler/linker target, headers/libraries, runtime APIs, application manifest, package creation, installation, debugging, logs, tracing, crash reports and sample applications. A developer must be able to build a small program outside the kernel, package it, install it, launch it from the shell, use authorized services and remove it cleanly.

# 14. Research mandate — research and implement in parallel

Research is continuous and must directly inform implementation. Do not create a separate research-only project that delays completion, and do not adopt a technology solely because it is new.

Use primary sources first: RISC-V International specifications, UEFI Forum specifications, QEMU docs, OpenSBI docs, TianoCore/EDK2 docs, U-Boot docs, official compiler/ABI documentation, maintained upstream projects, security advisories and relevant peer-reviewed work. Use vendor docs for hardware facts and label vendor-only claims.

For each substantial choice, compare realistic alternatives using:

- Requirement coverage and correctness.
- Project maturity and maintenance activity.
- RISC-V64 support and exact extension/ABI needs.
- Compatibility with the current kernel and existing code.
- Security model and attack surface.
- Performance, memory and storage footprint.
- Boot/update/recovery implications.
- Availability of drivers, firmware and test hardware.
- Licensing and redistribution obligations.
- Build reproducibility and dependency health.
- Long-term maintenance and cost of divergence.
- Testability, fault injection and formal verification potential.

Maintain a technology radar with states such as Discovered, Researching, Experimental, Prototyped, Approved, Integrated, Validated, Qualified, Production, Deprecated and Rejected.

## 14.1 Required research domains

Research and make evidence-based choices for:

1. RISC-V privilege modes, ISA profiles/extensions, SBI versions/extensions, memory model and hart lifecycle.
2. QEMU `virt`, OpenSBI, EDK2 RISC-V UEFI, U-Boot and direct firmware payload boot trade-offs.
3. RISC-V UEFI handoff, Device Tree/ACPI discovery, UEFI memory map and removable-media behavior.
4. RISC-V ELF64, ratified psABI, toolchains, linker, relocations and dynamic loading.
5. Native user process model, VM design, syscall ABI and IPC semantics.
6. Memory isolation, Sv39/Sv48 availability, TLB shootdown, page permissions, PMP/IOMMU and DMA safety.
7. VirtIO block/network/GPU/input and real-board device drivers.
8. USB host controller, mass storage, HID, NVMe/MMC and removable-media behavior.
9. Storage options, transactional filesystems, snapshots, immutable systems and A/B update designs.
10. Graphics, framebuffer, compositor, windowing, accessibility and GUI toolkit choices.
11. Networking, TLS, certificate store, resolver and offline behavior.
12. Audio and media support for the chosen reference profile.
13. Init/service manager, user/session management, logging and diagnostics.
14. Application ABI, native runtime, Web Runtime/browser engine and package manager.
15. Installer, image builder, image verification, firmware compatibility and release distribution.
16. Secure boot, measured boot, owner-controlled keys, recovery and vulnerability response.
17. Formal methods, state-machine/model checking, fuzzing, property-based testing, sanitizers and fault injection.
18. Build reproducibility, CI artifact provenance, SBOM and license/security scanning.

Research outputs must identify the precise source/version/date, what the source actually guarantees, what is proposed, implementation impact, decision, rejected options, risks and verification plan. Never present an upstream roadmap, RFC, patch series, proof-of-concept or vendor feature table as a production-ready capability.

# 15. Implementation roadmap and exit criteria

This is a dependency-aware sequence, not permission to stop after a phase document. Close each phase with evidence and immediately continue to the next eligible task.

## Phase 0 — Preserve, verify and map the current system

**Objective:** Establish the exact baseline and avoid duplicating completed work.

Deliver:

- Current repository/worktree and environment report.
- Reproduced build/test/check/QEMU boot evidence.
- Architecture and source call-path map.
- Requirement-to-code-to-test matrix.
- Current system boundary diagram.
- Gap, risk, security and dependency registers.
- A clear statement of which six previous Next Move items are verified complete and which require corrections.

**Exit:** No undocumented assumption about current status remains on the critical path. Existing working boot and update paths are preserved.

## Phase 1 — Define the RISC-V64 reference machine and boot contract

**Objective:** Convert the current kernel boot into a reproducible standard platform boot path.

Implement or complete:

- Exact QEMU `virt` machine profile.
- Firmware/SBI adapter and extension negotiation.
- Multi-hart startup and failure handling.
- DTB/ACPI parsing with validation.
- UEFI/EDK2 boot integration where supported, including correct RISC-V boot-hart handoff; retain a documented direct OpenSBI path for development/recovery if useful.
- Aether boot manager and authenticated manifest.
- EFI removable-media boot file at the standard RISC-V64 path when using UEFI.
- Debug serial logs and boot diagnostics.

**Exit:** A clean build produces an image that boots through the documented firmware path without ad hoc host-side kernel injection. Boot metadata and memory/device discovery are validated. CI exercises both successful boot and defined firmware/handoff failures.

## Phase 2 — Protected native userspace and system services

**Objective:** Establish the actual OS process model.

Implement or complete:

- U-mode process execution and isolation.
- Stable ELF64 loader and selected RISC-V ABI.
- Versioned syscall ABI.
- Process/thread lifecycle and scheduler integration.
- Memory ownership, page permissions and safe copy-in/copy-out.
- IPC authorization and capability transfer.
- Service manager/init and service discovery.
- Logging, diagnostics, crash reporting and service health.
- Explicit integration between the existing VM and native processes.

**Exit:** A separately loaded userspace application runs without kernel privilege, is isolated from another application, calls a service through IPC, performs an authorized operation, and can be terminated/restarted without corrupting the kernel or another process. Negative tests prove access denial and fault containment.

## Phase 3 — Storage, device and network vertical slices

**Objective:** Make the OS capable of reading, writing, communicating and discovering real devices on the reference platform.

Implement or complete:

- VirtIO block and persistent filesystem mount.
- Safe storage service and VFS contracts.
- VirtIO network, DHCP/configuration, DNS and transport protocols needed for real connectivity.
- Input and display devices.
- USB controller and mass-storage support sufficient for removable boot/testing.
- Clock/randomness/device service integration.
- Device Graph and provenance.

**Exit:** Userspace applications can read/write persistent files, survive restart, access a network service and receive keyboard/pointer/display events via system services. Hardware discovery comes from actual firmware/device interfaces, not hidden QEMU constants.

## Phase 4 — Functional graphical desktop

**Objective:** Deliver a working visual system shell.

Implement or complete:

- Display backend and compositor.
- Window/surface lifecycle.
- Input routing and focus.
- Desktop/workspace, launcher, task switcher and status/notification area.
- Settings, file manager, terminal and text editor.
- Native application launch and exit.
- Accessibility semantics and keyboard navigation.
- Display scaling and resolution behavior for the reference profile.

**Exit:** A human can boot to the desktop, log in or complete first-run setup, open multiple real applications, use the keyboard and pointer, manage files, edit and save a document, change a setting, inspect system state and shut down/reboot normally. No core interaction is a mock.

## Phase 5 — Live media and installer

**Objective:** Deliver both preferred user entry paths.

Implement or complete:

- Reproducible raw USB image with GPT/ESP and RISC-V64 EFI boot path where UEFI is used.
- Live-mode detection and read-only/volatile base semantics.
- No automatic internal-disk writes or mounts.
- Integrated graphical installer plus text/serial fallback.
- Safe disk discovery, selection, confirmation and transaction log.
- Installation of boot manager, system generation and persistent user domain.
- Image validation and safe failure/rollback.
- QEMU USB-storage boot test, plus separate virtual installation disk.

**Exit:** One QEMU scenario boots from a virtual USB storage device into the live desktop; installs to a separate blank virtual disk; powers off; detaches the live device; boots from the installed disk; and verifies user files/settings persist. A second scenario proves the live environment does not modify an attached internal disk. The same raw image is inspected and documented for writing to physical USB media.

## Phase 6 — Package, application and network usability

**Objective:** Make the system useful beyond built-in demos.

Implement or complete:

- Native package manifest and repository/local package workflow.
- Signature/hash verification and atomic install/remove/update.
- SDK and sample applications.
- Application permissions/capabilities.
- Browser/Web Runtime research, integration and actual network page-loading test.
- Help and diagnostics UX.

**Exit:** A developer builds and packages a userspace application; a user installs it using the supported package workflow, launches it from the shell, uses authorized APIs, updates it and removes it. Full general-purpose release also demonstrates a real web browser/Web Runtime loading ordinary web content under the approved security boundary.

## Phase 7 — Update/recovery, security and release hardening

**Objective:** Qualify the actual installed product, not just an image that boots once.

Implement/test:

- Existing A/B generation update flow end to end.
- Signature/hash/version/target checks.
- Candidate boot health and health-commit.
- Failed update rollback.
- Recovery boot and data export.
- Filesystem durability and fault injection.
- Security review of syscall, IPC, capabilities, drivers, installer, boot and package paths.
- Dependency/license/security audit.
- Reproducible image generation and SBOM.

**Exit:** The system survives defined update interruption and corruption scenarios, returns to a known-good generation, preserves user data according to the documented policy, and has no unresolved critical/high defect in shipped paths. All release-blocking tests pass.

## Phase 8 — Physical RISC-V hardware qualification

**Objective:** Move beyond a virtual reference platform without overstating hardware support.

Select one board from researched evidence and, only when hardware is actually available, qualify:

- Firmware version and boot path.
- USB removable-media boot if supported.
- Persistent storage installation.
- UART/serial recovery.
- CPU/hart startup and ISA features.
- Memory and interrupts.
- Display/HDMI and input.
- Network.
- USB peripherals and mass storage.
- Audio and power management as claimed.
- A/B update, rollback and recovery.

**Exit:** The physical device can boot the live image (USB specifically if the board firmware supports it), install to its supported storage, boot the installed OS, use the claimed peripherals and recover from a failed update. Publish exact board revision, firmware, image hash, unsupported features and test evidence.

## Phase 9 — Stable first release

**Exit:** All First Usable Release requirements are `Verified`; all applicable release gates pass; images and manifests are reproducible; documentation is complete; no critical/high release blocker remains; QEMU and at least one physical profile have honest qualification results, or the release is explicitly labeled QEMU-only until physical hardware qualification is completed. Do not use the phrase “hardware-qualified” for VM-only evidence.

**Critical interpretation:** Phase 9 closes only the first usable reference-release milestone. It does NOT close the AetherOS project, satisfy the complete platform inventory or permit the agent to report the ultimate goal as met. Immediately continue through the platform-completion phases below.

## Phase 10 — Complete shared platform semantics and kernel execution contracts

Complete every approved cross-platform semantic contract, kernel invariant and execution-model item from the canonical architecture, including the later kernel convergence requirements. Reconcile the current RISC-V kernel's privilege model, capsule/process model, native syscall model, capability namespace/rings, thread semantics, scheduling/resource leases, interrupt/event/wait-set semantics, memory authority, async IPC options, fault isolation and native/compatibility execution domains. Implement and integrate the chosen designs; do not stop at ADRs or type definitions.

**Exit:** Each mandatory kernel/execution invariant is traced to enforced code paths, negative and concurrency tests, integration tests and a system-level user/process scenario.

## Phase 11 — Complete AHDA, driver framework and target hardware profiles

Complete device discovery, Device Graph, bus and class interfaces, resource arbitration, driver placement/isolation, DMA/IOMMU, hotplug, reset/recovery, firmware management, driver package/signing/rollback and target-specific hardware support. Add and qualify the approved RISC-V physical profile(s), then implement other approved architecture/profile targets from the canonical matrix.

**Exit:** Every declared hardware profile has a documented discovered-device matrix, working essential drivers, explicit unsupported devices, fault/recovery evidence and conformance results. No QEMU-only claim is presented as physical coverage.

## Phase 12 — Complete storage, multimedia, graphics and external-device capabilities

Complete storage backends and semantics, snapshots/content addressing/GC/backup/restore, external displays, docking/topology, GPU/compute interfaces, multimedia shared buffers, audio graph, camera/ISP, sensor framework, USB classes, input devices and device health. Add hardware-specific drivers and user-space services by profile.

**Exit:** All required class APIs have at least one genuinely working backend per applicable profile, cross-service integration is exercised, and failure/power/thermal and persistence behavior is qualified.

## Phase 13 — Complete the communications and connectivity ecosystem

Complete network stack and policy, wired and wireless networking, Bluetooth, NFC, UWB, GNSS, cellular/telephony where applicable, network intent enforcement, secure name resolution, trust-store operations and user-facing diagnostics.

**Exit:** The networking and radio inventory is complete for each platform profile; real hardware/virtual endpoints are tested bidirectionally; unsupported radios are explicitly identified rather than represented by stubs.

## Phase 14 — Complete all runtimes, package management and application ecosystem

Complete Aether Native Runtime/SDK/ABI, native application lifecycle, package management, repository workflows, dependency management, signed package transactions, Web Runtime/browser and Android Compatibility Environment to the depth required by the canonical spec. Complete driver SDK/package lifecycle and compatibility test matrices.

**Exit:** Developers can build, debug, package, sign/test-sign, install, run, update and remove supported applications; users can manage applications through the integrated shell; compatibility claims pass real application suites.

## Phase 15 — Complete Context-centric interface and all platform shells

Complete Context, Provider, Workspace and Activity semantics; Visual/Semantic/Input Graph; Context Router; Intent Gateway; accessibility; automation; deterministic actions; notifications; multi-display and continuity; desktop/laptop, tablet/foldable and mobile shells. Integrate native, web and compatibility applications under common identity, policy and UI contracts.

**Exit:** Each approved shell is a fully interactive profile experience, all UI controls use real services, accessibility/automation/security semantics pass conformance tests, and shared application/context continuity works as specified.

## Phase 16 — Complete Aether Intelligence, privacy and policy governance

Implement the approved local/hybrid/cloud intelligence model, model manager, inference backends, NPU/GPU/CPU scheduling, Context projections, AI memory, retrieval, planning, agents, system command interface, tool capabilities, owner consent, revocation, audit and resource/privacy controls. Integrate AI safely across the same platform without bypassing the kernel/capability architecture.

**Exit:** AI features operate through least-privilege services and are demonstrably unable to access undeclared resources or turn model output into authority; offline, degraded, revoked and user-approval paths are tested.

## Phase 17 — Complete resource governance, reliability, backup, virtualization and enterprise operations

Complete resource contracts/leases, CPU/GPU/NPU scheduling, memory-pressure ladder, power and thermal policies, battery/radio efficiency, health/diagnostics, observability, crash containment, self-healing, backup/restore, factory reset, virtualization and enterprise controls that the canonical specification requires.

**Exit:** Resource/fault/energy/thermal budgets are measured, cross-profile policy behavior is validated, and supported failures are contained or recovered without unrelated system/data corruption.

## Phase 18 — Complete system integration, profile conformance and final AetherOS qualification

Integrate every subsystem across boot, services, security, resources, storage, runtimes, UI, package lifecycle, update and recovery. Build a complete end-to-end and per-profile requirement matrix. Remove or formally retire obsolete prototypes, disconnected APIs and superseded code only after preserving history and confirming there are no consumers. Produce reproducible full-system artifacts and qualification evidence for all declared supported profiles.

**Exit:** Every mandatory requirement in the complete current canonical specification and accepted decisions is Verified and passes its applicable profile-specific qualification gate; every SHOULD requirement is implemented or has an explicit approved deviation; every MAY item has a recorded disposition; no mandatory item remains hidden in a first-release, future-work or backlog label. The full integrated product—not merely its kernel or first reference image—meets the final definition of done in Section 23.

# 16. Required test and qualification program

Use a layered test strategy:

- Unit tests for algorithms, state machines and boundary conditions.
- Component tests for kernel, drivers, services and runtimes.
- ABI/interface tests for syscall, IPC, driver and package contracts.
- Integration tests using actual QEMU VirtIO/firmware devices.
- Deterministic QEMU boot tests with serial log assertions and guest health checks.
- End-to-end live-media boot tests.
- End-to-end installation and reboot-from-installed-disk tests.
- Negative tests for invalid ELF, bad pointers, wrong capabilities, invalid IPC messages, corrupt manifests and bad signatures.
- Multi-hart and concurrency tests.
- Memory-safety, isolation and resource-exhaustion tests.
- Storage crash-consistency and corruption tests.
- Update interruption, rollback and recovery tests.
- Network integration tests with real endpoints/services.
- Graphics and input behavior tests.
- Package install/update/uninstall tests.
- Fuzzing and property-based tests for parsers, loaders, protocol messages, filesystems and untrusted metadata.
- Static analysis, dependency review, secret scan and license compliance.
- Boot-time, memory-footprint, storage-footprint, responsiveness and throughput measurements.

Tests must prove meaningful behavior. Avoid vacuous assertions, test-only fake paths, mocks as substitutes for actual integration, hidden feature skips and pass-on-error scripts. A VM boot is evidence for that VM profile only. A successful build is not a boot qualification. A unit-tested storage crate is not proof of install/recovery durability. A passing QEMU USB simulation is not physical-board USB qualification.

For every run record:

- Exact commit SHA and worktree state.
- Exact command and working directory.
- Host OS and tool versions.
- Target triple, QEMU version/machine/CPU and virtual hardware.
- Firmware/bootloader/image hashes.
- Exit code.
- Test counts and failures.
- Relevant serial, firmware, guest and installer logs.
- Known limitations and what the test does not prove.

Do not suppress warnings or lower test requirements to get green CI. Fix root causes.

# 17. Host and user-data safety rules

All boot, installer, partition, storage, update, filesystem and recovery testing must use disposable QEMU disks/images, snapshots or explicitly designated test devices.
Never, without specific user approval and a device-specific safety plan:

- Partition, format, erase or overwrite a physical disk.
- Write a raw image to a USB stick or SD card on the user's computer.
- Modify the host EFI System Partition or boot entries.
- Change Secure Boot, TPM, BitLocker, firmware or boot order settings.
- Flash board, BIOS, UEFI, embedded-controller or peripheral firmware.
- Reboot the user's machine into AetherOS.
- Install privileged host tooling or alter machine-wide environment settings.
- Remove or overwrite existing user data.

For image-writing tests, verify the target path, file type, size and disposable status immediately before the operation. Never use destructive disk utilities against a real host disk. Keep development keys and secrets out of source control, chat and logs.

# 18. Engineering execution and agent orchestration

Operate as a hands-on implementing lead, not a report generator. Keep architectural knowledge and integration responsibility in the primary agent.

Use specialist subagents when useful: kernel/RISC-V, firmware/UEFI, bootloader, Rust systems, memory management, scheduler, IPC/security, filesystem/storage, VirtIO/device drivers, graphics/compositor, networking, userspace/runtime/ABI, installer/release, testing/formal methods and documentation. Only one subagent may be active at a time. Each assignment must be bounded, non-overlapping and specify requirements, exact scope, files/interfaces, forbidden actions, acceptance criteria, commands and evidence. Independently inspect every subagent result before integration.

For every implementation cycle:

1. Refresh Git state and inspect user changes.
2. Select the highest-priority dependency-ready task.
3. Read the authoritative requirement, architecture, ADR/RFC and affected code paths.
4. Research current maintained standards/implementations.
5. Identify security, lifecycle, data and compatibility impact.
6. Define acceptance tests before coding.
7. Implement a coherent vertical slice.
8. Integrate every required consumer.
9. Run focused tests, regression tests and relevant QEMU boot flows.
10. Inspect the complete diff, generated files and dependency changes.
11. Update source documentation, requirements traceability, status and handoff.
12. Close the task only with attributable evidence.
13. Immediately continue to the next ready task.

Use the repository's actual branch and contribution policy. Preserve all user changes and Git history. Never use destructive reset/clean, force-push, history rewriting or blind branch switching. Do not change remotes, push, publish a release, create tags or make other remote writes unless explicitly authorized. The project can be locally committed only when repository policy or the user's explicit direction allows it. Keep the local branch aligned with the owner's workflow; do not import rules from other projects.

Do not change global OpenCode configuration or install global agents. Use only capabilities actually available in the current environment. Detect whether commands are running in PowerShell, cmd, Git Bash, WSL or another shell and use commands valid for that shell. Capture native process exit codes immediately. Keep diagnostic output visible and attributable.

# 19. Requirements and project records

Maintain bidirectional traceability from each requirement to:

- Canonical source and requirement ID.
- Architecture decision and interface.
- Implementation path and enforcing call path.
- Tests and exact evidence.
- Platform profile and release scope.
- Known limitation and risk.
- Current status and acceptance authority.

Use the following status vocabulary consistently:

- Uninspected
- Not Started
- Research Required
- Designed
- Implementing
- Implemented
- Integration Pending
- Verification Pending
- Verified
- Blocked
- Approved for Deferral

Keep implementation status distinct from integration, security review, platform compatibility, release qualification and production status. Do not create fabricated percentages or counts.

Maintain or update the project's existing authoritative equivalents of:

- `AGENTS.md` / execution rules.
- Canonical system specification and architecture map.
- ADR/RFC and requirement registry.
- Implementation inventory and roadmap.
- Boot/firmware and target profile documents.
- Hardware/driver support matrix.
- Security threat model and risk register.
- ABI/syscall/IPC interface documentation.
- Filesystem/storage and A/B update/recovery documentation.
- Live-media and installer design/user guide.
- Desktop/shell/HCI and application/runtime documentation.
- Build/test/CI instructions and supported-target matrix.
- Release manifest, SBOM, checksums, provenance and known limitations.
- Session continuation/handoff.

Do not create a duplicate source of truth if a canonical project record already exists. Do not say a guarantee is implemented unless code and tests support it. Do not silently resolve conflicts with approved architecture; record a proposed ADR and seek the minimum required owner decision when the decision changes a fundamental contract.

# 20. Technology and architecture decision discipline

The current RISC-V64 kernel is the implementation baseline. Do not replace it with Linux, a different kernel or a new language/runtime merely to take a shortcut or to claim novelty. At the same time, the canonical AetherOS specification allows reuse of mature hardware foundations where appropriate. If the existing custom-kernel strategy creates a major blocker to a usable release, research the alternatives quantitatively and prepare an evidence-based ADR comparing at least:

- Continuing the current kernel and completing drivers/userspace.
- Reusing selected firmware, bootloader, device, filesystem, networking or userland components.
- Adopting a more mature kernel/hardware substrate while preserving AetherOS semantic contracts above it.

Compare development cost, time to usable OS, driver availability, security, isolation, performance, licensing, upstream divergence, long-term maintenance and impact on the user's intended project. Do not migrate without approval of the fundamental kernel strategy. Continue independent implementation while the decision is pending.

Prefer mature, maintained components with compatible licenses and a sound RISC-V port. Audit supply-chain health and dependency vulnerabilities. New code is justified when it implements a real AetherOS semantic requirement or fills a demonstrated gap. Avoid unnecessary rewrites, unused generic abstractions, placeholder drivers, fake device data, monolithic temporary demos and copy/pasted vendor code without license review.

# 21. First usable release acceptance checklist — intermediate milestone

The first usable AetherOS release is complete only when all applicable items below are evidenced:

### Boot and media
- [ ] Reproducible RISC-V64 build and image pipeline.
- [ ] Documented OpenSBI/SBI and UEFI/board boot path.
- [ ] RISC-V64 EFI removable boot application and correct path when using UEFI.
- [ ] Firmware/device discovery and handoff validated.
- [ ] QEMU boots from a virtual USB mass-storage image.
- [ ] Live session starts without modifying attached internal storage.

### Kernel and userspace
- [ ] Kernel privilege boundary is explicit and enforced.
- [ ] Protected user-mode native processes and address spaces.
- [ ] Stable ELF64/psABI and syscall ABI.
- [ ] Existing runtime VM has a documented, tested security boundary.
- [ ] IPC, capabilities and services work end to end.
- [ ] Process/service failure is handled safely.

### System capabilities
- [ ] Persistent VFS/filesystem and user data.
- [ ] Device discovery and core VirtIO/USB drivers.
- [ ] Keyboard, pointer, display and shell input path.
- [ ] Networking works against a real test endpoint.
- [ ] Timekeeping, logs, diagnostics, shutdown and reboot.
- [ ] Core graphical shell and essential applications are functional.
- [ ] Native package install/update/remove works.
- [ ] Browser/Web Runtime works for a full general-purpose release.

### Installer and installed operation
- [ ] Installer is usable from live mode.
- [ ] Target disk selection and explicit destructive-operation confirmation.
- [ ] Installs onto a separate virtual disk without affecting source media.
- [ ] Verifies installed artifacts and generation metadata.
- [ ] Reboots from the installed disk after live media is detached.
- [ ] User files/settings persist across reboot.
- [ ] Failure/cancellation behavior is safe and recoverable.

### Updates, recovery and security
- [ ] Existing A/B update path independently verified end to end.
- [ ] Candidate generation verification and health commit.
- [ ] Interrupted update and failed boot rollback verified.
- [ ] Recovery environment/path works independently.
- [ ] Critical/high security and data-loss findings resolved.
- [ ] Development vs production trust is clearly separated.

### Release quality
- [ ] All required tests and CI gates pass on the recorded SHA.
- [ ] Clean reproducible build produces the published artifacts.
- [ ] Artifact hashes, manifest, SBOM and provenance are available.
- [ ] User/developer/install/recovery docs are complete.
- [ ] QEMU and physical-hardware qualification are reported separately.
- [ ] Supported/unsupported hardware, features and known limitations are explicit.
- [ ] No false claim of universal RISC-V or hardware support.

## 21.1 Network-driver and network-stack acceptance gate

The supplied VirtIO network skeleton remains NOT COMPLETE until these gates are satisfied with evidence:

- [ ] VirtIO transport is discovered from the selected platform's firmware description and validated; no unverified hard-coded address is relied upon.
- [ ] Device reset/status lifecycle and feature negotiation conform to the applicable VirtIO version, including feature acceptance validation.
- [ ] RX/TX virtqueues are allocated, configured and activated with correct descriptor/available/used ring handling.
- [ ] Header layout, descriptor ownership, DMA/cache behavior, fences, lengths, alignment, endianness and queue notification follow the negotiated device contract.
- [ ] RX buffers are posted, consumed safely, delivered and replenished.
- [ ] TX sends actual packets, handles completions and releases buffer ownership.
- [ ] Link status and MAC behavior are correct; reset, timeout, queue failure and device removal have defined behavior.
- [ ] QEMU receives packets from the guest and the guest receives packets from QEMU; host packet capture or equivalent evidence verifies both directions.
- [ ] Network interface is registered with the network stack/service and used by userspace.
- [ ] Address configuration, ARP/neighbor discovery, DNS, routes and TCP/UDP are functional for the release profile.
- [ ] TLS and trust-store validation work for the required browser/package/application operations.
- [ ] System network UI and diagnostics reflect real state; permissions/capabilities govern application access.
- [ ] Positive, negative, malformed-input, saturation, concurrency, reset and fault-injection tests pass.

A method that returns a constant failure/empty result, an unimplemented branch, a fake success, a mock-only packet path or a driver test that never exercises an actual virtqueue cannot satisfy these criteria.

## 21.2 Whole-platform completion checklist — ultimate goal

This checklist applies after (and includes) the first usable release. It must be expanded into individual requirement IDs and profile rows from the live canonical specification; checking only the category headings is insufficient.

### Kernel and execution
- [ ] All required CPU/ISA and firmware paths, privilege boundaries, traps, interrupts, timers, SMP, memory management, scheduler, threads, processes/capsules, executable loader, syscall ABI and lifecycle semantics are implemented.
- [ ] IPC, capability namespaces/rings, capability transfer/attenuation/revocation/expiry/domain rules, event and wait-set semantics and native/compatibility execution boundaries are enforced end to end.
- [ ] All mandatory kernel invariants from the canonical architecture/RFC/registry are implemented and verified.

### Device and driver architecture
- [ ] AHDA and Device Graph are complete for all declared profiles.
- [ ] Firmware, ACPI/DT, PCIe, USB, SoC buses, DMA/IOMMU, resource allocation, driver ABI, isolation, VCD, driver packages, trust and rollback are implemented where applicable.
- [ ] Every device class required by every supported profile has a working driver/service or an explicit documented hardware limitation consistent with the approved support tier.

### Storage and boot/update
- [ ] Unified logical domains, filesystem/object APIs, transactional persistence, immutable/mutable/ephemeral semantics, snapshots, generations, content integrity, encryption, accounting, backup/restore, migration and recovery are integrated.
- [ ] Boot trust, generation descriptors, signing, measured/verified boot, owner keys, anti-rollback and platform-specific recovery are complete to the guarantees actually supported by hardware.
- [ ] System, firmware, driver and application update domains have tested transactional lifecycle and failure recovery.

### Networking and connectivity
- [ ] All required wired/wireless network transports and protocol layers are functional and integrated with policy, sockets, UI and applications.
- [ ] Bluetooth, NFC, UWB, GNSS, cellular and telephony are implemented for profiles that include those requirements.

### Graphics, input and multimedia
- [ ] Display/composition, 2D/3D APIs, GPU, shared-buffer synchronization, external monitors, docking, scaling, graphics recovery and software fallback are implemented per profile.
- [ ] Unified input, keyboard, pointer, touch, pen, controller and accessibility paths are implemented as supported.
- [ ] Audio, camera/ISP, sensors and media lifecycle/privacy/resource controls are implemented for applicable hardware profiles.

### Services, runtimes and applications
- [ ] All required system services, service discovery, typed IPC, lifecycle, fault handling and observability are integrated.
- [ ] Native runtime/ABI/SDK, packages, dependency management, repositories, signing, updates, removal, debugging and developer tooling are functional.
- [ ] Web Runtime/browser and Android compatibility meet the canonical compatibility target with real application suites.

### User experience and platform profiles
- [ ] Context/Provider/Workspace/Activity semantics and Visual/Semantic/Input Graphs are implemented with policy, privacy, accessibility and automation.
- [ ] Desktop, laptop, tablet, foldable and mobile shells required by the canonical scope are complete and integrated.
- [ ] Cross-device Context continuity and common application state work across supported profiles.

### Intelligence, security and resources
- [ ] AI/model/agent/runtime, memory, retrieval, tool-use, consent, capability, audit and privacy architecture are implemented within the defined security boundary.
- [ ] Identity, authentication, least privilege, capability attenuation, encrypted state, privacy/data locality, key management and threat mitigations are integrated.
- [ ] Resource contracts/leases, CPU/GPU/NPU/DSP scheduling, memory pressure, power/thermal/energy governance and accounting are operational on each relevant profile.

### Reliability and engineering ecosystem
- [ ] Health/diagnostics, tracing, crash containment, self-healing, backup/restore, reset, virtualization, simulator/fault injection and test harnesses are complete.
- [ ] Full SDK/driver SDK, system build/image pipeline, certification/conformance, reproducible releases, SBOM/provenance, documentation and maintenance mechanisms are complete.
- [ ] All required integrations are exercised in complete system scenarios, not just component tests.

# 22. First user demonstration scenario

The first convincing end-to-end demonstration must follow this exact user journey in a disposable QEMU environment:

1. Build the AetherOS release image from the repository using the documented command.
2. Verify the image checksum and layout.
3. Start QEMU with the documented RISC-V64 `virt` profile, firmware, display, keyboard/pointer, USB controller, virtual network and two separate storage devices: a removable-media source and a blank installation target.
4. Boot from the removable-media source through the selected firmware path.
5. Reach a live AetherOS graphical desktop.
6. Confirm the live-mode indicator and verify that the second disk has not been modified or mounted writable automatically.
7. Open the file manager, terminal, settings, text editor and system monitor.
8. Create and save a test document in the live session; demonstrate its volatility unless explicit live persistence is enabled.
9. Verify network connectivity using the system's network UI and a real test service.
10. Open the installer, inspect the disk inventory and select the separate blank target disk.
11. Review the exact partition/generation plan and explicitly approve it.
12. Install AetherOS and verify the installed image/filesystem.
13. Shut down and detach the live-media device while retaining the installed target.
14. Boot from the installed virtual disk through the documented firmware path.
15. Complete first-run user setup and reach the installed desktop.
16. Create a user file, change a setting, install or run a separately packaged application, reboot and verify that all intended persistent state remains.
17. Apply a test update, validate health commit and reboot.
18. Run a deliberate failed-update/rollback test on a snapshot and prove recovery to the previous generation without losing user data.
19. Export logs, image hashes and a machine-readable qualification report.

Automate this scenario in CI to the extent the tools permit. A human-readable screencast or screenshot may supplement the evidence, but it cannot replace automated assertions, boot logs and persisted-state verification.

# 23. Definition of done and reporting

## 23.1 Milestones do not close the project

The following are separate milestones and MUST NOT be conflated:

1. Kernel compiles.
2. Kernel boots in QEMU.
3. Kernel boots through the standard firmware path.
4. Subsystem scaffold/API exists.
5. Subsystem behavior is implemented.
6. Subsystem is integrated with its real consumers.
7. End-to-end system behavior is verified.
8. A first usable release is qualified.
9. All approved AetherOS profiles and architectural capabilities are implemented and integrated.
10. The complete AetherOS platform is finally qualified.

Passing an earlier level never implies a later level. In particular, cargo build success, unit-test counts, CI-green status, a QEMU serial boot or the existence of a struct/function cannot be reported as completion of a feature that requires runtime behavior.

## 23.2 Required implementation-status vocabulary

Use explicit states per requirement and per feature:

- **Uninspected:** source and implementation have not been audited.
- **Research required:** standards/architecture/technology assessment is incomplete.
- **Designed:** contracts and decision exist, but implementation may not.
- **Scaffold only:** types, constants, interfaces, empty handlers, placeholder return values, wiring shells or compile-only code exist. This is NOT implementation completion.
- **Partially implemented:** some real behavior exists; explicit required behavior remains absent.
- **Implemented:** real behavior exists for the specified requirement, but consumer integration or qualification may remain.
- **Integrated:** actual boot/service/runtime/application consumers use the implementation through intended interfaces.
- **Verified:** acceptance, negative, boundary, integration and end-to-end tests demonstrate the specified behavior.
- **Qualified:** the implementation has passed the required target/profile/hardware/release qualification gates.
- **Blocked:** an exact dependency, decision or hardware limitation prevents progress.
- **Approved deviation/deferral:** a specific requirement has an owner-approved disposition with consequences, risk and future conditions documented.

A feature may be called complete for a release only when every applicable mandatory requirement reaches the required Verified/Qualified state for that release. A whole-platform requirement may not be removed from the total inventory because it is absent from an early release.

## 23.3 Strict rules against false-positive completion

The primary agent MUST apply all of the following:

- No feature is complete because a type, constant, trait, API, module, CLI command, UI screen, configuration flag or documentation paragraph exists.
- No driver is complete until it controls a real or faithfully conformant emulated device through its actual transport and handles data in both directions where applicable.
- No network driver is complete when send/receive are hard-coded to fail, return zero, discard packets or use only mock data.
- No storage feature is complete until real read/write, persistence, crash and recovery behavior work on a backing device/image.
- No process model is complete until actual user-mode isolation, syscalls, IPC, lifecycle and fault containment are exercised.
- No shell is complete until its controls invoke real services and real applications, rather than local UI-only state changes.
- No installer is complete until it safely creates a bootable installation and the installed OS boots independently with persistent state.
- No update system is complete until authentication, staging, candidate boot, health commit, interruption, rollback and user-data semantics are tested.
- No security feature is complete from a policy declaration alone; enforcement must occur at the actual authority boundary.
- No cross-platform feature is complete based on one profile; report and verify support per profile.
- No tests are accepted as feature evidence unless they can detect meaningful regressions; vacuous assertions, mocks without real-boundary coverage, ignored tests, constant-return paths and test-only implementations do not close requirements.
- Do not classify a whole phase as complete based on a subset of bullets. Each requirement must be individually traced and closed.
- Never downgrade a mandatory requirement to “future work” merely to close a release or phase.

If work is only a scaffold, say so directly and continue implementing it. If a phase is partly complete, report the exact remaining requirements and keep the phase open.

## 23.4 Final AetherOS completion gate

The project may be declared **AetherOS PLATFORM COMPLETE** only when all of the following are simultaneously true:

1. The current canonical specification, all later convergence provisions, accepted ADRs, RFCs, requirement registries and owner decisions have been inventoried in full. There are no untraced mandatory requirements.
2. Every MUST requirement is implemented, integrated, Verified and Qualified on every applicable supported platform profile.
3. Every SHOULD requirement is either implemented to the required evidence level or has a specific, approved, risk-assessed deviation. The agent cannot approve its own deviation where owner governance is required.
4. Every MAY feature has a recorded disposition and does not silently alter the core architecture.
5. Desktop, laptop, tablet, foldable and mobile profiles required by the approved architecture are real, tested and share the common semantic platform. Each profile has its own documented hardware/firmware/driver/support matrix.
6. Kernel, device, storage, network, graphics, audio, camera, sensor, input, system services, runtimes, package ecosystem, UI shells, AI/intelligence, security/privacy, identity, resource/power/thermal management, virtualization, diagnostics, backup, update, recovery, installer and developer facilities are integrated into the same product architecture.
7. The integrated system boots, operates, updates and recovers end to end. It is not a collection of separately passing crates or unrelated system images.
8. Each supported profile has an honest reference-hardware qualification record. QEMU qualification, physical-board qualification and broader compatibility are clearly separated.
9. Reproducible builds, images, package repositories, SBOM/provenance, release signing/trust, tests, conformance and documentation are complete.
10. There are no unresolved critical/high defects affecting shipped paths, security boundaries, user-data durability, bootability or recovery.
11. Performance, footprint, power and thermal budgets have measured baselines and accepted deviations.
12. The final requirement matrix, product matrix, compatibility matrix, known-limitations list and qualification evidence agree with the actual shipped implementation.

A First Usable Release, a QEMU-only release, a physical reference-board release or an intermediate multi-profile milestone MUST be labelled as that milestone. None may be described as final AetherOS platform completion while mandatory whole-platform work remains.

## 23.5 Continuation rule

After each milestone, immediately select the next dependency-ready incomplete requirement from the whole-platform inventory. Do not stop after Phase 9, after a green CI run, after a driver scaffold, after publishing an image or after completing the current AGENTS.md “Next Move” list. The project remains active until Section 23.4 is satisfied or the owner explicitly changes the final goal.


Do not call AetherOS “complete,” “stable,” “production-ready,” “hardware-qualified” or “securely booted” based on subjective judgment or a single green build.

At each milestone report:

- Exact local commit and working-tree state.
- What the previous report claimed and what was independently revalidated.
- Requirements completed by ID and scope.
- Files/interfaces changed and why.
- Tests, builds, QEMU scenarios and exact results.
- Installation/live/update/recovery behavior demonstrated.
- Security, storage durability and hardware limitations.
- Performance/resource measurements and baseline.
- Decisions/ADRs and any unresolved owner decisions.
- Next dependency-ready task.

For each release report, state which milestone it represents and which requirements it closes. For the final AetherOS platform report, state every supported system profile, exact QEMU/physical hardware and firmware coverage, all live/install/update/recovery routes, image/package hashes, complete requirement closure, conformance results, implemented/integrated/verified/qualified status per subsystem, all approved deviations and every remaining unsupported item. If any mandatory whole-platform requirement remains open, the report MUST say that AetherOS PLATFORM COMPLETE has not been achieved.

A **QEMU-qualified First Usable Release** may be delivered before physical board qualification only if it is labeled honestly and the physical-hardware limitation is prominent. A **physical live-USB and installer-qualified release** additionally requires a named, tested physical RISC-V64 board whose firmware actually boots the removable USB media and whose installed system completes the first-boot and recovery tests.

The live USB and installed RISC-V64 system are essential first product outcomes, but they are not the ultimate project completion point. The ultimate target is the complete AetherOS platform described in Section 4.2 and the canonical system specification: every approved profile and feature set implemented, integrated, verified and qualified, delivered as one coherent operating system with the required live, install, update and recovery experiences.

---

# Appendix A — Current research findings and implementation implications

These are research leads and engineering constraints, not claims that the local project already implements the related standards.

1. **QEMU `virt` is a suitable initial virtual reference platform.** QEMU documents it as a generic virtual board and lists generic RV32GC/RV64GC harts, CLINT, PLIC, UART, RTC, SiFive test device, VirtIO-MMIO transports and a generic PCIe host bridge. It also creates a Device Tree Blob by default and expects guest software to discover actual devices from it. This makes it appropriate for repeatable RISC-V kernel, storage, network and device tests, but it is not physical-board evidence.
2. **OpenSBI is a practical M-mode firmware/SBI provider for QEMU.** QEMU's RISC-V documentation describes the default OpenSBI firmware path, and OpenSBI documents its QEMU `virt` platform. The AetherOS kernel should negotiate the SBI functionality it needs instead of assuming every optional extension is present.
3. **EDK2 provides a RISC-V QEMU `virt` UEFI path.** The TianoCore `OvmfPkg/RiscVVirt` documentation describes EDK2 as an S-mode payload for previous M-mode firmware such as OpenSBI and gives a QEMU/RISC-V build path. Treat firmware version, pflash layout and support status as pinned inputs.
4. **RISC-V UEFI removable-media naming is standardized.** The UEFI Boot Manager specifies `\EFI\BOOT\BOOTRISCV64.EFI` for RISC-V 64-bit removable-media applications. The UEFI removable-media specification describes FAT12/FAT16/FAT32 support. A correct file name alone does not guarantee USB firmware support; test firmware enumeration and controller drivers.
5. **RISC-V UEFI handoff has architecture-specific requirements.** The RISC-V UEFI protocol defines a boot-hart identity protocol because the direct-kernel register convention cannot simply be assumed under UEFI application calling conventions. Inspect and implement the correct handoff.
6. **SBI support should be feature-negotiated.** The RISC-V Boot and Runtime Services requirements specify SBI v2.0 or later and HSM, with some extensions required conditionally depending on hardware features (for example timer and IPI/remote-fence mechanisms). Confirm the current ratified specification/version and implement the exact extension matrix for AetherOS's selected reference profile.
7. **Pin a stable RISC-V ABI.** The current RISC-V ELF psABI site exposes a November 2026 2.0-rc0 development snapshot while the 1.0 ABI is listed as ratified. Do not make a pre-release ABI the AetherOS binary contract without a deliberate, isolated decision. Pin a ratified compatible ABI and toolchain baseline.
8. **Physical RISC-V boards are not interchangeable.** Their boot ROM, SPL, U-Boot/EDK2, USB boot, Device Tree, display, GPU and peripheral support vary. Public board-support matrices report substantial differences even among JH7110-family devices. Select a physical reference board by actual upstream support and available test hardware, not by marketing specifications.
9. **EDK2/QEMU can model a useful full test machine.** The upstream EDK2 RISC-V `virt` documentation includes a QEMU example using VirtIO GPU, QEMU xHCI, USB keyboard, RNG, VirtIO networking and block storage. This is a useful basis for the live USB and install-to-separate-disk qualification harness.

# Appendix B — Primary research references

Check current versions and project status before implementation; these links are starting points, not substitutes for reading the normative text and recording exact versions.

- QEMU RISC-V system documentation: https://www.qemu.org/docs/master/system/target-riscv.html
- QEMU RISC-V `virt` machine documentation: https://www.qemu.org/docs/master/system/riscv/virt.html
- OpenSBI QEMU `virt` platform: https://github.com/riscv-software-src/opensbi/blob/master/docs/platform/qemu_virt.md
- TianoCore EDK2 RISC-V `virt` platform: https://github.com/tianocore/edk2/blob/master/OvmfPkg/RiscVVirt/README.md
- RISC-V Boot and Runtime Services specifications: https://docs.riscv.org/reference/brs/
- RISC-V SBI specification: https://docs.riscv.org/reference/sbi/
- RISC-V SBI Hart State Management extension: https://docs.riscv.org/reference/sbi/ext-hsm.html
- RISC-V UEFI protocol specification: https://docs.riscv.org/reference/platform-software/uefi/
- UEFI Specification — Boot Manager: https://uefi.org/specs/UEFI/2.11/03_Boot_Manager.html
- UEFI Specification — Media Access: https://uefi.org/specs/UEFI/2.11/13_Protocols_Media_Access.html
- RISC-V ELF psABI: https://riscv-non-isa.github.io/riscv-elf-psabi-doc/
- RISC-V privileged architecture: https://docs.riscv.org/reference/isa/priv/
- Ubuntu RISC-V QEMU boot/install reference: https://ubuntu.com/hardware/docs/boards/how-to/ubuntu_supported/qemu-riscv/
- Debian RISC-V hardware support notes: https://wiki.debian.org/RISC-V/
- RISC-V International developer boards: https://riscv.org/developers/boards/
- OASIS VirtIO 1.2 specification (network device, virtqueues, feature negotiation and packet formats): https://docs.oasis-open.org/virtio/virtio/v1.2/virtio-v1.2.html
- QEMU RISC-V `virt` platform device map and virtual-device documentation: https://www.qemu.org/docs/master/system/riscv/virt.html
- EDK2 RISC-V QEMU `virt` platform: https://github.com/tianocore/edk2/blob/master/OvmfPkg/RiscVVirt/README.md
- smoltcp Rust no-std TCP/IP stack: https://github.com/smoltcp-rs/smoltcp
- embassy-net Rust no-std asynchronous network stack: https://docs.rs/embassy-net/latest/embassy_net/

---

# Appendix C — Version 4.0 corrective clarification — why the previous “Phase 3 complete” report is not accepted

**Date:** 21 November 2026
**Supersedes:** Any interpretation of Version 3.0 that treated the First Usable Release as the entire AetherOS project goal or treated a driver scaffold as a completed vertical slice.

The latest implementation report says that the VirtIO network framework has been added, but `virtio_net_send()` returns `false` and `virtio_net_recv()` returns `0`. By the report's own description, the code does not transmit or receive packets and full device emulation/data-path validation remains pending. It therefore cannot satisfy the Phase 3 exit criterion, which explicitly requires userspace applications to access a network service and requires real device discovery rather than hidden constants. It is scaffold-only or partially implemented, depending on the actual initialization and queue code.

The report's phrase “Phase 3 features now complete” is incorrect and must be amended in local project records. At most, the report may say: “VirtIO network scaffolding added; target build succeeds; 174 tests pass across 12 crates; actual TX/RX and end-to-end network behavior remain unimplemented/unverified.” Preserve the test/build evidence, but do not inflate its meaning.

The report's “Next steps would be full VirtIO device emulation in CI” is also too narrow. The next task is not merely adding a test emulator. QEMU already provides a virtual VirtIO network device/backend; the guest driver must implement the VirtIO contract and exchange real packets with that device. CI should then exercise the real guest driver/backend path. A mock remains useful for malformed-device and fault-injection tests, but cannot replace the actual QEMU device.

The OASIS VirtIO 1.2 network-device specification defines distinct receive and transmit virtqueues (and an optional control queue), with receive buffers posted to the device and outgoing packets submitted to the transmit queue. It specifies feature-dependent network-header fields and detailed driver/device requirements. The implementation must follow the selected transport mode and negotiated features rather than treat MMIO register access and a status write as a complete network driver.

The QEMU `virt` board is explicitly a generic virtual platform, not real hardware, and exposes multiple virtio-mmio transport slots. This supports deterministic virtual qualification but reinforces that the guest must discover the selected device through the platform description and that a QEMU pass is not physical-board qualification.

**Immediate correction action for the local agent:** Reopen the current `reference/crates/aether-arch-riscv64/src/main.rs`; inspect the exact VirtIO block path and new network path; compare both with OASIS VirtIO 1.2 and the actual QEMU `virt` Device Tree; create a requirement-linked gap list; implement a complete tested RX/TX driver and connect it to a real network stack/service; prove bidirectional packets and a userspace network client; only then close the network requirements. Do not stop at the first green unit-test run.

**Evidence references:** OASIS VirtIO 1.2 network device specification; QEMU RISC-V `virt` documentation; current EDK2 RISC-V `virt` documentation; current smoltcp and embassy-net documentation. Recheck upstream versions and licensing at implementation time.


# Final instruction

Begin with Phase 0 in the existing local AetherOS workspace. Preserve the working kernel, integrated storage/IPC/runtime/A-B paths and all user work. Verify the stated baseline, inspect the complete canonical architecture, then implement the next dependency-ready vertical slice. Research while implementing, test at the real system boundary, update the project records and continue automatically.

Do not stop at “the kernel boots.” Do not stop at “the CI is green.” Do not stop after the live image starts. The goal is an operating system that boots from removable media, gives a person a functional live environment, installs safely to a separate disk, boots from that installation, retains user data, runs userspace applications, provides real system services and can update and recover without losing the user's work. These are initial usability gates. Continue afterward until the entire approved AetherOS platform and every applicable subsystem described in this mandate and canonical specifications are implemented, integrated, verified and qualified. Never report the ultimate goal met while any mandatory whole-platform requirement remains open.