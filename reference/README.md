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
```

The runner emits newline-delimited JSON and exits non-zero on any failed case.

## Current evidence

- Explicit root creation; no public ambient mint operation.
- Monotonic rights and bounds attenuation.
- Domain-local generational handles.
- Explicit cross-domain transfer requiring `GRANT`.
- Lineage revocation across derived and transferred capabilities.
- Stale-handle, integer-forgery, type, bounds, and rights rejection.
- Differential scenarios against the abstract and table-backed providers.

## Non-goals for this slice

- Canonical 96-byte import/export records or cryptography.
- Persistence, distributed authority, time/expiry, or recovery authority.
- Concurrent capability-space access.
- Kernel, CHERI, or hardware-provider integration.

