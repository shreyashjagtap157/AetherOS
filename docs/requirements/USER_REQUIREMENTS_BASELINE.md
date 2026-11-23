# AetherOS — reconstructed user requirements and complete product baseline

Classification: Informative

**Canonical repository:** `https://github.com/shreyashjagtap157/AetherOS`  
**Canonical branch:** `main`  
**Current reference implementation target:** RISC-V64, QEMU `virt`  
**Ultimate product scope:** one coherent operating system across desktop, laptop, tablet, foldable and mobile operating profiles  
**Active goal source:** [AetherOS Complete Platform Goal v4.0](../project/AetherOS_Complete_Platform_Goal_v4_2026-11-21.md)

> This is an informative reconstruction of the owner's product requirements and conversation decisions. The `AOS-UR-*` identifiers below are traceability keys, not constitutional Requirement-IDs and not a substitute for IDs declared in the architecture, RFCs, ADRs or repository governance tools. They do not declare any RFC accepted or any implementation complete.

## 1. Mission, product identity and architectural philosophy

### AOS-UR-MISSION — A real, usable general-purpose operating system

AetherOS is to become a real operating system that people can boot, interact with, install, update, recover, develop applications for and use for ordinary computing—not merely a kernel experiment, reference-model collection, boot log, set of subsystem crates, research paper or mock UI.

The system must have a real end-to-end user experience, hardware/runtime/service integration, persistent state, application execution, security boundaries, lifecycle management, update/recovery and a developer ecosystem. Its components must have defined authority, ownership, lifecycle, APIs, consumers, error behavior, tests and documentation, all connected into one coherent product.

### AOS-UR-ONE-PLATFORM — One platform, multiple operating regimes

Desktop, laptop, tablet, foldable and mobile devices are not separate operating systems. They share one coherent semantic model, common services, security model, resource contracts, storage model and application concepts wherever technically appropriate. Specialization belongs in explicit platform profiles, power/thermal/input policies, hardware capability packages, optimized implementations and experience shells.

Do not make desktop assumptions the authority for mobile, or mobile assumptions silently break desktop/laptop. Reconcile both against the shared architecture. Add future device classes when the shared model and declared profile support them.

### AOS-UR-SEPARATION — Separate semantics from replaceable mechanisms

Define stable semantic contracts and authority boundaries; leave implementation mechanisms replaceable where no architectural invariant requires a fixed choice. Do not encode a software/logical concept as a physical partition, runtime, process, device or UI surface solely for convenience. Preserve real separation where required by hardware, security, reliability, firmware, lifecycle or compatibility.

Keep distinct:
- system semantics and observable guarantees;
- common platform services;
- profile-specific policies and optimizations;
- hardware/bus/vendor-specific mechanisms;
- user-experience shells;
- reference models, prototypes and production implementations.

### AOS-UR-QUALITY — High assurance without unsupported claims

The aspiration is state-of-the-art, high-performance, secure, reliable, efficient, extensible, hardware-aware, privacy-respecting and AI-capable computing. Evaluate trade-offs quantitatively and preserve simplicity/accountability. Use formal methods, verification, test evidence and independent review where applicable, but never claim proof, safety certification, broad hardware support or production readiness without the relevant external/technical evidence.

## 2. Reference product and release journeys

### AOS-UR-RV64 — Initial reference target

- First reference architecture: 64-bit RISC-V.
- Initial qualification target: QEMU `virt`.
- Define the exact supported firmware/boot route, privilege/hart model, SBI/OpenSBI or other firmware contract, memory map, interrupt/timer model, boot payload and artifact formats.
- A normal supported boot path must be documented and reproducible; do not require a developer-only direct-kernel injection command for every product boot.
- QEMU is an initial controlled qualification platform, not proof of physical RISC-V board compatibility. Physical boards require their own firmware, device, storage, network, display, input and failure qualification.

### AOS-UR-LIVE — Safe live removable-media environment

A person must be able to write a published AetherOS image to removable media, boot a compatible RISC-V64 system, reach a useful environment, explore and run applications and shut down/reboot safely.

Live mode must:
- identify itself clearly as a live session;
- avoid writing to internal/attached disks by default;
- avoid automatically mounting unknown internal storage writable;
- distinguish ephemeral live state from explicitly enabled live persistence;
- preserve the source media while installation targets another selected disk;
- expose system health, device and storage state intelligibly;
- allow safe shutdown/reboot and clear handling of faults.

