# AetherOS

AetherOS is the working name for the **Ideal Computing Platform**: a frozen v1.1 architectural specification for a general-purpose operating system built around capability-only authority, thirteen normative invariants, and five peer architectures (Composition, Execution, State, Communication, Observability).

The repository contains the specification suite — **not** a working kernel, compiler, or hardware design. Implementation repositories will be separate.

## Status

| Item                              | State                                                                 |
|-----------------------------------|-----------------------------------------------------------------------|
| Top-level architecture            | **Frozen v1.1** (`ARCH-INV-001` … `ARCH-Profile-001`)                 |
| Normative RFCs                    | **41 of 44 produced**; **3 Pre-Phase-1 drafts** under review           |
| Conformance Test Suite            | Methodology frozen; executable suite not started                       |
| Implementation                    | Phase 1 (x86-64 / ARM64 execution providers) **not started**          |

## What Is In This Repository

```
AetherOS/
├── docs/        The full specification suite (entry: docs/README.md)
└── .kilo/       Local editor / agent configuration
```

The canonical entry point is **[`docs/README.md`](docs/README.md)**. From there the reading order is:

1. `docs/REPOSITORY-PRINCIPLES.md` — one-page orientation
2. `docs/Platform-Architecture-Specification-v1.1.md` — frozen architecture with 13 invariants
3. `docs/ARCHITECTURE-GOVERNANCE.md` — how the spec is amended
4. `docs/00-RFC-Index.md` — master index of the 41 normative RFCs (and the 3 Pre-Phase-1 drafts)

Peer architectures are organised under `docs/RFCs/`:

| Directory                            | RFCs | Includes                                                |
|--------------------------------------|------|---------------------------------------------------------|
| `docs/RFCs/00-Composition/`          | 5    | RFC-0002 … RFC-0006                                     |
| `docs/RFCs/01-Execution/`            | 8    | RFC-0007 … RFC-0014                                     |
| `docs/RFCs/02-State/`                | 9    | RFC-0015 … RFC-0023                                     |
| `docs/RFCs/03-Communication/`        | 7    | RFC-0024 … RFC-0030                                     |
| `docs/RFCs/04-Observability/`        | 6    | RFC-0031 … RFC-0036                                     |
| `docs/RFCs/05-Cross-Cutting/`        | 5    | RFC-0037 … RFC-0041                                     |
| `docs/RFCs/06-Pre-Phase-1/`          | 3    | RFC-ALLOC-001, RFC-COMPILER-001, RFC-DRIVER-001 (Draft) |

Supporting materials live alongside the specs:

- `docs/Glossary.md` — terminology used across all documents
- `docs/REVISION-HISTORY.md` — what changed across architecture versions
- `docs/CHANGELOG.md` — suite-level change log
- `docs/Hardware-Support-RFC.md` — hardware provider architecture
- `docs/IMPLEMENTATION-ROADMAP.md` / `IMPLEMENTATION-STATUS.md` — phased implementation plan and live status
- `docs/PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md` — interfaces blocking Phase 1
- `docs/generated/` — tool-emitted artifacts (e.g. `DECISION-INDEX.md`); never edited by hand
- `docs/tools/` — Python scripts and `repository-policy.yaml` consumed by the repo's governance / verification toolchain

## Quick Links

- **One-page orientation:** `docs/REPOSITORY-PRINCIPLES.md`
- **Architecture in 20 minutes:** `docs/Platform-Architecture-Specification-v1.1.md`
- **Implementer reading order:** Execution → State → Communication → Composition → Observability → Cross-Cutting
- **For reviewers:** `docs/REVIEWER-GUIDE.md` is mandatory pre-reading

## What This Repository Is Not

It is **not**:

- A kernel, bootloader, hypervisor, driver, or any working code.
- A compiler or runtime implementation.
- A hardware schematic, SoC design, or benchmark suite.
- An application ABI or wire-format protocol (those live in the RFCs that define them).

It **is**:

- A frozen architectural specification with thirteen invariants.
- 41 normative RFCs across five peer architectures.
- A verification-tier methodology (`RFC-0040`) and a conformance-test methodology (`Conformance-Test-Suite-Methodology.md`).
- A governance process (`ARCHITECTURE-GOVERNANCE.md`, `Governance-RFC.md`).
- Five deployment profiles (`Profiles-Specification.md`).

## Vocabulary of Authority

- **Constitutional** — the document defines a primitive the architecture depends on. (`Platform-Architecture-Specification-v1.1.md`)
- **Normative** — RFC or methodology. Specifies what conformant implementations must do.
- **Generated** — emitted by tools in `docs/tools/`; never edited by hand, never authoritative.
- **Informative** — orientation, history, status. Useful, never load-bearing.

Only **one** document may be authoritative for any given fact. The architecture is the highest authority in the suite; RFCs derive from it; everything else is reference.
