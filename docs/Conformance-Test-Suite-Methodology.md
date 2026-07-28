# Conformance Test Suite Methodology

Classification: Normative
Authoritative Source: Conformance-Test-Suite-Methodology.md
Requirement-ID: CTS-METH-001

**Status:** Active. This document defines the methodology through which an implementation's conformance to the Platform Architecture Specification and its RFCs is measured. The CTS does not itself declare conformance criteria; it exists to make conformance measurable.

---

## 1. Goals of Conformance Testing

Conformance testing in this suite serves three purposes:

1. **Verifiability.** A reviewer can independently verify that a published conformance declaration matches reality.
2. **Bug prevention.** Implementers receive early signals about drift from invariants before they ship.
3. **Architectural accountability.** Deviations from normative requirements are discoverable; they do not require consensus or interpretation.

The CTS is not a benchmarking system and not a performance regression tool. Those concerns have separate solutions and are out of scope of this document.

---

## 2. Scope and Out-of-Scope

### 2.1 In scope

- Functional conformance to every MUST requirement in every RFC.
- Conformance to every architectural invariant I-1 through I-13.
- Conformance to profile-specific requirements (per `Profiles-Specification.md`).
- Conformance-to-violation forensics (capturing evidence of every failure).
- Bootstrap-path attestation reproducibility (per Section 5 of the Platform Architecture Specification).

### 2.2 Out of scope

- Performance excellence (only conformance to performance contracts is in scope).
- Compatibility with specific applications or ecosystems (covered by projection-layer conformance).
- Security penetration testing (this suite assumes Tier A components are formally verified; security testing complements but is not part of conformance testing).

---

## 3. Conformance Test Layers

### 3.1 Layer 1 — Invariant Conformance

Each architectural invariant I-1 through I-13 has a corresponding invariant conformance test. These tests are required regardless of profile. Tests are designed so that:

- A non-conformant implementation provably cannot pass them.
- A conformant implementation provably cannot pass by accident.
- Each test cites the invariant it verifies by ID (e.g., `CTS-INV-I-12`).

### 3.2 Layer 2 — RFC Conformance

Each normative RFC MUST contributes at least one conformance test per MUST requirement. SHOULD requirements require conformance tests when the implementation claims to satisfy them.

RFC conformance tests cite the MUST they verify by `RFC-XXXX.N` reference.

### 3.3 Layer 3 — Profile Conformance

Each profile (Embedded, Desktop, Cloud, HPC, Safety-Critical) has declared profile-specific tests in `Profiles-Specification.md`. Implementations declaring a profile MUST satisfy the corresponding tests.

### 3.4 Layer 4 — Cross-Conformance

Conformance tests that span multiple RFCs. Examples:

- A capability-mediated file-system access test that fails if either the Capability Mediation (`RFC-0013`) or POSIX projection (`RFC-0020`) violates its invariants.
- A bootstrap-path test that requires both `RFC-0007` and `RFC-0037` to satisfy their invariants.

---

## 4. Test Specification Format

Each conformance test MUST be specified in the form:

```
CTS-TEST-name: <unique identifier>
RFC-REQUIREMENT: <RFC-XXXX.N or I-X identifier>
PURPOSE: <single-sentence>
SETUP: <required initial state>
EXECUTION: <ordered sequence of operations>
EXPECTED: <observable result>
FAILURE-MODES: <executable failures and how they're categorized>
REPRODUCIBILITY: <seed, time-bound, or platform-binding>
```

Tests that cannot satisfy this format MUST be rejected as non-conformant specifications.

---

## 5. Test Categories

### 5.1 Functional tests
Verify MUST requirements. Pass/fail only.

### 5.2 Property-based tests
Verify invariants under sequences of random operations. Used for invariants and contracts that must hold over many inputs.

### 5.3 Conformance forensics
Verifies that a reported conformance violation is real and reproducible. Cited by IDs that the implementation MUST agree to reproduce.

### 5.4 Bootstrap conformance fixtures
Domain-specific fixtures reproducing the bootstrap path documented in `RFC-0007` Section on Bootstrap. Required for I-12. Bind to a specific hardware root of trust or to a test-fixture equivalent.

### 5.5 Verification-tier conformance
If Tier A is claimed, the implementation MUST publish the formal proof artifacts or their machine-checkable representations as part of conformance telemetry.

---

## 6. Conformance Evidence

Three classes of conformance evidence are required:

