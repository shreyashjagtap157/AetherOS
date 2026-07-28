# Platform Architecture Specification v1.1

Classification: Constitutional
Authoritative Source: Platform-Architecture-Specification-v1.1.md
Requirement-ID: ARCH-INV-001, ARCH-Peer-001, ARCH-Boot-001, ARCH-Sched-001, ARCH-Persist-001, ARCH-Dist-001, ARCH-HW-001, ARCH-Profile-001

**Status:** Validation Candidate. The v1.1 text is the current constitutional baseline, but implementation, executable-model, security, compatibility, and performance evidence may require governed amendment. It SHALL NOT be described as frozen until the Phase A and Phase B evidence gates in `IMPLEMENTATION-ROADMAP.md` pass.

**Path of progression:** v1.0 → v1.1 (see `Revision-History.md` for material changes).

---

## 1. Scope and Status

This document defines the architecture of a general-purpose computing platform. It is the top-level specification that all subsequent RFCs must conform to.

**Validation Candidate** means the invariants remain authoritative design hypotheses and change only through an explicit Architecture Amendment RFC, but the project actively seeks evidence that may justify amendment. This status prevents implementation inconvenience from silently changing the architecture without pretending that an unimplemented design is empirically final.

This document does **not** specify:
- Hardware implementation
- Compiler internals
- Application-level ABI (defined separately in the compiler/runtime RFC series, listed as future-direction RFCs)
- Wire-format protocols (defined by Communication architecture RFCs)

This document **does** specify:
- The invariant set every conforming implementation must satisfy
- The five peer architectures and their boundaries
- The capability security model at architectural level
- The verification tier system
- The complexity budget heuristic
- The persistence–state distinction
- The composition contract model
- The observability requirement
- The deployable profiles
- The root-of-trust and bootstrap model
- The authority–address orthogonality invariant
- The economic cost of verification
- The quantifiable complexity metric recommendations

---

## 2. Architectural Invariants

These invariants are normative. Every conforming implementation must satisfy them. The full Conformance Test Suite is defined in `Conformance-Test-Suite-Methodology.md`; this document names the invariants only.

| #     | Invariant                                                                    | Statement                                                                                                                                                                       |
|-------|------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| I-1   | Authority mediation                                                          | Every resource access is authorized through a capability token. A capability is necessary but not sufficient — the architecture additionally requires a separately defined address for the resource to be reachable. |
| I-2   | No ambient authority                                                         | No process, thread, or component has authority it has not been explicitly granted through capability transfer.                                                                  |
| I-3   | Least privilege by default                                                   | Newly created capabilities grant the minimum rights necessary; rights elevation requires explicit derivation.                                                                   |
| I-4   | Observable state transitions                                                 | Every persisted state mutation produces an observable, attributable, and auditable event before commit.                                                                        |
| I-5   | Transactional persistence                                                    | No persistent mutation becomes externally visible until its enclosing transaction commits atomically.                                                                          |
| I-6   | Cryptographic integrity                                                      | Every identifiable persistent object is verifiable against its declared content-addressed identity.                                                                            |
| I-7   | Forward extensibility                                                        | No normative architectural change shall invalidate conforming implementations of prior versions.                                                                               |
| I-8   | Verifiable core                                                              | Components in the highest verification tier (Class A) must admit machine-checked proofs of their stated invariants.                                                            |
| I-9   | Replaceable mechanisms                                                       | For every architectural mechanism, at least one substitutable mechanism must exist or be specified as future-compatible without violating invariants.                           |
| I-10  | Explicit failure semantics                                                   | Every interface contract must declare its failure modes including partial-failure behavior.                                                                                    |
| I-11  | Declarable profiles                                                          | Every conforming implementation declares the subset of profiles, mechanisms, and tiers it implements; the absence of a declaration is a conformance violation.               |
| **I-12** | **Origin-of-authority specification**                                     | **The platform shall specify how initial authority is legitimately created at bootstrap. The absence of an origin-of-authority specification is a conformance violation.**       |
| **I-13** | **Address–authority semantic orthogonality**                               | **A capability authorizes an action; an address locates a resource. Possession of an address alone grants no authority, and authority validation does not synthesize an undeclared address. A tagged or packed hardware representation MAY carry both when its semantics preserve independent bounds, rights, derivation, and validation; shared representation is not itself a collapse.** |

