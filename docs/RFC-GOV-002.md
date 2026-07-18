# RFC-GOV-002: Evidence-Based Amendment Principle and Decision Record Requirement

Classification: Normative
Authoritative Source: RFC-GOV-002
Requirement-ID: GOV-EBP-001

**Governance Baseline v1.0**
**Status:** Frozen
**Evolution:** Amendment only

**Status:** Draft

---

## Abstract

This RFC specifies the Evidence-Based Amendment Principle that governs when architectural amendments to the frozen OS architecture may be admitted. It also specifies the structural requirement for per-RFC Decision Record appendices and the Observation Record lifecycle.

---

## 1. Decision Record Requirement

### 1.1 Scope

Every normative RFC MUST include a Decision Record appendix before it may transition to `Status: Accepted`.

### 1.2 Schema

```yaml
Decision Record

Status: Accepted
Problem:
Alternatives Considered:
Why Rejected:
Accepted Decision:
Architectural Consequences:
Verification Consequences:
Implementation Consequences:
CTS Impact:
Supersedes:
Superseded By:
```

### 1.3 Acceptance Constraint

An RFC moves from `Draft` to `Accepted` status only when its Decision Record appendix is populated. The CI gate `tools/verify-decision-records.py` enforces this constraint.

### 1.4 Migration Plan

All existing normative RFCs SHALL be amended to include a populated Decision Record as a single batch amendment under v1.1 → v1.2, not as forty-one separate amendments.

---

## 2. Evidence-Based Amendment Principle

### 2.1 Admissible Evidence Classes

Architectural amendments are admitted only when supported by at least one admissible constitutional evidence class:

- **Conformance evidence**
  Definition: A CTS test demonstrates the current specification is internally unsatisfiable. The test is in the project's CTS repository, executed in CI, and shows a documented failure mode.

- **Verification evidence**
  Definition: A formal proof demonstrates the current invariants cannot simultaneously hold under the current RFC set. The proof is reproducible from public artifacts and survives version control.

- **Security evidence**
  Definition: A validated threat model surfaces an unenforced architectural requirement. The threat model is recorded in a reviewable artifact and the gap between the threat model and the current specification is explicit.

### 2.2 Openness of the Evidence Taxonomy

The list in §2.1 is the current constitutional evidence taxonomy. The taxonomy may be amended through ordinary constitutional amendment process. Amendments MAY introduce new evidence classes. The list itself does not require amendment merely to be extended.

### 2.3 Architecture Review Trigger

A documented Observation Record with disposition `Forwarded to Governance` MAY, at the discretion of the Governance body, trigger an Architecture Review. The Review proceeds under ordinary amendment procedure. The Review's authority is ordinary amendment authority. The observation's authority is solely the right to convene.

### 2.4 No Shortcut Clause

Observation Records do not satisfy the Evidence-Based Amendment Principle on their own. An amendment whose justification rests on implementation cost, implementation inconvenience, runtime performance regression, scheduled RFC maintenance burden, or under-specification of edge cases — and which cannot independently cite an admissible constitutional evidence class — is not admissible.

When such an amendment is proposed at the gate, the proposal SHALL be rejected and reframed. Reframing is not grounds for reconsideration under procedural failure; it is a substantive change to the proposal.

### 2.5 Implementation Strategy Closure

When operational evidence surfaces and no constitutional evidence is constructible, the proper response is an Implementation Strategy Amendment under the existing governance rules, not a Specification or Architecture Amendment.

---

## 3. Amendment Procedure Constraints

The amendment procedure defined by ARCHITECTURE-GOVERNANCE.md is supplemented with the following constraints:

- A gate check SHALL verify that any proposed architectural amendment cites at least one admissible evidence class.
- An amendment rejected at the gate SHALL NOT be re-proposed without reframing. Reframing is not grounds for reconsideration under procedural failure; it is a substantive change to the proposal.

---

## 4. Glossary Anchor

`amendment`, `Architecture Amendment`, `conformance evidence`, `verification evidence`, `security evidence`, `Observation Record`, `Architecture Review`, `Implementation Strategy Amendment`, `Resolution Target`, `constitutional evidence taxonomy` — see Glossary.md for normative definitions.

---

## Decision Record

Status: Accepted

Problem: The repository needed an explicit, enforceable Evidence-Based Amendment Principle and a per-RFC Decision Record requirement to prevent architectural drift through convenience or elegance arguments.

Alternatives Considered:
- Permit amendments on operational cost grounds alone.
- Use a purely reputational gate (High-confidence reviewers only).
- Maintain a standalone Architecture Decision Log file.

Why Rejected:
- Operational cost alone does not establish that the architecture is wrong; it establishes that the implementation strategy may need revision.
- Reputational gates do not scale; a reviewer with High confidence in one domain may have Low confidence in the relevant domain.
- A standalone ADL creates competing authority with RFCs; per-RFC Decision Records keep rationale adjacent to the specification it explains.

Accepted Decision: Per-RFC Decision Records with a generated cross-cutting index, three admissible evidence classes (Conformance, Verification, Security), an Architecture Review trigger via Observation Records, and a No Shortcut clause that rejects implementation-cost-only amendments at the gate.

Architectural Consequences: Adds procedural discipline to the amendment process. Requires back-filling Decision Record appendices into all 41 existing RFCs.

Verification Consequences: CI gates enforce Decision Record presence and evidence class citation at the amendment gate.

Implementation Consequences: Back-fill of 41 RFCs required. CI tooling must be deployed before Phase 1 implementation begins.

CTS Impact: No CTS test changes. CI gates are tooling-level discipline checks, not conformance tests.

Supersedes: Governance-RFC.md amendment process (absent the Evidence-Based Amendment Principle specifically).

Superseded By: None.

---

Classification: Normative
Authoritative Source: RFC-GOV-002