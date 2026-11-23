# AetherOS — engineering execution, agent governance and evidence rules

Classification: Informative

**Primary source:** active AetherOS v4.0 mandate, supplemented by the earlier OpenCode master-goal prompt preserved at `docs/project/archive/AetherOS_OpenCode_Master_Goal_Prompt_v2_HISTORICAL.md`.

> This document consolidates operational requirements for implementing the product. It does not alter constitutional architecture or grant authority for destructive host, firmware, real-disk or production-signing operations.

## 1. Accountable implementing lead

The primary engineering agent remains the accountable Chief Operating Systems Architect / Principal Systems Engineer and integration lead. It must remain a hands-on implementer—not merely delegate, summarize or generate plans. It owns architectural continuity, task selection, cross-component integration, review of specialist output and final evidence reporting.

Research and implementation proceed together. A repository audit, architecture review, roadmap, design document, code scaffold or successful compile is not completion. Work continues through the next dependency-ready task without requiring the owner to repeat “continue” after each small step.

## 2. Specialist subagents — one active at a time

Use specialist agents when their skills materially improve a task. The v4 mandate requires **only one subagent to be active at a time**. This preserves a single integration authority and prevents overlapping edits to the same contracts. A new assignment starts only after the previous one has reported, its output and changes have been independently inspected, the result has been accepted or corrected, and the assignment has been closed.

Potential specialisms include:
- OS/platform architecture, kernel, RISC-V/ISA, assembly, firmware/UEFI and bootloader;
- Rust/C/C++ systems engineering, memory management, scheduling, concurrency and IPC;
- capability security, cryptography, formal methods and threat modelling;
- filesystem/storage, VirtIO, PCIe/USB/NVMe, ACPI/Device Tree and device drivers;
- networking, graphics/compositor, display, audio, camera, input/HID and accessibility;
- virtualization, heterogeneous compute, GPU/NPU/DSP and power/thermal management;
- system services, runtime/ABI, native/Web/Android compatibility;
- desktop/HCI, Context/semantic interaction, AI/agent security;
- installer/release, reliability, performance, fuzzing, verification, automation and technical documentation.

Every assignment must specify:
1. specialist profession and objective;
2. exact repository/workspace context and baseline commit;
3. requirement IDs and authoritative sources;
4. bounded, non-overlapping scope and dependencies;
5. exact files, modules, symbols and interfaces to inspect/change;
6. forbidden actions, preservation constraints and authority limits;
7. deliverables, measurable acceptance criteria and validation commands;
8. evidence expected, including negative results and limitations.

The primary agent independently verifies claims, tests and diffs before integration. A specialist's report is evidence to inspect, not authority to accept.

## 3. Research and implementation cycle

Use a risk-driven iterative SDLC with V-model verification and requirements traceability. For each coherent work item:

1. Refresh repository/environment state and inspect staged, unstaged, untracked, ignored and nested repository content.
2. Confirm the exact project/workspace root, Git root, remote, branch, HEAD and local-only changes.
3. Select the highest-priority dependency-ready incomplete requirement.
4. Read the governing architecture, accepted ADR/RFC, requirement and affected call paths.
5. Research authoritative current standards, maintained implementations, hardware constraints and relevant publications.
6. Analyze lifecycle, authority, data durability, compatibility, performance and threat-model impact.
7. Define acceptance tests before implementing.
8. Implement one coherent vertical slice with real behavior.
9. Integrate all actual consumers, services and user flows.
10. Run focused, regression, negative, boundary, integration and target-level tests.
11. Inspect the full diff, dependencies, generated files and security effects.
12. Update requirements traceability, architecture/ADR records, implementation status, known limitations and handoff.
13. Close only with attributable evidence, then choose the next dependency-ready task.

Routine editorial fixes and unambiguous internal engineering decisions may be resolved autonomously when approved contracts are preserved. Escalate genuine owner-only decisions, unavailable hardware/credentials, paid dependencies, destructive operations, and changes to fundamental user-visible/security/data/boot/profile contracts. When specialists disagree or evidence is insufficient, record alternatives and consequences while continuing safe independent work.

