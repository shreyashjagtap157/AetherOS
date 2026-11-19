# Handoff: RFC-0011 Layer 1 Scheduler State Model (Host Reference)

Classification: Operational
Authoritative Source: HANDOFF-SCHED-MODEL.md

**Date:** 2026-11-19
**Purpose:** Snapshot of the uncommitted working-tree scheduler slice so a new session can resume.
**Classification:** Operational

---

## 1. What this milestone is

Phase A deliverable of `docs/IMPLEMENTATION-ROADMAP.md` §3 item 5: an executable host model of
scheduler states. Until now, no scheduler code or model existed anywhere in the repository
(`docs/IMPLEMENTATION-STATUS.md` had no scheduler entry; RFC-0011 was normative text only).

This slice adds `aether-sched-model`, a bounded deterministic Layer 1 scheduler state model per
RFC-0011 §3 (Layer 1 only). It is **not** a production scheduler, **not** a proof of bounded
decision latency (RFC-0011.3/.6), and has no SMP, Layer 2 or Layer 3. It is a host reference in
the same sense as the RV64/Sv39 mapping model: executable semantics + conformance harness, not
kernel evidence.

Declaration required by RFC-0011.2/.3 for a declared algorithm class:

- **Class:** EDF (Earliest Deadline First), uniprocessor.
- **Selection rule:** least `(deadline_ns, tid)` among READY threads. Total order →
  identical inputs always select identically (RFC-0011.9).
- **Admissibility (RFC-0011.7, uniprocessor U = 1):** `sum(Ei / Pi) <= 1` with exact rational
  arithmetic in checked `u128` (per-step gcd reduction). Overflow fails closed with
  `SchedError::AccountingOverflow`; over-utilization fails with `NotAdmissible`.
- **Quantum (RFC-0011.11/.12):** each dispatch receives the configured quantum; on exhaustion
  the thread transitions `RUNNING → PREEMPTED` and is re-queued to `READY` with
  `deadline += period`.
- **Time model:** caller-supplied `advance(dt_ns)` deltas; the model never reads a clock.
- **Events:** RFC-0011.5-style transition events (`SchedEvent { thread, from, to, reason }`),
  including one `DeadlineMissed` event per deadline period (flag re-armed on re-queue).

Explicitly out of scope (recorded limitations): EEVDF and bounded-latency proofs; Layer 2
advisory tuning / Layer 3 offline learning; SMP and load balancing; RFC-0008 dispatch
integration; RFC-0028 synchronization integration; priority-aware (PEDLF) selection — priority
is stored per RFC-0011.8 (0–255) but deliberately **not** consulted by EDF selection.

## 2. Changes (uncommitted, working tree)

New crate `reference/crates/aether-sched-model/` (`src/lib.rs`, 10 unit tests). Workspace member
`aether-sched-model` added to `reference/Cargo.toml`; `reference/Cargo.lock` updated accordingly
(one new path package, no third-party dependencies).

CTS runner (`aether-cts-runner`): three new cases (now 22 total, exit 0):
- `CTS-SCHED-ADMIT-001` — exact admission: 1/2+1/3+1/6 = U exactly 1 admitted; a further thread rejected.
- `CTS-SCHED-DETERMINISM-001` — identical scheduling outcomes across permuted thread insertion orders.
- `CTS-SCHED-QUANTUM-001` — quantum expiry → Preempted → Ready with `deadline += period`;
  required transition events emitted in order; no phantom deadline miss.

`aether-cts-runner` gains dependency `aether-sched-model` (path).

## 3. Verification actually executed (2026-11-19, Windows, Rust 1.95.0)

- `reference/check.sh`: exit 0 — fmt, locked check/test (debug + release), Clippy `-D warnings`,
  Rustdoc `-D warnings`, CTS 22/22.
- Root `./check.sh`: exit 0 — repository policy, python bytecode, supply-chain (6 packages),
  reference gate, reproducible reference (`/Brepro`), release evidence 138 files verified.
- `cargo test --workspace`: **71 passed, 0 failed** (12 sched-model, including the D1/D2 regression tests).

## 4. Verifier findings and fixes (this milestone)

Adversarial verifier (orch-task-verifier) first verdict: GATE FAIL on CRIT-4.

- **D1 (Medium, FIXED):** `advance()` advanced a quantum-expired thread's
  deadline inside `requeue()` before the miss scan, dropping a deadline miss
  when quantum expiry and deadline crossing coincided in one call. Fixed by
  pre-validating (atomic call; `ClockOverflow` mutates nothing) and scanning
  misses against pre-tick deadlines before the quantum/preempt path.
  Regression test: `deadline_miss_is_charged_when_quantum_expiry_crosses_the_deadline`.
- **D2 (Low, FIXED):** a `requeue()` failure from `advance()` or
  `requeue_yielded()` could strand a thread in `PREEMPTED` (or re-queue
  partially). Both are now atomic via pre-validation. Regression test:
  `clock_overflow_pre_validation_leaves_state_untouched`.
- **D3–D6 (Info, unfixed by design / documented):** `DeadlineMissed` carries no
  miss-degree magnitude (RFC-0012.8) and no RFC-0031 envelope; extreme
  coprime denominators can trip `AccountingOverflow` on admissible sets
  (fail-closed, unsound-free); `remaining_budget_ns` is accounted but never
  enforced; `advance()` preempts on saturation (dt > quantum), the correct
  direction.
- Re-verification (same session, 2026-11-19): **ALL PASS** — CRIT-4 passes; both
  fixes verified by trace against current code (coincident-case miss charged
  exactly once) and by re-run (12/12 tests, 22/22 CTS). No new defects found.
  Residual, unchanged and accepted as out of scope: D3 (no miss-degree / no
  RFC-0031 envelope on `DeadlineMissed`), D4 (fail-closed `AccountingOverflow`
  incompleteness on extreme denominators), D5 (budget accounted, never enforced),
  D6 (saturating preempt), implicit event order (`DeadlineMissed` after
  `Requeued`, pre-tick state tag) — all documented here and in the module docs.

## 5. Known limitations and boundaries

- Selection is linear in READY count; no constant-time bound claimed (CTX-SCHED-001 open).
- `exit` from `BLOCKED`/`READY`/`YIELDED`/`RUNNING` is allowed (any non-`EXITED`); exiting frees
  the thread's admission share. `EXITED` is a terminal reservation for the tid (no reuse).
- `unblock` leaves the deadline unchanged and refreshes quantum; `advance` charges budget and
  quantum by saturation (host monotonic-delta semantics, not hardware scheduling time).
- Deadline misses are accounted one-shot per period; they do not affect admission after the fact
  and are not integrated with events outside the model.
- Not kernel-integrated; MemoryRuntime remains the only serialized runtime boundary.
- Uncommitted, unmerged, un-CI-validated. Coexists with the capability-lifetime slice from the
  same session in the same working tree (by instruction to continue, not to fork).

## 6. State and next actions

- Working tree: all changes listed by `git status --short` remain uncommitted and unpushed.
- Do NOT commit or push without the user's explicit decision (AGENTS.md permits local commits
  only with policy/authorization).
- Possible next steps (dependency-ready):
  1. Independent verification of this slice (in flight at time of writing).
  2. Composition host model (RFC-0002/0004) — remaining Phase A deliverable #5 item.
  3. UASA interface contracts review (Phase A #4) — needs user/owner input on evidence
     requirements; high ambiguity.
  4. Do not start Phase B (QEMU RISC-V64 vertical slice) before the user decides
     reference-target approval; see `docs/PRODUCT-FEASIBILITY-AND-SCOPE.md`.
