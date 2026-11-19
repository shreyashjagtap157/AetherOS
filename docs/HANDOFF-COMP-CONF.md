# Handoff: RFC-0006 Composition Conformance Declaration Model

Classification: Operational
Authoritative Source: HANDOFF-COMP-CONF.md

**Date:** 2026-11-19
**Purpose:** Snapshot of the uncommitted working-tree conformance slice so a new session can resume.
**Classification:** Operational

---

## 1. What this milestone is

Phase A / composition-conformance enabler: RFC-0006 names the composition fixtures a
conforming implementation must pass and requires conformance declarations to cite both
test IDs and the executing artifact ID (RFC-0006.4; methodology §8), to classify failures
per the §10 taxonomy, and to record substitutions (RFC-0006.6) and justified omissions.
Until this slice, none of that machinery existed in the repository.

This slice adds a `conformance` module to `aether-comp-model`. It is **not** a CTS
executor, emits no RFC-0034 audit records, computes no conformance claim for a real
deployment, and performs no I/O. It models the declaration the CTS must produce.

Declared semantics (module docs are authoritative):

- `CORE_COMPOSITION_TESTS` enumerates RFC-0006.1's mandatory set: RFC-0002 MAN-1..5,
  RFC-0003 RT-1..4, RFC-0004 DEPR-1..5, RFC-0005 PROF-1..4 (18 fixtures).
- Every core fixture must be accounted for: executed, substituted (substitute executed,
  RFC-0006.6), or omitted with justification. Otherwise: `MissingCoreTest`.
- Failure-mode coherence enforced: a failing fixture must carry a §10 failure mode
  (else `FailureNotClassified`); a passing fixture must not (else `InconsistentFailureMode`).
  Failure modes follow the methodology §10 taxonomy (ConformanceViolation, ConformanceGap,
  ImplementationBug, TestBug, EnvironmentalFailure).
- Substitutions declared must be executed (`SubstitutionNotExecuted`); omissions require
  non-empty justification (`UnjustifiedOmission`); duplicate fixture identifiers are caller
  errors (`DuplicateResult`).
- A profile claim requires a non-empty subset (`MissingProfileCoverage`); a claim is
  evaluated only for the fixture vector provided, never in the abstract.
- `artifact` is a 64-bit FNV-1a hash over (CTS version, implementation version, all
  results); deterministic; non-cryptographic (consistent with other host-model digests);
  a bookkeeping identifier, not a release attestation.
- `conformant` reports whether this fixture vector supports a conformance claim; it is a
  statement about THIS vector only. Failing fixtures are reported in `gaps` with their
  classification; omitted core fixtures with justification are recorded as gaps
  (conformant = false), not rejected — the declaration must be emittable in order to
  disclose gaps (methodology §10 motive).
- Events are not emitted; the declaration is a value.

## 2. Changes (uncommitted, working tree)

- New module `reference/crates/aether-comp-model/src/conformance.rs` (crate root declares
  `pub mod conformance;`).
- 8 unit tests (full-pass declaration + deterministic artifact; missing core → rejection;
  failure classification + gap reporting; passing-with-mode rejection; substitution
  coverage + acceptance; empty profile subset rejection + subset failure gap; omitted
  core gap; duplicate results + unjustified omission).
- CTS runner: two new cases (now **34 total**): `CTS-COMP-CONF-001`
  (valid declaration, deterministic artifact) and `CTS-COMP-CONF-002`
  (missing core fixture and unclassified failure rejected with the right ids).

## 3. Verification actually executed (2026-11-19, Windows, Rust 1.95.0)

- `reference/check.sh`: exit 0 — fmt, locked check/test (debug+release), Clippy `-D warnings`,
  Rustdoc `-D warnings`, CTS 37/37.
- Root `./check.sh`: exit 0 - release evidence 151 files verified, supply-chain 7 packages.
- `cargo test --workspace`: **132 passed, 0 failed** (53 comp-model).

## 3.1 Verifier findings and fixes (rounds 1-3, same session)


Adversarial verifier (orch-task-verifier) first verdict: **GATE BLOCKED (T1, T3, T5)**.

- **D1 (Moderate, FIXED):** the artifact omitted omissions, substitutions (incl. their
  justifications), and the claimed profile; it also had no field separators / length
  prefixes and an unpinned `FailureMode` discriminant. Fixed: canonicalized hashing of
  length-prefixed fields, results canonicalized by fixture id (order-independent),
  `#[repr(u8)]` on `FailureMode` (`map_or(MAX, |m| m as u8)` encoding), artifact now binds
  (cts_version, implementation, profile, full results, substitutions, omissions).
- **D2 (Moderate, FIXED):** `Substitution.justification` was never checked; empty
  substitution records passed. Fixed: mandatory non-empty justification
  (`UnjustifiedSubstitution`), plus core-target membership, substitution-vs-substitute
  equality rejection, and duplicate-record rejection (`InvalidSubstitution`).
