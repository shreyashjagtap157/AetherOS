# Architecture Governance — ARCHITECTURE-GOVERNANCE.md

Classification: Constitutional
Authoritative Source: ARCHITECTURE-GOVERNANCE.md
Requirement-ID: GOV-MON-001, GOV-COMP-001, GOV-TRAC-001, GOV-DET-001

**Governance Baseline v1.0**
**Status:** Frozen — Evolution by Amendment Only

**Purpose:** The constitutional document of the Ideal Computing Platform specification suite. Defines how the architecture evolves, who may approve changes, what categories of change exist, and what the canonical source of truth is.
**Source:** Derived from Governance-RFC.md, REVIEWER-GUIDE.md, and architectural review consensus.

---

## 1. Purpose and Scope

This document is the supreme governance authority for the repository. It takes precedence over all other governance documents including `Governance-RFC.md` in matters of process classification and authority.

It answers:
- Who can approve changes?
- What qualifies as which category of change?
- What is the canonical source of truth?
- When does implementation override documentation?
- When does documentation override implementation?
- How are RFCs superseded, deprecated, or replaced?

---

## 2. The Four Governance Categories

Every proposed change to the repository falls into exactly one of these four categories. The category determines the process, the authority required, and the review ceremony.

### 2.1 Category: Architecture Amendment

**Definition:** A change that modifies the frozen architectural invariants (Section 2 of `Platform-Architecture-Specification-v1.1.md`), the five-peer decomposition (Section 3), the bootstrap model (Section 5), or any invariant that could invalidate a conformant v1.1 implementation.

**Examples:**
- Adding a new architectural invariant
- Removing an existing invariant
- Changing the definition of an existing invariant
- Adding or removing a peer architecture
- Changing the Critical-Path Admission Test criteria
- Changing the bootstrap stage order

**Required authority:** Signed Architecture Amendment RFC, 30-day review freeze, explicit approval from at least two reviewers with High confidence classification.

**Process:**
1. Proposer writes Architecture Amendment RFC citing the specific invariant(s) affected
2. 30-day review period opens; all reviewers notified
3. Each reviewer produces an Impact Matrix (REVIEWER-GUIDE.md Section 4)
4. Proposer responds to every finding
5. If two High-confidence reviewers approve with no unresolved Architecture findings, the amendment is accepted
6. On acceptance: update Platform-Architecture-Specification-v1.1.md, increment version (v1.1 → v1.2), update Revision-History.md

**Version bump:** Minor (v1.1 → v1.2) for invariants; Major (v2.0) only if the change invalidates existing conformant implementations.

---

### 2.2 Category: Specification Amendment

**Definition:** A change to an individual RFC that affects its normative requirements, data structures, protocol behavior, or failure semantics. Does not affect architecture invariants.

**Examples:**
- Adding a new MUST requirement to an existing RFC
- Modifying a data structure layout
- Changing a protocol sequence
- Adding a new failure mode to an existing table
- Adding an optional feature to an existing RFC

**Required authority:** The RFC's author(s) + one peer reviewer.

**Process:**
1. Proposer writes the change with a clear rationale and Impact Matrix
2. Reviewer validates the Impact Matrix and confirms no architecture invariants are affected
3. If the change affects conformance tests, CTS fixtures are updated accordingly
4. On acceptance: update the RFC, increment the RFC's revision number, update Revision-History.md

**Version bump:** No version bump to the platform architecture document. The RFC's own revision number increments.

---

### 2.3 Category: Editorial Amendment

**Definition:** A change that improves clarity, corrects cross-references, reformats, fixes typos, or clarifies existing requirements without altering any normative requirement.

**Examples:**
- Rewording a requirement sentence for clarity without changing intent
- Fixing a broken cross-reference
- Reformatting a data structure for readability
- Adding an example that was previously missing
- Updating a citation to the same source
- Correcting a typo

