# Handoff: AetherOS Pre-Phase-1 RFC Work — Exhaustive Project Review

Classification: Informative
Authoritative Source: HANDOFF-PRE-PHASE-1-RFCs.md

**Date:** 2026-06-09
**Purpose:** Complete state snapshot so work can continue in a new chat session.
**Classification:** Operational

---

# Part 1: What AetherOS Is

AetherOS is a formal computing platform architecture — not a hobby OS project. The project answers one question:

> What would an operating system architecture look like if every measurable dimension were optimized simultaneously, with tradeoffs explicit rather than accidental, and with each subsystem specified precisely enough to be implemented?

The answer is a **specification suite**: normative invariants, five peer architectures, a verification-tier system, complexity budgets, and 44 RFCs (41 original + 3 pre-Phase-1 blockers + the methodology file counted as the 41st cross-cutting RFC). There is **zero implementation code**. The entire project is frozen architectural specification and normative RFCs.

---

# Part 2: Complete File Inventory

## Top-Level Documents (19 files)

| File | Status | What It Contains |
|------|--------|-----------------|
| `Platform-Architecture-Specification-v1.1.md` | **Frozen v1.1** | 373 lines. 13 invariants (I-1 through I-13), five-peer decomposition, capability security model, origin of authority and 5-stage bootstrap, verification tiers (A/B/C/D), complexity budgets (5 quantitative metrics), three-layer scheduling, persistence/state boundary, distributed execution boundary (v1=single domain), hardware universality, profiles reference, required RFCs list |
| `ARCHITECTURE-GOVERNANCE.md` | **Active** | 445 lines. Four governance categories (Architecture/Specification/Editorial/Governance Amendment), source of truth hierarchy, RFC lifecycle states, version numbering, consensus/disagreement resolution, authority hierarchy (6 principles), evidence-based amendment principle (3 admissible evidence classes: Conformance, Verification, Security), governance telemetry |
| `Governance-RFC.md` | **Frozen** | Superseded by ARCHITECTURE-GOVERNANCE.md but retained for historical reference. Defines RFC document conventions (11 required sections), amendment categories, compliance levels (MUST/SHOULD/MAY), numbering scheme, conformance declaration format |
| `RFC-GOV-002.md` | **Frozen** | Evidence-Based Amendment Principle. Per-RFC Decision Record requirement. Three admissible evidence classes. Architecture Review trigger via Observation Records. No Shortcut clause (implementation cost alone is not admissible). CI gate enforcement |
| `Profiles-Specification.md` | **Active** | 5 deployment profiles: Embedded, Desktop, Cloud, HPC, Safety-Critical. Each declares mandatory/optional/forbidden mechanisms, default verification tiers, minimum conformance test requirements. Profiles never relax invariants; they select mechanism subsets |
| `Conformance-Test-Suite-Methodology.md` | **Active** | 4-layer CTS: Layer 1 (Invariant Conformance), Layer 2 (RFC Conformance), Layer 3 (Profile Conformance), Layer 4 (Cross-Conformance). Test specification format, test categories (functional, property-based, forensics, bootstrap, verification-tier), conformance evidence (run-time, static, sustainability), 3 execution modes (sandbox, hardware-attested, reproducibility) |
| `Glossary.md` | **Frozen** | 100+ term definitions |
| `00-RFC-Index.md` | **Active** | Master index of 44 RFCs. Cross-index by invariant (I-1 through I-13 mapped to RFCs). Reading order recommendations |
| `IMPLEMENTATION-ROADMAP.md` | **Revised** | 5-phase plan: Phase 0 (Design Validation — complete), Phase 1 (Reference Implementation RISC-V + Formal Foundation, months 1-9), Phase 2 (x86-64 + ARM64 deployment ports, months 9-15), Phase 3 (Application Compatibility, months 15-21), Phase 4 (Performance Engineering, months 21-30), Phase 5 (Experimental Research Interfaces, months 30-42+). Pre-Phase 1 requirements: three blocker RFCs |
| `IMPLEMENTATION-STATUS.md` | **Active** | Live status tracker. All 44 RFCs listed with status. Hardware provider status (RISC-V reference, x86-64/ARM64 Phase 2, GPUs Phase 3, TPU/FPGA/DPU Phase 4, quantum/neuromorphic Phase 5 experimental). CTS status (all not implemented). Key milestones with dates |
| `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md` | **Active (Revision 2)** | 1155 lines. 14 blocking requirements (REQ-INTF-000, REQ-ALLOC-01 through 04, REQ-COMP-01 through 05, REQ-DRIVER-01 through 05). 7 cross-RFC interfaces (INTF-000 through INTF-006). Invariant coverage map. CTS coverage. Formal verification obligations. Three-track dependency model (Specification/Engineering/Verification). Implementation Contracts layer (Section 11). Updated critical path with HAL-BOOT before Minimum Executor |
| `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md` | **Accepted** | Option C: Two-layer architecture-neutral substrate. Native Capability Provider (NCP) for CHERI hardware (Phase 3+). Software Capability Provider (SCP) for Phase 1 on conventional RV64. Unforgeability model, provenance model, representation, derivation, revocation, address interaction, fault semantics, trust boundary, performance contract, verification boundary. Migration path to native provider. 4 CTS tests declared |
| `Hardware-Support-RFC.md` | **Active** | Execution-provider contract, HAL operations struct, HAL-BOOT (minimal boot substrate) vs HAL-RISCV (full substrate). RISC-V Sv39/Sv48 HAL design. Hardware universality. GPU as substrate-neutral execution interface |
| `REVIEWER-GUIDE.md` | **Active** | Mandatory pre-review workflow: 7-step process (Understand → Reconstruct → Validate → Review → Impact Assessment → Specification Update → Conformance Review). Reconstruction Artifact requirement. Impact Matrix mandatory. Confidence levels (High/Medium/Low). Disagreement resolution. Annual review cycle |
| `REPOSITORY-PRINCIPLES.md` | **Informative** | 7 principles: One Authoritative Source Per Fact, Authority Flows Downward, Generated Artifacts Never Authoritative, Reviews Reconstruct Before Criticizing, Observation Is Not Amendment, Architecture Changes Require Constitutional Evidence, Governance Preserves Architectural Integrity |
| `OBSERVATION-RECORD.md` | **Operational** | Template for filing Observation Records. Fields: ID, Date, Environment, Evidence, Affected RFCs/CTS, Severity, Confidence, Disposition (Informational/Investigation/Closed/Forwarded to Governance), Resolution Target. No Architectural Recommendation clause |
| `CHANGELOG.md` | **Generated** | Change history. v1.0 → v1.1 material changes. All RFCs listed. Strategic direction changes documented |
| `Revision-History.md` | **Informative** | Suite-level history. v1.0 → v1.1 material changes explained. Why v1.1 stopped here (edits ceased discovering invariants and started re-discovering them) |
| `README.md` | **Informative** | Entry point. Document map. How to read the suite. Status. Sources consulted (seL4, Fuchsia/Zircon, CHERI, Capsicum, Linux, Windows NT, QNX, MINIX3, Barrelfish, Kronos) |

