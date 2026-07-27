# Persistence, Audit, and External-Effect Semantics

Classification: Operational
Authoritative Source: Platform-Architecture-Specification-v1.1.md, RFCs/02-State/RFC-0016.md, RFCs/04-Observability/RFC-0031.md
Requirement-ID: GOV-EFFECT-001
Status: Research Baseline

## 1. Scope

This document supplies the implementation research baseline for observable persistence without forcing one expensive audit record per byte write, and for coordinating transactions with real-world effects that cannot be rolled back. It does not claim that arbitrary physical effects are atomic.

## 2. Audit commitment before commit

A transaction constructs one canonical **audit commitment** before its durable commit point. The commitment covers transaction identity, actor, delegated authority reference, ordered or canonicalized mutation-set digest, affected object identities, policy decision, causal parent, monotonic sequence/epoch, and intended external effects. Many mutations MAY be batched under one commitment. The full diagnostic payload MAY be stored out of line, but the authoritative digest and required attribution cannot be sampled away.

The storage provider atomically persists the state commit record and audit commitment in one write-ahead-log group or equivalent failure-atomic unit. Publication occurs only after durability rules are satisfied. Observability consumers receive asynchronous projections after commit. This preserves the semantic invariant while removing tracing, formatting, network export, and indexing from the commit hot path.

Implementations SHALL benchmark single-object, batched, contended, and recovery paths; bound queue memory; apply backpressure rather than silently lose authoritative audit data; redact secrets before canonicalization; and cryptographically chain checkpoints rather than signing every individual mutation.

## 3. External-effect classes

| Class | Examples | Contract |
|---|---|---|
| P | Local persistent state | Atomic commit/abort |
| I | Idempotent effect | Retry using stable operation key |
| D | Deduplicated effect | Receiver records operation key before action |
| C | Compensatable effect | Commit plus explicit compensating action; not rollback |
| R | Irreversible effect | Human/policy authorization and durable intent before dispatch |
| O | Observational input | Record value/provenance; cannot be rolled back |

Network transmission, actuator movement, printing, external payment, and device firmware operations SHALL NOT be advertised as transactionally rolled back.

## 4. Durable intent protocol

1. Validate authority and preconditions.
2. Allocate a globally unique operation ID and idempotency key.
3. Persist state mutations, audit commitment, and an outbox intent atomically.
4. Publish the committed state.
5. A capability-confined effect dispatcher sends the intent.
6. The receiver deduplicates where supported and returns evidence.
7. Persist acknowledged, rejected, uncertain, or compensation-required outcome.
8. Retry only according to the effect class and deadline.

A crash between steps 4 and 7 yields **uncertain**, not fictitious success or rollback. Operators and applications receive an explicit reconciliation handle. Exactly-once delivery is claimed only inside a proven common transaction domain; elsewhere the contract is at-least-once plus deduplication, at-most-once with possible loss, or explicit uncertainty.

## 5. Device and DMA effects

Drivers expose prepare/submit/cancel/query/reconcile where hardware permits. DMA buffers remain capability-owned until completion or reset evidence exists. Device reset is not assumed to undo a command. Irreversible commands require a durable intent and, for safety-critical profiles, a policy interlock.

## 6. Research prototype

The first executable state prototype SHALL implement a host-side append-only log with checksummed frames, transaction and outbox records, crash truncation recovery, idempotency keys, Merkle/checkpoint commitments, and deterministic fault injection at every persistence boundary. It precedes selection of an on-disk filesystem.
