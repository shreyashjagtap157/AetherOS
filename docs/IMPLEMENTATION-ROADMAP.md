# Implementation Roadmap — Evidence-Driven Programme

Classification: Operational
Authoritative Source: IMPLEMENTATION-ROADMAP.md
Requirement-ID: GOV-ROADMAP-001
Status: Validation Roadmap

**Updated:** 2026-07-27. This roadmap replaces calendar optimism with evidence gates. Dates are forecasts only after a staffed team measures throughput. Passing a phase gate, not elapsed time, authorizes the next maturity claim.


## Current owner-goal and implementation reconciliation — 23 November 2026

The active product-goal authority is now [AetherOS Complete Platform Goal v4.0](project/AetherOS_Complete_Platform_Goal_v4_2026-11-21.md), supported by the [reconstructed requirements](requirements/USER_REQUIREMENTS_BASELINE.md) and [source reconciliation](requirements/SOURCE_RECONCILIATION.md). This 27 July evidence roadmap remains useful for architecture/research gates, but its old baseline and phase forecasts are historical snapshots. It must not override the v4.0 whole-platform scope or be used to claim that the first usable release is the final project goal.

**Observed public `main` HEAD:** `91478e592be1077bd79a0996f4207faf53d9fdc8`.

- RISC-V64 QEMU `virt` boot qualification passed on this exact commit: [run 131](https://github.com/shreyashjagtap157/AetherOS/actions/runs/45012374998).
- Historical main Reference validation run 184 stopped at the documentation-authority gate because the v3 goal lacked `Classification:` metadata. PR-head [run 45229893980](https://github.com/shreyashjagtap157/AetherOS/actions/runs/45229893980) passed 104 document/authority checks, 59 glossary terms, 42 RFC structural checks, 179 Rust tests and all 38 CTS cases.
- The 21–22 November implementation commits report boot/trap handling, executor/context switching, IPC, storage wiring, shell command handling, a userspace VM path, A/B update/recovery, VirtIO block/network work, TCP data transfer, PMP protection and protected process execution. Treat commit summaries as inspection leads, not proof that every listed subsystem has complete end-to-end behavior.
- PR-head QEMU [run 133](https://github.com/shreyashjagtap157/AetherOS/actions/runs/45229894224) observed the `hello` boot marker and passed ELF checks before the 60-second timeout. The previous workflow masked the timeout status; PR #12 corrects it and requires a fresh run. This is a bounded QEMU boot-marker scenario, not the first usable live-media/installer release, physical-board support, complete integration or whole-platform qualification.

### Current product delivery horizons

- **First usable release:** v4.0 Phases 0–9, from baseline recovery and RISC-V64 boot contract through protected userspace, real storage/network/device slices, functional desktop, safe live media and installer, package/app use, updates/recovery/security hardening, physical target qualification and a stable release.
- **Ultimate platform completion:** v4.0 Phases 10–18, covering shared platform semantics, complete device/driver/hardware profiles, multimedia/graphics, communications, runtimes and applications, Context-centric UX and shells, governed intelligence/privacy, resource/reliability/enterprise operations, and full integration/profile conformance/final qualification.
- The detailed acceptance criteria and non-negotiable scope are in v4.0. Keep every unimplemented requirement Open with dependencies and evidence criteria. These horizons do not replace the architecture's own phase/RFC verification gates.

---

## 1. Historical baseline captured 27 July 2026 (not the current implementation state)

There is currently no kernel, bootloader, HAL, target driver, target runtime, filesystem implementation, benchmark programme, formal machine proof, supported hardware configuration, or application compatibility result in this repository. PR #7, PR #8, PR #9 and PR #10 have been merged into `main`, bringing the executable **host reference CTS prototype** to 17 cases, including an experimental RV64/Sv39 memory-mapping model, dispatch-facing address-space activation/residency checks, per-access live capability revalidation, and a serialized host façade for capability/mapping state. Main head `ac55bb3` passed its push validation workflow, including capability-space tests in debug and release, all CTS cases, repository and supply-chain validators, formatting, Clippy, Rustdoc, reproducibility, and the evidence gates. These are host-reference results, not target-kernel or product conformance. The uncommitted working tree adds scheduling-time lifetime enforcement (RFC-0037.14), capability flags (RFC-0037.7) and CAPF_BOUND transfer refusal (RFC-0037.15) in both reference providers, with two additional CTS cases (19 total); those changes are locally verified but not committed, merged or CI-validated. Forty-one architecture RFCs and three draft implementation-boundary RFCs are design inputs, not implementation evidence. The architecture is a validation candidate, not frozen fact.

The complete objective resembles a new kernel, distribution, compatibility ecosystem, hypervisor, SDK, driver programme, verification programme, and enterprise product. Even with extensive AI assistance it is a multi-year, likely multi-decade ecosystem effort. AI may accelerate drafting and test generation; it does not supply hardware documentation, redistribution rights, independent review, proof validity, user adoption, or maintenance accountability.

## 2. Programme rules

- QEMU is supported before real boards; one reference device is supported before a hardware family.
- A vertical slice precedes breadth.
- Compatibility begins with guests, then measured personalities—not a universal syscall rewrite.
- No new compiler backend until a conventional ELF/C ABI and LLVM/rustc target extension prove insufficient.
- Unsafe privileged code has an explicit inventory and reviewer.
- Feature catalog size is unrestricted; selected installation and privileged TCB have budgets.
- Research providers never block the production core.
- Every gate records functionality, security, recovery, latency, throughput, memory, image size, power where measurable, and remaining unknowns.

## 3. Phase A — Specification reconciliation and executable models

**Minimum plausible duration:** 6–18 months after staffing. **Current state:** in progress; the first capability reference/provider/CTS slice exists under `reference/`, a transaction/external-effect reference slice exists, and a host Layer 1 scheduler state model (uniprocessor EDF, RFC-0011 §3) now exists, while composition, update, UASA, target-kernel, and formal models remain unimplemented.

Deliverables:

1. Accept RFC-0043 rights algebra and RFC-0042 update/rollback governance.
2. Resolve canonical-versus-live capability representations and CHERI semantic orthogonality.
3. Approve compatibility tiers and product non-goals.
4. Complete the UASA–AetherOS interface contracts from `UASA-INTEGRATION-STATUS.md`; obtain and independently reproduce the underlying UASA implementation evidence before provider selection.
5. Build executable host models of capability derivation/revocation (done), composition (done: manifest validation, resolution, profile selection, routing, conformance declarations), transaction/audit commitment (done for the single-node reference: failure-atomic commit, snapshot isolation and write-write conflict detection per RFC-0016.8/.9, byte-granularity crash-consistency sweep per PERSISTENCE-AND-EXTERNAL-EFFECTS.md §6, durable-intent outbox dispatcher per §4 steps 5-8), outbox effects (done at host-model level; real-transport delivery remains out of scope), scheduler states (Layer 1 uniprocessor EDF done; Layer 2/3 and SMP open), and failure semantics (partially covered by the crash sweep and the §10 failure-mode taxonomy).
6. Build the CTS runner before the target kernel.
7. Establish reproducible toolchains, threat model, TCB budget, ABI evolution policy, benchmark protocol, unsafe-code policy, and supply-chain controls.

**Exit gate:** models pass property tests and injected failures; unresolved contradictions are recorded; the first vertical-slice interfaces are approved. No “frozen” label is restored merely for completing documents.

The product-version realization of these gates is defined in `VERSIONED-PRODUCT-IMPLEMENTATION-PLAN.md`.

## 4. Phase B — QEMU RISC-V64 vertical slice

**Historical status text superseded:** the earlier 19 November note said boot was not verified because QEMU was absent on that development host. The current public `main` has since passed the CI QEMU RISC-V64 boot qualification on 22 November 2026 (run 131, ID `45012374998`). This validates the workflow's declared QEMU boot scenario only; it does not qualify the complete Phase B exit gate or the first usable release.

Implement only: OpenSBI/QEMU boot, minimal executor, physical/page allocation, Sv39 address spaces, traps/timer, compact capability spaces, one IPC path, one isolated component, UART, virtio-block, append-only transactional state prototype, batched audit commitment, outbox recovery, deterministic test harness, and crash/fault injection.

**Exit gate:** reproducible boot; unauthorized operations fail; one component can be killed and restarted; transaction recovery passes every injected crash point; hot-path budgets are measured; no unexplained authority exists.

## 5. Phase C — Usable reference system

**Minimum plausible duration:** 18–36 months. **Current state:** not started.

Add SMP, scheduling, timers, capability revocation, resource accounting, virtio-net/input/GPU, user-space network stack, package/image composition, signed A/B update and rollback, native C and Rust SDKs, WebAssembly Component/WASI runtime, observability, debugger support, and an initial storage provider selected after UASA review.

**Exit gate:** self-hosted or hermetic application builds, 30-day stress run, recovery and update campaigns, fuzzing, independent security review, published performance/TCB/footprint results, and a maintained reference configuration.

## 6. Phase D — Deployment ports and hardware

**Minimum plausible duration:** 24–48 months, overlapping carefully. **Current state:** not started.

Port to QEMU x86-64 and AArch64, then one documented physical platform each. Add UEFI/ACPI as needed, PCIe, MSI/MSI-X, IOMMU, NVMe, USB xHCI, selected network devices, firmware framebuffer, and eventually selected GPUs. Every driver is isolated where feasible; foreign drivers remain in guests or driver VMs.

**Exit gate:** cross-architecture semantic CTS, suspend/resume where claimed, DMA isolation tests, hardware fault campaigns, published hardware compatibility list, and stable update/recovery across all reference devices.

## 7. Phase E — Compatibility products

**Minimum plausible duration:** 3–8+ years and continuous. **Current state:** not started.

Order: Linux guest/virtio integration; OCI workloads in Linux micro-VMs; POSIX source profile; high-value Linux user ABI subsets; Windows guest integration; selected Win32 personality support; Android guest/framework integration; driver VMs; broader graphics and audio. Each result is recorded at C0–C9 from `COMPATIBILITY-STRATEGY.md`.

**Exit gate:** none globally. Each application family, ABI version, or guest configuration has its own supported-product gate. Compatibility is permanently ongoing.

## 8. Phase F — Enterprise and assurance expansion

**Minimum plausible duration:** 5–12+ years and continuous. **Current state:** not started.

Add fleet management, HA/recovery, key lifecycle, audit export, backup/restore, compliance evidence, LTS branches, certification profiles, performance engineering, proof expansion, red-team exercises, support operations, and vendor qualification.

Formal verification prioritizes semantic models and the smallest security-critical implementation surfaces. A claim covering a full distribution names every excluded compiler, firmware, device, service, and hardware assumption. Safety-critical certification is a separate product programme, not inherited from the desktop/cloud profile.

## 9. Experimental future providers

CHERI/native capabilities, CXL, confidential computing, DPU, FPGA, tensor, quantum, neuromorphic, and photonic providers proceed in isolated research tracks after their contracts and hardware are real. They do not carry delivery dates in the production roadmap. Promotion requires hardware access, failure semantics, security analysis, CTS, benchmarks, maintainer ownership, and rollback/fallback.

## 10. Staffing and economic reality

An initial credible core team needs kernel/architecture, Rust and low-level systems, formal methods, storage, security, toolchain, test/release, and documentation expertise. Broader compatibility and hardware require dedicated driver, graphics, virtualization, networking, filesystems, developer-experience, legal/compliance, SRE, and partner teams. Cost models SHALL include long-term maintenance, security response, CI hardware labs, proof repair, and documentation—not only initial implementation.

## 11. AI-native operation — validation objective

AI-native operation is **not** a new architectural primitive, subsystem, or RFC. It is a conformance scenario that forces AetherOS's existing architecture to prove itself under the hardest realistic autonomous workload.

### 11.1 Core position

The architecture **permits** AI operation at the capability-model level. Whether AetherOS **demonstrably provides** AI operation requires an executed vertical scenario with implementation evidence. Until such evidence exists, these two statements must remain distinguished:

```text
Architecture permits AI operation        — conceptual capability
AetherOS demonstrably provides AI operation — requires executed scenario
```

The objective is therefore: **AI-native operation must be an explicit validation objective of AetherOS, even if it is not a separate architectural subsystem or RFC.**

### 11.2 Required demonstration scenario

A connected, authorized AI must be able to operate the machine through native semantic interfaces. The primary demonstration scenario is:

```text
AI request: "Inspect the failing test, fix the implementation, run the tests, commit"
    ↓
identity/session
    ↓
manifest
    ↓
capability derivation
    ↓
scope/policy
    ↓
capability mediation
    ↓
execution
    ↓
audit
    ↓
revocation/recovery
    ↓
verifiable result
```

This scenario is superior to a trivial "create a file" demonstration because it exercises multiple resource classes and authority transitions in one realistic workflow:

```text
read project
    ↓
spawn tools
    ↓
write source
    ↓
execute compiler/tests
    ↓
inspect results
    ↓
modify again
    ↓
Git operation
    ↓
commit
```

### 11.3 Machine-readable authority provenance

Every AI operation must generate a machine-readable authority provenance graph answering: **"Why was this AI allowed to perform this exact operation?"**

Example structure:

```text
AI Session
   │
   ├── Manifest M-104
   │       │
   │       └── Capability C-7
   │               │
   │               ├── Resource: project/*
   │               ├── Operations: read/write
   │               └── Lifetime: session
   │
   └── Request R-8821
            │
            └── Mediation Decision D-8821
                    │
                    ├── invariant checks
                    ├── policy checks
                    ├── scope checks
                    └── execution provider
```

Mechanical answer:

```text
Because:
Manifest M-104
→ derived capability C-7
→ resource scope project/X
→ operation write
→ policy P-19
→ mediation decision D-8821
→ invariant set {I-2, I-3, I-13}
```

This is a far stronger AI-security demonstration than showing an agent can operate a desktop UI. The provenance graph must be emitted as part of CTS telemetry (per `Conformance-Test-Suite-Methodology.md` Section 8).

### 11.4 Three-level AI control hierarchy

The hierarchy is progressive loss of semantic certainty. Native control provides structured resources, typed operations, explicit capabilities, and deterministic authorization. UI/vision provides pixels, inferred intent, and simulated input. AetherOS shall make native interfaces preferable wherever available and make lower-level interaction an explicitly identifiable compatibility path.

```text
Tier 1 — Native semantic control        structured + typed + explicit + deterministic
        ↓ (loss of semantic certainty)
Tier 2 — Compatibility/API automation   translated / projected semantics
        ↓ (loss of semantic certainty)
Tier 3 — UI/vision automation           pixels + inferred intent + simulated input
```

### 11.5 Implementation composition

The AI validation scenario uses the existing implementation chain without modification:

`RFC-0002 → RFC-0003 → RFC-0013 → RFC-0034 → RFC-0037 → RFC-0039 → RFC-0040`

No new AI subsystem is created. The scenario is measured against the same invariants, CTS layers, and verification tiers as every other workload.

### 11.6 End state

The computer is directly operable by machine intelligence through native semantic interfaces, but the intelligence never becomes the authority. AetherOS remains the authority boundary. Compatibility and visual automation are available as fallbacks, explicitly identified as lower-certainty paths.

## 12. Stop/review triggers

Review with the project owner before continuing when an invariant forces unacceptable measured overhead; legal terms block a required compatibility target; hardware documentation is unavailable; a legacy behavior would enter the privileged core; proof cost grows faster than the protected risk; UASA semantics conflict with State invariants; or a phase misses its evidence gate twice. The review presents measured alternatives: redesign, isolate, virtualize, recompile, narrow the claim, defer, or explicitly fund the cost.

AI-native validation scenario triggers additional review when:
- A mediation decision cannot be answered with a provenance graph
- Tier 3 (UI/vision) becomes the only available AI control path on a native-capable platform
- The authority provenance graph exceeds observable audit granularity
- AI operation demonstrates authority escalation outside the capability derivation chain