## RFC Directory Structure (44 RFCs across 6 subdirectories + 3 Pre-Phase-1)

```
RFCs/
├── 00-Composition/     (5 RFCs: RFC-0002 through RFC-0006) — All Active
├── 01-Execution/       (8 RFCs: RFC-0007 through RFC-0014) — All Active
├── 02-State/           (9 RFCs: RFC-0015 through RFC-0023) — All Active
├── 03-Communication/   (7 RFCs: RFC-0024 through RFC-0030) — All Active
├── 04-Observability/   (6 RFCs: RFC-0031 through RFC-0036) — All Active
├── 05-Cross-Cutting/   (5 RFCs: RFC-0037 through RFC-0041) — All Active
└── 06-Pre-Phase-1/     (3 RFCs) — All Drafted
    ├── RFC-ALLOC-001.md    (418 lines)
    ├── RFC-COMPILER-001.md (418 lines)
    └── RFC-DRIVER-001.md   (414 lines)
```

### Complete RFC Inventory

**Composition (5 RFCs, all Active):**
| RFC | Title |
|-----|-------|
| RFC-0002 | Component Manifest |
| RFC-0003 | Capability Routing |
| RFC-0004 | Dependency Resolution |
| RFC-0005 | Profile Selection |
| RFC-0006 | Composition Conformance |

**Execution (8 RFCs, all Active):**
| RFC | Title |
|-----|-------|
| RFC-0007 | Execution Domain + Bootstrap (5-stage bootstrap model) |
| RFC-0008 | Dispatch |
| RFC-0009 | Address Space (page sizes, NUMA, CoW, mapping rights) |
| RFC-0010 | Interrupt Routing |
| RFC-0011 | Scheduler (Layers 1+2: deterministic core + advisory tuning) |
| RFC-0012 | Time |
| RFC-0013 | Capability Mediation (the enforcement boundary) |
| RFC-0014 | Execution Conformance |

**State (9 RFCs, all Active):**
| RFC | Title |
|-----|-------|
| RFC-0015 | Object Identity |
| RFC-0016 | Transaction Model |
| RFC-0017 | Integrity Model |
| RFC-0018 | Lifecycle FSM (8-state lifecycle for persistent objects) |
| RFC-0019 | Storage Provider Interface (removability test) |
| RFC-0020 | POSIX Projection |
| RFC-0021 | Object Projection |
| RFC-0022 | Key-Value Projection |
| RFC-0023 | State Conformance |

