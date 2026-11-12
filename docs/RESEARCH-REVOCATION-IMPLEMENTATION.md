# Capability Revocation Implementation Research and Decision

Classification: Operational
Authoritative Source: This document, constrained by RFC-0038 and RFC-0039
Requirement-ID: GOV-REV-001
Status: Experimental implementation decision — host reference only
Last reviewed: 2026-11-12

## 1. Scope and decision boundary

This record evaluates the host Software Capability Provider's revocation data structure. It does not amend the architecture, RFC-0038, RFC-0039, the canonical capability-token format, or the production verification tier. It records a prototype implementation choice and its evidence boundary.

The relevant AetherOS contract is transitive revocation across both derivation and transfer. RFC-0039 §3 expects derivation and transfer graphs to be maintained incrementally and traversed during revocation. RFC-0039.11 additionally requires a declared, testable completion bound. That last requirement is not yet satisfied by this host prototype.

## 2. Baseline problem

Before this change, each live provider entry stored its entire ancestor lineage in a `Vec<u64>`. Both authorization and the provider's live-parent check scanned that vector and checked each ancestor against a revoked-ID set. For a capability with derivation depth d, this adds O(d) revocation work to each such lookup. It also duplicates ancestor identifiers across descendants.

The existing abstract `aether-cap-model` intentionally retains this ancestry-based approach. It is useful as a structurally independent semantic oracle for differential testing.

## 3. Research and alternatives

### 3.1 Indexed derivation/transfer tree — selected for the reference provider

Each capability receives one lineage-node identifier. A node records its parent, child identifiers, whether it is revoked, and whether a live local handle still refers to it. Derivation and transfer both append an edge from the source node to the new node.

Revocation first collects and validates the complete reachable subtree, then marks every node in that subtree revoked. The traversal is iterative rather than recursive. Authorization checks the resolved node's revocation flag rather than scanning all ancestors.

This matches the broad derivation-tree approach documented by seL4, while preserving AetherOS's current semantic result (`CapError::Revoked`) instead of deleting descendant slots. It is a prototype design choice, not a claim of semantic equivalence with the seL4 kernel.

### 3.2 Per-entry ancestor vectors — rejected for the provider hot path

This remains the independent oracle design. It is simple and directly expresses ancestor validity, but lookup cost and per-entry storage grow with derivation depth. It is retained in the model so the optimized provider is not its own only correctness oracle.

### 3.3 Epoch or generation schemes — not selected yet

Epoch-based invalidation can make selected revocation checks inexpensive, but the exact semantics for revoking an arbitrary intermediate capability, preserving unaffected siblings, transferring across domains, and reusing identifiers need more design work. A single global epoch would over-revoke unrelated branches. More elaborate schemes must be compared against the explicit graph before adoption.

### 3.4 Delete-on-revoke semantics — not selected

seL4's documented `CNode_Revoke` deletes child capabilities in its capability derivation tree. AetherOS's current provider and CTS expect descendant handles to remain resolvable but fail authorization with a typed revoked result. Deletion, object lifetime, and handle-removal semantics must therefore remain separate until the AetherOS contract is deliberately amended.

## 4. Implemented host-reference design

- A provider entry stores one lineage-node identifier, not a copied vector of all ancestors.
- The provider maintains parent/child links for both same-domain derivation and cross-domain transfer.
- Revocation validates the complete reachable closure before changing flags, avoiding a partially marked subtree if an internal graph node is missing.
- The subtree is marked eagerly; later authorization performs one revocation-node lookup.
- Graph traversal is iterative and cycle-tolerant through a visited set.
- Removing a handle marks its node as no longer live. A node is reclaimed only when it has no live handle and no children. Reclamation walks upward, preserving intermediate nodes that are still ancestors of live descendants.

## 5. Complexity and limits

Let k be the number of nodes in the revoked subtree and e the number of child edges traversed.

- Authorization: expected O(1) revocation-node lookup after handle resolution, using the host provider's `HashMap`. This is not a worst-case execution-time proof.
- Revocation: O(k + e) traversal and marking, plus temporary memory proportional to the affected subtree.
- Graph storage: O(n + e) for the retained lineage graph. Retired leaf paths are pruned once no live descendants depend on them.
- Handle removal: may prune a chain of now-unneeded ancestor nodes; removing a node also updates its parent's child vector.

This design moves work from every authorization to revocation. It does **not** establish a fixed revocation deadline. Dynamic allocation, hash-table behavior, subtree size, synchronization, cross-core visibility, notifications, and hardware mapping invalidation remain outside this prototype's evidence. The reference implementation is single-threaded and does not model concurrent revoke/use linearization.

## 6. Validation added

The host provider unit tests cover:

- Revoking one child branch without revoking its parent or sibling.
- Revocation propagation through a transfer into another domain.
- Revocation through an intermediate lineage node whose local handle has been removed.
- Reclamation of retired lineage metadata after the last live descendant is removed.
- Iterative revocation over a deep derivation chain.

The machine-readable CTS adds a provider-versus-independent-model case for subtree scope, transfer propagation, and preservation of the parent and sibling. The model is intentionally not rewritten to use the provider's graph.

The PR-branch CI run on 2026-11-12 passed 10 `aether-capspace` unit tests, 13 machine-readable CTS cases, the other reference-model suites, the reproducibility gate, and release-evidence validation for 130 files. A passing build or CTS run is reference-model evidence only. It does not qualify a kernel implementation or prove the RFC-0039.11 performance bound.

## 7. Outstanding research and implementation work

1. Compare graph traversal with bounded-capacity arrays, indexed arenas, intrusive derivation lists, per-domain revocation epochs, and object-generation invalidation.
2. Establish capability-count and subtree-size budgets for each deployment profile.
3. Define a linearization point and concurrency protocol for revoke versus derive, transfer, authorize, and remove.
4. Specify event delivery and holder-notification semantics without making revoked authority usable while notification is delayed.
5. Integrate address-space unmapping, page-table permission changes, and TLB shootdown into the memory-revocation contract.
6. Define the durable/auditable revocation record and its relationship to the RFC-0017 integrity model.
7. Add deterministic operation-count tests and reproducible microbenchmarks; do not infer WCET from wall-clock microbenchmarks.
8. Evaluate static or bounded data structures for a kernel implementation. The current randomized `HashMap` is a host-reference convenience, not an accepted kernel hot-path structure.
9. Preserve the independent model and add generated operation-sequence differential tests.

## 8. Sources and provenance

- AetherOS `RFC-0038`: capability transfer and revocation lineage.
- AetherOS `RFC-0039`: transitive revocation, incremental derivation/transfer graphs, notification, audit, and completion-bound obligations.
- seL4 API reference, `CNode_Revoke`: https://docs.sel4.systems/projects/sel4/api-doc.html
- seL4 Reference Manual, §3.2 Deletion and Revocation: https://sel4.systems/Info/Docs/seL4-manual-9.0.0-mcs.pdf
- Systems Research @ GWU, capability-based OS design discussion of delegation trees and revocation latency trade-offs: https://www2.seas.gwu.edu/~parmer/posts/2016-04-06-capability-based-design.html

The cited systems are comparative prior art. Their implementation details and assurance results are not inherited by AetherOS.
