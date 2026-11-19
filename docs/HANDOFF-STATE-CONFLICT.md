# Handoff: RFC-0016.8/.9 Snapshot Isolation and Write-Write Conflict Detection

Classification: Operational
Authoritative Source: HANDOFF-STATE-CONFLICT.md

**Date:** 2026-11-19
**Purpose:** Snapshot of the uncommitted working-tree transaction slice so a new session can resume.
**Classification:** Operational

---

## 1. What this milestone is

Phase A deliverable evidence for RFC-0016 (Transaction Model): the state reference
engine previously had failure-atomic commit, torn-tail recovery, replay rejection,
duplicate rejection, uncertain-effect recovery, and audit commitments — but **no**
concurrency semantics. RFC-0016.8 (minimum Snapshot Isolation) and RFC-0016.9
(write-write conflict detection; second writer aborts with `WriteWriteConflict`)
were unimplemented. This slice closes that gap for the single-node host model.

## 2. Changes (uncommitted, working tree)

`aether-state-model` (`reference/crates/aether-state-model/src/lib.rs`)
- `PreparedTransaction` now carries `base: HashMap<ObjectId, VersionId>`, a snapshot
  of the committed visible state as of `prepare()` (RFC-0016.8).
- New `PreparedTransaction::snapshot_read(object)`: own staged version if present
  (read-your-writes), otherwise the snapshot version; concurrent commits are
  invisible.
- `commit()` checks every staged mutation against the snapshot before any frame is
  appended: the first committer wins; a second writer aborts with
  `StateError::WriteWriteConflict` and **no frame is appended**, so a conflict
  abort leaves durable state untouched (all-or-nothing preserved).
- The durable frame format is unchanged: `base` is destructured out in
  `encode_commit` and never enters the log; existing replay/recovery behavior is
  untouched. `StateError` stays `Copy` (unit `WriteWriteConflict` variant; the
  failing object is identifiable from the staged mutation set).
- Recovery semantics unchanged: prepared-not-committed transactions vanish on
  crash, which is consistent with RFC-0016.13 (recovery of uncommitted
  transactions as ABORTED — they never became durable).

CTS runner: two new cases (now 28 total, exit 0):
- `CTS-STATE-CONFLICT-001` — first committer wins; second aborts with
  `WriteWriteConflict`; no frame appended on conflict; first commit visible.
- `CTS-STATE-SNAPSHOT-001` — snapshot reads exclude concurrent commits;
  read-your-writes applies; fresh transactions observe committed state.

## 3. Verification actually executed (2026-11-19, Windows, Rust 1.95.0)

- `reference/check.sh`: exit 0 — fmt, locked check/test (debug+release), Clippy `-D warnings`,
  Rustdoc `-D warnings`, CTS 28/28.
- Root `./check.sh`: exit 0 — release evidence 142 files verified, supply-chain 7 packages.
- `cargo test --workspace`: **88 passed, 0 failed** (12 state-model, incl. the three new
  conflict/snapshot tests).

## 4. Known limitations and boundaries

- Single-node, single-writer-at-a-time reference engine (host Mutex absent by design;
  `StateEngine` is `!Sync` in practice for the mutation paths via `&mut self`).
- No nested transactions (RFC-0016.10), no `FAILED` two-phase external coordination
  (RFC-0016.4 distributed participants), no RFC-0016.12 event emission, no lock-based
  blocking — conflicts are detected at commit (optimistic), never blocked.
- `snapshot_read` exposes only VersionIds, not payload bytes; payload isolation is
  out of scope for the reference model.
- Snapshot is a full clone of the visible map per `prepare()` — fine for the host
  model, not for production (documented; no memory benchmarks claimed).
- Uncommitted, unmerged, un-CI-validated. Coexists with the capability/scheduler/
  composition slices in the same working tree.

## 5. State and next actions

- Independent verifier (orch-task-verifier, same session as the other slices' reviews)
  verdict 2026-11-19: **ALL PASS** — no regression to the durable format; conflict check
  traced correct in all committed/never-committed cases; CTS cases proven non-vacuous by
  scratch-tree mutation experiments; no unsafe/panic paths outside tests. LOW/INFO findings
  recorded, unfixed by design (D1: commit's append→apply atomicity rests on an unreachability
  invariant, not structure; D2: `snapshot_read` returns the first staged duplicate — duplicates
  rejected at commit; D3: public `visible_version()` bypasses SI; D4: no golden byte-vector
  frame test; D5: RFC-0016.4/.10/.12/.13/.14 unimplemented; D6: `encode_commit`'s `..` drops
  future fields from the audit commitment silently). Slice is safe to keep as the narrow
  claim; none of these are merge blockers.
- Do NOT commit or push without the user's explicit decision (AGENTS.md).
- Remaining Phase A: update/rollback model (RFC-0042 is a draft whose acceptance is
  evidence-gated — needs user/governance input, not silent implementation), outbox
  effect depth, UASA interface contracts (blocked on evidence reproduction and owner
  input), composition conformance fixtures (RFC-0006), or the first Phase B planning
  slice once the user approves a reference-target decision.