**Communication (7 RFCs, all Active):**
| RFC | Title |
|-----|-------|
| RFC-0024 | Message Format |
| RFC-0025 | Endpoint Model |
| RFC-0026 | Routing (v1 single-domain restriction) |
| RFC-0027 | Capability Transfer |
| RFC-0028 | Synchronization |
| RFC-0029 | Streaming |
| RFC-0030 | Communication Conformance |

**Observability (6 RFCs, all Active):**
| RFC | Title |
|-----|-------|
| RFC-0031 | Event Model |
| RFC-0032 | Trace Span |
| RFC-0033 | Replay Format |
| RFC-0034 | Audit Log |
| RFC-0035 | Diagnostic Query |
| RFC-0036 | Observability Conformance |

**Cross-Cutting (5 RFCs, all Active):**
| RFC | Title |
|-----|-------|
| RFC-0037 | Capability Token Format (96-byte token, I-13 orthogonality) |
| RFC-0038 | Capability Transfer Protocol |
| RFC-0039 | Capability Revocation Protocol |
| RFC-0040 | Verification Tier Methodology |
| RFC-0041 | CTS Cross-Cutting Supplement (counted as the 41st RFC) |

**Pre-Phase 1 (3 RFCs, all Drafted):**
| RFC | Title | Resolves |
|-----|-------|----------|
| RFC-ALLOC-001 | Allocator Hierarchy | INTF-001, INTF-004, INTF-005 |
| RFC-COMPILER-001 | Compiler and Runtime Boundary | INTF-002, INTF-003, INTF-006 |
| RFC-DRIVER-001 | Driver Framework Architecture | Consumes all interfaces |

**Future RFCs (Not Started):**
| RFC | Title | Status |
|-----|-------|--------|
| RFC-0042 | Cohort Update Governance | Not started |
| RFC-0043 | Capability Rights Algebra | Not started |
| RFC-Q1 | Quantum Execution Provider Contract | Deferred (Phase 5) |
| RFC-N1 | Neuromorphic Provider Interface | Deferred (Phase 5) |
| RFC-P1 | Photonic Storage and Network Provider | Deferred (Phase 5) |

---

# Part 3: The 13 Architectural Invariants (Frozen)

These are the constitutional core. They do not change without Architecture Amendment RFC.

| # | Invariant | Statement |
|---|-----------|-----------|
| I-1 | Authority mediation | Every resource access authorized through capability token. Capability necessary but not sufficient — address also required. |
| I-2 | No ambient authority | No process/thread/component has authority not explicitly granted through capability transfer. |
| I-3 | Least privilege by default | Newly created capabilities grant minimum rights; rights elevation requires explicit derivation. |
| I-4 | Observable state transitions | Every persisted state mutation produces observable, attributable, auditable event before commit. |
| I-5 | Transactional persistence | No persistent mutation externally visible until enclosing transaction commits atomically. |
| I-6 | Cryptographic integrity | Every identifiable persistent object verifiable against declared content-addressed identity. |
| I-7 | Forward extensibility | No normative change invalidates conforming prior-version implementations. |
| I-8 | Verifiable core | Tier A components must admit machine-checked proofs of stated invariants. |
| I-9 | Replaceable mechanisms | Every mechanism must have at least one substitutable alternative without violating invariants. |
| I-10 | Explicit failure semantics | Every interface contract must declare failure modes including partial-failure behavior. |
| I-11 | Declarable profiles | Every implementation declares profile/mechanism/tier subset; absence is conformance violation. |
| I-12 | Origin of authority | Platform shall specify how initial authority is legitimately created at bootstrap. |
| I-13 | Address–authority orthogonality | Capability authorizes action; address locates resource. Collapsing them is conformance violation. |

---

# Part 4: The Three Pre-Phase-1 RFCs — Exhaustive Detail

## RFC-ALLOC-001: Allocator Hierarchy (418 lines)

**Status:** Draft
**Invariants:** I-1, I-2, I-3, I-8, I-13
**Depends on:** RFC-0037, RFC-0009, RFC-0011, INTF-000
**Resolves:** INTF-001 (Memory Region Capability Type), INTF-004 (DMA Capability Semantics), INTF-005 (Stack/Heap Allocation Capability)

**Two-layer architecture:**
1. Physical Memory Manager — buddy allocation, NUMA topology, huge pages, fragmentation control, zone policies
2. Capability Authority Manager — wraps allocation with capability mediation, constructs tokens, enforces derivation/narrowing/revocation

**Four capability types defined:**
| Type | Name | Grants | Held by |
|------|------|--------|---------|
| Type 1 | CAP_MEMORY | CPU read/write/execute of physical pages | Component, driver, kernel |
| Type 8 | CAP_DMA_MEMORY | Device-originated DMA access | Driver (via explicit grant) |
| Type 9 | CAP_IOMMU_CONFIG | IOMMU translation table programming | Capability-mediator ONLY |
| Type 10 | CAP_MMIO_REGION | Device register access | Driver |