### AOS-UR-INSTALL — Real installer and durable installed system

From the live environment the user can launch a real installer, inspect the actual device inventory, select a distinct target drive, review a precise partition/filesystem/boot/update-generation plan, explicitly confirm destructive changes, install and verify AetherOS, reboot without the live media and boot the installed system.

Installer guarantees:
- never assume a fixed target device or partition number;
- never confuse source/live medium with target;
- display device identity, size, existing content and proposed changes before confirmation;
- require explicit user confirmation before destructive operations;
- handle capacity, unsupported layouts, I/O faults, interruption and power loss;
- stage writes transactionally where possible, verify image/filesystem/boot metadata and report failures;
- provide recovery/rollback where the operation's stage permits it;
- protect user data and never modify unrelated host storage;
- support clean re-install, upgrade and recovery scenarios under declared policy.

The installed system must retain accounts, credentials/identity state, settings, applications and user files across shutdowns, reboots and normal updates. Persistence must be tested across the actual boot/install/update lifecycle.

## 3. Kernel, protected execution and core architecture

### AOS-UR-KERNEL — Complete and integrated execution foundation

Implement and integrate the actual kernel responsibilities required by the selected architecture, including:
- boot and CPU/hart bring-up;
- memory discovery, physical-frame management, virtual address spaces and MMU/page tables;
- privilege boundaries and hardware protection (PMP and appropriate MMU/IOMMU mechanisms);
- trap/exception/interrupt vectors, timers and safe interrupt handling;
- preemption, scheduling, dispatch and task/process/thread lifecycle;
- user/kernel transitions and defined system-call ABI;
- inter-process communication, synchronization and event delivery;
- resource allocation and ownership;
- driver/device interfaces and system service boundaries;
- fault containment, diagnostics and predictable shutdown/reboot.

A kernel that only prints text or executes all application code in privileged context is not a completed operating system.

### AOS-UR-USERSPACE — Protected user processes and ABI

Applications must execute as protected user-mode processes with isolated address spaces, explicit syscall and IPC interfaces, controlled resource access and declared failure/termination behavior. Define the executable format, loader, calling conventions, ABI versioning, process environment, memory layout, syscall contracts, signals/events/cancellation where adopted, process creation/exec/wait and application lifecycle.

The runtime VM and native process path must have a clear relationship. A VM integrated into a privileged task is not a substitute for a protected process model. Native APIs are the primary AetherOS application model.

### AOS-UR-CAPABILITY — Capability-based authority

Permissions must be expressed primarily as capabilities, rather than only as broad application identity roles. Capabilities should be explicit, least-privileged, attenuable, transferable only under policy, revocable and auditable. Validate authority at the required point of use and bind it to the relevant object/domain/lifecycle context.

Examples include camera capture, microphone recording, precise location, storage read/write, network access, Bluetooth/NFC control, GPU rendering, NPU inference and raw device access. Processes, applications and agents must receive authority through explicit policy and user consent. No hidden ambient privilege, unbounded authority inheritance or privileged AI exception.

### AOS-UR-IPC — Typed communication and lifecycle contracts

Use typed/validated IPC, explicit endpoints, routing, capability transfer, synchronization, streaming, backpressure, cancellation, fault handling and observability. The semantics must define how sender/receiver identity, object lifetime, authority, ordering, failures and externally visible effects behave. Do not treat an in-memory queue as proof of complete IPC architecture.

## 4. Hardware discovery, device model and drivers

### AOS-UR-DEVICE — Unified device graph and hardware abstraction

Normalize discovered hardware into a common Device Graph while preserving differences among ACPI, Device Tree, UEFI/firmware tables, PCI/PCIe, USB and vendor/bus discovery. Device identity, parent/child relationships, capabilities, dependencies, resources, power state, hot-plug lifecycle and driver bindings must be discoverable and auditable.

### AOS-UR-DRIVER — Layered driver framework

Use a layered design with:
- a kernel hardware substrate/HAL and minimum trusted primitives;
- selectively in-kernel drivers only where justified by performance, trust or architecture;
- isolated userspace device services/drivers where appropriate;
- high-level stable system APIs independent of bus/vendor mechanics;
- explicit driver manifests, dependencies, lifecycle, trust/signing, privileges, resource limits, failure containment, update and rollback.