**Required authority:** Any contributor; no formal review required.

**Process:**
1. Proposer makes the change directly
2. No version bump to any document
3. Change is noted in the document's inline revision history block

**Important:** If a reviewer or author cannot agree on whether a change is editorial or specification, the dispute resolves to the higher category (Specification Amendment).

---

### 2.4 Category: Governance Amendment

**Definition:** A change to this document, `Governance-RFC.md`, or the REVIEWER-GUIDE.md.

**Examples:**
- Changing the Architecture Amendment process
- Adding a new governance category
- Modifying the Reviewer Guide workflow

**Required authority:** Same as Architecture Amendment (30-day freeze, two High-confidence approvals).

**Rationale:** Governance changes affect how all future decisions are made. They deserve the highest scrutiny.

---

## 3. Source of Truth

### 3.1 The Canonical Source

The canonical source of truth for this repository is the repository itself. Git is the authoritative record. No external document, email, chat thread, or personal communication constitutes a valid architectural decision unless it is recorded in the repository.

This means:
- Architecture Amendment RFCs must be merged into the repository to be valid
- Decision records must be in the Decision Record appendix of the relevant RFC (see RFC-GOV-002)
- A cross-cutting decision index is maintained at `generated/DECISION-INDEX.md` (generated, not authoritative)
- RFC status changes must be recorded in `IMPLEMENTATION-STATUS.md`
- Version bumps must be reflected in `Revision-History.md`

### 3.2 Documentation vs. Implementation Priority

In most cases, documentation and implementation should converge. When they diverge:

| Situation | Priority | Reason |
|-----------|----------|--------|
| Implementation violates an architectural invariant | **Documentation** wins | Invariants are normative; violations are non-conformant |
| Implementation extends an RFC without changing requirements | **Implementation** wins | RFCs describe minimums, not maximums |
| Documentation specifies behavior that implementation cannot achieve | **Implementation** wins; documentation must be corrected | Implementation feasibility is a forcing function for specification accuracy |
| Implementation is missing a required feature | **Documentation** wins | Missing features cannot be waived without an Architecture Amendment |
| Dispute over whether a change is editorial or specification | **Escalate to Specification Amendment** | The higher category always applies |

---

## 4. RFC Lifecycle

### 4.1 RFC States

Each RFC is in exactly one of these states:

| State | Meaning |
|-------|---------|
| **Active** | Current normative specification; may be amended |
| **Deprecated** | Superseded by a later RFC; included for reference; not to be implemented anew |
| **Superseded** | Replaced by a specific later RFC; included for historical record |
| **Experimental** | Research extension; not for production use |
| **Removed** | Deleted from the repository; not to be referenced |

### 4.2 RFC Supersession

An RFC supersedes another when the newer RFC satisfies the same architectural requirement more completely. The old RFC is marked **Superseded**, not deleted. Supersession requires:
- The new RFC covers all functionality of the old RFC
- The new RFC explicitly names the RFC it supersedes
- The change is a Specification Amendment (not Architecture Amendment)
- The old RFC's revision history notes the supersession

### 4.3 RFC Removal

An RFC is **Removed** only by an Architecture Amendment when an architectural invariant is deleted. Removed RFCs are archived in a `/archive/` directory with a note on why they were removed.

---

## 5. Version Numbering

### 5.1 Architecture Version

The architecture version follows Semantic Versioning (MAJOR.MINOR):
- **MAJOR** increment: An Architecture Amendment that invalidates existing conformant implementations
- **MINOR** increment: An Architecture Amendment that adds to or refines existing invariants without invalidating conformant implementations

Current version: **v1.1**

### 5.2 RFC Revision

Each RFC carries its own revision number, independent of the architecture version. RFC revisions increment on each Specification Amendment. RFC revisions reset to 1 when an RFC is first written.

Example: `RFC-0037.md` is at revision 3 (three Specification Amendments have been made since it was first written).