1. **Run-time evidence.** Observable behaviour recorded during CTS execution. Persisted in conformance-claim fixtures.
2. **Static evidence.** Specification of invariants, formal proofs, audits. The CTS treats these as inputs but verifies their existence and version.
3. **Sustainability evidence.** Records that the implementation intends to maintain conformance over time. Includes policies for Tier A maintenance (`RFC-0040`).

---

## 7. Reference Conformance Environment

The CTS is runnable in three execution modes:

1. **Sandbox mode.** Tests run against an implementation in a process-virtualized environment with synthetic roots of trust. Suitable for early development.
2. **Hardware-attested mode.** Tests run against an implementation on hardware providing a real root of trust. Required for Safety-Critical profile conformance.
3. **Reproducibility mode.** Tests run with deterministic seed, time, and platform-binding. Required for bootstrap attestation reproducibility.

Implementations MUST achieve reproducibility for the Top-Tier Conformance tests under Hardware-attested mode for Safety-Critical profile.

---

## 8. Conformance Telemetry

Each CTS run produces a conformance artifact including:

- CTS version
- Implementation version
- Test run timestamp
- Required tests skipped (with justification)
- Tests passed
- Tests failed
- Tests failed due to conformance forensics (with capture)

The artifact is the basis for conformance claims. A conformance declaration citing artifact IDs is conformant; one without is non-conformant.

---

## 9. Tier-Specific Conformance Obligations

### 9.1 Tier A

- Every claim in the Tier A attestation MUST be independently verifiable by running a proof-checker against a published proof.
- Each Tier A component MUST have at least one cross-implementation verification fixture (where two implementations of the same interface exist).

### 9.2 Tier B
- Every claim in the Tier B attestation MUST be auditable against a published model-checking trace.

### 9.3 Tier C
- Every property-tested claim MUST specify the property, the test generator, and the seed and time budget for reproducibility.

### 9.4 Tier D
- Standard test coverage expectations are documented per profile.

---

## 10. Failures and Forgiveness Modes

When a CTS test fails, the failure mode is classified:

- **Conformance violation** — fails the relevant MUST or invariant.
- **Conformance gap** — fails a SHOULD or profile-specific requirement.
- **Implementation bug** — test correctly specified; implementation fails.
- **Test bug** — test itself non-conformant; test MUST be re-specified.
- **Environmental failure** — test environment fails; precise test reruns may differ.

The CTS run output MUST classify failures per this taxonomy.

---

## 11. Profile Subset Conformance

Implementations claiming a subset of profiles MUST run the test set derived per `Profiles-Specification.md` Section 4 plus the full Core Conformance set minus profile-skipped tests.

The CTS provides explicit profile-subtraction descriptors. A test omitted for a profile reason MUST be enumerated in the conformance declaration.

---

## 12. Conformance Reporting

Conformance reporting uses a single declarative format published by implementations, referencing CTS artifact IDs. The format is normative in `Conformance-Test-Suite-Methodology.md` Section 8.

Reporting outputs:
- A conformance claim, citing artifact IDs, with caveats.
- Roll-ups per profile.
- A ranked list of conformance gaps with planned remediation dates.

---

## 13. Conformance Testing Tools

This specification does not mandate a single test-runner. Implementations MAY choose any test-runner that meets:
- Determinism requirements for reproducibility.
- Run-time evidence preservation requirements.
- Conformance-forensic classification.

CTS tools include published test executors for reference implementations.

---

## 14. Specifying CTS Tests Within RFCs

Each normative RFC MUST include a section titled **"Conformance Tests"** that enumerates test IDs and the requirements each test verifies. Tests in that section follow the format in Section 4.

MUST requirements without a corresponding CTS test are themselves non-conformant specifications.

---

## 15. CTS Versioning

The CTS itself has versions. CTS version bumps with new invariants, mandatory tests, or broken or withdrawn tests are Architectural Amendments of the CTS. Editorial refinements of test specifications are Specification Amendments.

---

## 16. Specifying Stub vs. Complete RFCs

A normative RFC MUST have at least one conformance test per MUST requirement before being adopted. "Spec-only" RFCs without CTS are invalid and not part of v1.

---

## Revision History — Conformance Test Suite

| Date       | Change                                                                       |
|------------|------------------------------------------------------------------------------|
| this cycle | Initial release of CTS methodology. Defines 4 test layers, test categories, tiers, profile subset derivation, evidence classes. |
