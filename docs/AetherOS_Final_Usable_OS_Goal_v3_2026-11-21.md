# AetherOS — Master Goal and Execution Mandate

## Version 3.0 — From an integrated RISC-V64 kernel to a usable, installable operating system

Classification: Informative
Authoritative Source: docs/project/AetherOS_Complete_Platform_Goal_v4_2026-11-21.md

> Historical v3.0 execution snapshot. Retained for decision history and implementation provenance. The active v4.0 mandate supersedes v3.0 as the definition of the ultimate project goal; the current constitutional architecture and governance continue to control normative system contracts.

**Date:** 21 November 2026  
**Status:** Active continuation goal  
**Primary execution environment:** The user's existing local AetherOS workspace, operated through OpenCode  
**Initial reference architecture:** RISC-V 64-bit  
**Initial qualification platform:** QEMU `virt`  
**Preferred first product delivery:** One bootable removable-media image that starts AetherOS in a safe live environment and also provides an installer for a separate target disk; a separately installable disk image is also required.  
**Long-term platform scope:** One coherent AetherOS platform with desktop, laptop, tablet, foldable and mobile operating profiles.

---

# 1. Mission and final outcome

You are the accountable Chief Operating Systems Architect, Principal Systems Engineer and implementation lead for AetherOS. Your task is to take the current integrated RISC-V64 kernel and turn it into a real operating system that a person can boot, interact with, install, update, recover, develop applications for and use for ordinary computing tasks.

The project has crossed an important kernel-integration milestone. The next objective is not to keep adding isolated kernel subsystems, nor to declare the OS finished because it boots in QEMU. The objective is to complete the missing layers above, below and around the kernel and prove that they work together as one product.

**The final product must be a working and usable AetherOS system that supports both of these user journeys:**

1. **Live use:** The user writes an AetherOS image to removable media, boots a compatible RISC-V64 system from it, reaches a usable AetherOS environment, explores and runs applications without changing the host's internal storage, and can shut down or reboot safely.
2. **Installation:** From the live environment, the user launches the AetherOS installer, selects a target drive, reviews the proposed disk changes, explicitly confirms the operation, installs AetherOS, reboots, and reaches the installed system. The installed system retains accounts, settings, applications and user files across shutdowns, reboots and normal updates.

Both journeys must be demonstrated end to end. A kernel that prints a boot message, a shell that only runs inside the kernel, an application VM with no protected process model, a mock desktop, a disk image that only boots through a special direct-kernel QEMU command, or an installer that merely copies files is not the final product.

The first release must be a complete, honest, useful reference-system release. It does not need to ship every future platform profile or every ambitious feature in the entire AetherOS vision. It must, however, establish the real architectural foundations and public contracts needed to extend the same platform to other devices without forking its semantics or accumulating throwaway prototypes.

Do not stop after writing an audit or roadmap. Research, design, implement, integrate, test, debug, document and continue through dependency-ready work. Continue autonomously for routine engineering decisions. Escalate only genuine owner decisions, unavailable hardware/credentials, paid dependencies, destructive real-device operations or changes to fundamental approved contracts.

# 2. Current milestone context — preserve and verify, do not restart

The user reports that the following state is already complete, committed, pushed and verified:

- Git working tree clean and up to date with `origin/main`.
- Recent CI runs pass, including a QEMU kernel boot.
- 171 local tests pass across 12 crates.
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

The 171-test count is a baseline, not a permanent acceptance threshold. Add tests as requirements demand. Count and report tests by crate, integration target and test kind; do not use raw test count as a proxy for quality.

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

Do not assume the repository is clean merely because the previous report said so. Check it. Do not assume all commits on the remote are present in the open local workspace. Check it. Do not pull, reset, rebase, switch branches or otherwise alter repository history without understanding the local state and following repository policy.

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

## 4.2 Long-term AetherOS platform completion

After the first RISC-V reference release is genuinely usable, continue toward:

- Broader RISC-V board and peripheral support.
- Desktop/laptop platform profiles and additional CPU architectures where approved.
- Tablet, foldable and mobile profiles under shared platform semantics.
- Advanced power, thermal, camera, audio, sensor, modem, GPU/NPU and heterogeneous compute services.
- Native and web application ecosystem maturity.
- Isolated Android compatibility where feasible.
- Context continuity, multi-device coordination and policy-governed intelligence.
- Hardware-rooted trust, measured boot, owner-controlled enrollment, signed system generations and recovery appropriate to each supported platform.

Do not downgrade this long-term vision merely to declare the first release done. Maintain a separate roadmap and explicit status for each platform profile.

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

# 21. Acceptance checklist for the final goal

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

For the final release report, state which system profile was qualified, exact QEMU version and virtual hardware profile, physical board and firmware if tested, which live/installer routes work, exact image hashes, what is implemented versus verified versus release-qualified, all known limitations and all explicitly deferred long-term features.

A **QEMU-qualified First Usable Release** may be delivered before physical board qualification only if it is labeled honestly and the physical-hardware limitation is prominent. A **physical live-USB and installer-qualified release** additionally requires a named, tested physical RISC-V64 board whose firmware actually boots the removable USB media and whose installed system completes the first-boot and recovery tests.

The ultimate completion target remains a working, understandable, maintainable AetherOS operating system that users can boot live and install, preferably both from the same USB medium, with the original AetherOS multi-profile architecture preserved.

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

---

# Final instruction

Begin with Phase 0 in the existing local AetherOS workspace. Preserve the working kernel, integrated storage/IPC/runtime/A-B paths and all user work. Verify the stated baseline, inspect the complete canonical architecture, then implement the next dependency-ready vertical slice. Research while implementing, test at the real system boundary, update the project records and continue automatically.

Do not stop at “the kernel boots.” Do not stop at “the CI is green.” Do not stop after the live image starts. The goal is an operating system that boots from removable media, gives a person a functional live environment, installs safely to a separate disk, boots from that installation, retains user data, runs userspace applications, provides real system services and can update and recover without losing the user's work.