**Three-authority model (INTF-004):** CPU access (CAP_MEMORY), device DMA access (CAP_DMA_MEMORY), IOMMU programming (CAP_IOMMU_CONFIG — never given to drivers).

**Six zones:** ZONE_DMA, ZONE_DMA32, ZONE_NORMAL, ZONE_HIGH, ZONE_MMIO, ZONE_KERNEL.

**Boot allocator → capability-gated allocator transition:** Single-use boot allocator in ZONE_KERNEL. Decommissioned after Capability Root constructs origin capability. One-way irreversible transition.

**Stack/heap capabilities:** Every thread gets CAP_MEMORY for stack (READ+WRITE, bound to domain). Heap accessed through capability-gated system call with RIGHT_DERIVE + RIGHT_WRITE.

**Slab allocator:** For fixed-size kernel objects (capability tokens, domain descriptors, thread control blocks). NOT capability-gated (kernel-internal). NOT on scheduler hot path.

**NUMA policies:** NUMA_LOCAL, NUMA_NODE(N), NUMA_INTERLEAVE, NUMA_PREFERRED.

**Deterministic allocation for scheduler (I-8):** Layer 1 scheduler structures allocated from pre-allocated pool at boot. Hot path MUST NOT invoke page allocator, slab allocator, or capability-gated allocation.

**Data structures:** memory_region_descriptor, dma_region_descriptor, stack_descriptor, heap_descriptor, boot_allocator_record, phys_region_descriptor, alloc_request.

**Failure semantics:** 12 failure modes defined (ECAPSTALE, EALLOCSIZE, EALLOCEXHAUST, ENUMAUNAVAIL, EALLOCDMAZONE, bootstrap halted, non-conformant).

**9 CTS tests:** CTX-ALLOC-001 through CTX-ALLOC-009.

**6 verification obligations:** Capability Authority Manager (VT-A), buddy algorithm (VT-C), scheduler pool (VT-A), DMA mediation (VT-A), NUMA (VT-D), boot transition (VT-A).

**4 open questions:** Boot allocator pool size, heap derivation depth (max 16 suggested), DMA coherency (coherent assumed in Phase 1), guard page mechanism.

---

## RFC-COMPILER-001: Compiler and Runtime Boundary (418 lines)

**Status:** Draft
**Invariants:** I-1, I-2, I-12, I-13
**Depends on:** RFC-0037, RFC-0038, RFC-0025, RFC-0007, RFC-ALLOC-001
**Resolves:** INTF-002 (Capability Register Convention), INTF-003 (Component Entry Protocol), INTF-006 (Error Propagation at ABI)

**Four ABI layers:**
1. ABI Layer — calling conventions, register preservation, stack frame layout, capability register convention
2. Capability Passing Layer — how tokens cross function/trust boundaries
3. Component Lifecycle Layer — entry, exit, error propagation
4. Linkage Layer — PIC, symbol resolution, load-time relocations

**Capability register convention (INTF-002):** In Phase 1 (SCP), capabilities in conventional integer registers. No dedicated hardware capability register class. Capability-carrying registers are caller-saved. Atomic load of both halves. Faulting representation yields ECAPSTALE.

**Capability passing protocol:** Max 1 capability as return value. No stack-passed capability tokens in calling convention (only as explicit save/restore). Trust boundary crossing always mediated by capability-mediator.

**Object model:** object_header (identity, header_size, object_size, capability_slots, flags). OBJ_SEALED, OBJ_TRANSIENT, OBJ_DOMAIN_BOUND flags. Null capability = 96-byte zeroed token.

**Component entry protocol (INTF-003):** 5-step entry stub: (1) Stack pointer validation, (2) TLS pointer init, (3) Capability environment establishment, (4) Root capability grant, (5) ComponentEntered event emission. Component does NOT set up its own entry environment.

**Component exit protocol:** 4-step exit stub: (1) Capability namespace release (all CAPF_BOUND capabilities revoked), (2) Memory release, (3) ComponentExited event emission, (4) Control transfer to parent.

**Error propagation (INTF-006):** CAP_ERROR (type 11) — 96-byte token with zeroed seal. NOT a valid capability. Error code in `rights` field. 8 error codes: ERR_NOMEM, ERR_NOCAP, ERR_FAULT, ERR_RANGE, ERR_PERM, ERR_DEPEND, ERR_TIMEOUT, ERR_BUSY. Uncaught exceptions delivered as error token to registered fault handler.

**PIC and GOT:** Components compiled as PIC. Global Offset Table for capability-relative relocations. Load-time resolution (full dynamic linking deferred to Phase 3). Symbol table distinguishes capability symbols from data symbols.

**Address space layout:** Guard → Stack → TLS → Code (PIC, RIGHT_EXECUTE) → Data (RIGHT_READ+RIGHT_WRITE) → GOT → Heap → Guard. Optional ASLR.

