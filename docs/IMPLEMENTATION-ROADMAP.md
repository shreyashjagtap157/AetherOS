# Implementation Roadmap — Ideal Computing Platform

Classification: Operational
Authoritative Source: IMPLEMENTATION-ROADMAP.md

**Status:** Revised per architectural review — 2026-06-09.
**Changes from prior version:** RISC-V as reference platform; formal proof-driven design from Day One; GPU provider as substrate-neutral; Phase 5 recast as experimental research interfaces; driver framework, allocator, and compiler boundary RFCs added as pre-Phase 1 requirements.

**This document is guidance, not specification.** The architecture RFCs (`Platform-Architecture-Specification-v1.1.md` and the 41 RFCs) are the normative references.

---

## 1. Strategic Direction Changes from Prior Version

### 1.1 RISC-V64 as Reference Platform (was: Phase 3 target)

The previous roadmap listed RISC-V as a later-phase addition. The revised strategy places RISC-V64 **first** — not because it is the most commercially important target, but because it is the most **verifiable** target.

**Why RISC-V first:**
- Cleaner ISA than x86-64 or ARM64; smaller trap model
- Formal verification tooling increasingly centered around RISC-V (seL4 MCS kernel now verified on RISC-V)
- Privilege specification is explicit and human-readable
- Memory model significantly easier to reason about for proof
- The seL4 team's published RISC-V verification results provide a concrete foundation to build on

**Platform roles (revised):**

| Role | Platform | Rationale |
|------|----------|-----------|
| **Reference** | RISC-V64 | Where every invariant is first proved correct |
| **Primary deployment** | x86-64 | Ecosystem compatibility |
| **Secondary deployment** | ARM64 | Ecosystem compatibility |
| **Research platform** | RISC-V + others | Where new provider classes are explored |

Reference means: every invariant is first validated here. Deployment means: we port the proven architecture. This ordering follows how formally engineered systems evolve — not how hobby OS projects typically operate.

### 1.2 Proof-Driven Design from Day One (was: verify-later option)

The previous roadmap presented a binary choice: verify immediately or verify later. The revised approach is neither.

**New approach:**
- Formal invariants, preconditions, postconditions, and refinement relations are written **before** first line of HAL implementation code (Day One)
- Complete machine proofs (Isabelle/Coq/Why3) begin **after** the corresponding implementation stabilizes — not after design
- This is the approach history repeatedly rewards: writing invariants before code forces design clarity; proving before code stabilizes wastes effort on things that will change