### 5.3 CTS Version

The CTS version is separate. CTS version bumps with Architecture Amendments. Bug-fix CTS changes between Architecture Amendments do not bump the version.

---

## 6. Consensus and Disagreement Resolution

### 6.1 What Constitutes Consensus

Consensus on an Architecture Amendment requires:
- At least two High-confidence reviewers
- No unresolved Architecture-category findings from any reviewer
- No unresolved Governance-category findings from any reviewer

Consensus on a Specification Amendment requires:
- At least one reviewer (any confidence level)
- No unresolved Specification or Architecture findings

Consensus on an Editorial Amendment requires:
- No requirement for review; any contributor may make the change

### 6.2 Disagreement Resolution

When a reviewer and author disagree and cannot resolve it through written exchange:

1. Both parties write a formal position document (max 500 words each)
2. A third reviewer from the project's advisory list is consulted
3. The advisory reviewer renders a binding decision with a written rationale
4. The decision and rationale are recorded in the relevant RFC's Decision Record appendix

### 6.3 Advisory Reviewers

The project maintainer maintains a list of at least three named advisory reviewers who are competent to adjudicate Architecture Amendment disputes. Advisory reviewers must have:
- Formal systems research or engineering background
- No direct involvement in the disputed change
- Demonstrated expertise in the relevant subsystem

---

## 7. Implementation Override of Documentation

In rare cases, an implementation discovery may show that documentation is wrong. In such cases:

1. The implementation team opens an **Observation Record** (see `OBSERVATION-RECORD.md`)
2. The discrepancy is classified: implementation is incorrect, or documentation is incorrect
3. If documentation is incorrect: the relevant RFC is amended via Specification Amendment within 30 days
4. If implementation is incorrect: implementation is corrected before the next release
5. In neither case may implementation remain in violation of an architectural invariant

---

## 8. Review Process Requirements

Every review must classify every finding into one of these nine categories:

| Category | Applies To | Routes To |
|----------|-----------|----------|
| **Editorial** | Any document | Author; resolved as editorial |
| **Specification** | Individual RFCs | RFC author; Specification Amendment process |
| **Architecture** | Platform-Architecture-Specification-v1.1.md invariants only | Architecture Amendment process |
| **Implementation** | Code (HAL, CTS, services) | Implementation team |
| **Conformance** | CTS fixtures or methodology | CTS revision process |
| **Performance** | Performance requirements or benchmarks | Performance engineering |
| **Security** | Security properties | Security review board |
| **Verification** | Formal proofs or CTS tier claims | Verification team |
| **Governance** | This document, Governance-RFC.md, REVIEWER-GUIDE.md | Governance Amendment process |

Only **Architecture** findings may modify the frozen architecture. All other categories route to their respective processes.

---

## 9. Reviewer Qualifications

| Confidence Level | Requirements |
|-----------------|--------------|
| **High** | Has read all affected RFCs in full; has validated reconstruction with maintainer; has formal verification or systems research background in the relevant domain |
| **Medium** | Has read all affected RFCs; has reconstructed the architecture; has not validated with maintainer or lacks direct implementation experience |
| **Low** | Has read only the affected section; findings at this level are treated as questions, not prescriptions |

Findings at Low confidence that read as definitive prescriptions are the most dangerous output of architectural review.

---

## 10. Required Repository Artifacts

Every architectural decision must be recorded in the repository. The following files are mandatory and must be kept current:

| File | Purpose | Updated By |
|------|---------|-----------|
| `Platform-Architecture-Specification-v1.1.md` | Validation-candidate constitutional baseline | Architecture Amendment only |
| `Governance-RFC.md` | Amendment process | Governance Amendment |
| `REVIEWER-GUIDE.md` | Review process | Governance Amendment |
| `ARCHITECTURE-GOVERNANCE.md` | This document | Governance Amendment |
| Per-RFC Decision Record (`RFC-XXXX.md` appendix) | Decision records | RFC author on amendment |
| `IMPLEMENTATION-STATUS.md` | Live implementation status | Every accepted change |
| `CHANGELOG.md` | Change history | Every accepted change |
| `00-RFC-Index.md` | RFC registry | Every new or changed RFC |
| `[RFC]/RFC-XXXX.md` | Individual RFCs | Specification Amendment |

