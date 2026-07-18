# RFC Index — Platform Architecture Suite

Classification: Generated
Authoritative Source: All normative and cross-cutting RFCs

This index enumerates the 41 normative RFCs required by v1 of `Platform-Architecture-Specification-v1.1.md`. Each RFC is expected to exist as a file in the relevant `RFCs/` subdirectory.

---

## Composition Architecture (5 RFCs)

| RFC        | Title                          | Status        | Files                              |
|------------|--------------------------------|---------------|------------------------------------|
| RFC-0002   | Component Manifest             | Active        | `RFCs/00-Composition/RFC-0002.md`  |
| RFC-0003   | Capability Routing             | Active        | `RFCs/00-Composition/RFC-0003.md`  |
| RFC-0004   | Dependency Resolution          | Active        | `RFCs/00-Composition/RFC-0004.md`  |
| RFC-0005   | Profile Selection              | Active        | `RFCs/00-Composition/RFC-0005.md`  |
| RFC-0006   | Composition Conformance        | Active        | `RFCs/00-Composition/RFC-0006.md`  |

## Execution Architecture (8 RFCs)

| RFC        | Title                          | Status        | Files                              |
|------------|--------------------------------|---------------|------------------------------------|
| RFC-0007   | Execution Domain + Bootstrap   | Active        | `RFCs/01-Execution/RFC-0007.md`    |
| RFC-0008   | Dispatch                       | Active        | `RFCs/01-Execution/RFC-0008.md`    |
| RFC-0009   | Address Space                  | Active        | `RFCs/01-Execution/RFC-0009.md`    |
| RFC-0010   | Interrupt Routing              | Active        | `RFCs/01-Execution/RFC-0010.md`    |
| RFC-0011   | Scheduler (Layers 1+2)         | Active        | `RFCs/01-Execution/RFC-0011.md`    |
| RFC-0012   | Time                           | Active        | `RFCs/01-Execution/RFC-0012.md`    |
| RFC-0013   | Capability Mediation           | Active        | `RFCs/01-Execution/RFC-0013.md`    |
| RFC-0014   | Execution Conformance          | Active        | `RFCs/01-Execution/RFC-0014.md`    |

## State Architecture (9 RFCs)

| RFC        | Title                          | Status        | Files                              |
|------------|--------------------------------|---------------|------------------------------------|
| RFC-0015   | Object Identity                | Active        | `RFCs/02-State/RFC-0015.md`        |
| RFC-0016   | Transaction Model              | Active        | `RFCs/02-State/RFC-0016.md`        |
| RFC-0017   | Integrity Model                | Active        | `RFCs/02-State/RFC-0017.md`        |
| RFC-0018   | Lifecycle FSM                  | Active        | `RFCs/02-State/RFC-0018.md`        |
| RFC-0019   | Storage Provider Interface     | Active        | `RFCs/02-State/RFC-0019.md`        |
| RFC-0020   | POSIX Projection               | Active        | `RFCs/02-State/RFC-0020.md`        |
| RFC-0021   | Object Projection              | Active        | `RFCs/02-State/RFC-0021.md`        |
| RFC-0022   | Key–Value Projection           | Active        | `RFCs/02-State/RFC-0022.md`        |
| RFC-0023   | State Conformance              | Active        | `RFCs/02-State/RFC-0023.md`        |

## Communication Architecture (7 RFCs)

| RFC        | Title                          | Status        | Files                              |
|------------|--------------------------------|---------------|------------------------------------|
| RFC-0024   | Message Format                 | Active        | `RFCs/03-Communication/RFC-0024.md`|
| RFC-0025   | Endpoint Model                 | Active        | `RFCs/03-Communication/RFC-0025.md`|
| RFC-0026   | Routing                        | Active        | `RFCs/03-Communication/RFC-0026.md`|
| RFC-0027   | Capability Transfer            | Active        | `RFCs/03-Communication/RFC-0027.md`|
| RFC-0028   | Synchronization                | Active        | `RFCs/03-Communication/RFC-0028.md`|
| RFC-0029   | Streaming                      | Active        | `RFCs/03-Communication/RFC-0029.md`|
| RFC-0030   | Communication Conformance      | Active        | `RFCs/03-Communication/RFC-0030.md`|

