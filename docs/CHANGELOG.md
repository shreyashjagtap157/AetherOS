# Changelog — Ideal Computing Platform

Classification: Generated
Authoritative Source: Amendment records in ARCHITECTURE-GOVERNANCE.md

All notable changes to the Ideal Computing Platform specification suite are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] — Active Development

### Added
- Complete 41-RFC normative specification suite across 5 peer architectures
- `Platform-Architecture-Specification-v1.1.md` — frozen architecture with 13 invariants
- `Governance-RFC.md` — amendment process and RFC conventions
- `Profiles-Specification.md` — 5 deployment profiles (Embedded, Desktop, Cloud, HPC, Safety-Critical)
- `Conformance-Test-Suite-Methodology.md` — 4-layer CTS methodology
- `Glossary.md` — 100+ term definitions
- `00-RFC-Index.md` — master RFC index with invariant-to-RFC cross-reference
- `Revision-History.md` — architecture revision log

### Added (RFCs)
- **Composition (5):** RFC-0002 Component Manifest, RFC-0003 Capability Routing, RFC-0004 Dependency Resolution, RFC-0005 Profile Selection, RFC-0006 Composition Conformance
- **Execution (8):** RFC-0007 Execution Domain + Bootstrap, RFC-0008 Dispatch, RFC-0009 Address Space, RFC-0010 Interrupt Routing, RFC-0011 Scheduler (Layers 1+2), RFC-0012 Time, RFC-0013 Capability Mediation, RFC-0014 Execution Conformance
- **State (9):** RFC-0015 Object Identity, RFC-0016 Transaction Model, RFC-0017 Integrity Model, RFC-0018 Lifecycle FSM, RFC-0019 Storage Provider Interface, RFC-0020 POSIX Projection, RFC-0021 Object Projection, RFC-0022 Key-Value Projection, RFC-0023 State Conformance
- **Communication (7):** RFC-0024 Message Format, RFC-0025 Endpoint Model, RFC-0026 Routing, RFC-0027 Capability Transfer, RFC-0028 Synchronization, RFC-0029 Streaming, RFC-0030 Communication Conformance
- **Observability (6):** RFC-0031 Event Model, RFC-0032 Trace Span, RFC-0033 Replay Format, RFC-0034 Audit Log, RFC-0035 Diagnostic Query, RFC-0036 Observability Conformance
- **Cross-Cutting (5):** RFC-0037 Capability Token Format, RFC-0038 Capability Transfer Protocol, RFC-0039 Capability Revocation Protocol, RFC-0040 Verification Tier Methodology, RFC-0041 CTS Cross-Cutting Supplement

### Added (Implementation)
- `IMPLEMENTATION-ROADMAP.md` — phased implementation plan (Phase 1–5: x86-64/ARM64 boot → core platform → RISC-V/POWER/GPU → TPU/FPGA/DPU → quantum/neuromorphic)
- `Hardware-Support-RFC.md` — hardware abstraction architecture (execution-provider contract, HAL, x86-64, ARM64, GPU, TPU, FPGA, DPU, quantum deferred, CXL/bus interconnects)
- `CHANGELOG.md` — change log tracking all additions and revisions

### Changed
- Architecture promoted from v1.0 to v1.1 (frozen)
- Added I-12 (origin of authority) and I-13 (address–authority orthogonality) invariants
- Added bootstrap model (5-stage: Firmware → Minimum Executor → Capability Root → Composition Root → User Components)
- Added quantitative complexity budgets (5 metrics)
- Added economic cost tracking for verification tiers
- Added Communication–Execution layering acknowledgment

### Added (Review Process)
- `REVIEWER-GUIDE.md` — mandatory pre-review information gathering, Impact Matrix, review confidence levels, file change requirements

### Added (Implementation)
- `IMPLEMENTATION-ROADMAP.md` — fully revised phased implementation plan with four major strategic changes:
  * RISC-V64 as reference platform (was: Phase 3 target)
  * Proof-driven design from Day One, complete proofs after stabilization (was: binary verify-now/verify-later choice)
  * GPU provider as substrate-neutral execution interface, CUDA/HIP/SYCL as compatibility layers (was: CUDA provider)
  * Phase 5 recast as experimental research interfaces (was: production quantum milestone)
  * Pre-Phase 1 requirements: Driver Framework RFC, Allocator Hierarchy RFC, Compiler Boundary RFC added as blockers
- `IMPLEMENTATION-STATUS.md` — live implementation status tracker
- `CHANGELOG.md` — change log tracking all additions and revisions

---

## [v1.1] — 2026-06-09 — Frozen Architecture

### Added
- 13 architectural invariants (I-1 through I-13)
- 5 peer architecture decomposition (Composition, Execution, State, Communication, Observability)
- Critical-Path Admission Test for kernel privilege
- Three-layer scheduler model
- Removability test for storage providers
- 5 deployment profiles

### Security
- Capability-only authority model (no UID/GID/ACL in core)
- Merkle integrity verification for all persistent objects
- Tamper-evident audit log
- Formal verification tier requirements

---

## [v1.0] — 2026-06-09 — Initial Architecture

### Added
- Initial AetherOS concept (aspirational vision)
- Five-peer architecture decomposition
- "10/10 everywhere" performance claim (superseded in v1.1)
- AI scheduler proposal (superseded by three-layer model)
- Pure microkernel direction (superseded by critical-path hybrid kernel)
- Distributed kernel proposal (deferred to v2)

---

## Future Milestones

| Milestone | Description | Status |
|-----------|-------------|--------|
| v2.0 | Multi-domain federation, distributed execution | Planned |
| v2.1 | Cohort update governance, capability rights algebra | RFC-0042/0043 pending |
| Implementation Phase 1 | x86-64 and ARM64 execution providers | Not started |
| Implementation Phase 2 | GPU and TPU acceleration providers | Not started |
| Implementation Phase 3 | Quantum processor abstraction layer | Not started |
| Implementation Phase 4 | RISC-V, POWER, and FPGA providers | Not started |
| CTS v1 | Full conformance test suite executable | Not started |
| Platform v1 | First conformant implementation | Not started |