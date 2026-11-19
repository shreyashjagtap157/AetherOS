# Handoff: RFC-0005 Profile Selection Host Model

Classification: Operational
Authoritative Source: HANDOFF-PROFILE-SELECTION.md

**Date:** 2026-11-19
**Purpose:** Snapshot of the uncommitted working-tree profile-selection slice so a new session can resume.
**Classification:** Operational

---

## 1. What this milestone is

Phase A composition conformance work: RFC-0006.1 requires the profile-selection
host model to exist (CTS-COMP-PROF-*), and `docs/Profiles-Specification.md`
Section 4 derives profile-specific conformance from it. Until this slice, RFC-0005
was normative text with no executable model anywhere in the repository.

This slice adds a `profile` module to `aether-comp-model` implementing the decision
half of RFC-0005. It is **not** the Composition Root, activates nothing in a running
system, emits no RFC-0034/RFC-0005.9 events (reasoning vector instead), and makes no
claim of profile conformance for any real deployment.

Declared semantics (module docs are authoritative):

- Activation = `mandatory ∪ declared_optional ∪ extensions` (explicit opt-in only;
  defaults activate nothing extra, RFC-0005.5).
- Rejections fail closed with typed errors: empty mandatory set (`EmptyMandatory`),
  no declarations (`EmptyDeclarations`), a declaration's own mandatory set overlapping
  its own `forbidden` set (`ForbiddenMandatory`), and a declaration's own optional/
  extension opt-in overlapping its own `forbidden` set (`ForbiddenDeclared`).
  Cross-profile opt-in overlaps with another profile's `forbidden` set are
  **suppressed**, not rejected (most restrictive wins); same-profile self-conflicts
  are rejected.
- Combination unions mandatory/declared-optional and forbidden sets; a mechanism
  (mandatory or declared opt-in) required by one profile and forbidden by another is
  **suppressed** (never activated) and recorded with a reasoning line; the strictest
  tier floor applies across the combination (RFC-0005.6).
- Manifests weaker than a declaration's `minimum_tier` are rejected with
  `TierConflict` (RFC-0005.3/.7); tier comparison is the crate-wide ordinal
  (A strongest < B < C < D).
- Combination unions mandatory/declared-optional and forbidden sets; a mechanism
  mandated by one profile and forbidden by another is **suppressed** (never
  activated) and recorded with a reasoning line; the strictest tier floor applies
  across the combination (RFC-0005.6).
- Mechanism names are opaque strings (lexical handling); the model does not
  enumerate the official catalog (`Profiles-Specification.md`).
- Events are not emitted; decisions carry `reasoning: Vec<String>`.
- Out of scope (recorded): mechanism catalog curation, real mechanism activation,
  RFC-0006 conformance reporting, RFC-0003 capability routing (no token layer
  exists yet), multi-instance semantics, event emission.

## 2. Changes (uncommitted, working tree)

- New module `reference/crates/aether-comp-model/src/profile.rs`.
- Crate root now declares `pub mod profile;`.
- 7 unit tests (activation defaults, tier rejection, forbidden-opt-in rejection,
  combination suppression + strictest floor, empty-mandatory rejection, empty
  mechanism-name fail-closed) plus 5 regression tests added in round 2 (D1:
  ForbiddenMandatory in both entry points; D2: EmptyDeclarations; positive
  extension opt-in; cross-profile opt-in suppression; combine-side ForbiddenMandatory) = **13 profile tests**
  (26 comp-model out of **100 workspace tests**).
- CTS runner: two new cases (now 30 total, exit 0): `CTS-PROF-SELECT-001`
  (activation set, default-off extensions, floor rejection) and
  `CTS-PROF-COMBINE-001` (union of mandatory sets, strictest floor, combination
  acceptance for Tier A).

## 3. Verification actually executed (2026-11-19, Windows, Rust 1.95.0)

- `reference/check.sh`: exit 0 — fmt, locked check/test (debug+release), Clippy
  `-D warnings`, Rustdoc `-D warnings`, CTS 30/30.
- Root `./check.sh`: exit 0 — release evidence 145 files verified, supply-chain 7 packages.
- `cargo test --workspace`: **100 passed, 0 failed** (26 comp-model).

