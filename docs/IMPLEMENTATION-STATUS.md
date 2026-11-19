# Implementation Status — Ideal Computing Platform

Classification: Operational
Authoritative Source: IMPLEMENTATION-STATUS.md

This document tracks the implementation status of every component in the platform. It is updated as implementation progresses and is the authoritative source for "what is done and what is not."

---

## Architecture Documents — Status

| Document | Status | Notes |
|----------|--------|-------|
| `Platform-Architecture-Specification-v1.1.md` | **Validation Candidate** | Current baseline; evidence-driven amendments remain expected before freeze |
| `Governance-RFC.md` | **Frozen** | Amendment process defined |
| `Profiles-Specification.md` | **Frozen** | 5 profiles defined |
| `Glossary.md` | **Frozen** | 100+ terms |
| `REVIEWER-GUIDE.md` | **Active** | Mandatory review workflow, Impact Matrix |
| `Hardware-Support-RFC.md` | **Active** | Phase 1–5 hardware support; RISC-V reference platform |
| `IMPLEMENTATION-ROADMAP.md` | **Revised 2026-07-27** | Evidence-gated, multi-year programme with brutally honest baseline and stop/review triggers |
| `VERSIONED-PRODUCT-IMPLEMENTATION-PLAN.md` | **Active 2026-08-01** | Product gates from empty 0.0.0 through minimal 1.0.0, complete 2.0.0, and post-2.0 evolution |
| `DEPENDENCY-AND-SUPPLY-CHAIN-POLICY.md` | **Active prototype policy** | Locked/offline Cargo source and license checks plus deterministic evidence/SBOM generation; advisory mirror and signed provenance remain open |
| `PRODUCT-FEASIBILITY-AND-SCOPE.md` | **Active** | Product priorities, installer composition, feasibility, AI verification limits, production definition |
| `COMPATIBILITY-STRATEGY.md` | **Active** | C0–C9 claim tiers; native, Linux, Windows, Android, VM, container, and driver lanes |
| `PERSISTENCE-AND-EXTERNAL-EFFECTS.md` | **Research Baseline** | Batched audit commitments, outbox protocol, irreversible-effect semantics |
| `UASA_COMPREHENSIVE_RESEARCH_REVIEW_PACKET.md` | **Imported from main** | Informative 2026-07-27 research handoff; reported results are not independently reproduced |
| `UASA-INTEGRATION-STATUS.md` | **Integration Review Active** | Cross-impact mapping, ZNS gates, work packages, and evidence boundary defined |
| `IMPLEMENTATION-STATUS.md` | **Active** | Live status tracker |
| `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md` | **Active (Revision 2)** | INTF-000 capability enforcement substrate resolved; 14 blocking requirements; 7 cross-RFC interfaces; three-track dependency model; Implementation Contracts layer; corrected boot ordering (HAL-BOOT → Minimum Executor) |
| `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md` | **Accepted** | Two-layer architecture (Native Capability Provider reserved for Phase 3+; Software Capability Provider for Phase 1); the unblocked architecturally-binding decision |

---

## RFCs — Status

### Composition (5 RFCs)

| RFC | Status | Notes |
|-----|--------|-------|
| RFC-0002 Component Manifest | **Written** | Normative spec complete |
| RFC-0003 Capability Routing | **Written** | Normative spec complete |
| RFC-0004 Dependency Resolution | **Written** | Normative spec complete |
| RFC-0005 Profile Selection | **Written** | Normative spec complete |
| RFC-0006 Composition Conformance | **Written** | Normative spec complete |

### Execution (8 RFCs)

| RFC | Status | Notes |
|-----|--------|-------|
| RFC-0007 Execution Domain + Bootstrap | **Written** | Normative spec complete; I-12 bootstrap model defined |
| RFC-0008 Dispatch | **Written** | Normative spec complete |
| RFC-0009 Address Space | **Written** | Normative spec complete |
| RFC-0010 Interrupt Routing | **Written** | Normative spec complete |
| RFC-0011 Scheduler (Layers 1+2) | **Written** | Normative spec complete |
| RFC-0012 Time | **Written** | Normative spec complete |
| RFC-0013 Capability Mediation | **Written** | Normative spec complete |
| RFC-0014 Execution Conformance | **Written** | Normative spec complete |

### State (9 RFCs)

