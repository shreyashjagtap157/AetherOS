# Implementation Roadmap — Evidence-Driven Programme

Classification: Operational
Authoritative Source: IMPLEMENTATION-ROADMAP.md
Requirement-ID: GOV-ROADMAP-001
Status: Validation Roadmap

**Updated:** 2026-07-27. This roadmap replaces calendar optimism with evidence gates. Dates are forecasts only after a staffed team measures throughput. Passing a phase gate, not elapsed time, authorizes the next maturity claim.

## 1. Brutally honest baseline

There is currently no kernel, bootloader, HAL, driver, runtime, filesystem implementation, executable CTS, benchmark, formal machine proof, supported hardware configuration, or application compatibility result in this repository. Forty-one architecture RFCs and three draft implementation-boundary RFCs are design inputs, not implementation evidence. The architecture is a validation candidate, not frozen fact.

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

**Minimum plausible duration:** 6–18 months after staffing. **Current state:** in progress in documentation only.

Deliverables:

1. Accept RFC-0043 rights algebra and RFC-0042 update/rollback governance.
2. Resolve canonical-versus-live capability representations and CHERI semantic orthogonality.
3. Approve compatibility tiers and product non-goals.
4. Complete the UASA–AetherOS interface contracts from `UASA-INTEGRATION-STATUS.md`; obtain and independently reproduce the underlying UASA implementation evidence before provider selection.
5. Build executable host models of capability derivation/revocation, composition, transaction/audit commitment, outbox effects, scheduler states, and failure semantics.
6. Build the CTS runner before the target kernel.
7. Establish reproducible toolchains, threat model, TCB budget, ABI evolution policy, benchmark protocol, unsafe-code policy, and supply-chain controls.

**Exit gate:** models pass property tests and injected failures; unresolved contradictions are recorded; the first vertical-slice interfaces are approved. No “frozen” label is restored merely for completing documents.

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

## 11. Stop/review triggers

Review with the project owner before continuing when an invariant forces unacceptable measured overhead; legal terms block a required compatibility target; hardware documentation is unavailable; a legacy behavior would enter the privileged core; proof cost grows faster than the protected risk; UASA semantics conflict with State invariants; or a phase misses its evidence gate twice. The review presents measured alternatives: redesign, isolate, virtualize, recompile, narrow the claim, defer, or explicitly fund the cost.
