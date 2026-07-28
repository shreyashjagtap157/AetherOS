# UASA–AetherOS Integration Review

Classification: Operational
Authoritative Source: UASA-INTEGRATION-STATUS.md, Platform-Architecture-Specification-v1.1.md
Requirement-ID: GOV-UASA-001
Status: Review Packet Available; Implementation Evidence Pending

## 1. Source acquisition and evidence boundary

The UASA review packet was retrieved from `origin/main` at merge commit `2fae160` (`docs/UASA_COMPREHENSIVE_RESEARCH_REVIEW_PACKET.md`) on 2026-07-27. AetherOS classification and import-provenance metadata were added; the packet body was otherwise copied without alteration.

This resolves the previous **missing document** blocker, but not the implementation-evidence blocker. The packet explicitly says that the UASA repository was unavailable when the packet was prepared and that its described files, tests, state counts, gates, and results were reported rather than independently reproduced. AetherOS therefore treats the packet as an informative research handoff—not as an accepted filesystem specification, implementation, benchmark, or proof.

## 2. Classification and integration decision

UASA is best classified as a **provider-independent semantic storage architecture with a transactional object/version/root model and emerging physical execution providers**. It is not yet a mountable filesystem or a production replacement for existing filesystems.

The design is directionally compatible with AetherOS State Architecture because both separate logical meaning from replaceable storage mechanisms, require explicit transaction and recovery semantics, preserve identity independently from projection, and constrain claims to available evidence. UASA SHALL initially enter AetherOS as an experimental storage-provider/semantic-engine candidate. It SHALL NOT replace RFC-0015 through RFC-0023 or become boot-critical until the conflicts and evidence gates below are closed.

## 3. Strong alignment

| UASA concept | AetherOS mapping | Initial assessment |
|---|---|---|
| Stable ObjectID independent of name and placement | RFC-0015 object identity | Strong semantic alignment; representation and collision protocol still require agreement |
| Immutable versions, roots, snapshots, visibility | RFC-0015/RFC-0016/RFC-0018 | Suitable refinement candidate |
| Semantic engine owns meaning; provider reports physical facts | RFC-0019 replaceable storage provider | Strong alignment and useful provider non-interference rule |
| Explicit transaction chain before visibility/root advancement | RFC-0016 transactional persistence | Stronger physical-placement refinement for zoned media |
| Evidence-bounded durability | RFC-0016/RFC-0017 and verification tiers | Adopt as a required durability vocabulary rather than a Boolean |
| Unknown physical outcomes | AetherOS explicit failure semantics and external-effect reconciliation | Adopt as a first-class state; never convert timeout into assumed failure |
| PlacementEvidence before visible ZNS mapping | State integrity and recovery | Promising ZNS-specific contract |
| Orphans, obligations, and safe reclamation | RFC-0018 lifecycle | Requires a single combined obligation algebra |
| Capability ∩ policy ∩ context | AetherOS capability authority | Potentially compatible, but UASA must consume—not redefine—the AetherOS rights algebra |
| Canonical semantic projection and differential providers | CTS/provider equivalence | Strong validation technique if normalization cannot erase errors |

## 4. Required architectural resolutions

### 4.1 Authority ownership

AetherOS owns authority semantics through RFC-0043 and the active capability provider. UASA may define storage-specific object types and operations but SHALL NOT create a parallel capability format, root authority, revocation system, identity principal, or ambient recovery privilege. Every semantic transaction, physical request, placement mapping, reclamation action, migration, and repair operation must carry an attenuated AetherOS authority reference.

Recovery requires an explicitly bootstrapped recovery capability and auditable policy; “fail closed” cannot silently invent authority when normal identity services are unavailable.

### 4.2 Identity mapping

UASA ObjectID maps to AetherOS persistent object identity only after the following are specified: byte format and version; generation/collision behavior; UUIDv7 clock rollback; import and migration identity ownership; clone semantics; tenant/security-domain scoping; deletion and reuse; namespace binding; and optional content identity privacy.

Object identity, immutable version identity, content identity, namespace identity, and physical placement identity remain distinct. A path, inode, LBA, zone, provider handle, or content digest alone is not the persistent ObjectID.

### 4.3 Transaction and visibility mapping

The candidate refinement is:

```text
PREPARED
  -> ALLOCATED
  -> PHYSICALLY_PLACED
  -> MAPPING_PERSISTED
  -> VERSION_VISIBILITY_COMMITTED
  -> ROOT_ADVANCED
  -> DURABILITY_CONFIRMED(contract, failure_domain, evidence)
```

This is not assumed to be one universal synchronous path. Batching, group commit, mapping indirection, and pipelining are permitted when the externally observable refinement is equivalent. Visibility may not reference guessed placement. Root advancement may not expose an uncommitted version. Durability may not exceed evidence.

The relationship between UASA transaction IDs and AetherOS transaction/audit commitment IDs must be one-to-one or explicitly nested, never merely correlated by timestamps.

### 4.4 Unknown outcomes and external effects

Lost completions, controller resets, provider death, cancellation races, or inaccessible device state produce `PHYSICAL_OUTCOME_UNKNOWN`. Blind resubmission is forbidden when it could duplicate a physical write or mapping. Reconciliation uses stable request identity, payload digest, controller/namespace/zone identity, queue generation, returned placement evidence, and durable intent records.

An unknown outcome remains visible to recovery and operators until proven placed, proven not placed, safely superseded, or quarantined. The implementation must integrate this state with `PERSISTENCE-AND-EXTERNAL-EFFECTS.md` rather than inventing a second outbox/reconciliation protocol.

