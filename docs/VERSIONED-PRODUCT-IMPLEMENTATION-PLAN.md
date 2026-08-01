# AetherOS Versioned Product Implementation Plan — 0.0.0 to 2.x

Classification: Operational
Authoritative Source: VERSIONED-PRODUCT-IMPLEMENTATION-PLAN.md
Requirement-ID: GOV-PRODUCT-PLAN-001
Status: Active Planning Baseline

**Established:** 2026-08-01
**Scope:** Product implementation versions, not architecture-document versions.
**Current repository mapping:** Host reference prototype only; approximately a partial `0.0.3` evidence set. No product version has shipped.

## 1. Purpose and interpretation

This plan starts at a hypothetical empty implementation (`0.0.0`), maps the evidence already present in this repository, defines a minimal useful product at `1.0.0`, expands it to the intended complete supported product at `2.0.0`, and reserves `2.0.1+` for independently gated evolution. It does **not** claim that changing a version number makes a gate pass.

A release exists only when every mandatory gate for one named configuration has reproducible evidence. Work may proceed in parallel, but versions cannot skip unmet safety, security, recovery, or compatibility gates. Architecture version `v1.1` and product version `1.0.0` are separate namespaces.

“State of the art” is a selection process, not an adjective. Each subsystem must have:

1. current alternatives and prior art reviewed at design time;
2. explicit correctness, security, latency, throughput, memory, storage, energy, portability, and maintenance budgets;
3. a reference model or precise contract;
4. negative, property, fuzz, fault-injection, and recovery tests appropriate to its risks;
5. measurements against at least one mature baseline where comparable;
6. a recorded tradeoff decision and rollback path;
7. an owner and support lifetime.

No implementation can maximize all properties simultaneously. Correctness and recoverability come first, followed by security, required compatibility, predictable performance, privileged-TCB size, installed size, and optional breadth.

## 2. Feasibility conclusion

### 2.1 Implementable now

Current technology is sufficient to build a capability-oriented, memory-safe-where-practical, microkernel/multiserver OS with:

- RISC-V64, AArch64, and x86-64 ports;
- isolated user-space services and drivers;
- virtual memory, preemptive scheduling, IPC, networking, storage, graphics, audio, USB, and power management;
- signed reproducible images, measured boot, atomic updates, rollback, SBOMs, provenance, and fleet operation;
- native Rust/C applications, WebAssembly components, POSIX source portability, and selected foreign compatibility through personalities or VMs;
- formal verification of carefully bounded semantic and privileged-core surfaces.

### 2.2 Implementable only incrementally

Broad hardware support, Linux and Win32 user ABI coverage, Android integration, high-performance graphics, enterprise certification, distributed storage, and proof expansion are feasible only as continuously maintained subproducts with named versions and test matrices.

### 2.3 Not a finite deliverable

Compatibility with every application, kernel module, proprietary driver, device, historical quirk, and future technology is not a closable requirement. Proprietary licensing, unavailable hardware documentation, physical failures, undecidability, specification errors, and undefined legacy behavior cannot be removed by schedule or compute. The product instead maintains compatibility tiers, provider interfaces, guests, driver VMs, and an evidence database.

## 3. Product definitions

### 3.1 `1.0.0` — Minimal Useful Reference System

`1.0.0` is a small but genuine general-purpose reference OS, not a demo. Its mandatory supported configuration is **QEMU `virt` RISC-V64 using OpenSBI**, with a published CPU, memory, interrupt, timer, UART, virtio, firmware, and emulator matrix. It includes:

- reproducible boot and recovery images;
- a small capability-mediated privileged core;
- physical and virtual memory, processes/domains, threads, preemptive scheduling, time, IPC, and lifecycle control;
- user-space service supervision and restart;
- UART, entropy, virtio-block, virtio-net, and a minimal console/input path;
- a crash-recoverable storage provider and conventional filesystem projection;
- IPv4/IPv6 networking sufficient for DHCP/static configuration, DNS, TCP, UDP, ICMP, and secure remote administration;
- native Rust and C SDKs, ELF loading, a minimal libc subset, shell, core utilities, service manager, package/image composition, logging, tracing, and diagnostics;
- signed A/B system updates, rollback, recovery, key rotation procedure, SBOM, provenance, and vulnerability-response process;
- WebAssembly Component/WASI support as an optional user-space runtime;
- one demonstrable isolated network service and one local interactive application;
- documented backup/restore, resource exhaustion, crash, corruption, security, performance, and 30-day stress evidence.

