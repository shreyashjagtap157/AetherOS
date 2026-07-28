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
`check.sh` is the complete local quality gate: formatting, all-target and
all-feature compilation, debug and release tests, Clippy with warnings denied,
rustdoc with warnings denied, and the executable CTS.

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
- Differential scenarios against the abstract and table-backed providers.
- Failure-atomic transaction frames with persisted, independently validated
  deterministic audit commitments.
- State and external-effect intents committed in one recoverable record.
- Torn-tail recovery, complete-frame corruption detection, and explicit
  uncertain-effect reconciliation.
- Duplicate object mutations, operation identifiers, and replayed transaction
  frames are rejected rather than silently overwriting recovered state.
- Capability slot and lineage identifier exhaustion returns a typed failure
  instead of wrapping or panicking.
- Transaction identifiers are allocated once and fail closed before wraparound;
  recovery bounds-checks attacker-controlled record counts before allocation.

## Non-goals for this slice

- Canonical 96-byte import/export records or cryptography.
- Production persistence formats, distributed authority, time/expiry, or
  recovery authority.
- Concurrent capability-space access.
- Kernel, CHERI, or hardware-provider integration.

The reference log uses a non-cryptographic checksum solely to exercise framing,
corruption detection, and recovery semantics. It must not be reused as the
production integrity or audit-commitment algorithm.
