# AetherOS — source reconciliation and architecture provenance

Classification: Informative

## 1. Current repository identity

| Item | Observed value |
|---|---|
| Canonical repository | `https://github.com/shreyashjagtap157/AetherOS` |
| Branch | `main` |
| HEAD inspected | `91478e592be1077bd79a0996f4207faf53d9fdc8` |
| Recursive tree | 218 entries; not truncated |
| Reference implementation | `reference/`, including RISC-V64 kernel/architecture and host semantic model crates |
| Current constitutional architecture | `docs/Platform-Architecture-Specification-v1.1.md` (Validation Candidate) |
| Active owner product goal | v4.0, stored under `docs/project/` |
| Initial qualification platform | QEMU RISC-V64 `virt` |

## 2. Source set and precedence

### Active sources

1. `docs/project/AetherOS_Complete_Platform_Goal_v4_2026-11-21.md` — active owner product/execution mandate, including first usable release and ultimate whole-platform completion.
2. `docs/Platform-Architecture-Specification-v1.1.md` — current constitutional architecture baseline; Validation Candidate, not frozen. It is the top-level architecture against which RFCs are reconciled.
3. `docs/ARCHITECTURE-GOVERNANCE.md` and applicable accepted ADRs — amendment and decision authority.
4. `docs/00-RFC-Index.md` and `docs/RFCs/` — detailed contracts, each with its own declared draft/proposed/accepted state and verification obligations.
5. `docs/Profiles-Specification.md`, `docs/Hardware-Support-RFC.md`, glossary, compatibility, persistence/external-effects and conformance methodology — profile and subsystem constraints.
6. `docs/IMPLEMENTATION-ROADMAP.md`, `docs/IMPLEMENTATION-STATUS.md`, source, tests and live workflow runs — implementation facts, always tied to exact revision/evidence.
7. This reconstructed requirements register — a traceability aid, not an authority that upgrades status.

### Historical sources

| Source | Reconciliation |
|---|---|
| `docs/AetherOS_Final_Usable_OS_Goal_v3_2026-11-21.md` | Earlier active continuation goal. Retained and now explicitly labelled informative/historical. v4.0 supersedes its interpretation that the first usable release could be mistaken for the complete project goal. |
| Archived `AetherOS-System-Specification-0.2(1)(2).md` | Preserved verbatim (apart from a clear archive preface) as historical v1.0 convergence source. It captured important user decisions around Context-centric HCI, Visual/Semantic/Input Graphs, capability security, storage, resource governance, Device Graph/AHDA, isolated Android compatibility, layered drivers and unified platform profiles. It is not current constitutional authority. |
| `AetherOS_OpenCode_Master_Goal_Prompt_v2.txt` from the prior project material | Historical execution prompt and inspection checklist. Its durable safety, baseline-verification, capability-model audit, test/evidence, session continuity and implementation requirements are incorporated where still current; its reported local paths, old commit state and old test counts are not current facts. |
| Earlier owner handoffs and chats | Useful as leads for requirements and historical decisions only; verify against active specs, exact code and current CI. |

## 3. How v1.0 and v1.1 relate

The recovered historical v1.0 convergence source explicitly recorded the shift to one OS with multiple operating regimes; Context as the canonical human-facing activity; correlated Visual, Semantic and Input Graphs; clear Context/application/Workspace/security separation; Android compatibility as an isolated environment, preferably virtualized; redundant bootable generations selected by measured storage/device constraints; layered drivers; and a normalized Device Graph that does not pretend ACPI, Device Tree, firmware and bus discovery are identical.

The actual repository currently identifies `Platform-Architecture-Specification-v1.1.md` as the constitutional and authoritative top-level architecture and says v1.1 remains a Validation Candidate. Therefore:
- preserve v1.0 as historical design provenance;
- use v1.1 and its accepted governance/ADR/RFC hierarchy as current architecture authority;
- reconcile any v1.0 idea missing or materially altered in v1.1 through an explicit review/ADR, not by copying it into implementation as if it remained binding;
- do not call v1.1 frozen before its declared evidence gates pass;
- do not silently narrow the owner's wider v4 whole-platform goal to the first release.

## 4. Product goals versus specification/implementation status

The v4 mandate defines the intended result and execution obligation. It is not evidence that every listed subsystem is implemented. Likewise:
- RFC text is not implementation;
- an accepted architectural decision is not a verified code path;
- host reference models do not constitute an actual hardware MMU or production driver;
- a crate compiling does not prove its behavior;
- QEMU boot does not prove installer, GUI, physical board, broad driver or final-platform qualification;
- phase labels do not establish acceptance without evidence.

The implementation status must be reconciled to source, tests, logs and exact HEAD. Keep unimplemented requirements visible and marked Open/Blocked/Deferred as appropriate.

## 5. Current Git and CI evidence (23 November 2026)

Observed public `main` HEAD: `91478e592be1077bd79a0996f4207faf53d9fdc8`.

- `RISC-V64 Boot Qualification` run 131 (ID `45012374998`) succeeded for this exact commit.
- `Reference validation` run 184 (ID `45012375003`) failed during `./check.sh` because `docs/AetherOS_Final_Usable_OS_Goal_v3_2026-11-21.md` lacked its mandatory `Classification:` metadata. No reference-evidence artifact was produced after this early failure.
- The v3 document now receives the missing informative classification and is explicitly marked historical in this documentation change.
- Commit `91478e5` records implementation work for VirtIO block/network drivers, TCP data transfer and protected process execution. The commit message is not a substitute for end-to-end verification of each subfeature.
- The successful QEMU boot is a real milestone, but the first usable release and ultimate v4 whole-platform objective remain unqualified.

After this docs-only correction, re-run the reference-validation gate and inspect any later failures. Do not assume that correcting the metadata proves the remaining code/model tests pass.

## 6. Requirement ID policy

The `AOS-UR-*` keys in `USER_REQUIREMENTS_BASELINE.md` are cross-document traceability labels. They deliberately do not reuse or pre-empt `ARCH-*`, RFC, ADR, INTF or existing project IDs. If the project later decides to make a reconstructed user requirement normative, allocate a permanent ID using the repository's accepted namespace policy, map it to its architectural contract and acceptance method, and amend specifications only through governance.

## 7. Owner safety and execution boundaries

Preserve all Git history and user work. Do not force-push, hard-reset, clean, overwrite workspaces, alter firmware/boot settings, modify real disks or install privileged host components without the appropriate separate authorization. Use disposable QEMU disks for installer and recovery testing wherever possible. Keep secrets and private signing material outside the repository.