These are the only architectural invariants. Anything not derivable from these invariants is a mechanism or implementation choice and may be revised through standard RFC process without architectural amendment.

---

## 3. The Five Peer Architectures (Asymmetric)

The platform is composed of five peer architectures. They are co-equal in architectural authority and operate through explicit contracts.

```
Platform
├── Composition Architecture
├── Execution Architecture
├── State Architecture
├── Communication Architecture
└── Observability Architecture
```

### 3.0 Asymmetry disclosure

The five peers are not perfectly symmetric in implementation dependency. In particular, Communication has a documented layering dependency on Execution: messages require an executor that performs dispatch, and primitives defined in Execution are required for Communication to function. Other peers do not imply this kind of dependency on Communication.

The architecture treats this as a documented layering relationship, not a reduction of Communication's peer status. v2 RFCs may revisit Communication layering only if multi-domain federation justifies relocation upward.

### 3.1 Why five, not four or six

- Three is too few — Communication is dominantly distinct from Execution and from Composition.
- Six (adding Security as a peer) is too many — Security is a cross-cutting property mediated by capabilities, not a peer.
- Knowledge architecture is documented as a non-normative future-direction; it is not part of v1.

### 3.2 Composition Architecture

**Concern.** How components are declared, instantiated, bound, versioned, composed, and governed.

**Owns:** Component-manifest format, capability routing, dependency resolution, inter-component contracts, update and rollback atomicity at composition level, profile selection, update governance.

**Does not own:** Resource accounting (Execution), persistence (State), transport details (Communication), telemetry format (Observability).

**Normative output:** `RFC-0002 Component Manifest`, `RFC-0003 Capability Routing`, `RFC-0004 Dependency Resolution`, `RFC-0005 Profile Selection`, `RFC-0006 Composition Conformance`.

### 3.3 Execution Architecture

**Concern.** The substrate on which components run. Includes scheduling, address-space management, interrupt handling, dispatch, capability enforcement, and execution-domain isolation. Includes the bootstrap path defined in Section 5 of this document.

**Owns:** Execution-domain model, dispatch protocol, address-space primitives, interrupt routing, scheduling primitives, capability mediation at execution boundary, time primitives, bootstrap protocol.

**Does not own:** What components exist (Composition), where state lives (State), how messages travel (Communication), what telemetry looks like (Observability).

**Normative output:** `RFC-0007 Execution Domain` (includes bootstrap protocol), `RFC-0008 Dispatch`, `RFC-0009 Address Space`, `RFC-0010 Interrupt Routing`, `RFC-0011 Scheduler`, `RFC-0012 Time`, `RFC-0013 Capability Mediation`, `RFC-0014 Execution Conformance`.

**Architectural rule — Critical-Path Admission Test.** Execution privilege is admitted only when the following are all satisfied and documented:
1. The operation lies on a latency-critical path.
2. User-space execution exceeds a specified latency budget.
3. Security gain from isolation is lower than the latency loss.
4. The component's interface can remain stable over long periods.
5. Verification cost remains below an established assurance budget.
Privilege admitted by ideology (pure microkernel, pure monolithic, etc.) is non-conformant.

### 3.4 State Architecture

**Concern.** Unified substrate for all durable and addressable identity, including persistent objects, transactional mutation, integrity verification, lifecycle management, and storage-provider abstraction.

**Owns:** Persistent identity, transactional mutation, integrity verification, lifecycle state machine, storage-provider abstraction, namespace projections (POSIX, object, key-value).

**Does not own:** Volatile memory semantics (Execution), in-transit messages (Communication), component binding (Composition), observability of state events (Observability).

**Normative output:** `RFC-0015 Object Identity`, `RFC-0016 Transaction Model`, `RFC-0017 Integrity Model`, `RFC-0018 Lifecycle FSM`, `RFC-0019 Storage Provider Interface`, `RFC-0020 POSIX Projection`, `RFC-0021 Object Projection`, `RFC-0022 Key-Value Projection`, `RFC-0023 State Conformance`.

