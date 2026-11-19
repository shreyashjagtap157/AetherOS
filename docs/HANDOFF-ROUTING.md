# Handoff: RFC-0003 Capability Routing Decision Model

Classification: Operational
Authoritative Source: HANDOFF-ROUTING.md

**Date:** 2026-11-19
**Purpose:** Snapshot of the uncommitted working-tree routing slice so a new session can resume.
**Classification:** Operational

---

## 1. What this milestone is

Composition-conformance enabler: RFC-0003 (Capability Routing) is consumed by RFC-0002
(manifests) and RFC-0006.1 (`CTS-COMP-RT-1..4`). Until this slice, RFC-0003 was normative
text with no executable decision model anywhere in the repository.

This slice adds a `routing` module to `aether-comp-model` implementing the decision half of
RFC-0003 §3. It is **not** the Composition Root, does **not** mint canonical RFC-0037
tokens (no 96-byte layout, no seal, no serialization), signs nothing, and makes no claim of
deployment or kernel conformance.

Declared semantics (module docs are authoritative):

- `Grant` (RFC-0002.6) gains `rights_ceiling: Option<Rights>` — the narrowing bound the
  grantor declares. `None` means the grant is a declaration, not authority, and confers
  no rights (fail closed).
- RFC-0003.3/.4: a grant's declared ceiling must be fully contained in the
  grantor authority (all-or-nothing containment, not intersection); a partially
  held ceiling is not derivable (`GrantNotDerivable`).
- A request for zero rights is denied under both policies (deliberate, fail-closed).
- `Attenuate`: granted rights are exactly the requested set; every requested right must be
  covered by the effective ceiling, otherwise `NarrowingFailed` (no partial grants).
- `Exact`: the request must equal the effective ceiling, otherwise `NarrowingFailed`.
- RFC-0003.7 environmental independence: `route` is a pure function of
  (grantor manifest, requestor manifest, grantor authority capability, request); there is
  no ambient input, no I/O, no clock.
- Granted authority inherits the grantor authority's resource, type, bounds, flags and
  expiry (a grantee never outlives its grantor).
- Denials are values, not errors: `DenialReason` ∈ `{ ManifestInvalid, GrantNotDerivable,
  NarrowingFailed }` — the RFC-0003.10 reason-code set.
- Revocation propagation (RFC-0003.9/.11, CTS-COMP-RT-4) is exercised and tested at the
  capability-provider layer (`CTS-CAP-REVOKE-*`, provider and oracle tests); this module
  deliberately does not duplicate it.
- Out of scope (recorded): token minting/signing/serialization (RFC-0037 layout, RFC-0038
  transfer protocol), pending/asynchronous outcomes (RFC-0003.2 case 3), audit event emission
  (RFC-0003.10 → RFC-0031 fields), namespace-based routing policy, lifetime bounds on grants
  (authority expiry inheritance carries the bound; RFC-0037.14 enforced by model/provider).

## 2. Changes (uncommitted, working tree)

- `aether-comp-model` now depends on `aether-types` (path); `Grant` extended with
  `rights_ceiling: Option<Rights>`; crate root declares `pub mod routing;`.
- New module `reference/crates/aether-comp-model/src/routing.rs`.
- 6 unit tests (attenuated grant bounded by ceiling and authority; ceiling not held by
  grantor; Exact policy match/mismatch; missing grant and `None` ceiling fail closed;
  invalid manifest rejection; environmental independence with 100 repeat evaluations).
- CTS runner: two new cases (now **32 total**): `CTS-ROUTING-GRANT-001`
  (attenuated grant narrowed to the requested rights) and `CTS-ROUTING-DENY-001`
  (beyond-ceiling, ceiling-not-held, absent grant, ceiling-less grant — typed reasons).

## 3. Verification actually executed (2026-11-19, Windows, Rust 1.95.0)

- `reference/check.sh`: exit 0 — fmt, locked check/test (debug+release), Clippy `-D warnings`,
  Rustdoc `-D warnings`, CTS 32/32.
- Root `./check.sh`: exit 0 — release evidence 146 files verified, supply-chain 7 packages.
- `cargo test --workspace`: **106 passed, 0 failed** (32 comp-model).

## 4. Known limitations and boundaries

- Routing decisions are manifest-level; rights are `aether_types::Rights` bitflags, not
  canonical RFC-0037 token bits (no serialization boundary here).
- RFC-0003.6: the decision does **not** take the namespace in which the request occurs as
  an input; grants are namespace-blind. Disclosed gap (Medium): any RFC-0003.6 conformance
  claim is blocked until namespace declaration semantics are defined (a design decision
  needing owner input, not a silent addition here).
- RFC-0003.10: no failure event emission; `DenialReason` is a return value only.
- RFC-0003.12: matching is linear in the grant count (host-model scale); the RFC's
  O(log N) MUST is not met.
- `Grant` has no revocability declaration (RFC-0003.8); revocation semantics remain
  devolved to the provider layer.
- Zero-rights requests are denied under both policies (deliberate, fail-closed).
- A requestor manifest that is itself invalid is denied (`ManifestInvalid`); grantor-side
  invalidity is also denied, consistent with fail-closed posture.
- `Attenuate` grants exactly the requested set (bounded by ceiling) rather than computing a
  separate narrowing step; attenuation monotonicity is inherited from the type system
  (granted ⊆ ceiling ⊆ authority).
- No audit event emission, no pending outcome, no namespace policy; see above.
- Uncommitted, unmerged, un-CI-validated. Coexists with seven other verified slices in the
  same working tree.

## 5. State and next actions

- Independent verifier (orch-task-verifier) verdict 2026-11-19: **ALL PASS** — no
  capability-amplification path; denial is monotone in authority; CTS proven non-vacuous
  by mutation testing of three breakages (grant rights mutation → GRANT-001 fails;
  NarrowingFailed drop → DENY-001 fails; authority-containment drop → DENY-001 fails).
  Defects dispositioned: D1 (doc ∩ wording → containment, fixed), D2 (handoff test count,
  fixed), D3 (zero-rights denial now documented), D4 (CTS-GRANT-001 now asserts
  object_type/flags/expires_at_ns inheritance), D5/D6/D7 (namespace, O(log N), event
  emission) documented as known gaps in §4. Do not cite CTS-ROUTING-* as
  CTS-COMP-RT-1..4 compliance (those identifiers do not exist in the runner).
- Do NOT commit or push without the user's explicit decision (AGENTS.md).
- Remaining Phase A: RFC-0006 conformance reporting fixtures (CTS-COMP-CONF-1..4),
  RFC-0042 update/rollback model (**draft, evidence-gated — acceptance is a governance
  decision requiring user input**), outbox effect depth, UASA interface contracts
  (blocked on evidence reproduction and owner input), or outbox/failure-semantics models.