**RISC-V specific ABI (Section 2.9):** 12 registers for 96-byte capability token. a0-a7 for capability arguments (max 2 capabilities in registers due to RV64 register count). t0-t6 for capability temporaries. s0-s11 for data only. Max 2 register-passed capabilities; additional via stack-passed slots.

**ABI state machine:** 10 states from COMPONENT_NOT_LOADED through COMPONENT_DESTROYED.

**11 CTS tests:** CTX-COMP-001 through CTX-COMP-011.

**6 verification obligations:** Capability register convention (VT-A), entry/exit stub (VT-C), error token (VT-A), object header (VT-A), PIC (VT-C), symbol resolution (VT-C).

**4 open questions:** Max capability arguments per call (2 in registers), error token sealing, TLS model complexity (flat for Phase 1), dynamic linking scope (static load-time for Phase 1).

---

## RFC-DRIVER-001: Driver Framework Architecture (414 lines)

**Status:** Draft
**Invariants:** I-1, I-2, I-3, I-4, I-7, I-9, I-10, I-11, I-13
**Depends on:** RFC-0039, RFC-0018, RFC-0031, RFC-0002, RFC-0004, RFC-0010, RFC-ALLOC-001, RFC-COMPILER-001, INTF-000
**Consumes:** INTF-001 through INTF-006 (all resolved upstream)

**Four framework components:** Driver Lifecycle Manager, Driver Capability Broker, Device Registry, Dependency Resolver.

**7-state lifecycle FSM:**
```
Discovered → Loaded → Initialised → Operating → Unloaded
                                    ↕ Faulted
                                    ↕ Suspended
```

**Driver manifest:** Extends component manifest (RFC-0002) with device_class, device_vendor, device_product, capability_count, dependency_count, flags (DRV_HOTPLUG, DRV_SUSPEND, DRV_REPLACABLE, DRV_CRITICAL).

**Four capability types for drivers:**
| Type | Name | Grants | Held by |
|------|------|--------|---------|
| Type 12 | CAP_DEVICE_CONTROL | Device register operations | Driver |
| Type 8 | CAP_DMA_MEMORY | Device DMA access | Driver |
| Type 10 | CAP_MMIO_REGION | Memory-mapped I/O access | Driver |
| Type 9 | CAP_IOMMU_CONFIG | IOMMU translation programming | Capability-mediator ONLY |

Driver receives CAP_DEVICE_CONTROL + CAP_DMA_MEMORY + CAP_MMIO_REGION. Does NOT receive CAP_IOMMU_CONFIG.

**Hot-plug (INTF-002):** Device add = capability grant event. Device remove = capability revocation event. New device capabilities isolated from existing drivers.

**Driver dependencies:** Declared in manifest. Cycle detection at load time. Dependent driver notified and faulted when dependency fails.

**Power management:** Suspend = capability suspension (SUSPENDED flag). Resume = capability restoration. Failed resume → Faulted.

**Fault containment:** Address space isolation + capability isolation. Max blast radius = driver's own address space and capability set. Driver cannot corrupt another driver.

**Unload protocol:** 6 steps: capability revocation, IOMMU cleanup, interrupt deregistration, memory release, event emission, address space destruction.

**Data structures:** device_record, driver_state, driver_capability_grant, driver_dependency.

**13 CTS tests:** CTX-DRIVER-001 through CTX-DRIVER-013.

**7 verification obligations:** Driver Capability Broker (VT-A), lifecycle FSM (VT-B), DMA mediation (VT-A), hot-plug revocation (VT-A), fault containment (VT-B), dependency ordering (VT-B), power management (VT-B).

**5 open questions:** Driver cohort update protocol, fault recovery depth (default 3 retries), multi-device drivers, IOMMU granularity (page-level for Phase 1), driver security sandboxing (capability-only for Phase 1).

---

# Part 5: The Dependency Matrix — Complete Structure

The `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md` (1155 lines, Revision 2) is the bridge from architecture to implementation.

## 14 Blocking Requirements

| Requirement | RFC | Resolves Interface |
|------------|-----|-------------------|
| REQ-INTF-000 | (Decision Record) | INTF-000 (Capability Enforcement Substrate) |
| REQ-ALLOC-01 | RFC-ALLOC-001 | INTF-001 (Memory Region Capability Type) |
| REQ-ALLOC-02 | RFC-ALLOC-001 | INTF-001 |
| REQ-ALLOC-03 | RFC-ALLOC-001 | INTF-004 (DMA Capability Semantics) |
| REQ-ALLOC-04 | RFC-ALLOC-001 | INTF-005 (Stack/Heap Allocation) |
| REQ-COMP-01 | RFC-COMPILER-001 | INTF-002 (Capability Register Convention) |
| REQ-COMP-02 | RFC-COMPILER-001 | INTF-002 |
| REQ-COMP-03 | RFC-COMPILER-001 | INTF-003 (Component Entry Protocol) |
| REQ-COMP-04 | RFC-COMPILER-001 | INTF-003 |
| REQ-COMP-05 | RFC-COMPILER-001 | INTF-006 (Error Propagation) |
| REQ-DRIVER-01 | RFC-DRIVER-001 | Consumes all |
| REQ-DRIVER-02 | RFC-DRIVER-001 | Consumes all |
| REQ-DRIVER-03 | RFC-DRIVER-001 | Consumes all |
| REQ-DRIVER-04 | RFC-DRIVER-001 | Consumes all |
| REQ-DRIVER-05 | RFC-DRIVER-001 | Consumes all |