| RFC | Status | Notes |
|-----|--------|-------|
| RFC-0015 Object Identity | **Written** | Normative spec complete |
| RFC-0016 Transaction Model | **Written** | Normative spec complete |
| RFC-0017 Integrity Model | **Written** | Normative spec complete |
| RFC-0018 Lifecycle FSM | **Written** | Normative spec complete |
| RFC-0019 Storage Provider Interface | **Written** | Normative spec complete; removability test defined |
| RFC-0020 POSIX Projection | **Written** | Normative spec complete |
| RFC-0021 Object Projection | **Written** | Normative spec complete |
| RFC-0022 Key-Value Projection | **Written** | Normative spec complete |
| RFC-0023 State Conformance | **Written** | Normative spec complete |

### Communication (7 RFCs)

| RFC | Status | Notes |
|-----|--------|-------|
| RFC-0024 Message Format | **Written** | Normative spec complete |
| RFC-0025 Endpoint Model | **Written** | Normative spec complete |
| RFC-0026 Routing | **Written** | Normative spec complete; v1 single-domain restriction |
| RFC-0027 Capability Transfer | **Written** | Normative spec complete |
| RFC-0028 Synchronization | **Written** | Normative spec complete |
| RFC-0029 Streaming | **Written** | Normative spec complete |
| RFC-0030 Communication Conformance | **Written** | Normative spec complete |

### Observability (6 RFCs)

| RFC | Status | Notes |
|-----|--------|-------|
| RFC-0031 Event Model | **Written** | Normative spec complete |
| RFC-0032 Trace Span | **Written** | Normative spec complete |
| RFC-0033 Replay Format | **Written** | Normative spec complete |
| RFC-0034 Audit Log | **Written** | Normative spec complete |
| RFC-0035 Diagnostic Query | **Written** | Normative spec complete |
| RFC-0036 Observability Conformance | **Written** | Normative spec complete |

### Cross-Cutting (5 RFCs)

| RFC | Status | Notes |
|-----|--------|-------|
| RFC-0037 Capability Token Format | **Written** | Normative spec complete; 96-byte token, I-13 orthogonality |
| RFC-0038 Capability Transfer Protocol | **Written** | Normative spec complete |
| RFC-0039 Capability Revocation Protocol | **Written** | Normative spec complete |
| RFC-0040 Verification Tier Methodology | **Written** | Normative spec complete |
| RFC-0041 CTS Cross-Cutting Supplement | **Written** | Normative spec complete |

### Pre-Phase 1 RFCs (Drafted)

| RFC | Title | Status | Notes |
|-----|-------|--------|-------|
| RFC-ALLOC-001 | Allocator Hierarchy | **Drafted** | Physical Memory Manager + Capability Authority Manager. Resolves INTF-001, INTF-004, INTF-005. I-1, I-2, I-3, I-8, I-13. |
| RFC-COMPILER-001 | Compiler and Runtime Boundary | **Drafted** | ABI, capability passing, component entry/exit, error propagation, PIC. Resolves INTF-002, INTF-003, INTF-006. I-1, I-2, I-12, I-13. |
| RFC-DRIVER-001 | Driver Framework Architecture | **Drafted** | Driver lifecycle FSM, capability model (device-control/DMA/MMIO/IOMMU-config), hot-plug, dependency, power management, fault containment. Consumes INTF-001–006. I-1–I-4, I-7, I-9–I-11, I-13. |

### Future RFCs (Required Before Implementation)

| RFC | Title | Status | Blocks |
|-----|-------|--------|--------|
| RFC-0042 | Cohort Update Governance | **Draft** | Candidate requirements exist; open decisions and executable evidence block acceptance |
| RFC-0043 | Capability Rights Algebra | **Draft** | Candidate algebra exists; mechanization and unresolved semantic choices block acceptance |
| RFC-Q1 | Quantum Execution Provider Contract | **Deferred** | Experimental research interface |
| RFC-N1 | Neuromorphic Provider Interface | **Deferred** | Experimental research interface |
| RFC-P1 | Photonic Storage and Network Provider | **Deferred** | Experimental research interface |

---

## Hardware Provider Status