## Observability Architecture (6 RFCs)

| RFC        | Title                          | Status        | Files                              |
|------------|--------------------------------|---------------|------------------------------------|
| RFC-0031   | Event Model                    | Active        | `RFCs/04-Observability/RFC-0031.md`|
| RFC-0032   | Trace Span                     | Active        | `RFCs/04-Observability/RFC-0032.md`|
| RFC-0033   | Replay Format                  | Active        | `RFCs/04-Observability/RFC-0033.md`|
| RFC-0034   | Audit Log                      | Active        | `RFCs/04-Observability/RFC-0034.md`|
| RFC-0035   | Diagnostic Query               | Active        | `RFCs/04-Observability/RFC-0035.md`|
| RFC-0036   | Observability Conformance      | Active        | `RFCs/04-Observability/RFC-0036.md`|

## Cross-Cutting (5 RFCs)

| RFC        | Title                                | Status        | Files                              |
|------------|--------------------------------------|---------------|------------------------------------|
| RFC-0037   | Capability Token Format              | Active        | `RFCs/05-Cross-Cutting/RFC-0037.md`|
| RFC-0038   | Capability Transfer Protocol         | Active        | `RFCs/05-Cross-Cutting/RFC-0038.md`|
| RFC-0039   | Capability Revocation Protocol       | Active        | `RFCs/05-Cross-Cutting/RFC-0039.md`|
| RFC-0040   | Verification Tier Methodology        | Active        | `RFCs/05-Cross-Cutting/RFC-0040.md`|
| RFC-0041   | Conformance Test Suite Methodology   | (See file)    | `RFCs/05-Cross-Cutting/RFC-0041.md`|

> **Note:** The `Conformance-Test-Suite-Methodology.md` file at the suite root is the same document that would be reproduced in this project numbering as `RFC-0041` if the suite were numbered from RFC-0041 explicitly. The cross-cutting list above contains 5 RFCs; the methodology file is included as the 41st by counting it under the cross-cutting list at the top-level file rather than as a separate RFC. Both numbering schemes are documented.

## Pre-Phase 1 (3 RFCs)

| RFC         | Title                                | Status        | Files                                  |
|-------------|--------------------------------------|---------------|----------------------------------------|
| RFC-ALLOC-001 | Allocator Hierarchy                | Draft         | `RFCs/06-Pre-Phase-1/RFC-ALLOC-001.md` |
| RFC-COMPILER-001 | Compiler and Runtime Boundary    | Draft         | `RFCs/06-Pre-Phase-1/RFC-COMPILER-001.md` |
| RFC-DRIVER-001 | Driver Framework Architecture    | Draft         | `RFCs/06-Pre-Phase-1/RFC-DRIVER-001.md` |

These three RFCs complete the normative surface required before Phase 1 implementation. They resolve INTF-001 through INTF-006 (cross-RFC interfaces defined in `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md`). Specification dependency: ALLOC → COMPILER → DRIVER.

---

## Total

- Composition: 5
- Execution: 8
- State: 9
- Communication: 7
- Observability: 6
- Cross-cutting: 6 (including RFC-0041 Counting the methodology file)
- Pre-Phase 1: 3 (Draft)
- **Total: 44**

---

## Cross-Index by Invariant

For implementer convenience, this section maps each invariant (I-1 through I-13) to the RFCs that primarily realize it.