`1.0.0` does **not** promise a desktop GUI, broad physical-hardware support, Linux binary compatibility, Windows applications, Android, containers without a guest, SMP optimization, proprietary GPUs, or UASA as the default filesystem.

### 3.2 `2.0.0` — Complete Supported Product Baseline

“Complete” means the supported catalog promised by this plan is operational, not that all possible computing is implemented. `2.0.0` adds maintained RISC-V64, AArch64, and x86-64 reference configurations; SMP/NUMA; mature storage/network/graphics/audio/input/USB/power stacks; desktop, cloud, developer, and selected embedded profiles; virtualization; initial Linux workload products; production installer/composer; enterprise operations; and an independently reviewed assurance case. Every feature remains profile-selectable.

### 3.3 `2.0.1+` — Open-ended evolution

Post-2.0 work is released by independent provider/profile/compatibility gates. Experimental technology never becomes core merely to increase the feature count.

## 4. Required architecture and technology baseline

Selections are reviewed and pinned per release; newer is not automatically better.

| Area | Baseline direction | Mandatory evidence before product use |
|---|---|---|
| Implementation language | Stable Rust `no_std` for new privileged code; narrowly inventoried assembly; C only behind isolated/reviewed boundaries | Compiler pin, unsafe inventory, Miri/fuzz/static analysis where applicable, ABI tests |
| Kernel shape | Capability microkernel with policy, drivers, filesystems, networking, compatibility, and most recovery logic outside the smallest privileged core | TCB size, syscall/IPC budgets, authority graph, fault-containment tests |
| First ISA/platform | RISC-V64 QEMU `virt`, OpenSBI/SBI, Sv39; architectural timer and supported interrupt controller declared exactly | Reproducible boot, trap/page-table tests, emulator/version lock |
| Later boot/platform | UEFI where required; ACPI/device tree as profile-specific discovery inputs | Malformed-table fuzzing, measured-boot chain, recovery boot |
| Live authority | Compact domain-local generational handles under a Software Capability Provider; provider-neutral semantics | Differential CTS, stale/forge/revocation tests, constant/bounded hot path |
| Native authority research | CHERI/native tagged provider kept orthogonal to semantic authority | Hardware assumptions, semantic equivalence, fallback and migration |
| IPC/IDL | Versioned capability-aware messages; bounded zero-copy/loan paths only when lifetime and revocation are proven | Parser fuzzing, cancellation/backpressure, priority-inversion analysis |
| Scheduling | Small deterministic core plus replaceable policy layer; admission/resource budgets | Model, WCET/bounds where claimed, overload/fairness tests |
| Storage | Provider-neutral transaction semantics; conventional block backend first; UASA only after independent evidence | Crash at every persistence point, corruption/space-loss/repair tests |
| Networking | Memory-safe user-space stack where performance permits; isolated acceleration path | Protocol conformance, fuzzing, hostile traffic, throughput/latency |
| Drivers | User-space Rust drivers first; IOMMU-mediated DMA; driver VM for irreducible foreign code | Register-model review, DMA denial tests, reset/hot-unplug campaigns |
| Applications | Native Rust/C ABI plus optional WebAssembly Component/WASI runtime | ABI/version tests, sandbox escapes, reproducible SDK |
| Virtualization | Hardware virtualization on supported later platforms; virtio device model | Isolation, migration/snapshot semantics, guest matrix |
| Updates | Content-addressed cohorts, threshold-signed metadata, A/B activation, anti-rollback and recovery | Power-cut campaign, compromised-key/rollback drills |
| Supply chain | Hermetic builds, lockfiles, signed provenance, SPDX/CycloneDX SBOM, vulnerability process | Rebuild comparison, dependency/license/advisory gates |
| Verification | Executable models and property tests first; model checking/proofs for capability, IPC, scheduling, VM, update, and recovery boundaries | Checked artifacts, named assumptions, proof maintenance owner |
| Observability | Structured events, traces, metrics, audit commitments; secrets excluded by construction | Schema/version tests, pressure behavior, privacy review |

## 5. Cross-cutting release gates

Every release candidate from `0.1.0` onward must publish:

- exact source revision, toolchain, dependency locks, build recipe, artifacts, SBOM, provenance, and licenses;
- threat model and trust/authority boundaries;
- unsafe and privileged-code inventory;
- test inventory with pass/fail/waiver records;
- benchmark protocol and results on named configurations;
- binary, boot-image, resident-memory, persistent-storage, and TCB sizes;
- compatibility and hardware matrices;
- crash/fault/security campaign results;
- known limitations, migration/rollback procedure, support owner, and end-of-life policy.

A red test, unexplained authority, unrecoverable supported update, silent data loss, critical untriaged vulnerability, non-reproducible release, or missing recovery path blocks promotion.

## 6. From zero to the minimal useful build (`0.0.0`–`1.0.0`)

### `0.0.0` — Empty product baseline

- No implementation claims.
- Establish product/version namespaces, repository layout, licenses, governance, contribution policy, and claim vocabulary.
- Record supported-host assumptions and prohibit “universal,” “verified,” and “production-ready” without scoped evidence.

**Gate:** project can distinguish requirements, models, prototypes, target code, generated artifacts, and product evidence.

### `0.0.1` — Reproducible engineering foundation

- Pin stable Rust and required components; create hermetic development container/lock definition.
- Add CI for formatting, compile, tests, Clippy/rustdoc with warnings denied, repository validators, license/advisory checks, and artifact retention.
- Add deterministic task runner, release manifest schema, SBOM/provenance generation, and dependency update policy.

**Gate:** two clean environments reproduce byte-identical reference artifacts or document every unavoidable difference.

**Current status:** in progress. The Rust toolchain, container base, CI actions,
locked Cargo operations, offline dependency-source/license gate, deterministic
host-reference comparison, evidence schema, SPDX package SBOM, and CI evidence
retention are implemented. The development container's Debian package
installation is not yet snapshot-pinned, advisory evidence is not mirrored, and
two independent clean builders have not yet reproduced a release; therefore the
`0.0.1` gate remains open.

### `0.0.2` — Truth, threat, and budget baseline

- Reconcile all status/roadmap contradictions.
- Define threat models for bootstrap, capability provider, kernel, drivers/DMA, storage, update, network, supply chain, and compatibility guests.
- Set initial privileged-code, boot-image, RAM, IPC latency, context-switch, capability-check, recovery-time, and power budgets.
- Define unsafe-code ownership and independent review rules.

**Gate:** no planned privileged component lacks an owner, authority description, failure policy, or measurable budget.

### `0.0.3` — Executable semantic reference

- Provider-neutral capability trait with abstract oracle and Software Capability Provider.
- Transaction/effect oracle with deterministic recovery.
- Machine-readable CTS fixtures, stable result schema, negative tests, and exact error semantics.
- Map every executable case to requirements and evidence records.

**Current status:** partial. The repository has capability and transaction/effect prototypes and 12 CTS scenarios, but not yet a stable provider trait, normative fixture format, or complete requirement traceability.

**Gate:** model/provider results match for all checked deterministic histories.

### `0.0.4` — Generated and adversarial evidence

- Property-based operation histories for authority conservation, attenuation, domain isolation, revocation, replay, and recovery.
- Coverage-guided fuzz targets for handles, messages, manifests, persisted frames, boot metadata, and future device descriptions.
- Deterministic fault injection for allocation failure, exhaustion, truncation, corruption, reordering, duplicate completion, timeout, and uncertain external effects.
- Sanitizer/Miri/loom-style host campaigns where meaningful.

**Gate:** published duration/seed/corpus campaigns produce no semantic divergence, panic, unbounded allocation, or unauthorized visible effect.

### `0.0.5` — Mechanized critical contracts

- Refine and accept capability-rights algebra only after per-object rights, lifetime, delegation depth, epoch, recovery, and multi-parent policy are resolved.
- Model capability transfer/revocation, transaction visibility, scheduler states, IPC cancellation, and update rollback in suitable proof/model-checking tools.
- Trace assumptions to executable tests; proofs never replace integration testing.

**Gate:** proof checkers/model checkers run reproducibly and every excluded assumption is explicit.

### `0.0.6` — Target contracts and toolchain

- Approve the allocator, compiler/runtime, driver, boot/HAL, IPC ABI, syscall ABI, object-lifecycle, storage-provider, and update contracts required by the vertical slice.
- Create target JSON/specification, linker scripts, ELF conventions, calling convention, unwind/panic policy, symbol/debug strategy, and host tools.
- Decide stable-versus-nightly compiler requirements based on evidence, not preference.

