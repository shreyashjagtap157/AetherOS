# Reviewer Guide — Ideal Computing Platform

Classification: Operational
Authoritative Source: REVIEWER-GUIDE.md
Requirement-ID: GOV-REV-001

**Status:** Active.
**Applies to:** All architectural reviews of this specification suite.
**Source:** Review workflow principles established during architectural review of v1.1.

---

## 1. Purpose

This guide governs how reviewers approach the Ideal Computing Platform specification suite. It exists because architectural documents rarely contain sufficient context for an external reviewer to distinguish intentional design decisions from errors — and because a review process that skips context-building produces false positives that cost more to resolve than they prevent.

The key principle is:

> **Understand before proposing. Reconstruct before criticizing.**

---

## 1A. Review Entry Criterion

**A review is not considered to have begun until the reviewer has produced a Reconstruction Artifact covering, in writing:**

1. **System Guarantees** — The invariants this architecture enforces and the behaviors the system promises at the public boundaries.

2. **Mechanisms** — The mechanisms by which those guarantees are produced, and how each mechanism contributes to which guarantee.

3. **Specifications** — The RFCs that constitute the current frozen architecture, and the version of the architecture currently in force.

4. **Evidence** — What proof, conformance, or review evidence currently supports each guarantee. What has not yet been verified.

5. **Known Uncertainties** — What the reviewer has not been able to reconstruct. Substantive disagreements between source documents.

6. **Falsifiability** — What would have to be observed for this reconstruction to be wrong.

7. **Confidence** — The reviewer's confidence level in their reconstruction (High, Medium, or Low), plus a one-paragraph justification.

Critiques submitted before this artifact is complete are **invalid** and must be reoriented.

### Reconstruction Confidence Statement (form)

```
Architecture Version: <v1.1 or later>
RFCs Considered: <RFCs read for this review>
Guarantees Enumerated: <count>
Mechanisms Enumerated: <count>
Reconstruction Confidence: <High | Medium | Low>
Justification: <one paragraph>
Falsifiability: <what would invalidate this reconstruction>
Known Uncertainties: <explicit inventory>

Reconstructed Areas:
- <list>

Areas Intentionally Not Reconstructed:
- <list, with one-line justification for each>
```

**Scope Boundary**: The reviewer explicitly states which repository areas were intentionally not reconstructed. This prevents a reviewer from presenting an incomplete reconstruction as a complete one. A reviewer who has not read the governance RFCs, cross-cutting RFCs, or observability RFCs shall declare so before completing critique.

---

## 2. Reviewer Workflow

Every architectural review SHALL follow this sequence:

```
Step 1 — Understand
    Obtain: directory structure, document hierarchy, RFC index, implementation status

Step 2 — Reconstruct
    Build an internal architectural model from the documentation

Step 3 — Validate
    Confirm the reconstructed model with the project maintainer
    ONLY THEN: begin identifying defects

Step 4 — Review
    Propose changes against the validated model

Step 5 — Impact Assessment
    For every accepted change: identify affected files, RFCs, conformance tests

Step 6 — Specification Update
    Documentation updates are part of the architectural change, not cleanup

Step 7 — Conformance Review
    Re-run conformance implications after updates
```

Skipping Step 3 (validation) before Step 4 (review) produces review findings that are architecturally invalid because they were produced against a potentially incorrect understanding.

---

## 3. Pre-Review Information Gathering

Before proposing any modification, the reviewer SHALL obtain and internalize the following:

### 3.1 Repository Structure

