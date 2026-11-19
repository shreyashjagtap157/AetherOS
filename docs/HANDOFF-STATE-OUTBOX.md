# Handoff: Durable-Intent Outbox Dispatcher (PERSISTENCE §4 steps 5–8)

Classification: Operational
Authoritative Source: HANDOFF-STATE-OUTBOX.md

**Date:** 2026-11-19
**Purpose:** Snapshot of the uncommitted working-tree outbox slice so a new session can resume.
**Classification:** Operational

---

## 1. What this milestone is

`docs/PERSISTENCE-AND-EXTERNAL-EFFECTS.md` §4 defines eight steps for durable external
effects: validate authority → allocate id/idempotency key → persist intent atomically with
state → publish → capability-confined dispatcher sends → receiver deduplicates → outcome
recorded durably → retry per class/deadline. Steps 1–4 and step 7 (outcome recording,
uncertain reconciliation) were already implemented and verified in `aether-state-model`.
Steps 5, 6 and 8 existed only as normative text — nothing moved an intent from Pending to
Acknowledged against a receiver, deduped redelivery, or simulated the crash-after-send
uncertainty case end to end.

This slice adds `aether_state_model::outbox`, a deterministic host model of the dispatcher
and the receiver. It is **not** a network stack, transport, driver, payment rail, or
actuator; the receiver is a test double with explicit controls
(`rejecting(op)`, `crashing_after_send_on(op)`).

Declared semantics (module docs are authoritative):

- `Receiver` retains delivered operation ids (`seen`) — the dedup contract of §3 (I/D-class
  effects): duplicate offers are `Duplicate`, not re-executed.
- `crash_after_send_on(op)` simulates the applied-but-unacknowledged crash (step 8): the
  receiver applies the operation, then loses the acknowledgement; the dispatcher records
  `Uncertain` (per §8: never fictitious success or rollback).
- `OutboxDispatcher::dispatch`: for every `pending_effects()`, deliver to the receiver and
  record the outcome durably in the engine (`Acknowledged` / `Rejected` / `Uncertain`).
- `OutboxDispatcher::reconcile`: for every `uncertain_effects()`, re-attempt idempotently;
  dedup acknowledges; outcomes recorded the same way (PERSISTENCE §10 recovery reconciliation).
- Engine gains public `pending_effects()` / `uncertain_effects()` (sorted iterators).
- `DispatchReport` is a plain value; no events emitted (documented).
- Exactly-once is claimed for none of these paths; the contract is at-most-once with loss,
  at-least-once with deduplication, uncertainty elsewhere — per the document.
- Out of scope (recorded): real transports, capability-confined dispatch wiring into
  `aether-capspace`, retry classes/deadlines per effect class, authorization interlocks for
  irreversible effects, receiver durability across process restarts (the receiver double
  persists for the session; the model is single-machine), multi-node semantics.

## 2. Changes (uncommitted, working tree)

- New module `reference/crates/aether-state-model/src/outbox.rs`; crate root declares
  `pub mod outbox;`.
- `StateEngine` gains `pending_effects()` / `uncertain_effects()` (sorted).
- 4 unit tests: fresh deliveries acknowledged (`dispatch`), crash-after-send → Uncertain →
  reconcile acknowledges (dedup redelivery), receiver rejection recorded (`Rejected`),
  outcomes survive recovery.
- CTS runner: one new case (now **38 total**): `CTS-OUTBOX-DISPATCH-001`
  (commit → dispatch → both acknowledged; then a crash-after-send effect ends Uncertain
  and reconciles to Acknowledged on deduplicated redelivery).

## 3. Verification actually executed (2026-11-19, Windows, Rust 1.95.0)

- `reference/check.sh`: exit 0 — fmt, locked check/test (debug+release), Clippy `-D warnings`,
  Rustdoc `-D warnings`, CTS 38/38.
- Root `./check.sh`: exit 0 — release evidence 153 files verified, supply-chain 7 packages.
- `cargo test --workspace`: **137 passed, 0 failed** (22 state-model).

## 3.1 Verifier findings and fixes (round 1)

Adversarial verifier (orch-task-verifier) verdict: **GATE BLOCKED (C2, C4, C6)**.

- **D1/C2 (Medium, FIXED):** `Receiver::deliver` consumed the crash injection on ANY deliver() call, silently disarming it when the declared crash op was not delivered first; op ended `Acknowledged` (fictitious success). Fixed: crash injection now compares without consuming unless it matches (one-shot, fires only for the declared op). Regression tests: `outbox_crash_injection_fires_only_for_the_declared_operation` (crash on higher-id op 22 with [21,22] pending asserts 21 Acked / 22 Uncertain), plus an in-CTS sub-scenario (op 104) at CTS-OUTBOX-DISPATCH-001.
- **D2/C4 (Low, FIXED):** rejecting operations bypassed dedup (`Rejected` re-returned, `seen` insert dead). Documented instead of reordering (reject-then-dedup would flip a Rejected op to Acknowledged on reconcile, an illegal engine transition): rejecting operations are idempotently re-rejected; `seen` records processing, not outcome eligibility. Module docs updated.
- **D3/C6 (Low-Med, FIXED):** reconcile path was unpinned — a blind-ack reconcile (records Acknowledged without consulting the receiver) passed both the unit test and the CTS phase. Fixed at both layers: unit test reconciles against a receiver scripted to `rejecting(22)` and asserts `Rejected`; CTS-OUTBOX-DISPATCH-001 gains the same sub-scenario for op 104 (blind-ack would fail it).
- **D4/D5/D6/D7 (Low/Info, FIXED/documented):** PERSISTENCE §8/§10 citation drift removed (document has §1-§6); "capability-confined dispatcher" overclaim removed (confinement is future work in aether-capspace); EffectClass-unused / step-8-policy absence documented; engine-error-during-dispatch recovery path documented; module docs now state the capability/confinement, retry-class, R-class interlock, receiver-durability, and compensation boundaries explicitly.
- Round-2/3 verdict: **ALL PASS** — crash injection fires only for the declared op
  (second-order case re-verified empirically by the verifier's scratch re-run); reject
  path documented/coherent; blind-ack reconcile now fails both layers; no overclaim in
  module docs; fresh-target clippy clean; 137 workspace tests / 22 state-model / 38 CTS.
  No remaining defects; INFO nits recorded (CTS layer pins D3 not D1; `seen` insert in
  reject path is documented intentional bookkeeping).

## 4. Known limitations and boundaries

- The receiver double retains `seen` in memory; a real external receiver dedups on its own
  durable state — this models the receiver's contract, not its implementation.
- Effects with `EffectClass::Irreversible` get the same dispatch path as deduplicated ones;
  no authorization interlock is modeled (out of scope; §5 requires policy gating elsewhere).
- Two dispatch/reconcile passes are separate calls; ordering between a crashed pass and a
  subsequent reconcile is the caller's responsibility (demonstrated in the CTS case).
- `Receiver::crashing_after_send_on` fires exactly once (single deterministic injection);
  multiple crash points are not modeled.
- Uncommitted, unmerged, un-CI-validated. Coexists with nine other verified slices in the
  same working tree.

## 5. State and next actions

- Independent verifier review is the next step; findings to be recorded/resolved.
- Do NOT commit or push without the user's explicit decision (AGENTS.md).
- Remaining Phase A after this slice: RFC-0042 update/rollback governance (draft —
  acceptance is the user's decision), UASA interface contracts (data/owner-dependent),
  or RFC-0006 §5 fixtures + §4 audit emission. Phase B (QEMU RV64) needs the user's
  reference-target decision.