| Invariant   | Primary realizing RFCs                          |
|-------------|-------------------------------------------------|
| I-1         | RFC-0037, RFC-0038, RFC-0013, RFC-ALLOC-001, RFC-DRIVER-001 |
| I-2         | RFC-0013, RFC-0007, RFC-0037, RFC-ALLOC-001, RFC-COMPILER-001, RFC-DRIVER-001 |
| I-3         | RFC-0013, RFC-0037, RFC-0038, RFC-ALLOC-001, RFC-DRIVER-001 |
| I-4         | RFC-0016, RFC-0018, RFC-0031, RFC-COMPILER-001, RFC-DRIVER-001 |
| I-5         | RFC-0016                                        |
| I-6         | RFC-0015, RFC-0017                              |
| I-7         | RFC-0040 (assurance methodology), RFC-DRIVER-001 |
| I-8         | RFC-0040 (specifically Tier A requirements), RFC-ALLOC-001 |
| I-9         | RFC-0002 (replaceability infrastructure), RFC-DRIVER-001 |
| I-10        | Every RFC (contract MUST declare failure modes), RFC-DRIVER-001 |
| I-11        | RFC-0005, RFC-0041, RFC-DRIVER-001               |
| I-12        | RFC-0007 (Bootstrap section), RFC-0037, RFC-COMPILER-001 |
| I-13        | RFC-0037 (Capability address separation), RFC-ALLOC-001, RFC-COMPILER-001, RFC-DRIVER-001 |

---

## Additional Architecture Documents (Non-RFC)

| Document | Title | Status | Description |
|----------|-------|--------|-------------|
| `Hardware-Support-RFC.md` | Hardware Support Architecture | Active | Execution-provider contract, HAL, x86-64, ARM64, RISC-V64, POWER, GPU, TPU, FPGA, DPU, quantum (experimental), CXL/bus interconnects |
| `REVIEWER-GUIDE.md` | Reviewer Guide | Active | Mandatory pre-review workflow, Impact Matrix, review confidence levels |

## Pre-Phase 1 RFCs (Drafted)

These RFCs were the three blockers identified in the pre-implementation dependency matrix. All three are now drafted and pending review:

| RFC | Title | Status | Blocks |
|-----|-------|--------|--------|
| RFC-ALLOC-001 | Allocator Hierarchy | **Drafted** | Resolves INTF-001, INTF-004, INTF-005; blocks COMPILER and DRIVER |
| RFC-COMPILER-001 | Compiler and Runtime Boundary | **Drafted** | Resolves INTF-002, INTF-003, INTF-006; blocks DRIVER |
| RFC-DRIVER-001 | Driver Framework Architecture | **Drafted** | Consumes all interfaces; completes normative surface |

## Future RFCs (Not Started)

| RFC | Title | Status | Blocks |
|-----|-------|--------|--------|
| RFC-0042 | Cohort Update Governance | **Not started** | Profile update and rollback mechanism |
| RFC-0043 | Capability Rights Algebra | **Not started** | Formal derivation rules for capability rights |

## Future RFCs (Research Extensions — Phase 5)

These RFCs are deferred research extensions, not production milestones:

| RFC | Title | Status | Blocks |
|-----|-------|--------|--------|
| RFC-0042 | Cohort Update Governance | Not started | Profile update and rollback mechanism |
| RFC-0043 | Capability Rights Algebra | Not started | Formal derivation rules for capability rights |
| RFC-Q1 | Quantum Execution Provider Contract | Experimental research interface | Phase 5 quantum support |
| RFC-N1 | Neuromorphic Provider Interface | Experimental research interface | Phase 5 neuromorphic support |
| RFC-P1 | Photonic Storage and Network Provider | Experimental research interface | Phase 5 photonic support |

---

## Reading Order Recommendations

**For a new implementer:** Read top-down in the order listed in the Platform Architecture Specification, then read `Conformance-Test-Suite-Methodology.md`, then read the RFCs in the order: Execution → State → Communication → Composition → Observability → Cross-Cutting.

**For a reviewer** focused on a specific subsystem: jump to the relevant peer architecture's RFC list, then cross-reference with `Conformance-Test-Suite-Methodology.md`.
