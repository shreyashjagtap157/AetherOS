# Handoff: PERSISTENCE §6 Crash-Consistency Fault Injection

Classification: Operational
Authoritative Source: HANDOFF-STATE-CRASH-SWEEP.md

**Date:** 2026-11-19
**Purpose:** Snapshot of the uncommitted working-tree crash-consistency slice so a new session can resume.
**Classification:** Operational

---

## 1. What this milestone is

`docs/PERSISTENCE-AND-EXTERNAL-EFFECTS.md` §6 requires the executable state prototype to
implement "crash truncation recovery … and deterministic fault injection at every
persistence boundary" **before** an on-disk filesystem is selected. The host reference
model (`aether-state-model`) already had torn-tail recovery and hand-written byte-cut
tests for one transaction; it had no systematic multi-frame sweep and no engine iteration
API for such a sweep.

This slice adds:

- `StateEngine::objects()` and `StateEngine::operations()` — sorted visibility iterators
  (public; the enumeration the sweep needs, exposed intentionally and documented).
- `aether_state_model::fault::CrashSweep` — sweeps **every** byte cut of a durable log
  (0..=len) and reports violations:
  - `RecoverFailed(cut)` — recovery rejected a cut;
  - `VersionRegressed(cut, object)` — a commit's version went backwards across a cut;
  - `MissingObject(cut, object)` — a previously visible object vanished;
  - `EffectRegressed(cut, operation)` — an effect in a terminal state (Acknowledged,
    Rejected, CompensationRequired) changed or disappeared across the cut.

  Properties are checked against the previous cut in ascending byte order — the honest
  crash model (a prefix can only grow by whole frames; torn frames are dropped by
  recovery).

Declared scope and non-goals:

- The sweep is a **verification harness**, not a storage provider, not a crash-consistency
  proof, and not part of the durable format. It does not inject faults into execution;
  it evaluates recovery outcomes at every possible byte boundary.
- It checks: recovery success at every cut, object-visibility monotonicity, version
  monotonicity, terminal effect-status stability. It does **not** check payload bytes,
  audit-commitment semantics at cut granularity, two-phase external effects, or
  multi-process linearizability (out of model scope by design).
- Determinism: no clocks, no randomness, no I/O.

## 2. Changes (uncommitted, working tree)

- New module `reference/crates/aether-state-model/src/fault.rs`; crate root declares
  `pub mod fault;`.
- `StateEngine` gains `objects()` and `operations()` (sorted iterators).
- 4 unit tests: clean multi-frame sweep is violation-free; corrupting one complete-frame
  byte is detected at later cuts (`RecoverFailed`); torn tail (mid-effect-frame truncation)
  stays clean; version regression (caller-chosen decreasing version ids) is flagged
  (`VersionRegressed`); a forged terminal→Pending effect frame is rejected at recovery
  (`RecoverFailed`).
- CTS runner: one new case (now **35 total**): `CTS-STATE-CRASH-001` (multi-frame clean
  sweep + corruption detected at later cuts).
- Verifier D1 addressed: firing tests added for `VersionRegressed`; a forged
  contradicting effect frame test pins the terminal-effect protocol boundary
  (framed frame → `apply_effect` → `valid_transition` rejection); reachability
  of `MissingObject`/`EffectRegressed` documented in module docs as defensive
  dead arms (untestable in the current append-only engine); D2 comment fixed.
- Round-2 verifier found **D6 (Moderate)**: the first version of the
  terminal-effect test appended an unframed payload, so recovery rejected it at
  the magic check without ever reaching `apply_effect` - the transition contract
  was unpinned. Fixed to `engine.append_frame(&encode_effect(...))` so recovery
  reaches `apply_effect` and rejects `Acknowledged -> Pending` via
  `valid_transition`. Mutation-checked: with `valid_transition` temporarily
  forced true, the test fails and the sweep reports `EffectRegressed`
  (arm confirmed live); reverted; production code unchanged.
- Round-3 verifier: **ALL PASS — GATE OPEN**. D3 doc nit fixed ("Iterates"
  → "Returns" for `objects()`/`operations()`); CTS-CRASH-001 fixtures and counts
  unchanged. Suite remains unsafe-free, 128/128 tests pass, 35/35 CTS green.

## 3. Verification actually executed (2026-11-19, Windows, Rust 1.95.0)

- `reference/check.sh`: exit 0 — fmt, locked check/test (debug+release), Clippy `-D warnings`,
  Rustdoc `-D warnings`, CTS 35/35.
- Root `./check.sh`: exit 0 — release evidence 151 files verified, supply-chain 7 packages.
- `cargo test --workspace`: **128 passed, 0 failed** (17 state-model).

## 4. Known limitations and boundaries

- Sweep cost is O(n log n)–O(n^2) in frame count for growing cut sets; fine for host
  reference logs, not for production crash exploration.
- `VersionRegressed`/`MissingObject` cannot trigger on a well-formed log; they exist to
  catch corruption that recovery mis-accepts (defense in depth), and currently only coincident
  cases through `RecoverFailed` are exercised by the tests.
- `EffectRegressed` covers terminal states only; uncertain/pending transitions are not
  cross-checked across cuts beyond the prefix property recovery already enforces.
- Audit-commitment and effect-byte-level regressions are covered only implicitly by the
  recover step; no independent commitment check is performed per cut.
- Uncommitted, unmerged, un-CI-validated. Coexists with eight other verified slices in the
  same working tree.

## 5. State and next actions

- Independent verifier review is the next step; findings to be recorded/resolved.
- Do NOT commit or push without the user's explicit decision (AGENTS.md).
- Remaining Phase A: RFC-0042 update/rollback governance (draft — acceptance is the user's
  decision), UASA interface contracts (data/owner-dependent), or RFC-0006 §5 fixtures
  (CTS-COMP-CONF-3/4) + §4 reporting. Phase B (QEMU RV64) needs the user's reference-target
  decision.