- **D3 (Moderate, FIXED):** contradictory accounting accepted — a fixture both executed
  (or substituted) and omitted silently flipped `conformant=false` with a misleading gap.
  Fixed: typed `DuplicateAccounting` rejection for (executed∪substituted) ∩ omitted and
  for duplicate omissions; omissions must name core fixtures only
  (`OmittedNonCoreFixture`); `core_results` may contain only core fixtures or declared
  substitutes (`UnexpectedFixture`).
- **D4/D5/D6 (Low/Info, FIXED/accepted):** length-prefixed fields, canonical order,
  `#[repr(u8)]`; unexpected fixtures and non-core omissions now rejected; both-substitute
  and original-claimable over-coverage documented as accepted.
- **D7 (Moderate, FIXED):** `CTS-COMP-CONF-002` now also asserts a failing core fixture
  produces a non-conformant declaration with non-empty gaps (catches the
  hardcoded-`conformant=true` mutant at the CTS layer, not only in unit tests).
- **D8 (Boundary, documented):** RFC-0006 §§4–5, §6 and RFC-0034 emission remain
  unimplemented by design; the narrow claim is unchanged.
- Refactor for the Clippy `too_many_lines` limit: validation extracted into
  `validate_declaration`; legacy-check ordering preserved.
- Second verifier round: **ALL PASS**. Residual Low items addressed before close:
  `FailureMode` discriminants pinned (`= 0..=4`); substitute reuse across records
  and substitutes that are themselves core fixtures rejected; golden artifact vector
  test pins the framing (`ArtifactId(15100182307975615485)`); module docs disclaim
  §8 timestamp/forensics capture; stale `DuplicateAccounting` doc comment fixed.
- Re-verification sent to the same verifier session; result recorded here when received.

## 3.2 RFC-0006 §4 reporting and §5 fixtures round (2026-11-19)

Added (this slice):
- `CompositionConformance::report()` -> `ConformanceReport`: manifests tested, active
  profile declarations, tier claims, substitution table, failure-forensics category
  counts (methodology §10, `ForensicsCounts`), cited artifact id, canonical executed
  fixture list.
- `CompositionConformance::artifact_is_cited()` (RFC-0006.4): non-zero artifact and
  non-empty executed fixture set; unit-testable by setting `artifact =
  ArtifactId(0)`.
- `DeclarationInput` gains `manifests_tested: Vec<String>` and `tier_claims: Vec<(String,
  Tier)>` (both empty by default in callers).
- CTS runner: two new cases `CTS-COMP-CONF-003` (artifact + fixture citation, canonical
  order, negative zero-artifact axis) and `CTS-COMP-CONF-004` (forensics counts match the
  methodology 10 taxonomy on a mixed-failure vector). Now **37 CTS cases**.
- CTS runner refactored so case list lives in `cases()` (Clippy `too_many_lines`).
- Verifier round 1 verdict: T3 FAIL (CONF-003 vacuous for an always-true citation).
  Fixed: CONF-003 now also constructs a zeroed-artifact declaration and asserts
  `!artifact_is_cited()` at the CTS layer. D1/D4 doc defects fixed.
- Verifier round 2 verdict: **ALL PASS**. Follow-up (recorded): the unit test now also
  pins the all-omitted/zero-fixture axis of `artifact_is_cited`
  (`artifact_citation_fails_for_zero_artifact`).
- RFC-0006.2's profile->Profiles binding, RFC-0006.3's RFC-0034 audit entry, and
  `ConformanceReport` emission as RFC-0034 audit records remain **not implemented**.

## 4. Known limitations and boundaries

- `CORE_COMPOSITION_TESTS` enumerates the RFC-0006.1 spec identifiers verbatim; the
  runner's own `CTS-*-NNN` cases are not mapped onto the spec fixture IDs (no traceability
  matrix yet; the fixture-to-model mapping remains a documentation task).
- Profile subset content is caller-supplied; the model does not enumerate the five
  profiles' §4 subsets from `Profiles-Specification.md`.
- FNV-1a is non-cryptographic; the artifact ID is a determinism/bookkeeping aid only and
  must not be presented as an attestation.
- No RFC-0034 audit emission; no CTS executor; no conformance claim for any real system.
- Uncommitted, unmerged, un-CI-validated. Coexists with seven other verified slices in the
  same working tree.

## 5. State and next actions

- Independent verifier review is the next step; findings to be recorded/resolved.
- Do NOT commit or push without the user's explicit decision (AGENTS.md).
- Composition track host models are now complete at prototype level: manifest (RFC-0002),
  routing (RFC-0003), resolution (RFC-0004), profile selection (RFC-0005), conformance
  declarations (RFC-0006). Remaining Phase A items: RFC-0042 update/rollback governance
  (draft — acceptance is the user's governance decision), UASA interface contracts
  (blocked on evidence reproduction/owner input), outbox-effect/failure-semantics
  depth, or Phase B scoping (needs the reference-target decision).
