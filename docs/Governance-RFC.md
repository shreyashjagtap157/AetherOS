# Governance RFC (Suite-Wide)

Classification: Constitutional
Authoritative Source: ARCHITECTURE-GOVERNANCE.md (supersedes this document)
Requirement-ID: GOV-GOV-001

**Status:** Frozen. This governance document applies to the Platform Architecture Specification and to all 41 RFCs in the suite.

> **Note:** This document has been superseded by `ARCHITECTURE-GOVERNANCE.md`. For Evidence-Based Amendment Principle and Decision Record requirements, see `RFC-GOV-002`. This document is retained for historical reference.

---

## 1. RFC Document Conventions

Each normative RFC MUST follow this structure:

1. **Header** — RFC number, title, status, date, dependencies on other RFCs.
2. **Scope** — What the RFC covers and what it explicitly excludes.
3. **Normative requirements** — Numbered requirements written as `RFC-XYZ.N: …` and using MUST/SHOULD/MAY language.
4. **Data structures** — Every data structure the RFC introduces gets a typed, sized definition.
5. **Protocols** — Every protocol gets a sequence of messages, plus failure branches.
6. **Algorithms and complexity** — Algorithms include bounds, expected case, failure case.
7. **Failure semantics** — Every interface declares its failure modes.
8. **Conformance obligations** — What implementers must demonstrate.
9. **Verification expectations** — What tier is appropriate for which component.
10. **Cross-references** — Invariant IDs (I-1, …), RFC numbers, section anchors.
11. **Revision history** — Per-RFC revision log.

Each normative requirement uses RFC-NNNN.M numbering. Implementations cite conformance by referencing these numbers.

---

## 2. Amendment Categories

Three categories of changes exist:

### 2.1 Architectural Amendment

Changes to:
- Section 2 invariants of `Platform-Architecture-Specification-v1.1.md`
- Section 3 peer decomposition

Process:
- Requires `git`-style signed RFC, distributed for review
- Requires explicit enumeration of which invariants or which peer boundaries change
- Requires justification addressing each requirement above
- Freezes for at least 30 days before acceptance
- Acceptance requires explicit Architecture-Amendment RFC number

### 2.2 Specification Amendment

Changes to any of the 41 normative RFCs.

Process:
- Standard RFC revision process
- Each change MUST be either backward-compatible or accompanied by a migrator specification
- Replaces or extends, never silently mutates, prior normative requirements.

### 2.3 Editorial Amendment

Changes that do not alter normative requirements, including:
- Reformatting
- Example wording changes
- Section reordering
- Cross-reference corrections

Process:
- Editor's discretion.
- Carries no version increment for the platform-architecture document.
- For individual RFCs, increments the RFC revision number on its revision history page.

---

## 3. Compliance Levels Within an RFC

Each normative requirement uses one of these verbs:

- **MUST / MUST NOT** — required for conformance. Absence is a conformance violation.
- **SHOULD / SHOULD NOT** — recommended. Implementations deviating MUST document the deviation in the conformance declaration.
- **MAY** — explicitly permitted. Implementations MAY choose to support or not, but if supported, all related MUSTs apply.

No other verbs appear in normative sections.

---

## 4. Numbering Scheme

- Architecture document: `Platform-Architecture-Specification-vX.Y.md`. Architectural amendments increment X; other revisions increment Y.
- Top-level supplementary specs: `Profiles-Specification.md`, `Conformance-Test-Suite-Methodology.md`, `Governance-RFC.md`, `Glossary.md`. Versions increment Y.
- RFCs: numbered `RFC-0001` through `RFC-0041`. RFCs never renumber. A numbered RFC's revisions are tracked inside the file's revision history block.
- Internal requirement numbering: `RFC-XXXX.N`, where N is a counter within the RFC.
- Architecture invariant IDs: `I-1` through `I-13`. IDs are never reused; obsolete IDs are noted in the revision history.

---

## 5. Conformance Declaration Format

Each implementation MUST publish a conformance declaration covering:
- Which profiles the implementation supports (Embedded, Desktop, Cloud, HPC, Safety-Critical, or a declared subset).
- Which verification tiers the implementation supports.
- Which acknowledgement profiles are supported.
- A list of deviations from SHOULD-clauses with justification.
- A list of all known conformance violations under remediation.

The conformance declaration is the legal basis for any conformance claim.

---

## 6. Document Status Markers

| Marker     | Meaning                                                                                       |
|------------|-----------------------------------------------------------------------------------------------|
| Frozen     | No normative changes are expected; only editorial amendments.                                |
| Active     | Open to specification amendments under standard process.                                       |
| Deprecated | Supplanted by newer RFC; implementations SHOULD migrate; maintained for legacy support only.  |
| Replaced   | Removed from normative use; cited for historical requirement only.                            |

---

## 7. Editorial Process

### 7.1 Editing workflow

1. **Proposal** — a change is proposed either as an Architecture-Amendment RFC or as a Specification-Amendment RFC.
2. **Discussion** — recorded comment period.
3. **Decision** — documented acceptance or rejection criteria.
4. **Implementation period** — for interconnected of architectural consequence, a freeze period is required.
5. **Publication** — the new revision is merged, revision history updated.

### 7.2 Author obligations

Authors of normative requirements MUST:
- State each requirement as a predicate or numbered MUST, not prose.
- Cite load-bearing sources explicitly when a constraint depends on external research.
- Provide at least one failure-model elaboration for each protocol.
- Quantify where quantifiability is meaningful (latency, throughput, complexity).

### 7.3 Reviewer obligations

Reviewers SHOULD challenge every requirement with three questions:
- Could different mechanisms satisfy this requirement under the same invariant?
- Has a known counterexample been considered?
- Is the verification cost commensurate with the tier declared?

---

## 8. Conformance Certification

No central authority issues certification in this suite. Implementations self-attest through their conformance declaration. Any party may independently test conformance against the published CTS methodology.

---

## 9. License and Reference

This suite is provided as a specification. Implementations are free to license per their own models. External references cited are the responsibility of their respective authors. Citing a reference here does not assert endorsement; it acknowledges the design landscape the architecture was built from.

---

## 10. Disputes and Interpretation

Disputes over RFC meaning are resolved by:
1. The RFC's normative section text.
2. The RFC's revision history.
3. The Platform Architecture Specification's invariants.
4. The Glossary's defined meaning.

When in doubt, normative text over commentary; invariants over mechanisms.

---

## 11. Amendment of This Document

This governance document may itself be amended by Specification-Amendment RFC process. It is not architecturally protected in the sense of Section 2.1, because governance is process, not architecture. However, changes to RFC structure (Section 1) and amendment categories (Section 2) have a 90-day mandatory freeze period, longer than the standard 30-day freeze.

---

## Revision History — Governance

| Date       | Change                                |
|------------|---------------------------------------|
| this cycle | Initial governance document released. |