Drivers must perform real device discovery, initialization, queue/buffer management, DMA/IOMMU safety, interrupts, power/lifecycle transitions and actual data transfer. A driver scaffold whose send/receive returns a constant, a hard-coded device identity or a mock provider is not a completed driver.

### AOS-UR-HARDWARE — Broad, staged hardware support

The platform direction includes processors and memory, storage controllers, PCIe, USB/USB-C, removable devices, keyboard/mouse/touch/pen/controllers, displays/docking, graphics/GPU, audio, cameras/ISP, sensors, network interfaces, Wi-Fi, Bluetooth, NFC, UWB, GNSS/cellular where applicable, power and thermal telemetry. Higher-end accelerator providers (GPU/NPU/DSP/FPGA/DPU and other emerging devices) belong in the long-term architecture where supported by research and qualified profiles.

Do not claim a device class or board works merely because an abstraction, model or driver skeleton exists. Validate on emulated and physical targets as declared.

## 5. Storage, filesystem, persistence and data safety

### AOS-UR-STORAGE — Logical separation with physically sensible storage

Minimize unnecessary physical partitions while providing rich logical domains for access control, ownership, quota/accounting, snapshots, encryption, rollback and policy. Do not impose a physical partition for every package, user, app or logical object unless a real hardware, security, compatibility or reliability invariant requires it.

Implement persistent filesystems and storage providers with clear integrity, transaction, crash-consistency, object identity, lifecycle and failure semantics. Cover metadata/data consistency, allocation, free-space management, permissions/capabilities, quotas, mount/unmount, removable media and recovery.

### AOS-UR-SNAPSHOT — Snapshots and generations

Provide transactional snapshots and copy-on-write or another evidence-backed mechanism for safe update, recovery, backup and state lifecycle. Updates must use redundant bootable generations and measured storage strategies appropriate to device profiles. Do not mandate physical A/B duplication for every platform nor ban it categorically; define the invariant (known-good rollback) and qualify a measured implementation.

### AOS-UR-ENCRYPTION — Data protection

Protect user/system data in transit and at rest where supported. Define identity, key lifecycle, secure storage, data locality, access control, backup/export and recovery implications. Encryption must not render recovery or upgrade guarantees meaningless; the trust model and key recovery limitations must be documented.

## 6. Boot, trust, updates and recovery

### AOS-UR-BOOT — Verifiable boot chain

Define a coherent chain from firmware and boot manager through signed/measured boot metadata, kernel, system image, drivers, services and user-space. Verify image identity, signature/provenance, version and policy before activation. Keep development/test signing distinct from production-trusted publisher signing.

### AOS-UR-UPDATE — Transactional updates

Support staged system generations, signature/hash validation, dependency and compatibility checks, health confirmation, activation and recovery to a known-good state. Make updates safe against power loss, device reset, partial downloads, interrupted writes and failed first boot. Define anti-rollback and key-rotation policy where required. Update packages and system generations must be traceable to reproducible build artifacts.

### AOS-UR-RECOVERY — Recovery is a real system capability

Provide boot recovery, rollback, backup/restore, repair, reset/reinstall and safe diagnostics. Recovery must remain possible when the active generation is corrupt or a new driver/service fails. Recovery design must account for persistent user data and distinguish recoverable system state from irreversible external effects.

## 7. Network and communications

### AOS-UR-NETWORK — End-to-end real networking

Build a complete, connected network path from physical/emulated device discovery and driver queues through link and network layers, transport, sockets/API and userspace application. Include as required:
- VirtIO and target hardware NICs;
- Ethernet framing, ARP and IPv4/IPv6;
- ICMP diagnostics;
- UDP and TCP connection lifecycle, data transfer, retransmission and shutdown;
- DHCP, DNS, routing and configuration;
- Wi-Fi, Bluetooth, cellular and other applicable connectivity providers;
- firewall/policy, per-capability network access, isolation and observability;
- suspend/resume, roaming and network-change lifecycle for mobile profiles.

A TCP handshake or protocol model without actual payload delivery, a loopback-only test or driver TX/RX stubs does not satisfy end-to-end networking. Test real traffic and userspace access to a test service on the declared QEMU/physical network.

## 8. Graphics, input, audio and human interaction

### AOS-UR-GRAPHICS — Functional graphical system