## 3.1 Verifier findings and fixes (this milestone)

Adversarial verifier (orch-task-verifier) first verdict: **GATE BLOCKED (T1, T2, T3, T5)**.

- **D1 (Medium, FIXED):** `apply_profile` activated a mandatory mechanism forbidden by its
  own declaration (fail-open). Now rejected with typed `ForbiddenMandatory`; the mandatory
  check also runs inside `combine_profiles` per declaration.
- **D2 (Medium, FIXED):** `combine_profiles(&[])` returned `Ok`. Now returns
  `ProfileError::EmptyDeclarations`.
- **D3/D4 (Low, FIXED):** `reasoning` is now sorted and deduplicated (it is an audit
  surface); activation count prints `activated.len()`, not raw list lengths.
- **D5 (Low, FIXED):** check order unified (EmptyMandatory → ForbiddenDeclared →
  ForbiddenMandatory → TierConflict) across both entry points; cross-profile opt-in
  conflicts are suppressed (most restrictive wins), self-conflicts are rejected;
  documented.
- **D6 (Medium, FIXED):** CTS-PROF-SELECT-001 now asserts positive extension opt-in
  activation and ForbiddenDeclared rejection; CTS-PROF-COMBINE-001 now gives Safety-Critical
  a forbidden mechanism that the desktop activates and asserts it is **suppressed** (absent
  from `activated`, reported in `suppressed`) — suppression and forbidden-union are now
  CTS-enforced, not only unit-tested.
- **D7 (Low, FIXED):** stale `aether-comp-model` module doc no longer claims profiles do
  not activate mechanisms.
- Unit tests added: ForbiddenMandatory rejection, EmptyDeclarations rejection, positive
  extension opt-in, cross-profile opt-in suppression.
- Re-verified (same session, 2026-11-19): **ALL PASS** — D1–D7 all verified fixed by
  trace and re-runs (25 comp-model tests, 30/30 CTS, clippy `-D warnings` exit 0). Zero
  defects above Low. Residual (recorded): suppression reasoning for cross-profile
  opt-in mechanisms now reads "(mandatory or declared opt-in)"; combine-side
  `ForbiddenMandatory` has an explicit unit test; boot-time (RFC-0005.8) and audit
  (RFC-0005.9/RFC-0034) halves remain unimplemented by design; RFC-0006.6 CTS
  substitution recorded (SELECT↔PROF-1/.2, COMBINE↔PROF-3; no substitution for
  PROF-4/audit).

Substitution note (RFC-0006.6): `CTS-PROF-SELECT-001` substitutes for CTS-COMP-PROF-1/.2,
`CTS-PROF-COMBINE-001` for CTS-COMP-PROF-3. The RFC-0005.8 boot-time role
(CTS-COMP-PROF-4 analogue) and CTS-COMP-PROF-2's audit obligations are not implemented;
no substitution is claimed for them. Substitution must be recorded in any future
conformance declaration (RFC-0006.6); none exists yet.

## 4. Known limitations and boundaries

- Tier comparison treats A as strongest ordinal; no evidence weighting.
- Combination suppression applies per-mechanism, not per-profile; a declaration
  suppressed of its only mandatory mechanism still passes tier/floor checks with
  an empty activated set (an empty activation is legal, not an error — noted).
- Suppression reasoning is recorded per mechanism, not as RFC-0034 audit records.
- Mechanism-name collision semantics (e.g. mixed namespaces "RFC-0011: x" vs "x")
  are the caller's responsibility; exact string match only.
- Uncommitted, unmerged, un-CI-validated. Coexists with the other slices in the
  same working tree.

## 5. State and next actions

- Independent verifier review is the next step; findings to be recorded/resolved.
- Do NOT commit or push without the user's explicit decision (AGENTS.md).
- Remaining Phase A after this slice: RFC-0006 conformance reporting fixtures,
  RFC-0003 capability routing host model (needs a rights/token layer; candidate
  next task), update/rollback host model (RFC-0042 is an evidence-gated draft —
  acceptance is a governance decision requiring user input), or outbox-effect
  depth. Composition track otherwise complete for MAN/DEPR/PROF host models.