**Gate:** a trivial freestanding ELF is reproducibly built, inspected, signed, loaded, and debugged without granting ambient authority.

### `0.1.0` — Reproducible boot nucleus

- Boot under pinned QEMU/OpenSBI; parse declared boot metadata defensively.
- Initialize stack, BSS, CPU-local state, early UART, panic/crash record, trap vector, timer, and shutdown/reboot.
- Produce separate normal and recovery images.

**Gate:** 10,000 deterministic boots plus malformed-input/fault campaigns; every panic produces bounded diagnostics and no secret leakage.

### `0.2.0` — Memory and protection nucleus

- Physical-page allocator, reserved-region tracking, Sv39 address spaces, ASIDs where supported, guarded stacks, copy/validation primitives, and typed kernel objects.
- Capability-space core with bounded lookup, generation retirement, rights/bounds/type/domain checks, explicit root creation, and revocation.
- Eliminate dynamic allocation from declared interrupt/scheduler hot paths.

**Gate:** isolation, exhaustion, fragmentation, stale mapping/TLB, forged-handle, revocation, and allocator-invariant campaigns pass; budgets are measured.

### `0.3.0` — Execution, time, IPC, and lifecycle

- Threads, domains/processes, preemption, deterministic scheduler core, policy service, timers, cancellation, endpoint IPC, capability transfer, bounded messages, notifications, and wait primitives.
- User/kernel transition, syscall dispatch, structured error ABI, service lifecycle, kill/restart, and resource accounting.

**Gate:** no lost wakeups or authority amplification in modeled/tested histories; priority inversion and overload behavior are bounded and published.

### `0.4.0` — Device and durable-state vertical slice

- User-space driver framework, device discovery subset, IRQ routing, DMA window API, IOMMU abstraction, entropy, UART, virtio-block, and virtio-net.
- Block-backed append/recovery implementation derived from the state oracle, with explicit flush/barrier contract, checksums, dual metadata roots, space accounting, and repair scanner.
- Minimal hierarchical namespace/file projection sufficient for configuration, packages, logs, and user data.

**Gate:** crash at every persistence transition; corrupt/lost/duplicate/reordered completion tests; unauthorized DMA and hot-unplug/reset campaigns; verified backup/restore.

### `0.5.0` — Minimal userland and SDK

- Component loader, init/service manager, identity/session service, configuration, logging, shell, core utilities, secure clock policy, random API, and diagnostics.
- Native Rust SDK, C ABI/libc subset, IDL generator, build templates, debugger/symbol workflow, and optional WebAssembly runtime.
- Capability-safe filesystem, process, time, IPC, and network APIs.

**Gate:** hermetic hello-world, local interactive application, service crash/restart, SDK compatibility, malformed ELF/component, and resource-limit tests pass.

### `0.6.0` — Networked useful system

- User-space IPv4/IPv6 stack, neighbor discovery, DHCP/static setup, DNS resolver, routing, ICMP, UDP, TCP, sockets/projection, firewall policy, and secure remote administration.
- Network service isolation, certificate/key storage, time bootstrap policy, and rate/resource limits.

**Gate:** protocol suites, packet fuzzing, hostile-network soak, loss/reordering, connection exhaustion, throughput/latency, and secret-handling tests pass.

### `0.7.0` — Composition, packages, update, and recovery

- Deterministic dependency/profile solver and signed manifest.
- Content-addressed package/image format with licenses, SBOM, provenance, hardware/authority declarations, and reproducible builds.
- Threshold-signed A/B cohort update, schema migration declaration, anti-rollback evidence, key rotation/revocation, health confirmation, rollback, and offline recovery.

**Gate:** power cut at every update transition, full-disk, bad signature, expired/revoked key, downgrade, incompatible schema, and recovery-media drills pass.

### `0.8.0` — Security and operational completeness

- Measured/verified boot integration for the reference platform where emulated evidence is meaningful.
- Audit export, least-authority service manifests, credentials/key lifecycle, backup/restore tooling, quota/resource governance, secure defaults, and vulnerability response process.
- Continuous fuzzing and reproducible benchmark dashboard.

**Gate:** internal red-team campaign, threat-model closure, recovery-time objective tests, restore verification, and zero unresolved critical findings.

### `0.9.0` — Release candidate and compatibility floor