```
docs/
├── README.md                    # Entry point and document map
├── CHANGELOG.md               # Change history
├── Platform-Architecture-Specification-v1.1.md  # Frozen top-level architecture
├── Governance-RFC.md           # Amendment process
├── Profiles-Specification.md   # Deployment profiles
├── Conformance-Test-Suite-Methodology.md  # CTS methodology
├── Glossary.md                 # Terminology
├── 00-RFC-Index.md             # Master RFC index
├── Revision-History.md          # Architecture revision log
├── Hardware-Support-RFC.md      # Hardware provider architecture
├── IMPLEMENTATION-ROADMAP.md   # Phased implementation plan
├── IMPLEMENTATION-STATUS.md    # Live status tracker
├── REVIEWER-GUIDE.md           # This document
└── RFCs/
    ├── 00-Composition/         # RFC-0002 through RFC-0006
    ├── 01-Execution/            # RFC-0007 through RFC-0014
    ├── 02-State/               # RFC-0015 through RFC-0023
    ├── 03-Communication/       # RFC-0024 through RFC-0030
    ├── 04-Observability/       # RFC-0031 through RFC-0036
    └── 05-Cross-Cutting/       # RFC-0037 through RFC-0041
```

### 3.2 Current Frozen Status

| Item | Version | Date | Status |
|------|---------|------|--------|
| Architecture | v1.1 | 2026-06-09 | Frozen — requires Architecture Amendment RFC to change |
| RFCs | 41 normative | 2026-06-09 | Written; no RFC is deferred to future specification |
| CTS Methodology | Active | 2026-06-09 | Active specification |

### 3.3 Known Intentional Trade-offs

The following are deliberate architectural decisions, not defects:

| Decision | Rationale | RFC Reference |
|----------|-----------|---------------|
| v1 = single execution domain; distributed defer to v2 | Distributed semantics too underspecified to frozen in v1 | RFC-0007, RFC-0026 |
| Merkle + CoW persistence core; capability-aware storage as layered extension | Operational evidence strongly favors block-based integrity over capability-graph for large datasets | RFC-0017, RFC-0015 |
| Three-layer scheduler with ML never on hot path | Formal verification and real-time guarantees require deterministic hot path | RFC-0011 |
| GPU provider = substrate-neutral execution interface; CUDA/HIP/SYCL = compatibility layers | Prevents provider contract from becoming CUDA-shaped | Hardware-Support-RFC.md |
| Quantum = experimental research interface, not production milestone | Stable quantum OS abstraction unknown; extension point only | Hardware-Support-RFC.md §10 |
| RISC-V64 = reference platform; x86-64/ARM64 = deployment platforms | Cleaner ISA + better formal tooling; reference before deployment | IMPLEMENTATION-ROADMAP.md |
| Formal proof invariants written before implementation; complete proofs after stabilization | Unstable code proof is expensive; retrofitting proofs is more expensive | RFC-0040, IMPLEMENTATION-ROADMAP.md |
| Driver framework, allocator hierarchy, compiler boundary: RFCs required before Phase 1 | These are where mature kernels spend majority of engineering budget; omit now = crisis later | IMPLEMENTATION-ROADMAP.md |

### 3.4 Known Implementation Deviations

| Item | Deviation | Severity | Tracking |
|------|-----------|----------|---------|
| No HAL implementations yet | Phase 1 not started | Blocking | IMPLEMENTATION-STATUS.md |
| No CTS executable code | Fixtures declared in RFCs; no implementation | Blocking | IMPLEMENTATION-STATUS.md |
| RFC-0042 (Cohort Update Governance) | Not yet written | Future | IMPLEMENTATION-STATUS.md |
| RFC-0043 (Capability Rights Algebra) | Not yet written | Future | IMPLEMENTATION-STATUS.md |
| Driver Framework RFC | Not yet written; required before Phase 1 | Blocking | IMPLEMENTATION-STATUS.md |
| Allocator Hierarchy RFC | Not yet written; required before Phase 1 | Blocking | IMPLEMENTATION-STATUS.md |
| Compiler Boundary RFC | Not yet written; required before Phase 1 | Blocking | IMPLEMENTATION-STATUS.md |

### 3.5 Conformance Status

No implementation exists yet. All conformance declarations are pending. The CTS is declared but unimplemented. Until an implementation exists, there are no conformance violations — only missing implementations.

---

## 4. Impact Matrix — Required for Every Review Finding

Every architectural review finding MUST conclude with an Impact Matrix. This is not optional.

