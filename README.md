# AetherOS

## Current verified status — 12 November 2026 (PR-branch evidence)

> **Project classification: architecture/specification and host-reference prototype, not a working OS kernel.** There is still no bootable kernel, compiler, hardware design or QEMU RISC-V64 product slice in this repository.
>
> PR #7, PR #8, PR #9 and PR #10 are merged into `main`. The host-reference CTS baseline on `main` is 17 cases (including per-access live capability revalidation and the serialized host runtime). The working tree adds a further 21 cases (CTS-CAP-EXPIRY/BOUND-001, CTS-SCHED-*, CTS-COMP-*, CTS-STATE-CONFLICT/SNAPSHOT/CRASH-001/OUTBOX-DISPATCH-001, CTS-PROF-SELECT/COMBINE-001, CTS-ROUTING-GRANT/DENY-001, CTS-COMP-CONF-001..004) bringing it to 38; these additions are validated locally (full `reference/check.sh` and root `./check.sh` pass, exit 0) but are not merged, not committed, and not CI-validated. The host model is not a real kernel MMU or production scheduler.
>
> `main` validation on `ac55bb3` passed the full gate including the reproducibility check ([workflow run](https://github.com/shreyashjagtap157/AetherOS/actions/runs/43245580188)). On this Windows host the reproducibility step previously failed identically on unmodified `main` because MSVC `link.exe` embeds a wall-clock PE `TimeDateStamp`; `tools/verify-reproducible-reference.sh` now passes `-C link-arg=/Brepro` on MINGW/MSYS/CYGWIN hosts so the gate's byte-identical-build assertion holds there too (Linux CI unchanged; assertion preserved, not weakened). The full `./check.sh` now exits 0 on this host. This is still a host-reference prototype: no kernel CTS, real MMU backend, production scheduler, formal proof, or product release is claimed.

---
AetherOS is the working name for the **Ideal Computing Platform**: a candidate architecture and research programme for a general-purpose operating system built around capability-only authority, thirteen candidate invariants, and five peer architectures (Composition, Execution, State, Communication, Observability).

The repository contains the specification suite — **not** a working kernel, compiler, or hardware design. Implementation repositories will be separate.

## Status

| Item                              | State                                                                 |
|-----------------------------------|-----------------------------------------------------------------------|
| Top-level architecture            | **Validation candidate v1.2**; not frozen until executable evidence exists |
| RFC corpus                        | **42 legacy proposals + 3 Pre-Phase-1 drafts**; metadata and semantics under reconciliation |
| AI-native validation objective    | **Explicit**; AI-native operation is a conformance scenario, not a new RFC or subsystem |
| Conformance Test Suite            | **17 cases on main; 38 with the uncommitted working-tree slices (capability lifetime/flags, scheduler Layer 1, composition models incl. conformance reporting, transaction conflicts, crash-consistency sweep and outbox dispatch) (local gates green; CI pending)**; no kernel CTS yet |
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
| `docs/RFCs/05-Cross-Cutting/`        | 7    | RFC-0037 … RFC-0043                                     |
| `docs/RFCs/06-Pre-Phase-1/`          | 3    | RFC-ALLOC-001, RFC-COMPILER-001, RFC-DRIVER-001 (Draft) |

Supporting materials live alongside the specs:

- `docs/Glossary.md` — terminology used across all documents
- `docs/REVISION-HISTORY.md` — what changed across architecture versions
- `docs/CHANGELOG.md` — suite-level change log
- `docs/RESEARCH-REVOCATION-IMPLEMENTATION.md` — research and implementation decision for the host capability provider's revocation graph
- `docs/RESEARCH-RV64-MAPPING-REVOCATION.md` — capability-authorized memory mapping, PTE permissions, and RISC-V shootdown research
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
