# Ideal Computing Platform — Documentation Suite

Classification: Informative
Authoritative Source: Platform-Architecture-Specification-v1.1.md

This directory contains a validation-candidate computing platform architecture specification produced after extensive cross-examination of modern operating-system research, capability systems, microkernel design, distributed systems work, formal verification practice, storage engineering, and recent production systems (Linux, Windows NT, macOS/XNU, Fuchsia/Zircon, seL4, QNX, MINIX3, Barrelfish, Kronos, and others).

The suite is the deliverable corresponding to one question:

> *What would an operating system architecture look like if every measurable dimension were optimized simultaneously, with tradeoffs explicit rather than accidental, and with each subsystem specified precisely enough to be implemented?*

The answer is not a single kernel. It is a **specification**: a set of normative invariants, a decomposition into five peer architectures, a verification-tier system, a complexity budget methodology, and a roadmap of the 41 RFCs needed to make any conforming implementation possible.

---

## Document Map

Top-level documents (read in order):

| File | Purpose |
|------|---------|
| `README.md` | This document. Entry point. |
| `project/README.md` | Active owner goal, product scope, source authority and navigation. |
| `project/AetherOS_Complete_Platform_Goal_v4_2026-11-21.md` | Active v4.0 whole-platform execution mandate. |
| `requirements/USER_REQUIREMENTS_BASELINE.md` | Reconstructed user requirements and traceability keys. |
| `requirements/SOURCE_RECONCILIATION.md` | Source provenance and v1.0/v1.1 authority reconciliation. |
| `Glossary.md` | Terminology used across all documents. |
| `Platform-Architecture-Specification-v1.1.md` | Validation-candidate constitutional baseline. |
| `PRODUCT-FEASIBILITY-AND-SCOPE.md` | Honest product priorities, limits, installation composition, and production criteria. |
| `VERSIONED-PRODUCT-IMPLEMENTATION-PLAN.md` | Product plan from an empty 0.0.0 baseline through minimal 1.0.0, complete 2.0.0, and later evolution. |
| `COMPATIBILITY-STRATEGY.md` | C0–C9 compatibility claims and implementation lanes. |
| `PERSISTENCE-AND-EXTERNAL-EFFECTS.md` | Efficient audit commitments and non-rollbackable I/O semantics. |
| `UASA-INTEGRATION-STATUS.md` | Filesystem integration status and required evidence. |
| `UASA_COMPREHENSIVE_RESEARCH_REVIEW_PACKET.md` | Imported UASA research handoff and self-declared evidence boundary. |
| `Revision-History.md` | What changed across versions and why. |
| `Governance-RFC.md` | How this specification is amended. |
| `Profiles-Specification.md` | Deployment profiles (Embedded, Desktop, Cloud, HPC, Safety-Critical). |
| `Conformance-Test-Suite-Methodology.md` | How conformance is measured. |
| `00-RFC-Index.md` | Master index of the 41 normative RFCs. |

Peer architecture subdirectories:

| Directory | Contains |
|-----------|----------|
| `RFCs/00-Composition/` | Composition Architecture RFCs |
| `RFCs/01-Execution/` | Execution Architecture RFCs |
| `RFCs/02-State/` | State Architecture RFCs |
| `RFCs/03-Communication/` | Communication Architecture RFCs |
| `RFCs/04-Observability/` | Observability Architecture RFCs |
| `RFCs/05-Cross-Cutting/` | Capability, verification, and conformance RFCs |

---

## How To Read This Suite

**If you want the product goal:** read `project/AetherOS_Complete_Platform_Goal_v4_2026-11-21.md`. **If you want the architecture in 20 minutes:** read `Platform-Architecture-Specification-v1.1.md`.

**If you want a specific subsystem:** find the relevant RFC in `00-RFC-Index.md`.

**If you intend to implement something:** read the product goal, top-level architecture, applicable accepted ADRs and peer architecture RFCs in order. Begin implementation only after reading the conformance test methodology and verifying the current implementation baseline.

**If you intend to critique:** every RFC is normative. Critique should reference specific invariants, specific RFC sections, and propose concrete mechanism alternatives — not architectural slogans.

---

## Status

The repository is both an architecture/specification suite and an active implementation repository. The current constitutional architecture remains `Platform-Architecture-Specification-v1.1.md`, classified as a **Validation Candidate**; it is not frozen until the declared evidence gates pass. The complete owner product goal is maintained separately in `project/AetherOS_Complete_Platform_Goal_v4_2026-11-21.md`. A first usable RISC-V64 release is an intermediate milestone, not the ultimate project goal.

As observed on 23 November 2026, public `main` is at `91478e592be1077bd79a0996f4207faf53d9fdc8`. The QEMU RISC-V64 boot qualification workflow succeeded on this exact commit (run 131, ID `45012374998`). The reference-validation workflow (run 184, ID `45012375003`) failed before reference tests because the v3 goal document lacked a required `Classification:` header; that metadata defect is corrected as part of the documentation reconciliation. Re-run the gate on the resulting commit and inspect all subsequent validation results.

The current repository contains host-side reference models and a RISC-V64 kernel/architecture implementation. These are separate evidence domains: a host model is not a hardware implementation, a successful QEMU boot is not a complete OS release, and a component test is not whole-system qualification. The v4 requirements register keeps incomplete subsystems, hardware profiles and ecosystem features visible.

The complete architecture and product-goal status must be read with the exact source revision, RFC/ADR acceptance state, test logs and target evidence. Specifications are not implementation evidence; implementation is not proof or certification.
## Document Conventions

Throughout this suite:
- **MUST / MUST NOT / SHALL / SHALL NOT** — normative conformance requirements.
- **SHOULD / SHOULD NOT** — recommended but not required for conformance.
- **MAY** — explicitly permitted but not required.
- **non-conformant** — explicitly forbidden by the specification.
- *Italic* — terminology defined in `Glossary.md`.
- **Bold** — emphasis on the load-bearing word in a sentence.
- `Code style` — filenames, identifiers, RFC numbers, type names.

Every RFC follows the structure documented in `Governance-RFC.md`.

---

## Sources Consulted

Major works that informed this specification include the seL4 verification papers and proof architecture, the Fuchsia documentation set, microkernel surveys, capability system literature (Capsicum, CHERI), Linux kernel design, Windows NT design documentation, NTFS/ReFS, ZFS, APFS, file-system design surveys, distributed-systems publications, IFC/CFI literature, and recent research kernels including Kronos, Barrelfish, Helios, and composite-capability designs.

Specific references are cited inline within individual RFCs where load-bearing.