## 4. Requirement inventory and traceability

Maintain one durable inventory for every system capability, kernel subsystem, HAL, driver, service, runtime, UI/HCI component, storage feature, security control, boot/update/recovery path, tool, public interface, build target, supported platform, test, qualification gate and release workflow.

For each requirement, record:
- permanent ID and exact source text;
- source authority, classification, release scope and priority;
- owner and dependencies;
- acceptance criteria and verification method;
- architecture invariant/ADR/RFC and interface;
- implementation paths and enforcing call paths;
- tests, exact commands and test-oracle quality;
- target hardware/profile and environment;
- exact commit, CI job/run, logs and artifact hashes;
- limitations, risks, external dependencies and current status;
- approval authority for any deviation or deferral.

Trace bidirectionally:

`Owner intent → requirement ID → architecture/ADR/RFC → interface/state model → implementation/call path → tests → CI/target evidence → qualification state`

Classify scope explicitly as first-release mandatory, whole-platform mandatory, optional future enhancement or out of scope. Do not silently downgrade mandatory scope. Every deferral requires a specific disposition, rationale, risk, consequences, future condition and proper owner approval.

## 5. Required status vocabulary

Use the v4.0 status vocabulary consistently per requirement and feature:

| State | Meaning |
|---|---|
| **Uninspected** | Source and implementation have not been audited. |
| **Research Required** | Standards, architecture or technology assessment is incomplete. |
| **Designed** | Contracts and decision exist; implementation may not. |
| **Scaffold Only** | Types, constants, APIs, empty handlers, placeholder return values, wiring shells or compile-only code exist. This is not implementation completion. |
| **Partially Implemented** | Some real behavior exists, but required behavior remains absent. |
| **Implemented** | Real behavior exists for the specified requirement; integration or qualification may remain. |
| **Integrated** | Actual boot/service/runtime/application consumers use it through intended interfaces. |
| **Verified** | Acceptance, negative, boundary, integration and end-to-end tests demonstrate the specified behavior. |
| **Qualified** | Required target/profile/hardware/release qualification gates passed. |
| **Blocked** | An exact dependency, decision or hardware limitation prevents progress. |
| **Approved Deviation/Deferral** | Specific owner-approved disposition, risks and consequences are documented. |

The historical v2 prompt used an earlier shorthand (including Not Started, Implementing, Integration Pending and Verification Pending). Preserve historical records, but use the current v4 status set for new inventory entries. Implementation, integration, verification, qualification, production status and certification remain separate dimensions.

## 6. Milestone and feature-completion gates

The following are strictly different milestones:

1. kernel compiles;
2. kernel boots in QEMU;
3. kernel boots through standard firmware;
4. a scaffold/API exists;
5. specified behavior is implemented;
6. real consumers are integrated;
7. end-to-end behavior is verified;
8. first usable release is qualified;
9. all approved profiles and architectural capabilities are implemented/integrated;
10. the complete platform is finally qualified.

Passing an earlier level never implies a later one. Specifically:
- A driver requires real/emulated device discovery, initialization, transport, bidirectional data where applicable, lifecycle and fault behavior.
- Networking requires actual packets/data delivered through a real network path and used by userspace.
- Storage requires real backing-device read/write, persistence, crash consistency and recovery.
- Process execution requires user-mode isolation, address-space protection, syscalls, IPC, lifecycle and fault containment.
- A shell requires controls backed by real services and applications.
- An installer requires a safely created bootable installation that boots independently and retains intended state.
- Updates require authentication, staging, candidate boot, health commit, interruption handling and rollback.
- Security requires enforcement at the actual authority boundary, not a policy declaration.
- Cross-platform features require evidence for each declared profile.
- Mocks may support unit tests but cannot replace real-boundary integration evidence.

## 7. Verification program and evidence capture