For a declared desktop/general-purpose release, provide real display initialization, framebuffers/composition, window management, rendering APIs, event dispatch, damage/refresh, input focus, scaling, clipboard and recovery from graphics failures. GPU acceleration and advanced rendering are staged by device profile, but a mock desktop is not sufficient.

### AOS-UR-INPUT — Unified multi-modal input

Provide coherent keyboard, pointer, touch, pen, controller, accessibility input and external-device handling. Define device discovery, focus, event routing, gesture policy, calibration, hot-plug, power and permission lifecycle. Include multi-display, docking, external input and profile-specific behavior where supported.

### AOS-UR-MEDIA — Multimedia services

Implement profile-relevant audio, camera, image-signal processing, sensors and media capture/playback with explicit privacy, capability, lifecycle, resource, latency and power controls.

### AOS-UR-ESSENTIAL-APPS — Useful base experience

The first graphical desktop should include a file manager, terminal, text editor, settings utility, application launcher/task switching, system monitor/diagnostics and useful basic apps. A first-run/setup flow should be understandable, accessible and safe.

## 9. User-facing model, contexts and cross-device experience

### AOS-UR-CONTEXT — Context-centric interaction model

Context is the canonical human-facing unit of activity. Applications remain deployment, runtime and security entities. Context must not be conflated with application ownership, Workspace organization, Activity execution, Provider identity or security authority.

Use the correlated:
- **Visual Graph** — visible surfaces and spatial relationships;
- **Semantic Graph** — explicit control/object meaning and relationships;
- **Input Graph** — valid actions, focus and interaction routing.

Where reliable semantic data exists, it is authoritative; visual/VLM interpretation is a controlled fallback with explicit confidence, provenance and error handling. User intent, active task and cross-application relationships should be represented without bypassing application security boundaries.

### AOS-UR-WORKSPACE — Workspace, Activity and Provider semantics

Define independent ownership and lifecycles for Context, Workspace, Activity, Task, application, Provider and user/session. Cross-app work should be organized around the user's task without turning UI grouping into a permission boundary or transferring application data/authority implicitly.

### AOS-UR-PROFILES — Desktop through mobile/foldable

Design coherent experiences for desktop, laptop, tablet, foldable and mobile profiles. Account for adaptive window/layout behavior, orientation, hinge/fold posture, touch/keyboard/pointer/pen, screen size, external display, focus, accessibility, session lifecycle, power and thermal policies. Profile differences must be explicit and testable.

### AOS-UR-CONTINUITY — Cross-device continuity

Support appropriately governed continuity of Context, user work and application state across supported profiles/devices. Define consent, synchronization, ownership, conflict handling, identity and privacy; do not imply continuity is available until an end-to-end implementation exists.

## 10. Runtime, applications, web and compatibility

### AOS-UR-NATIVE — Native runtime and application ABI

Native APIs remain the primary application model. Provide a stable, versioned ABI, runtime, SDK, package manifest, loader, dependency resolution, app lifecycle, permissions/capabilities and developer tools. Application processes must be sandboxed and resource-governed.

### AOS-UR-WEB — Real Web runtime/browser

A full general-purpose release requires a functional Web Runtime/browser capable of loading real pages, with network, graphics, input, security, lifecycle and update integration. Research RISC-V-capable engines, license, porting cost, sandbox boundary, ABI and security patching. A static HTML mock-up or nonfunctional web view does not meet the requirement.

### AOS-UR-ANDROID — Isolated Android compatibility

Android application compatibility is a compatibility environment, not the core OS architecture and not a project to clone the entire Android framework as native AetherOS. Prefer hardware-assisted virtualization when supported. Define ABI, filesystem, graphics, input, networking, identity/capability, process, lifecycle and policy boundaries; qualify the compatibility target with real application suites.

### AOS-UR-PACKAGES — Package and developer ecosystem

Provide application install/update/remove, dependencies, repository/catalogue, signature and integrity checks, sandbox/capability declarations, clean uninstall, rollback and diagnostics. Provide a coherent SDK, compiler/toolchain, native app build system, driver SDK, debugging, profiling, documentation, examples and conformance tests.

## 11. AI, automation, identity, security and privacy

### AOS-UR-AI — AI as a governed platform participant

AI/model/agent services may provide perception, retrieval, planning, memory, tool use and automation, but model outputs are untrusted observations or suggestions, not authoritative system facts. AI must not receive special ambient privilege.