| Category | Result |
|----------|--------|
| **Architecture** | No Change / Amendment Required |
| **RFCs Affected** | List all RFC numbers that require modification |
| **Implementation Impact** | None / Minor / Major |
| **Conformance Tests** | Update Required / No Change |
| **Migration Needed** | Yes / No |
| **Breaking Change** | Yes / No |
| **Review Confidence** | High / Medium / Low |

The Impact Matrix forces every observation to resolve into concrete engineering consequences rather than abstract opinion.

---

## 5. File Change Requirements

For every accepted review finding, the reviewer SHALL provide:

1. **Affected document(s):** Exact file path(s)
2. **Section(s) requiring modification:** Specific RFC number and section
3. **Rationale:** Why this change improves the architecture
4. **Normative vs. editorial:** Whether the change affects conformance requirements
5. **Dependent RFCs:** Any RFC that depends on the changed section
6. **Conformance test updates:** Whether CTS fixtures must change

Documentation updates are part of the architectural change, not separate cleanup. A review finding without an Impact Matrix is incomplete and SHALL NOT be merged.

---

## 6. What Requires Architecture Amendment vs. Specification Amendment

### Architecture Amendment (High Ceremony — See Governance-RFC.md)

Changes to:
- Section 2 invariants (I-1 through I-13) of `Platform-Architecture-Specification-v1.1.md`
- Section 3 peer decomposition
- Section 5 bootstrap model
- Any change that could invalidate a conforming v1.1 implementation

Process: 30-day freeze, signed RFC, explicit enumeration of affected invariants.

### Specification Amendment (Standard — See Governance-RFC.md)

Changes to any individual RFC:
- Must be backward-compatible or include migration specification
- Requires conformance test updates if behavioral requirements change
- Version increment in the RFC's own revision history

### Editorial Amendment (Minimal — See Governance-RFC.md)

Reformatting, example rewording, cross-reference corrections. No version increment of the top-level architecture document. No RFC revision.

---

## 7. Review Confidence Levels

| Level | Definition |
|-------|------------|
| **High** | Reviewer has read all affected RFCs in full, reconstructed the architecture, validated with maintainer, and has formal verification or systems research background relevant to the finding |
| **Medium** | Reviewer has read all affected RFCs, reconstructed the architecture, but has not validated with maintainer or lacks direct implementation experience with the specific subsystem |
| **Low** | Reviewer has read only the affected section, not the surrounding RFCs or architecture context. Findings at this level should be treated as questions rather than prescriptions. |

Low-confidence findings that read as definitive prescriptions are the most dangerous kind of architectural review. They appear confident but lack grounding.

---

## 8. Resolving Disagreement

When a reviewer and maintainer disagree:

1. The maintainer SHALL produce a written response explaining the intentional trade-off if the finding represents a deliberate design decision.
2. If the reviewer believes the finding represents a genuine architectural defect, both parties SHALL produce a written position document.
3. If the disagreement persists after written exchange, a third reviewer from the project's advisory list SHALL be consulted.
4. The written record of the disagreement and its resolution SHALL be archived in the repository as a design decision record.

---

## 9. Annual Review Cycle

Once per calendar year, all active RFCs SHALL be reviewed against:
- Current implementation experience (if any exists)
- Any new hardware platforms or usage patterns not anticipated
- Whether the implementation status tracker remains accurate
- Whether the known intentional trade-offs remain justified
- Whether the known implementation deviations have been resolved

Results of annual review SHALL be recorded in the CHANGELOG.md.

---

## 10. Sources

- Fuchsia RFC best practices — architecture documentation maintenance
- seL4 verification methodology — proof-driven design and proof maintenance
- RFC 7322 (RFC Style Guide) — specification writing standards
- ISO/IEC/IEEE 42030 (Architecture Evaluation Framework) — architectural evaluation methodology
- Practitioner discussions on architecture documentation synchronization

---

## Revision History

| Date       | Change |
|------------|--------|
| this cycle | Initial release. Reviewer workflow established. Pre-review information gathering requirements defined. Impact Matrix mandatory for all findings. |