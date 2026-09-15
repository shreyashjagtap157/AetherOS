# Implementation Roadmap — Evidence-Driven Programme

Classification: Operational
Authoritative Source: IMPLEMENTATION-ROADMAP.md
Requirement-ID: GOV-ROADMAP-001
Status: Validation Roadmap

**Updated:** 2026-07-27. This roadmap replaces calendar optimism with evidence gates. Dates are forecasts only after a staffed team measures throughput. Passing a phase gate, not elapsed time, authorizes the next maturity claim.

## 1. Brutally honest baseline

There is currently no kernel, bootloader, HAL, target driver, target runtime, filesystem implementation, benchmark programme, formal machine proof, supported hardware configuration, or application compatibility result in this repository. A 12-case executable **host reference CTS prototype** exists; it is not target-kernel or product conformance. Forty-one architecture RFCs and three draft implementation-boundary RFCs are design inputs, not implementation evidence. The architecture is a validation candidate, not frozen fact.

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

**Minimum plausible duration:** 6–18 months after staffing. **Current state:** in progress; the first capability reference/provider/CTS slice exists under `reference/`, and a transaction/external-effect reference slice exists, while composition, update, UASA, scheduler, target-kernel, and formal models remain unimplemented.

Deliverables:

1. Accept RFC-0043 rights algebra and RFC-0042 update/rollback governance.
2. Resolve canonical-versus-live capability representations and CHERI semantic orthogonality.
3. Approve compatibility tiers and product non-goals.
4. Complete the UASA–AetherOS interface contracts from `UASA-INTEGRATION-STATUS.md`; obtain and independently reproduce the underlying UASA implementation evidence before provider selection.
5. Build executable host models of capability derivation/revocation, composition, transaction/audit commitment, outbox effects, scheduler states, and failure semantics.
6. Build the CTS runner before the target kernel.
7. Establish reproducible toolchains, threat model, TCB budget, ABI evolution policy, benchmark protocol, unsafe-code policy, and supply-chain controls.

**Exit gate:** models pass property tests and injected failures; unresolved contradictions are recorded; the first vertical-slice interfaces are approved. No “frozen” label is restored merely for completing documents.

The product-version realization of these gates is defined in `VERSIONED-PRODUCT-IMPLEMENTATION-PLAN.md`.

## 4. Phase B — QEMU RISC-V64 vertical slice

**Minimum plausible duration:** 12–24 months after Phase A gate. **Current state:** not started.

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
