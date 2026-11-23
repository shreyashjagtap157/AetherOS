# Owner working preferences and repository expectations — AetherOS

Classification: Informative

This document records the owner's durable expectations for development execution and for the canonical AetherOS repository. It complements, but never overrides, the constitutional architecture, accepted ADRs/RFCs, repository safety policy or current user authorization.

## 1. Preferred working style

- **Inspect the real repository first.** Refresh the remote and inspect current branch/HEAD, commit history, actual working tree, nested repositories, all local modifications and CI before relying on remembered paths, old status reports or handoffs.
- **Follow this repository's Git process.** Use its documented branch naming and PR/squash workflow where required. Preserve history; never force-push or rewrite commits. Do not bypass branch protection or governance.
- **Protect user work and host state.** Never hard-reset, clean, overwrite, silently stash or discard existing files. Do not modify firmware, secure boot, host bootloader, machine-wide settings, privileged host components or real disks without the applicable explicit authorization. Prefer disposable QEMU disks for installation/recovery tests.
- **Implement, do not stop at plans.** When authorized, research, design, implement, integrate real consumers, test, debug, document and continue. An audit, architecture proposal, roadmap, boot message or passing component test is not completion.
- **Continue through dependency-ready work.** Do not repeatedly ask the owner to say “continue”. Handle routine, reversible engineering decisions autonomously when they preserve approved contracts. Escalate real owner-only choices, unavailable physical hardware/credentials, paid dependencies, destructive real-device operations and fundamental contract changes.
- **Research and engineering proceed together.** Check official specifications, standards, maintained implementations, peer-reviewed work, firmware/board documentation and current vendor guidance. Evaluate maturity, licensing, maintenance, security, performance, power, thermal impact and verification burden.
- **Use one specialist subagent at a time.** Assign bounded, non-overlapping tasks with explicit repository baseline, requirements, files/interfaces, forbidden actions, acceptance criteria and evidence. Close and independently verify one assignment before starting another. The primary agent remains the accountable architect and integration lead.
- **Fix root causes and retain hard tests.** Never weaken assertions, hide failures, accept vacuous tests, return success on error or mislabel stubs as functional drivers/services.
- **Report what is actually proven.** Separate proposed, scaffold-only, partially implemented, implemented, integrated, verified, qualified, blocked and approved-deferred state. Tie evidence to commit, target, environment, command, logs, CI run and artifact hashes.
- **Update all impacted documentation.** Maintain requirements, architecture/ADR/RFC links, profile matrix, implementation roadmap/status, risk and dependency records, tests, changelog and durable handoff.
- **Make meaningful milestones.** Complete and validate coherent vertical slices, inspect consumers and update records, then proceed to the next unblocked item. Avoid repetitive progress chatter in place of deliverables.

## 2. What the canonical AetherOS repository is expected to become

The repository must evolve into the complete, working AetherOS operating-system platform—not stop at an architecture corpus, a reference model, a kernel that boots, or a demonstration shell.

It must maintain an integrated, traceable body of:
- constitutional platform architecture, accepted decisions and governed subsystem RFCs;
- boot, firmware/SBI/UEFI contracts, kernel, HAL, memory management, interrupts, scheduling and protected user processes;
- capability-based authority, identity, secure boot, process/service isolation and real enforcement boundaries;
- complete device discovery, driver framework, real storage/network/display/input/audio and relevant hardware providers;
- filesystems, persistence, snapshots/generations, encryption, backup, transactional update and recovery;
- typed IPC, service lifecycle, fault containment and observability;
- native runtime/ABI/SDK, package/build/install/update/remove workflows and developer tools;
- functional desktop/tablet/foldable/mobile experience shells and the Context/Workspace/Activity/Provider model;
- governed Web Runtime and isolated Android compatibility where required by the approved target;
- AI/automation with untrusted model outputs, consent, attenuated capabilities, audit, privacy and resource limits;
- power/thermal/resource governance, reliability, diagnostics and enterprise operations;
- reproducible image/toolchain build, SBOM, provenance, signing and profile/hardware conformance;
- end-to-end qualification, user/developer documentation and sustainable maintenance processes.

A feature must be real, integrated and validated at its actual boundary. A virtio driver scaffold, network handshake without payloads, shell command that does not invoke a real service, privileged application demo, installer that merely copies files, or mock desktop is not the accepted outcome.

## 3. Product and experience expectations

- One coherent OS across desktop, laptop, tablet, foldable and mobile regimes; these are device/interaction form factors, not replacements for deployment, assurance or computation profiles.
- RISC-V64 QEMU `virt` is the initial reference qualification target; it is not the final hardware-support ceiling.
- A first usable release must boot through a documented firmware path from removable media, run safely as a live environment, install explicitly to a selected separate target disk, boot independently from that disk, retain user state and support normal use.
- A full desktop/general-purpose release must have an actually functioning GUI and useful native applications; a full general-purpose release also needs a real Web Runtime/browser.
- Native applications are the primary AetherOS model. Android compatibility is an isolated compatibility environment, not the core architecture.
- Context is the human-facing activity unit; Visual, Semantic and Input Graphs must work together while preserving application, workspace, activity, provider, identity and capability boundaries.
- AI is a governed participant, never a privileged exception. Model output is untrusted observation/suggestion; access and tool use require explicit consent, capabilities, policy, provenance, audit and revocation.
- The first usable RISC-V64 release is an intermediate milestone. Every whole-platform requirement and future accepted SHOULD-level obligation remains visible until implemented, integrated, tested and qualified.
- The project has no artificial completion ceiling; continue to evaluate new hardware, architectures, security, verification, runtimes, HCI, AI and resource technologies.

## 4. Authority, profiles and completion boundaries

This file and the [requirements suite](../requirements/README.md) express reconstructed owner intent. The current `Platform-Architecture-Specification-v1.1.md`, architecture governance, accepted ADRs and RFC status control system semantics. The active v4.0 product mandate defines whole-platform ambition and delivery scope but does not automatically promote Proposed RFCs to Accepted.

Always distinguish:
- deployment/computation profiles (Embedded, Desktop, Cloud, HPC, Safety-Critical);
- device classes (Desktop, Laptop, Tablet, Foldable, Mobile);
- ISA/board/firmware/device target;
- security/assurance profile;
- resource/power/thermal contract;
- experience shell.

Support a combination only when its interfaces, implementation, tests and target/profile qualification exist. Do not claim certification without an independent certification authority. Do not treat QEMU boot as proof of the complete OS or physical board support.
