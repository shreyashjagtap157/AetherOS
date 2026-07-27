# Product Feasibility, Scope, and Engineering Truth

Classification: Operational
Authoritative Source: PRODUCT-FEASIBILITY-AND-SCOPE.md
Requirement-ID: GOV-PROD-001
Status: Active

## 1. Product intent

AetherOS SHALL attempt the most comprehensive technically and legally implementable platform: a small privileged core, optional system services, install-time feature selection, multiple compatibility personalities, virtual machines for irreducible legacy environments, and forward-compatible provider contracts for new computation substrates. Ambition is not a conformance claim. Every claim requires a reproducible test, supported configuration, and measured budget.

The project optimizes the **installed deployment selected by a user**, not the size of the complete source tree or universal package repository. The complete distribution may be large. A deployment contains only the selected profile, hardware support, compatibility layers, languages, services, and recovery assets plus their dependency closure.

## 2. Non-negotiable honesty rules

1. “All”, “universal”, “production-ready”, “verified”, and “compatible” are prohibited release claims without a published scope and passing evidence.
2. Unsupported does not mean impossible; it means no maintained, tested contract exists yet.
3. A translated or virtualized application is supported only at the declared compatibility tier.
4. Formal models, proofs, tests, audits, and certifications are distinct evidence classes and SHALL NOT be substituted for one another.
5. AI-generated implementation or proof text is untrusted until independently compiled, tested, reviewed, and checked by the relevant proof kernel.
6. Proprietary protocols, firmware, applications, and drivers SHALL be used only under applicable licenses. Clean-room reimplementation requires independently documented teams and legal review.
7. New technology enters through versioned provider interfaces and experimental profiles; it does not enlarge the mandatory trusted core.

## 3. Optimization order

When goals conflict, decisions use this order unless a profile overrides it:

1. Correctness and recoverability.
2. Security and least authority.
3. Compatibility required by the selected deployment.
4. Predictable performance and efficiency.
5. Small trusted computing base.
6. Small installed footprint.
7. Breadth of optional features.

“Comprehensive” applies to the catalog. “Small” applies to the resolved installation and privileged core. Performance SHALL be reported per workload, platform, percentile, power envelope, and feature set; there is no universal fastest configuration.

## 4. Installer and image composition requirements

The installer SHALL ask, directly or through a chosen preset:

- hardware architecture and exact platform/device model;
- Embedded, Desktop, Cloud, HPC, Safety-Critical, Development, or Custom profile;
- native, POSIX-source, Linux-binary, Windows, Android, WebAssembly, container, and whole-OS VM compatibility needs;
- graphical shell, accessibility, localization, fonts, media, printing, and gaming needs;
- storage providers, UASA (when available), encryption, snapshots, replication, backup, and recovery needs;
- network protocols, server roles, remote administration, and offline requirements;
- GPU/accelerator vendors and compute/graphics APIs;
- developer toolchains and language runtimes;
- assurance level, measured boot, audit retention, update cadence, and availability target.

The composition solver SHALL calculate a deterministic dependency closure, show estimated download/installed/boot-memory sizes, identify conflicting requirements, preserve a recovery path, emit a signed manifest, and allow later atomic addition or removal. Unselected drivers and compatibility layers SHALL not be installed. Hardware discovery may recommend features but SHALL not silently grant authority or install proprietary terms.

## 5. Feasibility classification

| Objective | Classification | Required approach |
|---|---|---|
| Small capability-secure privileged core | Feasible | Microkernel/multiserver design; measured TCB budget |
| RISC-V64, x86-64, and AArch64 | Feasible, expensive | Shared semantic core plus independently tested HALs |
| Broad source compatibility | Feasible | Stable C ABI, POSIX projection, Rust SDK, WebAssembly components |
| Broad Linux user ABI | Feasible incrementally | Versioned Linux personality and/or Linux micro-VM |
| Windows applications | Partial | Wine-like personality where legal; Windows VM otherwise |
| Android applications | Partial | Android framework/container or Android VM |
| macOS/iOS applications | Severely constrained | Portable source where licensed; lawful guest virtualization only |
| Foreign kernel modules and drivers | Partial and high risk | Driver VM, device proxy, reimplementation, or guest OS |
| Proprietary GPU stacks | Vendor-dependent | Native vendor cooperation, API translation, or passthrough VM |
| Every historical and future app/device | Open-ended, never complete | Compatibility catalog; no universal claim |
| Fully verified Windows/Linux-scale ecosystem | Research programme | Proof-carrying boundaries and prioritized verification, not an assumed outcome |

## 6. Legacy mechanisms

AetherOS SHALL not put legacy semantics in the kernel merely to claim compatibility. A legacy ABI is implemented as a removable **personality** above modern native contracts, isolated in a component or guest. The native interface remains capability-oriented, asynchronous where appropriate, versioned, memory-safe where practical, and explicit about failure.

A legacy requirement is accepted only after recording: applications enabled, exact observable semantics, legal status, attack surface, maintenance owner, performance cost, containment boundary, conformance tests, and an exit/migration strategy. When an application fundamentally depends on obsolete behavior, the choices presented for review are: emulate it, virtualize its original OS, modify/recompile it, accept reduced compatibility, or defer it until a safe implementation exists.

## 7. Formal verification at scale

Unlimited time, compute, or language-model inference does not remove specification errors, undecidability, state explosion, proprietary opacity, hardware errata, undefined behavior, or proof-maintenance cost. NVIDIA NIM or other LLM infrastructure MAY draft models, invariants, tests, code, and proof candidates. It SHALL NOT approve its own output.

Verification expands in concentric boundaries:

1. Mathematical capability and transaction semantics.
2. Boot, IPC, address-space, scheduler, and capability kernel paths.
3. Memory allocators, IOMMU ownership, update/rollback, and storage recovery.
4. Critical user-space services and parsers.
5. Compatibility personalities by properties and differential tests.
6. Drivers using isolation, generated register models, fuzzing, and selected proofs.

The proof checker, compiler, hardware assumptions, generated code, and unsafe code inventory are explicitly included in each assurance claim.

## 8. Innovation policy

New CPU ISAs, CHERI-like capabilities, CXL memory, confidential computing, DPUs, FPGAs, GPUs, tensor accelerators, quantum devices, neuromorphic devices, and photonic devices enter via experimental provider contracts. A provider must declare memory consistency, authority mapping, isolation, failure, cancellation, timing, observability, attestation, update, and fallback semantics. Experimental support is never described as production support until hardware exists, CTS fixtures pass, and a maintainer commits to lifecycle support.

## 9. Production-ready definition

A configuration is production-ready only when its exact manifest has: supported hardware, reproducible builds, signed atomic updates and rollback, SBOM and provenance, threat model, independent security review, fuzzing, fault injection, recovery testing, performance and power baselines, compatibility results, operational documentation, vulnerability response ownership, backup/restore validation, and a stated support lifetime. Production readiness is per configuration—not inherited by every package in the catalog.