## 7 Cross-RFC Interfaces

| Interface | Resolved By | Description |
|-----------|------------|-------------|
| INTF-000 | INTF-000 Decision Record | Capability Enforcement Substrate — the precondition for all three RFCs |
| INTF-001 | RFC-ALLOC-001 | Memory region capability type (CAP_MEMORY, CAP_DMA_MEMORY) |
| INTF-002 | RFC-COMPILER-001 | Capability register convention at trust boundary |
| INTF-003 | RFC-COMPILER-001 | Component entry protocol (capability environment setup) |
| INTF-004 | RFC-ALLOC-001 | DMA capability semantics (three-authority model) |
| INTF-005 | RFC-ALLOC-001 | Stack and heap allocation capability |
| INTF-006 | RFC-COMPILER-001 | Error capability and error propagation at ABI |

## Three-Track Dependency Model

```
Specification Track:   ALLOC → COMPILER → DRIVER (strictly serial for interfaces)
Engineering Track:     Parallel after INTF-000 resolved and interfaces stable
Verification Track:    Architecture invariants → Formal models → RFC definitions → Implementation → CTS → Proof
```

## Updated Critical Path

```
Architecture Baseline v1.1                  ← COMPLETE
         │
INTF-000 Capability Enforcement Decision     ← COMPLETE
         │
Pre-Implementation Dependency Matrix         ← COMPLETE (this document)
         │
RFC-ALLOC-001                               ← DRAFTED
         │
RFC-COMPILER-001                            ← DRAFTED
         │
RFC-DRIVER-001                              ← DRAFTED
         │
Implementation Baseline v1.2                ← PENDING (after all 3 RFCs accepted)
         │
RISC-V Platform Contract (Pre-existing)
         │
Minimal RISC-V HAL (HAL-BOOT)               ← MUST come before Minimum Executor
         │
Minimum Executor                            ← Constructs origin capability
         │
Capability Root                             ← Creates initial namespace
         │
Composition Root                            ← Constructs initial service graph
         │
First isolated component                    ← Cold boots under CTS
         │
First real CTS run
         │
First Observation Record
         │
Evidence-driven evolution
```

## Implementation Contracts (Section 11)

The matrix defines an Implementation Contract schema that maps:
```
Requirement-ID → Interface ID → API/ABI → Implementation component → CTS test → Formal property → Evidence record
```

This is the layer between RFCs and code that turns governance into operational implementation control.

---

# Part 6: The INTF-000 Decision — Exhaustive Detail

## The Problem

RISC-V64 (Phase 1 reference) has no native hardware capability enforcement. The architecture must decide how capability tokens are unforgeable and provably validated.

## Three Options Considered

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A: Native CHERI-RISC-V | Hardware capability registers, tagged memory, sealed entries | Strongest Tier-A, hardware enforcement | Limited silicon availability, contradicts substrate-neutrality |
| B: Software emulation only | 96-byte tokens in integer registers, software mediator | Most portable | Security properties fundamentally different; forgeable if mediator bypassed |
| **C: Two-layer architecture-neutral** | Same semantic model, different enforcement providers | Architecture-neutral, substrate-neutral, Phase 1 feasible | Two enforcement codebases until CHERI hardware available |

## Accepted Decision: Option C

**Capability semantic model is architecture-neutral.** Enforcement delegated to provider selected per deployment.

**Two providers defined:**
1. **Native Capability Provider (NCP)** — CHERI-RISC-V hardware. Tier A. Phase 3+.
2. **Software Capability Provider (SCP)** — Conventional RV64. Tier B. Phase 1.

**Phase 1 implements SCP.** Rationale: QEMU RISC-V and current RISC-V silicon (SiFive FU740, StarFive JH7110) lack CHERI extensions.

**10 interface properties defined:** Unforgeability model, provenance model, representation, derivation, revocation, address interaction, fault semantics, trust boundary, performance contract, verification boundary.

**4 CTS tests:** CTS-INTF-000-1 through CTS-INTF-000-4.

**Migration path to native:** (1) NCP implementation accepted, (2) Conformance declaration updated, (3) Hybrid deployment exercising both providers, (4) SCP retained as removable layered extension.