Require explicit consent, bounded/attenuated capabilities, policy evaluation, revocation, audit trails, provenance, privacy controls, data locality, offline behavior and user-visible actions. Make model and agent execution subject to resource quotas, thermal/power governance and fault containment.

### AOS-UR-IDENTITY — User identity and sessions

Provide account lifecycle, authentication, credential and key management, session management, authorization, recovery and policy. Distinguish identity from capabilities: identity may inform policy but must not replace per-resource authority checks.

### AOS-UR-PRIVACY — Privacy and data control

Provide transparent permission grants, revocation, audit, access notifications where appropriate, data export/deletion, encryption, locality controls and clear ownership. Agents, applications and system services must not bypass privacy policy.

## 12. Resource, reliability and observability

### AOS-UR-RESOURCE — Resource contracts and governance

Make CPU, memory, storage, network, GPU, NPU, DSP, I/O, power, thermal and energy use explicit. Provide scheduling, admission, quotas, leases/reservations, accounting, pressure handling and fair policies where applicable. Mobile and foldable power/thermal behavior must be integrated into scheduling and lifecycle.

### AOS-UR-OBSERVABILITY — Diagnostics and operational accountability

Provide structured logs, event/trace spans, audit records, metrics, diagnostics and query facilities with defined privacy, integrity, retention and access policies. Support crash capture, fault correlation, service health, boot diagnostics and reproducible reports.

### AOS-UR-RELIABILITY — Fault containment and recovery operations

Include failure isolation, supervised service lifecycle, watchdog/health checks, crash containment, restart policy, rollback, backup/restore, reset, system repair, virtualization/simulation, fault injection and recovery testing. Prove behavior through complete failure scenarios, not only unit tests.

## 13. Release, build, governance and continuous research

### AOS-UR-BUILD — Reproducible system composition

Build complete system images and release artifacts from declared source/configuration. Record checksums, SBOM, dependency/license data, build provenance, signing state, supported profile/board matrix and installation instructions. Keep generated artifacts distinct from authoritative inputs. A reproducible host-side model build is not the same as a reproducible bootable product image.

### AOS-UR-QUALIFICATION — End-to-end qualification

Use architecture- and profile-specific acceptance suites, hardware/firmware qualification, negative tests, fault injection, security testing, persistence/recovery testing, performance and resource measurements, independent review and explicit release evidence. Every requirement traces:

`Owner intent → requirement key → constitutional invariant/RFC/ADR → design/interface → implementation → test/CTS → CI/target evidence → qualification state`

Keep statuses separate: Proposed, In Progress, Implemented, Partially Verified, Verified, Accepted (decision only), Blocked, Deferred, Superseded, Not Applicable (with rationale). Accepted is not Verified; code compiled is not a functioning subsystem.

### AOS-UR-RESEARCH — Research and implementation continue together

Research new architectures, kernels, capability models, memory/scheduling, storage, filesystems, device discovery, drivers, networks, graphics, HCI, AI, runtimes, accelerators, verification techniques and new hardware continuously. Investigate alternatives and trade-offs, build falsifiable prototypes and measure actual behavior. Keep research in parallel with dependency-ready implementation; do not stop at writing plans, audits or one release.

### AOS-UR-AGENTS — Controlled parallel engineering

Use specialists where available, with explicit interfaces and file ownership. One integration authority reconciles designs, reviews the integrated diffs, runs cross-component tests and records exact evidence. Avoid concurrent edits to the same architectural contract. Treat agent completion reports as claims to verify.

### AOS-UR-SAFETY — Protect the owner's host and data

Preserve Git history and local work. Do not reset, clean, overwrite workspaces, change firmware/boot state, install privileged host components, modify machine-wide environment or write to real disks without the necessary explicit authorization. QEMU disposable-disk testing is preferred for installer validation. Keep secrets and private signing material outside the repository.

## 14. First usable release versus ultimate completion

The first usable RISC-V64 release is an intermediate milestone. It must provide:
- supported firmware-mediated boot and a genuine bootable removable-media image;
- safe live mode that does not write to internal disks by default;
- installer and separately installable disk image;
- actual protected userspace and usable native application ABI;
- essential system services and reliable process/lifecycle behavior;
- functional graphical desktop on a declared desktop reference platform (or a documented usable text profile only where graphics are genuinely unavailable);
- keyboard and pointer input;
- persistent files/settings on the installed system;
- working storage, networking, time, user/session management, logs, shutdown/reboot;
- file manager, terminal, editor, settings, launcher/task switching, system monitor and basic apps;
- package/install/remove workflow with integrity;
- accessible first-run experience;
- transactional updates, health confirmation and rollback;
- reproducible artifacts, checksums, provenance, support matrix and instructions;
- functional browser/Web Runtime for a full general-purpose release.