**Architectural rule.** State is exposed as a single set of invariants (Identity, Consistency, Reachability — defined in `RFC-0015` and `RFC-0018`) but physical implementations may diverge. **Merkle + CoW is the normative recommended default**; alternatives are conformant if invariants hold.

### 3.5 Communication Architecture

**Concern.** All forms of message transfer between components and execution domains, regardless of encoding or transport.

**Owns:** Message format, endpoint addressing (uses capability primitives), routing semantics, capability transfer protocol, synchronization primitives, streaming and event semantics.

**Does not own:** Naming (Composition), persistence ordering (State), resource reservation (Execution), message auditing (Observability).

**Layering declaration.** Communication primitives are layered on top of Execution primitives. Communication cannot exist without Execution. This is a one-directional dependency.

**Normative output:** `RFC-0024 Message Format`, `RFC-0025 Endpoint Model`, `RFC-0026 Routing`, `RFC-0027 Capability Transfer`, `RFC-0028 Synchronization`, `RFC-0029 Streaming`, `RFC-0030 Communication Conformance`.

**Architectural rule — single-domain in v1.** v1 implements single-execution-domain communication. Multi-domain federation is explicitly deferred to v2.

### 3.6 Observability Architecture

**Concern.** Telemetry, tracing, replay, auditing, and diagnostic representation for every observable event in the platform.

**Owns:** Event model and schema, trace span primitives, replay format, audit-log structure, diagnostic query interface, causality tracking, telemetry contract templates.

**Does not own:** What events are emitted (each peer architecture specifies its own), how telemetry is collected (mechanism), where telemetry is stored (uses State).

**Normative output:** `RFC-0031 Event Model`, `RFC-0032 Trace Span`, `RFC-0033 Replay Format`, `RFC-0034 Audit Log`, `RFC-0035 Diagnostic Query`, `RFC-0036 Observability Conformance`.

**Architectural rule — required events.** Every peer architecture must emit events using the Observability event schema. Events are required for compliance with I-4. Telemetry may be filtered, sampled, or directed but not silently dropped without declaration in the component manifest.

---

## 4. Capability Security Model (Authority–Address Separation)

Capability mediation is the **single authority-control** primitive. It is **not** the single naming primitive.

| Conceptual role          | Architectural mapping                                              |
|--------------------------|---------------------------------------------------------------------|
| Authority identity       | Capability token                                                   |
| Authority grant          | Transfer or derivation                                             |
| Authority revocation     | Revocation protocol (defined in `RFC-0039`)                        |
| Authority audit          | Capability lineage tracking (defined in `RFC-0040`-family audit RFCs) |
| Subordinate authority    | Restriction (right narrowing), not addition                        |
| Persistence of authority | Capability storage in State                                        |
| **Resource location**    | **Address — a separately modeled primitive**                       |
| **Address lifetime**     | **Independent of capability lifetime**                              |
| **Reachability**         | **Address in namespace + capability authorizing the operation**     |

**Authority and address are orthogonal.** A capability without an address is meaningless; an address without a capability permits no operation. Conflating them is a conformance violation per I-13.

**There is no root, no UID, no GID, no ACL in the normative architecture.** POSIX user/group permissions are a projection implemented over capability mediation plus address resolution, not a parallel mechanism.

**Capability token format, address format, transfer protocol, revocation protocol, and rights algebra** are defined in `RFC-0037` through `RFC-0039` and `RFC-0043` (right algebra). They are explicitly out of scope of this top-level document.

---

## 5. Origin of Authority and Bootstrap Model

The platform cannot claim "no ambient authority" without specifying where the first authorized capability legitimately originates. This section defines that origin and the bootstrap path.

### 5.1 Root of Trust

The platform's root of trust is the boundary at which the first authorized capability is constructed. It is the platform's fundamental anchor of authority.

**The root of trust is necessarily non-arbitrary.** It cannot be derived from a capability (no capability yet exists). It is rooted in hardware attestation or a user-controlled physical act (boot confirmation, hardware-bound key, or equivalent). Conforming implementations MUST declare which root-of-trust mechanism they satisfy.