---

## 11. Review Session Structure

Every formal review session (Architecture Amendment or Governance Amendment) follows this structure:

```
1. Pre-review: reviewer reconstructs architecture and validates with author (REVIEWER-GUIDE.md Section 3)
2. Review: reviewer identifies findings by category
3. Classification: each finding assigned to a category (Section 8)
4. Impact Matrix: completed for every finding (REVIEWER-GUIDE.md Section 4)
5. Author response: author responds to every finding
6. Resolution: unresolved findings escalated per Section 6.2
7. Acceptance: if consensus requirements met (Section 6.1), change accepted
8. Repository update: author updates affected files per Impact Matrix
9. Decision record: Decision Record appended to relevant RFC
10. Notification: all reviewers notified of resolution
```

---

## 12. Document Classification and Authority Hierarchy

### 12.1 Classification Levels

Every document in this repository carries a `Classification:` header declaring its position in the authority hierarchy:

| Level | Value | Meaning |
|-------|-------|---------|
| 1 | Constitutional | Defines invariants and authority rules. Governed by Architecture Amendment process. |
| 2 | Normative | Defines specific behaviors, APIs, or requirements. Governed by Specification Amendment process. |
| 3 | Operational | Defines practices, workflows, or operational policy. Governed by Governance Amendment process. |
| 4 | Generated | Derived from Constitutional or Normative documents. Cannot be edited directly. |
| 5 | Informative | Explanatory material that does not constrain design. No governance weight. |

### 12.2 Authority Monotonicity Principle

**Requirement-ID: GOV-MON-001**

No document may derive authority from another document of lower classification. Constitutional documents may reference Operational documents for examples but never derive rules from them. Generated documents may never become authoritative through citation or repeated use.

> Monotonicity prevents lower-class documents from overriding higher-class documents.

### 12.3 Authority Completeness Principle

**Requirement-ID: GOV-COMP-001**

Every normative requirement has exactly one authoritative source. References may duplicate navigation but shall not duplicate authority. If two documents appear to define the same normative requirement, the repository is considered inconsistent until the conflict is resolved.

> Completeness prevents duplicate authorities from coexisting on the same normative requirement.

### 12.4 Authority Traceability Principle

**Requirement-ID: GOV-TRAC-001, GOV-DET-001**

Every normative requirement SHALL possess exactly one authoritative definition and SHALL be mechanically traceable from implementation, verification artifacts, generated documentation, and amendment history. Every generated artifact identifies the authoritative source from which each normative statement originated, lists the generator, generation time, generator version, source revision, and the complete set of source files that participated in generation. If traceability cannot be established automatically, the repository is considered inconsistent until repaired.

> Traceability defines the complete trace graph: from implementation and verification artifacts, through generated documentation, back to the authoritative definition.

### 12.5 Repository Determinism Principle

**Requirement-ID: GOV-DET-001**

Given identical repository contents and identical tool versions, every validator and every generator SHALL produce identical outputs. This requires:

- Deterministic ordering of output elements.
- Deterministic handling of timestamps (or explicit null/placeholder values where timing is unreproducible).
- Stable, idempotent formatting.
- Reproducible CI: identical inputs produce identical pass/fail results across runs.

Noise in generated artifacts causes ignored diffs. Ignored diffs hide real architectural drift. Repository Determinism is the property that prevents noise from becoming architecture.

### 12.6 Document Header Requirement

Every document in this repository MUST begin with the following header:

```
Classification: <level>
Authoritative Source: <document or generated source>
Requirement-ID: <namespace>-<slug>-<number>   # for Constitutional and Normative only
Requirement-Version: <integer>                # increments on substantive change
Requirement-State: Active | Deprecated | Reserved
```

Generated artifacts MUST additionally carry:

```
Generated-By: <tool path>
Generated-At: <ISO-8601 timestamp>
Generator-Version: <version>
Source-Revision: <git ref>
Generation-Inputs:
  - <source file 1>
  - <source file 2>
```

`Requirement-State: Active` is the default for all newly authored requirements. `Deprecated` marks a requirement that is superseded but retained for traceability. `Reserved` marks a requirement ID allocated but not yet populated.

---

## 13. Evidence-Based Amendment Principle (Proposed — RFC-GOV-002)

> The Evidence-Based Amendment Principle is currently in RFC-GOV-002 (Draft status). Upon acceptance of RFC-GOV-002, the following text becomes constitutional and this notice is removed.

### 13.1 Admissible Evidence Classes

Architectural amendments are admitted only when supported by at least one admissible constitutional evidence class:

- **Conformance evidence**: A CTS test demonstrates the current specification is internally unsatisfiable.
- **Verification evidence**: A formal proof demonstrates the current invariants cannot simultaneously hold under the current RFC set.
- **Security evidence**: A validated threat model identifies an unenforced architectural requirement.

This list is the current constitutional evidence taxonomy. It may be extended through ordinary constitutional amendment.

### 13.2 Architecture Review Trigger

Implementation and operational evidence accumulated in `OBSERVATION-RECORD.md` and meeting the Observation Record schema's evidence standards may, at the discretion of the Governance body, trigger an Architecture Review. The Review runs under ordinary amendment procedure. The observation's authority is solely the right to convene the Review.

### 13.3 No Shortcut Clause

Operational evidence does not satisfy the Evidence-Based Amendment Principle on its own. An amendment justified solely on implementation cost, implementation inconvenience, runtime performance regression, scheduled RFC maintenance burden, or under-specification of edge cases — without independent Conformance, Verification, or Security evidence — is not admissible and SHALL be rejected at the gate.

When such an amendment is rejected, the proper response is either:
- An **Implementation Strategy Amendment** under the existing governance rules, or
- An **Observation Record** with explicit `Resolution Target` pointing to the relevant domain.

### 13.4 Observation Record Definition

Observation Records are an operational artifact type defined by `OBSERVATION-RECORD.md`. Observation Records possess no amendment authority. Their sole formal consequence is the procedural right to convene an Architecture Review under §13.2.

---

## 14. Governance Telemetry

Governance Baseline v1.0 is instrumented to observe its own effectiveness. The following metrics are maintained by the Governance body and reviewed quarterly.

| Metric | Purpose |
|--------|---------|
| Observation Records created per quarter | Whether governance is seeing reality — if zero, the observation pipeline has failed |
| Architecture Reviews triggered per quarter | Whether the review trigger is calibrated — if zero after 12 months, the threshold is too high |
| Amendments admitted vs. rejected at gate | Whether the amendment gate is calibrated — if >50% rejection rate, gate is too strict; if 0% rejection, it may be too permissive |
| CI validator false-positive rate | Whether tooling produces friction without value — high false-positive rate means validators need tuning |
| Average RFC lifecycle (draft → accepted) | Whether governance is slowing engineering excessively — >6 months per RFC is a process concern |
| Decision Record population rate | Whether the Decision Record requirement is being maintained — CI monitors, governance resolves |
| Requirement-ID collision count | Whether the Completeness principle is being violated — any collision is a governance failure |

Governance telemetry is informational. It does not authorize amendments, bypass gates, or override constitutional rules. It exists solely to make governance effectiveness observable.

---

## Revision History

| Date       | Change |
|------------|--------|
| this cycle | Initial release. Supersedes Governance-RFC.md as the supreme governance authority for this repository. |