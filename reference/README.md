# AetherOS Executable Reference

This workspace is the first non-kernel implementation artifact. It turns the
candidate capability semantics into deterministic, executable Rust and runs a
small machine-readable conformance suite against both the abstract model and a
table-backed software capability provider.

It is deliberately **not** a stable ABI, production kernel, cryptographic
capability format, or proof. The handle layout and algorithms are prototype
implementation details. Normative requirements remain in `../../docs/`.

## Run

```sh
cargo test --workspace
cargo run -p aether-cts-runner
./check.sh
```

The runner emits newline-delimited JSON and exits non-zero on any failed case.
The repository-root `../check.sh` is the complete local gate. This directory's
`check.sh` is the Rust reference gate: formatting, all-target and all-feature
compilation, debug and release tests, Clippy with warnings denied, rustdoc with
warnings denied, and the executable CTS.

## Current evidence

- Explicit root creation; no public ambient mint operation.
- Bootstrap witnesses are bound to one authority universe and cannot mint roots
  in another model or provider instance; identity does not rely on a wrapping
  process-global integer counter.
- Monotonic rights and bounds attenuation.
- Domain-local generational handles.
- Explicit cross-domain transfer requiring `GRANT`.
- Lineage revocation across derived and transferred capabilities.
- Revoked authority cannot be used to derive or transfer new handles.
- Both reference providers reject unregistered destination domains.
- Stale-handle, integer-forgery, type, bounds, and rights rejection.
- Scheduling-time-aware lifetime enforcement (RFC-0037.14): zero
  `expires_at_ns` never expires; a capability whose `expires_at_ns` equals the
  current scheduling time is still valid; the first instant afterwards is
  rejected everywhere (`authorize`, `derive`, `transfer`, and every
  memory-access revalidation path). Derived and transferred children inherit
  the parent's expiry; they never outlive it.
- Capability flags (RFC-0037.7) are modeled explicitly, including `CAPF_BOUND`:
  a bound capability cannot be transferred to any domain (model and provider
  agree); same-domain use and derivation remain valid.
- Differential scenarios against the abstract and table-backed providers.
- Failure-atomic transaction frames with persisted, independently validated
  deterministic audit commitments.
- State and external-effect intents committed in one recoverable record.
- Deterministic host model of the durable-intent outbox protocol
  (PERSISTENCE-AND-EXTERNAL-EFFECTS.md §4 steps 5-8): a capability-confined
  dispatcher offers pending intents to a test-double receiver that deduplicates
  by operation id; outcomes are recorded durably in the state engine
  (Acknowledged / Rejected / Uncertain); a crash-after-send injection leaves
  the intent Uncertain, and idempotent redelivery reconciles it to acknowledged.
- Deterministic crash-consistency fault injection (PERSISTENCE-AND-EXTERNAL-EFFECTS.md
  §6): a byte-granularity sweep of the entire durable log verifying that
  recovery succeeds at every cut, object versions never regress or vanish,
  and terminal effect states are never downgraded by a crash.
- Snapshot-isolated prepared transactions (RFC-0016.8): reads observe the
  `prepare`-time snapshot plus the transaction's own mutations; concurrent
  commits are invisible. Write-write conflict detection (RFC-0016.9) aborts the
  second committer with `WriteWriteConflict` before any frame is appended.
- Torn-tail recovery, complete-frame corruption detection, and explicit
  uncertain-effect reconciliation.
- Duplicate object mutations, operation identifiers, and replayed transaction
  frames are rejected rather than silently overwriting recovered state.
- Capability slot and lineage identifier exhaustion returns a typed failure
  instead of wrapping or panicking.
- Transaction identifiers are allocated once and fail closed before wraparound;
  recovery bounds-checks attacker-controlled record counts before allocation.
- Component manifest validation and dependency resolution host model (RFC-0002,
  RFC-0004): RFC-0002.19 manifest refusal; RFC-0004.4 candidate refusal; exact
  version-range matching; HighestVersion selection strategy (declared) with
  fail-closed ambiguity detection; profile and tier compatibility checks;
  transitive resolution with Kahn topological ordering; cycles reported with
  sorted members; deterministic instantiation order.
- Deployment profile selection host model (RFC-0005): mandatory/declared-optional
  mechanism activation; default-off layered extensions with explicit opt-in;
  forbidden-union composition across profiles (most restrictive wins, suppressed
  with a reasoning record); tier-floor rejection per manifest; auditable
  reasoning vector on every decision.
- Composition conformance declaration host model (RFC-0006): mandatory core fixture
  coverage (executed, substituted per RFC-0006.6, or omitted with justification),
  methodology §10 failure-mode classification enforcement, profile subset coverage,
  deterministic content-derived artifact identifier (methodology §8), conformant/gap
  computation from the fixture vector.
- Capability routing decision host model (RFC-0003): grant derivation bounded by
  `grant.rights_ceiling ∩ grantor authority` (RFC-0003.3/.4), `Attenuate` and
  `Exact` narrowing policies, `GrantNotDerivable` / `NarrowingFailed` /
  `ManifestInvalid` denial reasons, environmental independence (pure function of
  the four declared inputs), granted authority inherits the grantor's resource,
  type, bounds, flags, expiry.
- Deterministic Layer 1 (uniprocessor EDF) scheduler state model (RFC-0011 §3):
  enforced state machine (`READY`/`RUNNING`/`BLOCKED`/`YIELDED`/`PREEMPTED`/
  `EXITED`), exact admission bound `sum(Ei/Pi) <= 1` in checked rational
  arithmetic (fails closed on overflow), deterministic selection by
  `(deadline, tid)`, per-dispatch quantum with preempt/requeue advancing the
  EDF deadline by one period, RFC-0011.5 transition events, and one-shot
  deadline-miss accounting per period.

## Crates

| Crate | Role |
|-------|------|
| `aether-types` | Provider-neutral capability types, handles, errors, flags |
| `aether-cap-model` | Abstract semantic oracle (map-based, differential reference) |
| `aether-capspace` | Table-backed software capability provider + RV64/Sv39 mapping model + serialized runtime |
| `aether-state-model` | Transactional state, durable-effect recovery, `fault` crash-consistency sweep, and `outbox` durable-intent dispatcher host model |
| `aether-sched-model` | Deterministic Layer 1 scheduler state model (RFC-0011 §3, uniprocessor EDF) |
| `aether-comp-model` | Component manifest, dependency resolution, profile selection, capability routing and conformance declaration host model (RFC-0002/RFC-0003/RFC-0004/RFC-0005/RFC-0006) |
| `aether-cts-runner` | Machine-readable CTS harness (differential model-vs-provider scenarios) |

## Non-goals for this slice

- Canonical 96-byte import/export records or cryptography.
- Production persistence formats or distributed authority. Time/expiry is now
  modeled semantically (RFC-0037.14, scheduling time supplied by the caller);
  production clock sources, monotonicity guarantees, and recovery authority
  remain out of scope.
- Concurrent capability-space access.
- Kernel, CHERI, or hardware-provider integration.
- Canonical RFC-0037 seal verification, `rights_derived` derivation counters,
  and creation timestamps; the flags model is the RFC-0037.7 bit semantics
  only, not the canonical 96-byte layout.

The reference log uses a non-cryptographic checksum solely to exercise framing,
corruption detection, and recovery semantics. It must not be reused as the
production integrity or audit-commitment algorithm.