- Freeze only the `1.0` supported native ABI/SDK surface with an evolution mechanism.
- Complete POSIX-source subset declaration; optional WASI component profile.
- Installer/image composer for the reference configuration, documentation, tutorials, administrator runbooks, diagnostic bundles, and compatibility database.
- Execute long stress, chaos, security, performance, footprint, and upgrade-from-every-supported-candidate campaigns.

**Gate:** independent security review; 30-day stress run; release-candidate update/rollback/restore drills; all blockers closed or explicitly removed from scope.

### `1.0.0` — Minimal Useful Reference System release

Ship only after the `1.0.0` product definition and every cross-cutting gate pass for the named QEMU RISC-V64 configuration. Publish source, binaries, recovery image, SDK, symbols, SBOM, provenance, test/benchmark evidence, threat model, known limitations, compatibility manifest, support window, and signing-key policy.

**Promotion prohibition:** a booting image, passing unit tests, or a demo application alone is not `1.0.0`.

## 7. From minimal product to complete supported product (`1.0.1`–`2.0.0`)

Patch versions contain compatible security, correctness, recovery, and narrowly scoped hardware fixes. Minor versions add profile/product capability behind explicit manifests and upgrade gates.

### `1.0.1`–`1.0.x` — Stabilization/LTS foundation

- Security and correctness fixes, recovery hardening, performance regressions, documentation, and SDK-compatible refinements.
- Establish LTS branch, coordinated disclosure, key ceremonies, reproducible rebuild service, telemetry-optional support bundles, and release rollback exercises.

### `1.1.0` — SMP and multi-architecture reference ports

- SMP, CPU hotplug where supported, scalable scheduling, shootdown, per-CPU allocation, RCU/epoch mechanisms with proofs/tests appropriate to use, and NUMA foundations.
- QEMU AArch64 and x86-64 ports with UEFI/ACPI as required and semantic CTS equivalence.

### `1.2.0` — Hardware platform and driver expansion

- PCIe, MSI/MSI-X, IOMMU, NVMe, USB xHCI, HID, firmware framebuffer, selected NICs, suspend/resume, thermal/power framework, and published hardware compatibility lab.
- Driver sandboxing, reset/recovery, generated register descriptions, and driver-VM framework.

### `1.3.0` — Desktop and accessibility profile

- Compositor/window server, input methods, font/text stack, clipboard/drag-and-drop mediation, audio, media sandbox, printing option, accessibility APIs, localization, settings, notifications, and graphical installer/shell.
- GPU begins with virtio-gpu; physical acceleration is per-vendor and never a universal claim.

### `1.4.0` — Cloud, container, and virtualization profile

- Hardware virtual machine monitor, virtio devices, confidential-workload hooks where supported, snapshots, resource accounting, virtual networking/storage, orchestration API, and Linux micro-VM based OCI workload product.
- Direct container hosting is deferred until its exact Linux semantic surface is implemented and measured.

### `1.5.0` — Storage scale and UASA candidate

- Mature volume management, encryption, snapshots, quotas, scrub/repair, replication and backup protocols, online growth, discard/zoned support, and storage observability.
- Integrate UASA only as a removable provider after independent reproduction, observational-equivalence, power-loss, repair, performance, and migration evidence. Maintain a conventional provider and rollback path.

### `1.6.0` — Compatibility products

- Declared POSIX source profile, selected Linux user ABI personality, Linux guests, WebAssembly components, and initial lawful Win32 personality/Windows guest integration.
- Every application family receives a compatibility tier, corpus, deviations, overhead, legal prerequisites, maintainer, and last-tested record.

### `1.7.0` — Developer and language ecosystem

- Stable native SDK, package registry, IDE/debug/profiling integrations, sanitizer support, language bindings, service templates, reproducible cross-build farm, API deprecation tooling, and compatibility CI.
- Self-hosting is optional and must not compromise hermetic builds.

### `1.8.0` — Enterprise operations

- Fleet inventory/policy/update waves, HA patterns, remote attestation where supported, centralized audit, backup orchestration, disaster recovery, secrets integration, compliance evidence exports, and supported migration between reference configurations.

### `1.9.0` — Assurance and 2.0 release candidate

- Expand machine-checked assurance over selected privileged paths; complete independent review and red-team programmes.
- Multi-month stress/chaos campaigns, cross-version migrations, fleet rollback, hardware fault labs, capacity testing, performance/energy tuning, and final API/profile compatibility review.