**Key consequence:** RFC-0037 (Capability Token Format) is UNCHANGED. The 96-byte token operates above the provider boundary.

---

# Part 7: The Debate with Your Friend — Complete History

## How It Started

Your friend conducted an architectural review of AetherOS across multiple sessions (separate from this conversation). The debate evolved through four phases.

## Phase 1: Friend's Initial Review

Friend identified that three RFCs (ALLOC, COMPILER, DRIVER) were missing and blocking implementation. Friend proposed: **Dependency Matrix → Three RFCs → Implementation.**

Friend's key arguments:
- The architecture is a formal system with 13 invariants, capability-only authority, address-authority orthogonality, and formal verification obligations — not a typical OS project
- The three RFCs cannot be written independently because their interfaces are more dangerous than the RFCs individually
- Allocator must come first because capability semantics must be settled before the compiler ABI or driver model is frozen
- Compiler boundary must come second because it defines the actual contract between architecture and executable code
- Driver framework must come last because it sits at the intersection of allocator and compiler
- The 42-month roadmap should be treated as a planning hypothesis, not a commitment (zero implementation lines exist)
- seL4 is the closest precedent for the verification approach

## Phase 2: Claude's Critique (This Agent)

Claude reviewed the friend's plan and identified 4 refinements needed:

1. **Add INTF-000 (Capability Enforcement Substrate) as a precondition** — because RISC-V has no CHERI hardware, the capability enforcement mechanism was unresolved. This was the most critical gap.
2. **Change formal verification from "after two CTS cycles" to "formal modelling at RFC freeze, machine proofs at implementation stability"** — too late for Tier A properties.
3. **Reorder boot critical path: HAL-BOOT → Minimum Executor** (not the reverse) — the Minimum Executor cannot run without any hardware abstraction.
4. **Add Implementation Contracts layer between RFCs and code** — the bridge from RFC prose to code that turns governance into operational control.

## Phase 3: Friend's Response

Friend agreed with all 4 refinements and said the plan was "substantially correct" but not yet "properly decided" until INTF-000 was resolved and the capability enforcement substrate was decided.

Friend's 10 assessment points:

| Area | Friend's Assessment |
|------|-------------------|
| Architecture → implementation transition | Correct |
| ALLOC → COMP dependency | Mostly correct |
| ALLOC → DRIVER dependency | Correct |
| COMP → DRIVER dependency | Correct in important areas |
| Three RFCs as pre-implementation work | Correct |
| Dependency matrix itself | Very strong |
| CTS planning | Good, but should start earlier |
| Formal verification sequencing | Needs adjustment (now fixed) |
| Minimum Executor sequencing | Needs adjustment (now fixed) |
| "No code until all three RFCs complete" | Too strict (engineering can parallelize) |

Friend's specific additional insights:
- The RISC-V capability model is the most serious unresolved architectural question (now resolved by INTF-000)
- The minimum HAL must come before the Minimum Executor (now reflected in critical path)
- The allocator RFC needs to be split conceptually into Physical Memory Manager + Capability Authority Manager (now done in RFC-ALLOC-001)
- DMA semantics need sharper distinction: CAP_MEMORY vs CAP_DMA_MEMORY vs CAP_IOMMU_CONFIG (now done in RFC-ALLOC-001)
- CTS should not wait for the three RFCs to finish (now reflected in parallel workstreams)
- An Implementation Contract layer is needed between RFCs and code (now in Section 11 of dependency matrix)
- The claim "every invariant is touched by the RFCs" doesn't mean "every aspect is a Phase-1 blocker" (language corrected in dependency matrix)

## Phase 4: This Session (Resolution)

All 4 refinements implemented:
1. INTF-000 resolved: Option C accepted (two-layer architecture-neutral substrate)
2. Dependency matrix updated to Revision 2 with all 8 edits
3. All three RFCs drafted: RFC-ALLOC-001, RFC-COMPILER-001, RFC-DRIVER-001
4. Index and status documents updated

## Status: What Remains After This Session

**The three RFCs are drafted but NOT yet reviewed by anyone.** The debate with your friend reached agreement on the plan. The next step would be:

1. Present the three drafted RFCs to your friend for architectural review
2. Friend follows the REVIEWER-GUIDE.md workflow (Reconstruct → Validate → Review)
3. Any findings go through Impact Matrix process
4. After all three RFCs are accepted: declare Implementation Baseline v1.2
5. Then: HAL-BOOT → Minimum Executor → Capability Root → Composition Root → First Component

---

# Part 8: Key Architectural Decisions (All Locked)