| Provider | Phase | Status | Notes |
|----------|-------|--------|-------|
| **RISC-V64** | **Phase 1** | **Not started** | **Reference platform** — primary verification target; cleanest ISA for formal work |
| x86-64 (Intel/AMD) | Phase 2 | **Not started** | Primary deployment platform (port from RISC-V reference) |
| ARM64 (ARMv8-A) | Phase 2 | **Not started** | Secondary deployment platform (port from RISC-V reference) |
| RISC-V64 Sv39/Sv48 HAL design | Phase 1 | Drafted | HAL structure defined in Hardware-Support-RFC.md |
| IBM POWER9/10 | Phase 3 | **Not started** | Port from RISC-V reference after Phase 2 |
| NVIDIA GPU (CUDA) | Phase 3 | **Not started** | **Substrate-neutral GPU provider**; CUDA compatibility layer above |
| AMD GPU (ROCm) | Phase 3 | **Not started** | GPU provider; HIP compatibility layer above |
| Intel Xe GPU | Phase 3 | **Not started** | GPU provider; Level Zero compatibility layer above |
| Google TPU | Phase 4 | **Not started** | Tensor workload lifecycle defined in Hardware-Support-RFC.md |
| AWS Inferentia | Phase 4 | **Not started** | Neuron SDK provider interface |
| Xilinx/AMD FPGA | Phase 4 | **Not started** | Partial reconfiguration; FPGA as execution domain |
| Intel FPGA | Phase 4 | **Not started** | OneAPI provider interface |
| NVIDIA BlueField DPU | Phase 4 | **Not started** | Storage + network provider |
| AMD Pensando DPU | Phase 4 | **Not started** | Storage + network provider |
| IBM Quantum | Phase 5 | **Deferred** | RFC-Q1 not written; experimental research interface only |
| Neuromorphic (Loihi) | Phase 5 | **Deferred** | RFC-N1 not written; experimental research interface only |
| Photonic Computing | Phase 5 | **Deferred** | RFC-P1 not written; experimental research interface only |

---

## CTS Implementation Status