### 4.5 Durability vocabulary

Every API result declares a durability contract and failure domain. At minimum the vocabulary distinguishes: accepted by semantic engine; submitted to provider; completed by provider; mapping journal persisted; device flush/FUA evidence; controller-cache assumptions; host power-loss survival; media persistence; replicated quorum; geographically independent durability; and recovery verification.

`success` without a declared durability level is invalid. Flush, FUA, barriers, kernel completion, NVMe completion, and physical power-loss survival are not interchangeable.

### 4.6 Audit overhead

UASA state transitions SHALL feed the AetherOS batched audit commitment. The authoritative transaction commitment covers request identity, version/root change, mapping-evidence digest, authority, durability contract, and unknown-outcome state. Full telemetry and raw controller status may be stored out of line. This avoids synchronous formatting/export per extent without weakening attributable commit semantics.

### 4.7 Provider contract

An ExecutionProvider may allocate buffers, submit/cancel operations, poll completions, report discovered capabilities, return raw status and placement evidence, and expose observed durability facts. It may not mark a version visible, advance a root, declare semantic commit, erase uncertainty, select authority, or reclaim live data.

The common provider contract must support a deterministic reference provider, conventional block provider, Linux `io_uring` research provider, SPDK/NVMe provider, ZNS provider, and a no-op provider used to measure semantic-engine overhead. Linux and SPDK providers are development tools/compatibility implementations, not dependencies of the eventual AetherOS kernel.

## 5. ZNS admission gates

ZNS integration remains experimental until all gates pass on emulation and real named hardware:

1. Static geometry is separated from dynamic zone state.
2. Open/active-zone limits are enforced under concurrency.
3. Zone Append uses device-returned placement; requested write pointer is never treated as final placement.
4. PlacementEvidence is bound to stable request, payload, provider, controller, namespace, zone, and queue generation.
5. The mapping is checksummed and durably persisted before version visibility.
6. Conventional-block and ZNS executions are observationally equivalent under a reviewed canonical projection.
7. Near-full, fragmentation, reset, cancellation, timeout, wrap, stale completion, and provider-death stress pass.
8. Unknown outcomes reconcile without blind retry.
9. Crash injection at every transition yields only an allowed recovered state.
10. Physical power-loss results do not exceed the declared durability contract.

## 6. Filesystem/product surface decision

The first recommended AetherOS use is **not** a root filesystem. It is a host-side and later user-space transactional object store for package/artifact snapshots or VM/container image versions. This exercises stable identity, versions, roots, migration, audit commitments, and provider equivalence while limiting POSIX and boot-recovery risk.

Only after that vertical slice passes should the project add a native file projection, then a constrained POSIX projection. POSIX rename, hard links, symlinks, sparse files, mmap, direct I/O, locks, append, permissions, quotas, fsync/fdatasync, directory durability, and crash outcomes each require explicit mappings and differential tests.

## 7. Evidence required before implementation claims

The underlying UASA repository and exact commit must be supplied with reproducible environment lockfiles, specification sources, formal models/configurations, reference semantics, provider code, complete tests, logs, model-checker output, mutation results, benchmark artifacts, hardware/firmware identifiers, and licenses.

Reported model checking establishes bounded model behavior only. It does not prove the implementation, arbitrary scale, unmodeled concurrency, real devices, or power-loss behavior. Acceptance requires independent reruns, documented state bounds and assumptions, refinement mapping, hostile-provider tests, mutation testing, coverage-guided fuzzing, sanitizers, crash injection, corrupted-media recovery, and real power-cut testing.

## 8. AetherOS implementation work packages

| Work package | Deliverable | Blocks |
|---|---|---|
| UASA-INTF-01 | Object/version/root/namespace mapping contract | semantic prototype |
| UASA-INTF-02 | AetherOS capability and recovery-authority binding | any privileged use |
| UASA-INTF-03 | Transaction/durability/unknown-outcome state refinement | provider implementation |
| UASA-INTF-04 | ExecutionProvider IDL and hostile-provider CTS | real backend |
| UASA-INTF-05 | Append-only reference provider with deterministic faults | differential oracle |
| UASA-INTF-06 | Conventional block mapping journal | ZNS comparison |
| UASA-INTF-07 | ZNS PlacementEvidence and durable mapping | ZNS visibility |
| UASA-INTF-08 | Obligation/reclamation algebra | garbage collection |
| UASA-INTF-09 | Native file and POSIX projection semantics | application compatibility |
| UASA-INTF-10 | Repair, scrub, export/import, upgrade, and rollback tools | production candidacy |

## 9. Production admission

UASA may be called an AetherOS production storage provider only after it has a stable format or migration contract; bounded memory and metadata amplification; fsck/scrub and forensic tools; online/offline repair; backup/restore; key rotation and cryptographic agility; quota and denial-of-service controls; multi-version upgrade/rollback; power-loss campaigns; hardware qualification; independent security review; performance and endurance baselines; documented operational procedures; and a named maintenance/security-response team.

Until then the accurate status is: **research architecture available for integration modeling; underlying implementation and reported evidence not yet independently available or reproduced**.

## 10. Immediate next action

Obtain the underlying UASA repository and independently reproduce the smallest decisive experiment: one Zone Append whose device-returned location becomes bound PlacementEvidence, whose mapping is durably recorded before visibility, and whose completion is deliberately lost to force deterministic unknown-outcome reconciliation after restart. This single vertical slice tests the central semantic/physical boundary with the greatest information value.