| Decision | Resolution | Document |
|----------|-----------|----------|
| Capability enforcement substrate | Option C: two-layer architecture-neutral. SCP for Phase 1. NCP for Phase 3+. | INTF-000 Decision Record |
| Capability token format | 96-byte fixed structure per RFC-0037. Unchanged by INTF-000. | RFC-0037 |
| DMA authority model | Three distinct types: CAP_MEMORY, CAP_DMA_MEMORY, CAP_IOMMU_CONFIG. IOMMU-config restricted to mediator. | RFC-ALLOC-001 |
| MMIO capability | Distinct type CAP_MMIO_REGION, separate from CAP_MEMORY. | RFC-ALLOC-001, RFC-DRIVER-001 |
| Error representation | CAP_ERROR (type 11) — zeroed seal, not a valid capability. | RFC-COMPILER-001 |
| Capability registers | Conventional integer registers (no CHERI). 12-register representation on RV64. Max 2 register-passed capabilities. | RFC-COMPILER-001 |
| Boot ordering | HAL-BOOT → Minimum Executor → Capability Root → Composition Root → First Component | Dependency Matrix Section 9 |
| Formal verification timing | Tier A models at RFC freeze. Machine proofs after implementation passes CTS for 2 consecutive cycles. | Dependency Matrix Section 6 |
| Specification dependency | ALLOC → COMPILER → DRIVER (strictly serial for interfaces) | Dependency Matrix Section 7 |
| Engineering dependency | Parallel after INTF-000 resolved and interfaces stable | Dependency Matrix Section 7 |

---

# Part 9: What Is NOT Yet Done

1. **RFCs are Drafted, not Accepted** — They need architectural review (by your friend or another reviewer following REVIEWER-GUIDE.md)
2. **No Implementation Baseline v1.2** — Cannot be declared until all three RFCs are accepted
3. **No CTS code** — Fixtures declared in RFCs; no executable tests exist
4. **No formal verification models** — Tier A models should begin at RFC freeze
5. **No HAL implementations** — HAL-BOOT not yet implemented
6. **No code at all** — Zero implementation lines in the entire project
7. **RFC-0042 (Cohort Update Governance)** — Not yet written
8. **RFC-0043 (Capability Rights Algebra)** — Not yet written
9. **Decision Records for existing 41 RFCs** — Not yet back-filled (RFC-GOV-002 requires them)

---

# Part 10: Pitfalls and Lessons Learned

## Technical Pitfalls
- **Large files with backtick characters break the read_file tool** — Use terminal `cat | head/tail` as workaround
- **Terminal commands with spaces in paths** — `cd` into directory first, use relative paths
- **RFC-0018.md is in `02-State/` not `01-Execution/`** — Don't assume file locations; check directory listing first

## Architectural Pitfalls to Avoid
- Do NOT combine memory management + authority management + DMA into one monolithic spec
- Do NOT let the compiler ABI accidentally become RISC-V-specific (architecture-neutral where possible)
- Do NOT let the driver framework define its own capability semantics (must consume from ALLOC and COMPILER)
- HAL-BOOT must come before Minimum Executor
- Capability tokens are 96 bytes (FIXED per RFC-0037)
- IOMMU-config capability is NEVER given to drivers

## Process Pitfalls
- The 42-month roadmap is a hypothesis, not a commitment
- "Every invariant is touched by the RFCs" does not mean "every aspect is a Phase-1 blocker"
- The three RFCs should be reviewed together, not in isolation — their cross-RFC interfaces are the most dangerous part

---

# Part 11: How to Continue in a New Chat

## Option A: Continue Implementation Planning
Paste this handoff and ask: "Continue from where we left off. The three RFCs are drafted but need architectural review. Begin the review process."

## Option B: Have Friend Review the RFCs
Paste this handoff plus ask: "Read all three pre-Phase-1 RFCs and conduct an architectural review following REVIEWER-GUIDE.md."

## Option C: Start CTS Scaffolding
Paste this handoff and ask: "Create CTS fixture scaffolding for the three pre-Phase-1 RFCs, marked SPEC_PENDING until the RFCs are accepted."

## Option D: Start Formal Modelling
Paste this handoff and ask: "Begin formal modelling for Tier A properties identified in the dependency matrix."

---

# Part 12: Reading Order for a New Session

For maximum efficiency, read documents in this order:

1. `HANDOFF-PRE-PHASE-1-RFCs.md` (this file)
2. `Platform-Architecture-Specification-v1.1.md` (frozen architecture)
3. `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md` (the precondition)
4. `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md` Section 9 (critical path)
5. `RFCs/06-Pre-Phase-1/RFC-ALLOC-001.md` (first RFC)
6. `RFCs/06-Pre-Phase-1/RFC-COMPILER-001.md` (second RFC)
7. `RFCs/06-Pre-Phase-1/RFC-DRIVER-001.md` (third RFC)
8. `RFCs/05-Cross-Cutting/RFC-0037.md` (capability token format — the most security-critical data structure)
9. `RFCs/01-Execution/RFC-0007.md` (execution domain + bootstrap)
10. `RFCs/01-Execution/RFC-0013.md` (capability mediation — the enforcement boundary)
