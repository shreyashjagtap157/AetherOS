# Handoff: RFC-0002/0004 Composition Host Model

Classification: Operational
Authoritative Source: HANDOFF-COMP-MODEL.md

**Date:** 2026-11-19
**Purpose:** Snapshot of the uncommitted working-tree composition slice so a new session can resume.
**Classification:** Operational

---

## 1. What this milestone is

Phase A deliverable of `docs/IMPLEMENTATION-ROADMAP.md` §3 item 5 ("executable host models of
… composition"). Until this slice, no composition code existed in the repository: RFC-0002
(manifest), RFC-0004 (dependency resolution), RFC-0005 (profile selection) and RFC-0003
(capability routing) were normative text only.

This slice adds `aether-comp-model`, a host-reference model of the manifest→resolution path.
It is **not** the Composition Root, **not** a package manager, **not** RFC-0002 canonical
serialization, and makes no claim of kernel or boot integration.

Design decisions (documented in the crate root docs):

- `ComponentId` is a host `u64` newtype; RFC-0002.1's SHA-3-512-over-canonical-representation
  addressing is out of scope (no cryptographic dependency in this workspace).
- One declared selection strategy: `HighestVersion` (RFC-0004.3). Pinned-via-override and
  lowest-version are not implemented. `(id, version)` ambiguity fails closed with
  `ResolutionError::AmbiguousProvider`.
- Version matching is exact overlap against `[min_version, max_version]`.
- Profiles: a provider must share ≥1 profile with its **dependent** (RFC-0004.2 "compatible
  profiles"); a provider whose tier is weaker than its dependent's is rejected (RFC-0004.2
  "compatible verification tiers").
- Invalid candidates are refused for selection (RFC-0004.4). If every candidate for a
  requirement is invalid or absent, resolution fails with `MissingProvider` for that component.
- Transitive closure over selected providers; Kahn topological order, sorted-member cycle
  reporting (self-dependency included), deterministic instantiation order
  (dependencies first, target last), RFC-0004.6 reason codes: MissingProvider, VersionMismatch,
  ProfileMismatch, TierMismatch, Cycle.
- Memoized selection: the same provider is selected for every consumer of a given requirement;
  no per-consumer variation (documented).
- Out of scope (recorded): RFC-0005 mechanism activation, RFC-0003 capability routing/grants,
  RFC-0004.8 conservative update behaviour, RFC-0004.10 per-instance semantics, attestations,
  content addressing, tier-ordering semantics beyond A>B>C>D ordinal comparison.

## 2. Changes (uncommitted, working tree)

New crate `reference/crates/aether-comp-model/` (`src/lib.rs`, 10 unit tests). Workspace member
in `reference/Cargo.toml`; lock updated; `aether-cts-runner` depends on it via path.

CTS runner: four new cases (now 26 total, exit 0):
- `CTS-COMP-RESOLVE-001` — highest-version selection, range filtering, distinct RFC-0004.6
  reason codes (missing / version / profile / tier).
- `CTS-COMP-CYCLE-001` — two-way dependency cycle reported with sorted members, no order returned.
- `CTS-COMP-ORDER-001` — deterministic instantiation order across runs; providers before dependents.
- `CTS-COMP-VALIDATE-001` — RFC-0002.19 rejection: zero id, max-instances = 0, no profiles,
  inverted requirement range; valid baseline accepted.

## 3. Verification actually executed (2026-11-19, Windows, Rust 1.95.0)

- `reference/check.sh`: exit 0 — fmt, locked check/test (debug+release), Clippy `-D warnings`,
  Rustdoc `-D warnings`, CTS 26/26.
- Root `./check.sh`: exit 0 — supply-chain (7 packages), release evidence 141 files verified.
- `cargo test --workspace`: **85 passed, 0 failed** (14 comp-model, incl. the D1/D2 regression tests).

## 3.1 Verifier findings and fixes (this milestone)

Adversarial verifier (orch-task-verifier) first verdict: **GATE FAIL (T2, T6)**.

- **D1 (HIGH, FIXED):** topological `in_degree` was keyed by component id; a target whose id
  also appeared as a selected provider overwrote its own entry (debug → `usize` underflow
  panic at `*degree -= 1`; release → silently reduced map, dependent instantiated before its
  dependency, and the cycle unreported). Fixed by keying all graph nodes by kind
  (`Node::Target` vs `Node::Provider(ComponentId)`); no entry can be merged or overwritten.
  Regression tests: `target_self_requirement_with_acyclic_provider_resolves_without_panic`,
  plus the rewritten `self_dependency_is_a_cycle` (true self-loop provider).
- **D2 (MEDIUM, FIXED):** `Cycle(members)` reported the residual in-degree set, which
  included the target and innocent downstream dependents. Fixed with `find_cycle()`:
  deterministic DFS with GRAY back-edge detection; `members` is exactly the back-edge
  slice, sorted and deduplicated. Regression test:
  `cycle_members_exclude_innocent_downstream_dependents` (expects `[1,2]`).
- **D3 (LOW, FIXED):** pool key vs `manifest.id` mismatch silently resolved incoherently;
  now fails closed with `ResolutionError::InconsistentCandidatePool`.
- **D4 (LOW, FIXED as docs):** `SoleCandidate` now documented as "sole valid candidate in
  pool"; events documented as one per unique memoized selection, discarded on failure.
- **D5 (LOW, FIXED as docs):** reason-code precedence (version → profile → tier; refusal of
  invalid candidates first) documented in module docs; multi-failure ambiguity stated.
- **D6 (LOW, FIXED):** whitespace-only `start`/`stop`/grant names rejected; tests added.
- Re-verified (same session, 2026-11-19): **ALL PASS** — D1–D6 all verified fixed by
  trace and re-runs (14/14 comp tests, 26/26 CTS). The `self_dependency_is_a_cycle`
  re-arm was confirmed legitimate correction, not test weakening. Residuals (R1–R4,
  non-blocking): `find_cycle` DFS is recursive (stack depth in host model, fine);
  `order` may legitimately contain the same id twice; no test for
  cycle-through-provider × diamond shapes; CTS still lacks AmbiguousProvider /
  SoleCandidate-positive / InconsistentCandidatePool cases.

## 4. Known limitations and boundaries

- Order complexity is `O((P + D) log P)`-ish from BTreeSet/BTreeMap operations; RFC-0004.11's
  `O(P × D)` bound is not the limiting factor and no complexity proof is claimed.
- Selection memoization means a provider invalid **after** selection (impossible: validated
  first) — not applicable; memoization is deterministic by construction.
- Tier comparison treats lower ordinal A as stronger (declaration order A<B<C<D); it is an
  ordinal, not an evidence model.
- Component ids are host values; collision semantics are the caller's problem.
- Uncommitted, unmerged, un-CI-validated. Coexists in the working tree with the capability and
  scheduler slices from the same session.

## 5. State and next actions

- Independent verifier review of this slice is the next step (see report when received; findings
  to be recorded/resolved).
- Do NOT commit or push without the user's explicit decision (AGENTS.md).
- Remaining Phase A deliverables per the roadmap: composition conformance fixtures,
  transaction/audit commitment depth, outbox effects, update rollback model, failure semantics —
  plus Phase B (QEMU RV64 vertical slice) which requires a user decision on reference target
  approval before meaningful kernel work can start.
