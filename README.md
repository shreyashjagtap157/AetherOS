# AetherOS

AetherOS is the working name for the **Ideal Computing Platform**: a candidate architecture and research programme for a general-purpose operating system built around capability-only authority, thirteen candidate invariants, and five peer architectures (Composition, Execution, State, Communication, Observability).

The repository contains the specification suite — **not** a working kernel, compiler, or hardware design. Implementation repositories will be separate.

## Status

| Item                              | State                                                                 |
|-----------------------------------|-----------------------------------------------------------------------|
| Top-level architecture            | **Validation candidate v1.2**; not frozen until executable evidence exists |
| RFC corpus                        | **41 legacy proposals + 3 Pre-Phase-1 drafts**; metadata and semantics under reconciliation |
| Conformance Test Suite            | **12-case host reference prototype passing**; no kernel CTS yet          |
| Implementation                    | Host semantic prototype only; QEMU RISC-V64 product slice **not started** |
| Product/feasibility baseline      | Active; see `docs/PRODUCT-FEASIBILITY-AND-SCOPE.md`                    |
| Compatibility strategy           | Active; see `docs/COMPATIBILITY-STRATEGY.md`                           |
| UASA storage integration         | Review packet imported from `main`; implementation evidence pending |

## What Is In This Repository

```
AetherOS/
├── docs/        The full specification suite (entry: docs/README.md)
├── reference/   Executable capability reference model and CTS prototype
└── .kilo/       Local editor / agent configuration
```

Run **`./check.sh`** for the complete local documentation and host-reference validation gate. The pinned Rust toolchain is declared in `rust-toolchain.toml`.

The canonical entry point is **[`docs/README.md`](docs/README.md)**. From there the reading order is:

1. `docs/REPOSITORY-PRINCIPLES.md` — one-page orientation
2. `docs/Platform-Architecture-Specification-v1.1.md` — validation-candidate architecture with 13 invariants
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
- `docs/VERSIONED-PRODUCT-IMPLEMENTATION-PLAN.md` — product releases from 0.0.0 through 2.x
- `docs/IMPLEMENTATION-ROADMAP.md` / `IMPLEMENTATION-STATUS.md` — phase gates and live status
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

- A kernel, bootloader, hypervisor, driver, or production implementation. The `reference/` host models are working prototype code.
- A compiler or runtime implementation.
- A hardware schematic, SoC design, or benchmark suite.
- An application ABI or wire-format protocol (those live in the RFCs that define them).

It **is**:

- A validation-candidate architectural specification with thirteen invariants that remain subject to evidence-driven amendment.
- 41 normative RFCs across five peer architectures.
- A verification-tier methodology (`RFC-0040`) and a conformance-test methodology (`Conformance-Test-Suite-Methodology.md`).
- A governance process (`ARCHITECTURE-GOVERNANCE.md`, `Governance-RFC.md`).
- Five deployment profiles (`Profiles-Specification.md`).
- An initial, explicitly non-production executable capability model, table-backed provider, and machine-readable CTS runner under `reference/`.

## Vocabulary of Authority

- **Constitutional** — the document defines a primitive the architecture depends on. (`Platform-Architecture-Specification-v1.1.md`)
- **Normative** — RFC or methodology. Specifies what conformant implementations must do.
- **Generated** — emitted by tools in `docs/tools/`; never edited by hand, never authoritative.
- **Informative** — orientation, history, status. Useful, never load-bearing.

Only **one** document may be authoritative for any given fact. The architecture is the highest authority in the suite; RFCs derive from it; everything else is reference.