### 5.2 Bootstrap Stages

```
I.   Firmware Root of Trust
        │   (hardware-bound attestation or user-confirmed origin)
        ▼
II.  Minimum Executor
        │   (constructs first capability; binds to hardware identity)
        ▼
III. Capability Root
        │   (creates initial namespace; delegates to Composition Root)
        ▼
IV.  Composition Root
        │   (constructs initial service graph from declared manifests)
        ▼
V.   User Components
        │   (operate under capabilities granted through Composition)
```

Each stage's transition rules are specified normative in the relevant RFC (`RFC-0007`, `RFC-0037`, `RFC-0002`). The stages above are the architectural constraint; the RFCs fill in mechanism.

### 5.3 Conformance Requirement

Compliance with I-12 requires that the bootstrap path:
- Declares the hardware or actor from which initial authority derives.
- Specifies the capability constructed at the minimum-executor boundary.
- Documents every step at which additional authority enters the system.
- Provides a reproducible attestation of the bootstrap path for conformance testing.

Implicit or undocumented authority creation is non-conformant.

### 5.4 Normative Output

- `RFC-0007 (Execution Domain)` MUST include a subsection defining the bootstrap protocol.
- The Capability RFCs MUST include a subsection defining construction of the origin capability.
- The Conformance Test Suite methodology MUST include bootstrap-path conformance fixtures.

---

## 6. Verification Tiers

Every component declares its assurance class:

| Tier | Definition        | Required evidence                                       |
|------|-------------------|---------------------------------------------------------|
| A    | Machine-verified  | Mathematically checked proof carried by CI              |
| B    | Model-checked     | State-space model with provable invariants              |
| C    | Property-tested   | Generative property tests with declared invariants       |
| D    | Conventional      | Standard unit, integration, regression testing          |

**Default tier.** Components that mediate capability enforcement, persistent identity, transactional commit, or integrity verification default to Tier A. Lowering requires documented justification logged in the conformance declaration.

### 6.1 Economic cost of verification

Verification tier is a long-term economic commitment, not a quality rubric. Tier A in particular implies sustained investment measured in person-years of proof maintenance for the lifetime of the platform.

**Architectural consequences:**
- Tier A admission requires a documented maintenance-cost estimate included in the conformance declaration.
- Tier A components MUST declare a proof-fragility model: how much surface change triggers re-proof and how re-proof effort scales.
- The Critical-Path Admission Test (Section 3.3) explicitly includes verification cost as an admission criterion.
- Tier B and C declarations include estimated lifecycle costs.
- The Composition Architecture includes a top-level verification-cost budget, distinct from complexity budgets described in Section 7.

Ignoring verification economics in architectural decisions results in platforms whose verification promises become unfunded and silently lapse. The architecture explicitly prohibits this failure mode.

---

## 7. Complexity Budgets (Quantitative)

Each architecture peer receives a complexity budget covering five metrics:

| Metric                    | Definition                                                                              |
|---------------------------|-----------------------------------------------------------------------------------------|
| **Operation count**       | Number of unique exported operations per peer                                           |
| **Distinct state count**  | Number of orthogonal state dimensions per peer                                          |
| **Dependency-graph diameter** | Maximum peer-crossings required for any conformance-defined operation                |
| **Proof-fragility class** | Categorical estimate of re-proof effort distribution                                    |
| **Concept set size**      | Enumerated set of concepts a developer must understand for confident use                |

Budgets become normative conformance thresholds only after the Conformance Test Suite methodology is finalized. Until then, they are advisory quantitative estimates used for architectural admission tests.

When a component or peer exceeds budget, the architecture MUST either:
1. Justify the excess through an Architecture-Amendment RFC, or
2. Split the component.

### 7.1 Failure-mode coverage budget (recommended)

A sixth recommended metric is failure-mode coverage: the fraction of declared failure modes for which conformance tests exist. Implementations SHOULD track this separately from the five normative metrics above.

---

## 8. Scheduling Architecture

Three-layer scheduler model:

**Layer 1 — Deterministic core.** Provably bounded decision time. Implements a documented class of scheduling algorithms (EDF, EEVDF, or equivalent). Guarantees hard-real-time schedulability claims. Determinism is a documented class property.

**Layer 2 — Advisory tuning.** Continuously adjusts Layer 1 parameters based on observed workload patterns. Never makes scheduling decisions. Tunable inputs and outputs are documented.

**Layer 3 — Offline learning.** Generates tuning recommendations through workload analysis. Has no ability to influence Layer 1 or Layer 2 in real time. Recommendations are versioned and auditable.

**Architectural rule.** No scheduling decision may be made by a non-deterministic component on the scheduling hot path. AI/ML-influenced scheduling is permitted only through Layer 2 or Layer 3.

---

## 9. Persistence / State Boundary

The disk separates from memory only because physics separates them. Architecturally, both are forms of *state* with different correct invariants.

| State class | Persistence             | Reachability semantics        |
|-------------|-------------------------|-------------------------------|
| Volatile    | Process / domain lifetime | Reachable while capability exists |
| Durable     | Transaction-committed   | Reachable while object exists |

Memory and storage share:
- Identity (modeled through State)
- Authorization (capability-mediated)
- Lifetime governance (subject to capability and object lifecycles)
- Versioning (memory optionally, storage mandatorily)

Memory and storage differ:
- Latency guarantees
- Failure semantics
- Recovery procedures

The State architecture models memory and storage uniformly in invariant terms while permitting mechanism divergence. A persistence projection (POSIX-file-style) is a derived view, not the underlying model.

---

## 10. Distributed Execution Boundary

| Version | Scope                                                                |
|---------|----------------------------------------------------------------------|
| v1      | Single execution domain                                              |
| v2      | Multiple execution domains with capability-mediated federation       |

The v1 → v2 transition is governed by explicit configuration. v1 implementations MUST declare non-distributed conformance.

This is a deliberate deferral, not an architectural limitation.

---

## 11. Hardware Universality

**The platform does not claim implementation uniformity across divergent computational substrates** (classical CPUs, quantum processors, neuromorphic systems, photonic accelerators). Substrate-driven differences are real and must be respected in mechanism design.

**What v1 promises:** A stable execution-provider contract. Hardware families that can implement that contract conform. Hardware families that cannot are documented as falling outside the platform's execution-provider scope.

The contract is substrate-independent. CPU, FPGA, accelerator-with-isolation, and managed-runtime providers may implement the same contract where meaningful.

---

## 12. Profiles

Deployment profiles (Embedded, Desktop, Cloud, HPC, Safety-Critical) are defined in `Profiles-Specification.md`. Profiles never relax architectural invariants; they select mechanism subsets.

---

## 13. Required RFCs

The 41 RFCs of normative specification required for v1 conformance are listed in `00-RFC-Index.md`. Concrete RFC content is in the `RFCs/` subdirectories.

---

## 14. Revision History

| Version | Date            | Material changes                                                                                                                                                                                                                                                                                                                                                                                  |
|---------|-----------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1.0     | this cycle      | Initial frozen architecture.                                                                                                                                                                                                                                                                                                                                                                        |
| 1.1     | this cycle      | Added **I-12 (origin of authority)** and **I-13 (address–authority orthogonality)**. Added Section 5 (Origin of Authority and Bootstrap Model). Added Section 6.1 (Economic cost of verification). Revised Section 4 with explicit authority–address orthogonality. Revised Section 7 to quantitative metrics. Refined Section 3.5 with explicit Communication–Execution layering declaration. Added Section 7.1 (recommended failure-mode coverage metric). |

---

## Document Status

This document is a **validation candidate at architectural level.** Governed revisions to Architectural Invariants (Section 2) and the five-peer decomposition (Section 3) require the Architecture-Amendment process defined in `Governance-RFC.md`. Editorial improvements to RFC listings, example rewording, or section ordering do not require an amendment.

Concrete RFCs are not part of this document and evolve independently.

This is the current constitutional baseline, not a final empirical claim. The next work is executable modeling, CTS implementation, prototype evidence, and governed reconciliation.