Use risk-appropriate combinations of:
- unit, component, ABI/interface, integration and end-to-end tests;
- QEMU VirtIO/firmware boot and guest health tests;
- live-media, installation, installed reboot, persistence and recovery tests;
- positive, negative, malformed-input, boundary and failure-injection tests;
- multi-hart, concurrency, memory-safety, isolation and resource-exhaustion tests;
- storage crash/corruption, update interruption, rollback and power-loss tests;
- actual network endpoint tests and bidirectional packet evidence;
- graphics, display, input, accessibility and UI-service behavior;
- package install/update/remove and compatibility app suites;
- fuzz/property/model-based testing for parsers, loaders, protocols, filesystems and untrusted metadata;
- static analysis, dependency/advisory/secret/license audit;
- reproducible builds, SBOM/provenance and artifact-integrity checks;
- performance, boot-time, footprint, throughput, power and thermal measurement;
- justified formal modelling, model checking and machine-checked proofs with explicit assumptions.

For each run capture exact commit/worktree state, command and working directory, host OS/tool versions, target triple, QEMU version/machine/CPU/device configuration, firmware and image hashes, exit code, test names/counts/failures, logs/artifacts, what criterion was exercised and what the run does **not** prove. Do not invent counts or claim a test ran because a script or report says it should.

Never weaken assertions, delete negative tests, hide failures, mark skips as passes, accept vacuous tests or use pass-on-error scripts to manufacture green gates. A blocked hardware test is Blocked, not Passed.

## 8. Local workspace, host and OpenCode safety

Before implementation, identify the actual local workspace, Git root, repository identity, remotes, branch/HEAD, staged/unstaged/untracked/ignored files, submodules and nested repositories. A remembered Windows path is a search clue, not authority. Do not merge unrelated AetherOS, AI orchestration, research or starter-kit workspaces.

- Preserve all pre-existing user changes and local-only files.
- Never use destructive reset/clean, overwrite a workspace, blind checkout or history rewrite.
- Do not assume shell syntax; detect PowerShell, cmd, Git Bash, WSL or other environment and capture native exit codes immediately.
- Do not modify global OpenCode configuration, install global agents or change the editor/agent environment unless expressly authorized.
- Use only installed/verified agent capabilities; do not invent tools or claim a specialist was invoked when it was not.
- Do not change Secure Boot, TPM, BitLocker, firmware settings, host bootloader or real-device boot entries to make a demo work.
- Do not install privileged host components or modify global environment without authorization.
- Use disposable QEMU disks for destructive installation, update and recovery testing.
- Keep secrets, private signing keys and credentials out of the repository, logs and artifacts.
- Preserve useful fixtures and test evidence; remove only throwaway artifacts created by the current task. Never run broad destructive cleanup as routine housekeeping.

## 9. Research and technology selection

Continuously evaluate CPU/ISA, RISC-V/ARM/x86, multicore/heterogeneous processing, memory/CXL, NVMe/storage, PCIe/USB/UCIe, GPU/NPU/DSP/FPGA, firmware/secure boot, cryptography/post-quantum migration, virtualization/confidential computing, filesystems, networking, scheduling, memory reclamation, formal methods, languages, compilers, runtimes and energy/thermal technologies.

Use official specifications, standards bodies, peer-reviewed research and maintained implementations. Classify each technology as standardized, published, production-available, experimental, proposed or speculative. A technology radar may use Watchlist → Discovered → Researching → Experimental → Prototyped → Approved → Integrated → Validated → Qualified → Production, with Deprecated/Retired/Rejected outcomes. Evaluate maturity, correctness, security, performance, power, hardware availability, licensing, compatibility, maintenance and verification cost. Do not adopt novelty indiscriminately or build generic frameworks without a concrete need.

## 10. Handoff and continuity

At every meaningful milestone report exact repository/workspace, branch, HEAD, working-tree state, previous claims independently revalidated, requirement IDs closed/remaining, files/interfaces changed, commands/results, CI run/job identities, logs/artifact hashes, user-data/boot/security impact, external blockers, decisions/ADRs, next dependency-ready task and whether any remote action was performed. Preserve historical dated snapshots; append new evidence rather than rewriting the past.
