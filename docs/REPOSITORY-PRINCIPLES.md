# Repository Principles

Classification: Informative
Authoritative Source: Platform-Architecture-Specification-v1.1.md,
ARCHITECTURE-GOVERNANCE.md, REVIEWER-GUIDE.md

A one-page orientation for new contributors. Full authority lies in the
documents referenced above; this file makes the principles legible at a glance.

---

## 1. One Authoritative Source Per Fact

Every normative requirement has exactly one authoritative source.
References may duplicate navigation but never authority. If two documents
appear to define the same requirement, the repository is inconsistent.

## 2. Authority Flows Downward, Never Upward

No document may derive authority from a document of lower classification.
Generated documents shall not become authoritative through citation or repeated
use. Implementation documentation does not become normative by being
widely read.

## 3. Generated Artifacts Are Never Authoritative

Generated Markdown files shall not be edited directly. Their authoritative
source is named in their classification header.

## 4. Reviews Reconstruct Before Criticizing

A review is not considered to have begun until the reviewer has produced
the Reconstruction Artifact defined in REVIEWER-GUIDE.md. Critiques
submitted before reconstruction is complete are invalid.

## 5. Observation Is Not Amendment

Observation Records possess no amendment authority. Operational evidence
may trigger an Architecture Review under ordinary amendment procedure;
it does not, by itself, satisfy the Evidence-Based Amendment Principle.

## 6. Architecture Changes Require Constitutional Evidence

Every architectural amendment is supported by at least one admissible
constitutional evidence class: Conformance, Verification, or Security.
Implementation cost, implementation inconvenience, benchmark regression,
or RFC maintenance burden are not, by themselves, admissible evidence.

## 7. Governance Exists To Preserve Architectural Integrity

The governance layer's purpose is not to optimize the architecture.
It is to protect the architecture from changes that would compromise its
long-term integrity. When a governance rule feels constraining, that is
the design working as intended.

---

## Reading Order

1. README.md
2. REPOSITORY-PRINCIPLES.md (this file)
3. Platform-Architecture-Specification-v1.1.md
4. ARCHITECTURE-GOVERNANCE.md
5. 00-RFC-Index.md
6. Per-RFC Decision Records via the generated DECISION-INDEX.md