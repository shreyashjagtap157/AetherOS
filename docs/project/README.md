# AetherOS project goals, requirements and authority

Classification: Informative

This folder is the curated entry point for the active owner product goal, working expectations, historical provenance and release/implementation scope.

## Read in this order

1. [Complete Platform Goal v4.0](AetherOS_Complete_Platform_Goal_v4_2026-11-21.md) — active owner execution mandate and ultimate product outcome.
2. [Requirements suite](../requirements/README.md) — structured product requirements, evidence/agent workflow and profile reconciliation.
3. [Owner working preferences and repository expectations](OWNER_WORKING_PREFERENCES_AND_REPOSITORY_EXPECTATIONS.md) — preferred execution style and the expected end state of this repo.
4. [User requirements baseline](../requirements/USER_REQUIREMENTS_BASELINE.md) — traceable product capability baseline.
5. [Engineering execution and evidence](../requirements/ENGINEERING_EXECUTION_AND_EVIDENCE.md) — accountable lead, one-at-a-time specialist delegation, status vocabulary and qualification discipline.
6. [Profile model reconciliation](../requirements/PROFILE_MODEL_RECONCILIATION.md) — deployment profiles, device classes, hardware targets and experience shells.
7. [Source reconciliation](../requirements/SOURCE_RECONCILIATION.md) — source provenance, v1.0/v1.1 relationship and live evidence.
8. [Historical archive index](ARCHIVE_INDEX.md) — preserved v1.0 architecture and earlier execution prompt provenance.
9. [Constitutional architecture v1.1](../Platform-Architecture-Specification-v1.1.md) — current architecture baseline, still a Validation Candidate.
10. [RFC index](../00-RFC-Index.md), [architecture governance](../ARCHITECTURE-GOVERNANCE.md), accepted decisions and active RFCs.
11. [Implementation roadmap](../IMPLEMENTATION-ROADMAP.md) and [implementation status](../IMPLEMENTATION-STATUS.md) — reconcile both with current source/CI.

## Authority hierarchy

| Layer | Authority / use |
|---|---|
| Owner's active whole-platform goal | Defines the intended product, user journeys, engineering scope and ultimate completion objective. The active v4.0 mandate is stored here. |
| Constitutional architecture | `docs/Platform-Architecture-Specification-v1.1.md` is the current repository architecture baseline. It remains a Validation Candidate, not frozen. |
| Governance, accepted ADRs and RFCs | Define approved contracts and amendment/evidence policy within their actual status/scope. |
| Requirements register | Maps owner intent to architecture, implementation and acceptance evidence; informative unless separately ratified. |
| Source, tests and CI | Determine implementation facts at an exact commit. |
| Archives and historical prompts | Preserve context; cannot override current architecture/governance or silently revive superseded proposals. |

The active product goal does not automatically promote proposed RFCs to Accepted. Conversely, an intermediate first usable release must not narrow the owner's ultimate platform ambition.

## Repository organization

- `docs/project/` — active product mandate, navigation, preferences and provenance.
- `docs/requirements/` — reconstructed requirements, execution/evidence contract and profile crosswalk.
- `docs/archive/` — preserved historical sources, explicitly informative.
- `docs/RFCs/`, `docs/Platform-Architecture-Specification-v1.1.md` — architecture contracts and governance.
- `docs/IMPLEMENTATION-ROADMAP.md`, `docs/IMPLEMENTATION-STATUS.md` — work order and revision-scoped evidence.
- `reference/` — host semantic models and RISC-V64 kernel/architecture implementation; inspect actual code and callers.
- `tools/`, `schemas/`, `.github/workflows/` — validators and CI/release evidence.

## Live evidence snapshot — 23 November 2026

Current public `main` HEAD inspected: `91478e592be1077bd79a0996f4207faf53d9fdc8`.

- The `RISC-V64 Boot Qualification` workflow succeeded on this exact commit (run 131, ID `45012374998`).
- The `Reference validation` workflow failed at `./check.sh` because the historical v3 goal document lacked the required `Classification:` header. This is a documentation-governance failure, not evidence that QEMU boot failed. The v3 document receives the missing classification in this change and is explicitly historical.
- The latest commit summary records VirtIO block/network driver work, TCP data transfer and protected user-mode process execution. Commit messages are change summaries; each behavior still requires its own end-to-end evidence.
- QEMU boot does not prove a complete usable desktop OS, physical hardware, live-media safety, installer, broad driver coverage or ultimate platform completion.
- The active goal requires first live-media/install delivery and subsequent whole-platform phases. Incomplete requirements remain Open.

Re-run `./check.sh` and inspect CI on the resulting commit. This page is not a replacement for executable verification.