Android compatibility, mobile telephony, every GPU/peripheral, universal board support, cloud AI and every office app need not block that first release. They must remain visible in the whole-platform inventory and be delivered according to explicit profiles/phases.

The **ultimate** completion scope includes all mandatory and accepted SHOULD-level capabilities in the governing architecture and owner-approved decisions across:
1. one coherent multi-profile OS;
2. kernel/execution/security;
3. complete device discovery and driver ecosystem;
4. storage/filesystem and data lifecycle;
5. secure boot, generations, update and recovery;
6. network and communications;
7. graphics, multimedia, external display and input;
8. integrated system services;
9. all required application runtimes, package and developer ecosystem;
10. Context-centric HCI and experience shells;
11. governed AI/automation;
12. identity/security/privacy/owner control;
13. resource/power/thermal governance;
14. health, backup, virtualization and reliability operations;
15. reproducible build, distribution, governance and documentation;
16. full-system integration, profile conformance and independent final qualification.

Nothing may silently disappear into an unspecified “future” category. Every unimplemented feature stays Open with dependencies, acceptance criteria and evidence requirements.

## 15. Execution sequence and acceptance discipline

The active v4 goal organizes first-release delivery into Phases 0–9:
- 0 preserve, verify and map the current system;
- 1 define RISC-V64 reference machine and boot contract;
- 2 protected native userspace and core services;
- 3 storage, device and network vertical slices;
- 4 functional graphical desktop;
- 5 live media and installer;
- 6 package, application and network usability;
- 7 update/recovery, security and release hardening;
- 8 physical RISC-V hardware qualification;
- 9 stable first release.

Ultimate platform completion continues through Phases 10–18:
- shared platform semantics/kernel contracts;
- complete AHDA, drivers and hardware profiles;
- storage/multimedia/graphics/external devices;
- communications/connectivity;
- runtimes/package/application ecosystem;
- Context-centric interface and platform shells;
- intelligence/privacy/policy governance;
- resources/reliability/backup/virtualization/enterprise operations;
- integration/profile conformance/final qualification.

The exact phase exit criteria are in the full v4 mandate. Work should proceed by dependency order and continue autonomously through unblocked tasks. Escalate genuine owner-only decisions, unavailable credentials/hardware, paid dependencies, destructive real-device operations or fundamental contract changes.

## 16. First end-to-end demonstration that must eventually pass

In a disposable QEMU environment:
1. Build the documented release image and verify checksum/layout.
2. Start RISC-V64 `virt` with the documented firmware, display, keyboard/pointer, USB, network and two separate disks (live source and blank target).
3. Boot through the product boot path into a live graphical desktop.
4. Verify live-mode indicator and ensure the target/internal disk has not been auto-mounted writable.
5. Use file manager, terminal, settings, editor and system monitor.
6. Create a live-session document and demonstrate ephemeral state unless live persistence is enabled.
7. Verify real network access to a test service.
8. Launch installer, inspect disks, choose the separate blank target and review its partition/generation plan.
9. Explicitly approve; install and verify the target.
10. Shut down and detach live media; boot the installed disk through the documented firmware path.
11. Complete first-run account setup, create a user file, change settings, install/run a separately packaged app, reboot and verify intended persistent state.

A boot message, shell in the kernel, mock GUI, direct-kernel QEMU-only boot, network stubs or blind file copy is not acceptance.

## 17. Status, change control and completion

- Preserve requirement IDs and history; add new requirements through controlled governance and traceability.
- Map requirements to constitutional invariants, RFC/ADR, subsystem, tests and qualification evidence.
- Research and test contradictions; do not silently harmonize.
- Keep the architecture Validation Candidate until its evidence gates pass; no artificial freeze.
- Keep the implementation-status document current and commit-scoped.
- Re-run the complete gate on exact HEAD and archive CI logs/artifacts.
- Never state complete when any mandatory requirement or accepted SHOULD-level requirement is unimplemented, unintegrated, untested or unqualified.
- A first release, boot checkpoint, kernel milestone or passing workflow never ends the project.

