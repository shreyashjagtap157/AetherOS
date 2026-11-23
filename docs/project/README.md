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

Public `main` after PR #12 squash merge: `8fe51c0428441c9951056641f839cb52f45e13fe`. PR #12: https://github.com/shreyashjagtap157/AetherOS/pull/12.

- The historical main-branch reference-validation run 184 failed before tests because the v3 goal file lacked the required `Classification:` header. PR #12 adds the metadata and marks v3 as historical.
- Merged-main [Reference validation run 45233948385](https://github.com/shreyashjagtap157/AetherOS/actions/runs/45233948385) passed 104 document/authority checks, 59 glossary terms, 42 RFC structural checks, 179 Rust tests and all 38 CTS cases; artifact 14848865763 was uploaded.
- Corrected PR-head [QEMU boot run 133](https://github.com/shreyashjagtap157/AetherOS/actions/runs/45232860086) validated the ELF, observed `hello` and captured the expected timeout status `124`; artifact 14846774779 was uploaded. The automatic main-head QEMU rerun 45233948258 remained in progress at the last status refresh.
- Boot evidence artifact 14848115771 was uploaded with the ELF and boot logs. This does not establish clean shutdown, long-duration stability, installer behavior or full OS qualification.
- The latest base-main commit records VirtIO block/network work, TCP data transfer and protected process execution. Commit messages remain implementation leads; each behavior still needs source and end-to-end evidence.
- A successful bounded QEMU boot does not prove a complete usable desktop OS, safe live-media behavior, installer, persistence, physical-board support, broad device coverage or ultimate platform completion.
- The active goal requires the first live-media/install experience and all subsequent whole-platform phases. Incomplete requirements remain Open.

Recheck the updated PR workflows after the boot-status correction and the merge result before treating the evidence as current.