### `2.0.0` — Complete Supported Product Baseline

Ship maintained embedded/reference, desktop, cloud/developer, and virtualization profiles on named RISC-V64, AArch64, and x86-64 configurations. Include mature native SDK/userland, storage/network/device stacks, composition/update/recovery, optional GUI and VM-based compatibility products, enterprise operations, and a scoped assurance case.

The release must publish what remains unsupported. `2.0.0` is not “compatible with everything,” “fully verified,” or “best at every workload.”

## 8. Evolution after `2.0.0` (`2.0.1+`)

### Compatible patch stream: `2.0.1` onward

Security, correctness, recovery, performance, and supported-device fixes with no undeclared ABI or authority expansion. Emergency releases still require signed provenance, regression tests, and rollback evidence.

### Future minor-version lanes

| Lane | Examples | Admission gate |
|---|---|---|
| Native capability hardware | CHERI-RISC-V or successor tagged architectures | Provider semantic equivalence, hardware threat model, benchmarks, fallback |
| Confidential computing | Confidential VMs, memory encryption, attestation | Side-channel scope, key ownership, migration/recovery semantics |
| CXL/disaggregated memory | Tiered/coherent memory providers | Consistency, poisoning, revocation, latency and failure containment |
| Accelerators | GPU, TPU, NPU, FPGA, DPU | IOMMU isolation, firmware/license lifecycle, cancellation/reset, CTS |
| Compatibility | Broader Linux/Win32/Android/BSD products | Named ABI/API version, differential corpus, legal review, owner |
| Storage | Distributed, erasure-coded, object, ZNS, archival providers | Provider equivalence, repair, space/metadata amplification, migration |
| Real-time/safety | Bounded scheduler, certified profiles | Separate hazard analysis, tool qualification, certification programme |
| New ISAs/platforms | Additional RISC-V profiles, POWER, emerging architectures | Maintained toolchain, HAL/CTS, hardware lab, lifecycle commitment |
| Research computation | Quantum, neuromorphic, photonic | Real hardware/API, explicit classical control/failure model, experimental profile |

Major `3.0.0` is justified only by a necessary incompatible product contract with a migration path, never by marketing schedule.

## 9. Workstreams and ownership

A credible programme needs independently accountable leads for:

- architecture/governance and requirements traceability;
- privileged kernel, HAL, and architecture ports;
- formal methods and semantic reference models;
- memory, scheduler, IPC, and real-time behavior;
- storage/UASA and recovery;
- networking and distributed systems;
- drivers, IOMMU, graphics, audio, USB, and power;
- toolchain, ABI, SDK, libc, WebAssembly, and developer experience;
- composition, packages, updates, supply chain, and release engineering;
- virtualization and compatibility products;
- security, cryptography, identity, audit, fuzzing, and red team;
- user experience, accessibility, localization, documentation, and support;
- performance, energy, hardware lab, SRE, legal/compliance, and partnerships.

AI systems may draft code, models, proofs, tests, and documentation, but cannot approve their own output. Human maintainers remain accountable for specifications, merges, signing keys, incident response, legal decisions, and product support.

## 10. Immediate critical path from the current repository

1. Correct status truth: this repository has a host CTS and transaction model; RFC-0042/0043 are Draft.
2. Add pinned toolchain, CI, locked builds, SBOM/provenance, dependency policy, and evidence retention (`0.0.1`).
3. Publish threat model and measurable budgets (`0.0.2`).
4. Extract provider-neutral capability interfaces and machine-readable fixtures (`0.0.3`).
5. Add generated histories, fuzzers, fault injection, benchmarks, and requirement traceability (`0.0.4`).
6. Resolve and mechanize RFC-0043 and required transaction/update contracts (`0.0.5`).
7. Approve target ABI/HAL/allocator/driver contracts and freestanding toolchain (`0.0.6`).
8. Only then begin the `0.1.0` RISC-V64 boot nucleus.

Starting broad compatibility, a GUI, physical GPU drivers, or a new filesystem before this path would increase code volume without demonstrating the architecture's load-bearing claims.

## 11. Definition of done

A version is done when its named supported configuration can be built, installed, booted, operated, updated, rolled back, diagnosed, backed up, restored, and securely retired using published artifacts and procedures; all mandatory tests and evidence pass; known limitations are explicit; and an owner accepts the support obligation. A feature is not done merely because code exists.