**Day One deliverables per RFC (already written in current RFCs — to be validated):**
- Formal invariants (stated in each RFC's Normative Requirements)
- State machine for every stateful subsystem
- Preconditions and postconditions for every protocol
- Refinement relation for every layered abstraction

**When stabilization triggers proofs (going forward):**
- An implementation is declared stable when it passes 100% of CTS fixtures without modification for two consecutive test cycles
- At that point, formal proof effort begins on Tier A components (scheduler core, capability mediation, execution domain boundary)

### 1.3 GPU Provider as Substrate-Neutral (was: CUDA provider)

The GPU provider exposes **asynchronous execution resources** — not CUDA, not HIP, not SYCL, not OpenCL. Those are compatibility layers that live above the provider.

This is a mandatory architectural constraint, not an implementation preference.

| Layer | What it is | What it is NOT |
|-------|-----------|----------------|
| GPU Execution Provider | Async execution resources, workitems, memory NUMA node | CUDA, HIP, SYCL, OpenCL, Vulkan Compute |
| GPU Compatibility Layer | CUDA/HIP/SYCL/OpenCL/Vulkan Compute | Part of application compatibility, not the provider |

The GPU provider contract does not know CUDA exists. CUDA compatibility is a user-space layer built on top.

### 1.4 Phase 5 as Experimental Research Interface (was: production milestone)

Phase 5 milestones are re-labeled as **experimental research interfaces** — not committed production features.

> **Quantum execution-provider research interface**: Define the extension point. Do not design the quantum operating system. Nobody knows today what the stable abstraction for quantum computation is.

> **Neuromorphic research interface**: Define the extension point for spiking neural network processors. Same reasoning.

> **Photonic research interface**: Define the extension point for optically-addressed storage and switching. Same reasoning.

This preserves the architecture's forward-extensibility invariant (I-7) without overcommitting to unimplementable specifications.

---

## 2. Pre-Phase 1 Requirements (Must Be Completed Before Any Implementation)

These are not optional. They are where mature kernels spend the most engineering effort and where the previous roadmap had the largest gaps.

### 2.1 RFC-DRIVER-001: Driver Framework Architecture

**Required before:** Any hardware target becomes implementable.

This RFC specifies:
- Driver lifecycle: discovery, loading, initialization, fault recovery, unload
- Driver isolation: drivers run as capability-sealed userspace processes
- DMA ownership: which device can access which memory ranges
- Hot-plug semantics: add/remove detection, capability renegotiation
- Power management integration: device suspend/resume
- Version negotiation between driver and hardware
- Capability binding: how drivers acquire and release device capabilities
- Driver-to-driver dependency and ordering

**This is not a driver.** It is the architecture that makes any driver possible.

### 2.2 RFC-ALLOC-001: Allocator Hierarchy

**Required before:** Any userspace component can be implemented.

This RFC specifies:
- Page allocator: physical page allocation, buddy system, fragmentation thresholds
- Slab allocator: kernel object allocation, cache sizing, growth/shrink policies
- NUMA policy: placement, migration, interleaving
- Huge page strategy: when to promote, when to split, what the application can influence
- Fragmentation control: proactive defragmentation triggers and bounds
- Allocator observability: allocation profiling, leak detection, corruption detection
- Interaction with address space (RFC-0009): who allocates what at each layer

### 2.3 RFC-COMPILER-001: Compiler and Runtime Boundary

**Required before:** The first component can be compiled for the platform.

This RFC specifies:
- ABI: calling conventions, register preservation, stack frame layout
- Capability passing conventions: which registers/registers fields carry capability tokens at function boundaries
- Scheduler integration: how the compiler emits work-signals for the scheduler (Layer 2 tuning, not hot path)
- Object model at the ABI level: how capability-addressable objects are represented in memory
- Exception and fault delivery: how hardware faults reach the correct component
- Position-independent code: how the component's address space is laid out
- Symbol resolution: how components import and export symbols via the Composition Architecture

**The compiler boundary must be stable before the first component is compiled.** Discovering ABI incompatibilities after components exist is a breaking change that requires a migration path.

---

## 3. Phased Implementation Plan

### Phase 0 — Design Validation (Already Effectively Complete)

This phase is already complete as a result of the 8+ rounds of architectural review. It is documented here for completeness and for any future reviewer who asks "was this design validated before implementation?"

**What was done:**
- Executable specification through RFC writing: every requirement is expressed as a numbered normative statement (`RFC-XXXX.N`)
- Invariant validation: every invariant in Section 2 of `Platform-Architecture-Specification-v1.1.md` was stress-tested through adversarial review
- Formal invariants in every RFC: each RFC contains preconditions, postconditions, state machines, and failure semantics
- Reference interpreter: the RFCs themselves serve as the behavioral specification; a reference interpreter would be a separate implementation project

**Outcome:** Architecture is frozen at v1.1. No further design validation is required before implementation.

---

### Phase 1 — Reference Implementation (RISC-V) + Formal Foundation (Months 1–9)

**Goal:** Produce a running kernel on RISC-V that passes all CTS tests, with formal invariants written for every Tier A component, and HAL implementations for RISC-V.

**Hardware targets:**
- RISC-V64 (SiFive FU740, StarFive JH7110, or QEMU simulation)

**What to implement:**

**Execution Architecture (RFC-0007 through RFC-0014):**
- Minimum Executor (5-stage bootstrap) on RISC-V
- Dispatch protocol on RISC-V harts
- Address space management with Sv39/Sv48 MMU
- Interrupt routing with RISC-V AIA (Advanced Interrupt Architecture)
- Layer 1 deterministic scheduler (EEVDF)
- Layer 2 advisory tuning (no ML on hot path)
- Capability mediation (HAL-emulated; RISC-V has no hardware CHERI yet)
- Time management (clint/MTIME)

**State Architecture (RFC-0015 through RFC-0018):**
- Object identity with SHA-3-256 (software implementation)
- Transaction model (two-phase commit)
- Merkle integrity (software Merkle tree)
- Lifecycle FSM

**Formal artifacts (concurrent with implementation):**
- Formal invariants for: scheduler core, capability mediation, dispatch protocol, bootstrap path
- Pre/postcondition specifications for: all public APIs in RFC-0007 through RFC-0013
- Refinement relation for: scheduler (Layer 1 algorithm ↔ formal model)
- **No complete machine proofs until corresponding implementation stabilizes** (per Section 1.2)

**Deliverable:** A conformant RISC-V kernel that passes all CTS fixtures for Execution and State architectures.

**Profiles:** Embedded (minimal). No POSIX projection required in Phase 1.

---

### Phase 2 — x86-64 and ARM64 Deployment Ports (Months 9–15)

**Goal:** Port the proven RISC-V reference implementation to x86-64 and ARM64, using the same platform core with hardware-specific HALs.

**Hardware targets:**
- x86-64 (Intel Skylake+ / AMD Zen 2+)
- ARM64 (ARMv8-A / ARMv8.2-A)

**What to implement (HAL additions):**
- x86-64 HAL: long mode paging (4-level), APIC, AVX-512 vector support, CET capability enforcement (where available), HPET timer
- ARM64 HAL: ARMv8-A long format paging, GICv3/v4, NEON/SVE2, PAuth + BTI (where available), ARM Generic Timer

**No changes to platform core** — only HAL. The execution-provider contract ensures this.

**Formal artifacts:**
- Formal invariants for x86-64 and ARM64 HAL additions
- Cross-provider capability tests (RISC-V ↔ x86-64 capability token exchange)

**Deliverable:** Platform running on x86-64 and ARM64. All CTS tests passing on both providers.

---

### Phase 3 — Application Compatibility (Months 15–21)

**Goal:** Enable legacy application execution and developer tooling.

**What to implement:**

**POSIX Projection (RFC-0020):** Full POSIX file, directory, link, and descriptor operations. Required for application compatibility.

**Communication Architecture (RFC-0024 through RFC-0030):** Full message format, endpoint model, routing, capability transfer, synchronization, streaming.

**Observability Architecture (RFC-0031 through RFC-0036):** Event model, trace spans, replay, audit log, diagnostic query.

**Driver Framework (RFC-DRIVER-001):** Driver discovery, lifecycle, DMA ownership, hot-plug. Initial driver set: UART, virtio-block, virtio-net.

**Compiler/Runtime (RFC-COMPILER-001):** GCC/Clang/LLVM backend support for the platform ABI. Capability token passing via registers.

**GPU Execution Provider:** Substrate-neutral. Memory as NUMA node. Work submission via capability-addressable work objects. CUDA/HIP/SYCL as compatibility layers above.

**Deliverable:** Developers can compile and run POSIX-compatible applications. GPU compute works via the provider interface.

---

### Phase 4 — Performance Engineering and Optimization (Months 21–30)

**Goal:** Achieve competitive performance through NUMA optimization, storage tuning, and scheduler refinement.

**What to implement:**

**NUMA Optimization:**
- Per-NUMA-node memory allocation policies
- NUMA-aware scheduling hints (Layer 2 advisory tuning)
- CXL 3.0 memory pool support

**Storage Optimization:**
- Tiering and placement policies
- Compression and deduplication services (as optional layered services, removable per RFC-0019)
- High-performance NVMe provider

**Scheduler Refinement:**
- Layer 1 formal proofs (scheduling algorithm correctness)
- Layer 2 tuning policies based on Phase 1+2 workload data
- HPC profile NUMA extensions

**Distributed Execution (preparatory):**
- Federation protocol design (RFC for v2; not implementation)
- Multi-provider scheduling model

**Deliverable:** HPC profile conformant. Cloud profile conformant. Storage providers meet performance budgets.

---

### Phase 5 — Experimental Provider Research Interfaces (Months 30–42+)

**Note:** These are research interfaces, not production features. Their inclusion indicates the architecture is designed to accommodate them — not that the platform commits to shipping them as conformant production components.

**TPU Provider (Phase 4a — can run in parallel with Phase 4):**
- Tensor workload lifecycle as capability-addressable objects
- Host/device memory sharing via address space model
- Interrupt-driven completion notification

**FPGA Provider (Phase 4b — can run in parallel with Phase 4):**
- Partial reconfiguration as capability-addressable execution domain
- PCIe BAR memory as storage/memory provider
- Xilinx/AMD Versal and Intel Agilex HALs

**DPU Provider (Phase 4c — can run in parallel with Phase 4):**
- NVIDIA BlueField / AMD Pensando as storage + network provider
- DPU resources as capability-addressable objects
- Onload network stack as provider below Communication Architecture

**Quantum Execution Provider Research Interface (Phase 5):**
- RFC-Q1 (not yet written): defines the quantum execution-domain extension point
- Classical/quantum co-execution via shared capability namespace
- No commitment to a specific quantum programming model

**Neuromorphic Research Interface (Phase 5):**
- RFC-N1 (not yet written): defines the neuromorphic provider extension point
- Spike events as interrupt-like notifications (RFC-0010 extension)

**Photonic Computing Research Interface (Phase 5):**
- RFC-P1 (not yet written): defines the photonic storage/network extension point
- Optical switching as zero-latency network provider abstraction

---

## 4. CTS Implementation Sequence

CTS implementation must track implementation, not lag behind:

1. **Phase 1 CTS** — Execution + State architecture fixtures for RISC-V
2. **Phase 2 CTS** — Cross-provider capability tests + x86-64/ARM64 provider tests
3. **Phase 3 CTS** — POSIX projection, Communication, Observability fixtures
4. **Phase 4 CTS** — NUMA, storage tiering, performance benchmarks
5. **Phase 5 CTS** — Provider-specific CTS for TPU/FPGA/DPU; experimental quantum CTS (will be minimal)

---

## 5. Milestone Summary

| Milestone | Focus | Duration | RISC-V First | Targets |
|-----------|-------|----------|--------------|---------|
| Pre-Phase 1 | RFC-DRIVER-001, RFC-ALLOC-001, RFC-COMPILER-001 | Months -3 to 0 | N/A | All Phase 1 blockers |
| Phase 1 | Reference implementation | Months 1–9 | ✓ | RISC-V64 boot; formal invariants |
| Phase 2 | Deployment ports | Months 9–15 | N/A | +x86-64, +ARM64 |
| Phase 3 | Application compatibility | Months 15–21 | N/A | POSIX, GPU provider, dev tooling |
| Phase 4 | Performance + optimization | Months 21–30 | N/A | HPC, Cloud, NUMA |
| Phase 5 | Experimental providers | Months 30–42+ | N/A | TPU, FPGA, DPU, quantum interface |

---

## 6. Key Revision Notes (Changes from Prior Roadmap)

| Prior Decision | Revised Decision | Rationale |
|---------------|-----------------|-----------|
| RISC-V = Phase 3 | RISC-V = Phase 1 (reference platform) | Verification alignment; cleaner ISA |
| Formal proofs = verify-later option | Proof-driven design Day One; complete proofs after stabilization | Neither immediate nor deferred; middle path |
| GPU provider = CUDA provider | GPU provider = substrate-neutral; CUDA = compatibility layer | Prevents provider contract from becoming CUDA-shaped |
| Quantum = Phase 5 production milestone | Quantum = experimental research interface (Phase 5) | Stable quantum OS abstraction unknown |
| Driver/allocator/compiler = Phase 2 | Driver/allocator/compiler RFCs = pre-Phase 1 requirements | These define where 70% of kernel engineering budget goes |

---

## Revision History

| Date       | Change |
|------------|--------|
| this cycle | Full revision. RISC-V as reference platform. Proof-driven design from Day One. GPU as substrate-neutral. Phase 5 as experimental research interface. Driver, allocator, compiler RFCs elevated to pre-Phase 1 requirements. |