| Test Category | Status | Notes |
|-------------|--------|-------|
| Execution Architecture CTS | **Not implemented as a full architecture suite** | PR #8 adds one host reference memory-mapping/revocation case; fixtures and remaining execution semantics are still not implemented |
| State Architecture CTS | **Prototype started** | Failure-atomic commit, uncertain-effect recovery, snapshot-isolated reads (RFC-0016.8), write-write conflict detection (RFC-0016.9), a byte-granularity crash-consistency sweep (PERSISTENCE-AND-EXTERNAL-EFFECTS.md §6), and a durable-intent outbox dispatcher (§4 steps 5-8: dedup by operation id, crash-after-send → Uncertain, idempotent redelivery reconciliation) implemented; no production storage provider |
| Communication Architecture CTS | **Not implemented** | Fixtures declared in RFCs; code not written |
| Observability Architecture CTS | **Not implemented** | Fixtures declared in RFCs; code not written |
| Capability reference CTS | **Prototype implemented on main** | Nine executable cases following the PR #7 merge: attenuation, transfer, subtree-scoped lineage revocation across domains, forged/stale-handle rejection, foreign-bootstrap rejection, unregistered-domain rejection, and revoked-parent rejection |
| Reference CTS runner | **Seventeen cases merged on main; thirty-eight with the uncommitted working-tree slices (local gates green; CI pending)** | Main contains eleven capability/memory/dispatch cases and four transactional-state/external-effect recovery cases, plus the two live-authority/serialized-runtime cases merged in PR #10. The working tree adds CTS-CAP-EXPIRY-001, CTS-CAP-BOUND-001, CTS-SCHED-ADMIT/DETERMINISM/QUANTUM-001, CTS-COMP-RESOLVE/CYCLE/ORDER/VALIDATE-001, CTS-STATE-CONFLICT/SNAPSHOT/CRASH/OUTBOX-DISPATCH-001, CTS-PROF-SELECT/COMBINE-001, CTS-ROUTING-GRANT/DENY-001, and CTS-COMP-CONF-001..004; this remains prototype evidence, not kernel conformance |
| RV64/Sv39 memory-mapping model | **Host reference prototype on main** | Capability-gated MAP plus R/W/X PTE permissions, 4 KiB/2 MiB/1 GiB mapping, canonical-range checks, mapping activation/rollback, unmap and retryable shootdown model; not a kernel MMU or hardware implementation |
| Dispatch address-space activation | **Host reference contract on main** | Explicit activation before simulated execution, pre-access residency registration, address-space switch/reactivation, failed-activation quarantine, and dispatch refusal while shootdown is pending; no production scheduler integration |
| Per-access capability mediation | **Merged on main (PR #10)** | Every simulated access resolves the mapping's recorded domain/handle, checks the exact resource byte and MAP plus operation-specific rights, then independently enforces PTE R/W/X; cached-TLB revocation regression included |
| Serialized memory runtime | **Merged on main (PR #10)** | MemoryRuntime owns the capability provider and mapping manager behind one mutex, serializing supported grant/derive/transfer/map/unmap/dispatch/access/remove/revoke/retry operations; host-only global blocking lock, not a production kernel synchronization design |
| Capability lifetime and domain binding | **Implemented on working tree; uncommitted; local verification only** | `aether-types` models RFC-0037.7 `CapabilityFlags` (TRANSIENT/REVOCABLE/NAMED/BOUND/SEALED, unknown bits rejected) and `expires_at_ns`; RFC-0037.14 enforced with strict `now_ns > expires_at_ns` in both the oracle (`aether-cap-model`) and the provider (`aether-capspace`) at every authorize/derive/transfer boundary including all memory-access revalidation paths; children inherit parent expiry; `CAPF_BOUND` blocks transfer (`CapError::DomainBound`); cross-domain presentation fails `WrongDomain`. See CTS-CAP-EXPIRY-001, CTS-CAP-BOUND-001, and the lifetime/flags unit tests in both crates. Not merged, not CI-validated, not kernel-integrated |
| Cross-Architecture CTS | **Partially prototyped** | Host abstract model and table-backed software provider only; kernel/hardware paths do not exist |
| Scheduler Layer 1 state model | **Implemented on working tree; uncommitted; local verification only** | `aether-sched-model` (new crate): uniprocessor EDF per RFC-0011 §3, exact `sum(Ei/Pi) <= 1` admission in checked rational arithmetic, deterministic `(deadline, tid)` selection, quantum preempt/requeue with deadline advance, RFC-0011.5 transition events, one-shot per-period deadline-miss accounting; Layer 2/3, EEVDF, SMP, RFC-0008 dispatch integration, and bounded-latency proof explicitly out of scope |
| Composition host model | **Implemented on working tree; uncommitted; local verification only** | `aether-comp-model` (new crate): RFC-0002.19 manifest validation subset; RFC-0004 resolution with exact version matching, profile/tier compatibility, HighestVersion strategy (fail-closed on ambiguity), transitive closure, Kahn topological ordering with sorted-member cycle reporting, deterministic instantiation order; RFC-0005 profile selection (mechanism activation, default-off extensions, forbidden-union composition, tier floors); RFC-0003 capability routing decision model (ceiling-contained attenuation, Attenuate/Exact policies, typed denial reasons, pure decision function); RFC-0006 conformance declaration model (core coverage, §10 failure classification, profile subset coverage, canonicalized content-derived artifact identifiers with golden vectors, substitutions/omissions validation, §4 reporting view — manifests tested, tier claims, substitution table, forensics category counts, artifact citation via `artifact_is_cited`); RFC-0002 canonical SHA-3-512 addressing, conservative update semantics and instance semantics explicitly out of scope |
| Hardware Provider CTS | **Not implemented** | Fixtures declared in Hardware-Support-RFC; code not written |

---

## Key Implementation Milestones

| Milestone | Target Date | Status | Notes |
|-----------|-------------|--------|-------|
| Pre-Phase 1: RFC-ALLOC-001 drafted | 2026-06-09 | **Drafted** | Allocator hierarchy RFC drafted; pending review and acceptance |
| Pre-Phase 1: RFC-COMPILER-001 drafted | 2026-06-09 | **Drafted** | Compiler boundary RFC drafted; pending review and acceptance |
| Pre-Phase 1: RFC-DRIVER-001 drafted | 2026-06-09 | **Drafted** | Driver framework RFC drafted; pending review and acceptance |
| Pre-Phase 1: Three RFCs accepted | TBD | **Not started** | All three RFCs accepted as Active; Implementation Baseline v1.2 |
| Phase 1: RISC-V reference boot | TBD | **Not started** | Primary reference platform (was: Phase 3) |
| Phase 2: x86-64 + ARM64 ports | TBD | **Not started** | Deployment platforms |
| All 5 peer architectures running | TBD | **Not started** | After Phase 2 |
| Multi-architecture support (Phase 3) | TBD | **Not started** | After Phase 2 |
| Acceleration providers (Phase 4) | TBD | **Not started** | TPU, FPGA, DPU |
| Experimental quantum research interface (Phase 5) | TBD | **Not started** | RFC-Q1 not written; experimental only |
| All CTS fixtures passing | TBD | **Not started** | All phases |
| First conformant platform release | TBD | **Not started** | After Phase 4 |

---

## Last Updated

PR #7, PR #8, PR #9 and PR #10 are merged into `main`, bringing the indexed capability-revocation graph, RV64/Sv39 host mapping prototype, dispatch activation contract, per-access live capability revalidation, and the serialized memory runtime into the main branch. The ancestry-based `aether-cap-model` remains the semantic oracle. The uncommitted working tree additionally models RFC-0037.7 capability flags and enforces RFC-0037.14 scheduling-time expiry and CAPF_BOUND transfer refusal in both the oracle and the provider, with twenty-one new CTS cases (38 total, including the RFC-0011 scheduler, the RFC-0002/0004 composition models, the RFC-0016 conflict/snapshot cases, the RFC-0005 profile-selection cases, the RFC-0003 routing cases, and the RFC-0006 conformance declaration cases) and an expanded unit-test suite (137 workspace tests, including boundary tests for the strict RFC-0037.14 comparison). Independent orch-task-verifier reviews: capability slice PASS (one non-blocking CTS-oracle blind spot, F1, resolved); scheduler slice first verdict GATE FAIL on deadline-miss accounting (D1) plus a stranded-`Preempted` overflow path (D2), both fixed with regression tests and re-verified to ALL PASS; composition slice first verdict GATE FAIL on an in-degree node-key collision (D1) and an over-broad cycle-members payload (D2), both fixed (kind-keyed graph nodes; exact DFS cycle extraction) with regression tests and re-verified to ALL PASS; transaction slice verified to ALL PASS (RFC-0016.8 snapshot isolation, RFC-0016.9 write-write conflict abort; durable format unchanged); RFC-0005 profile selection verified to ALL PASS (mandatory/declared-optional activation, default-off extensions, forbidden-union combination with most-restrictive suppression, tier floors; RFC-0006.6 CTS substitution recorded; no boot-time or audit claim); RFC-0003 capability routing decision model verified to ALL PASS (ceiling-contained attenuation, typed denials, environmental independence by construction; no amplification path; CTS mutation-proven non-vacuous; namespace/event/token minting disclosed as gaps, not claimed); RFC-0006 composition conformance declaration model verified to ALL PASS (canonicalized content-derived artifact identifiers, golden vectors across all framing paths, full substitution/omission validation; RFC-0006 §§4–6 and RFC-0034 emission remain unimplemented by design); crash-consistency byte-granularity harness added to the state reference (PERSISTENCE-AND-EXTERNAL-EFFECTS.md §6) and verified ALL PASS across three verifier rounds (D6: terminal-effect contract pinned at `apply_effect` via a framed forged frame, mutation-checked; VersionRegressed covered by a firing test). `./check.sh` now passes end-to-end on Windows with Rust 1.95.0, exit 0, including the reproducibility gate (fixed by adding MSVC `/Brepro` for MINGW/MSYS/CYGWIN hosts in `tools/verify-reproducible-reference.sh`; the pre-existing failure reproduced identically on pristine `main` and was a wall-clock PE `TimeDateStamp`, not weakened verification). The changes are not committed, not merged and not CI-validated. The runtime's global blocking mutex is a host reference mechanism only; production dispatch must not hold a blocking lock across potentially waiting TLB-fence callbacks. There is still no production scheduler (a host-reference Layer 1 EDF state model now exists in `aether-sched-model`; it is not a production scheduler, has no SMP/Layer 2/3, and carries no bounded-latency proof), no hardware `satp`/page-table implementation, physical-frame allocator, or hardware qualification. Global mappings, ASID lifecycle, PMP/PMA/IOMMU, and formal proofs remain open. Global mappings, ASID lifecycle, PMP/PMA/IOMMU, and formal proofs remain open. UASA implementation results remain reported rather than independently reproduced.

To update this document: edit the relevant table entry to reflect the current status, then update the "Last Updated" date below.

**Last Updated:** 2026-11-19 (17-case main baseline after PR #7-#10; 38-case working tree adds RFC-0037.14/CAPF_BOUND enforcement, flags modeling, an RFC-0011 Layer 1 scheduler model, an RFC-0002/0004/0005/0006 composition model, an RFC-0003 routing decision model, RFC-0016.8/.9 snapshot isolation + write-write conflict detection, a PERSISTENCE-AND-EXTERNAL-EFFECTS.md §6 crash-consistency sweep, and a §4-steps-5-8 durable-intent outbox dispatcher (round-1 verifier found a disarmed crash injection and an unpinned reconcile path; both fixed and re-verified ALL PASS); full `./check.sh` gate green on Windows incl. reproducibility via `/Brepro`; uncommitted)
