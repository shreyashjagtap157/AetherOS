# Glossary

Classification: Normative
Authoritative Source: Glossary.md
Requirement-ID: GOV-GLOS-001

This glossary defines terminology used across the Platform Architecture Specification and all 41 RFCs. Where a term has a normative meaning in this specification, the normative meaning takes precedence over any casual reading.

Each entry carries metadata for governance tooling. See `tools/verify-glossary.py` for CI enforcement of provenance consistency.

---

## A

### Address
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0007
Introduced by RFC: RFC-0007
Last modified by RFC: RFC-0007
Related invariants: I-13
Allowed synonyms: locator, resource locator
Forbidden synonyms: capability, authority, identity
Rationale: Address is explicitly separated from authority per I-13. This separation is load-bearing for the security model.
Notes: None
```
**Definition:** A separately-modeled primitive that locates a resource within a namespace. Orthogonal to capability. *See also* Authority, I-13.

### Advisory tuning (scheduler Layer 2)
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0011
Introduced by RFC: RFC-0011
Last modified by RFC: RFC-0011
Related invariants: I-8
Allowed synonyms: scheduler layer 2, L2 tuning
Forbidden synonyms: scheduler hot path, deterministic core
Rationale: Advisory tuning must never make scheduling decisions, per I-8. The naming distinguishes it clearly from Layer 1.
Notes: None
```
**Definition:** A scheduler subsystem that continuously adjusts Layer 1 parameters based on observed workload patterns but never makes scheduling decisions. *See also* Deterministic core, Offline learning.

### Ambient authority
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0007
Introduced by RFC: RFC-0007
Last modified by RFC: RFC-0007
Related invariants: I-2
Allowed synonyms: implicit authority, ambient permission
Forbidden synonyms: inherited authority, default authority
Rationale: I-2 forbids ambient authority categorically. The term must be unambiguous.
Notes: None
```
**Definition:** Authority held by a thread or component without having been explicitly granted to it. Forbidden by I-2.

### Architecture amendment
```yaml
Status: Active
Normative: Yes
Owner RFC: ARCHITECTURE-GOVERNANCE.md
Introduced by RFC: RFC-GOV-002
Last modified by RFC: RFC-GOV-002
Related invariants: I-1 through I-13
Allowed synonyms: architectural revision, invariant change
Forbidden synonyms: specification change, editorial change
Rationale: Architecture Amendment is the highest-ceremony change category and requires different process from Specification Amendment.
Notes: None
```
**Definition:** A formal revision to architectural invariants (Section 2 of Platform Architecture Specification v1.1) or the five-peer decomposition (Section 3). Governed by `ARCHITECTURE-GOVERNANCE.md` and `RFC-GOV-002`.

### Authority
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0007
Introduced by RFC: RFC-0007
Last modified by RFC: RFC-0007
Related invariants: I-2, I-13
Allowed synonyms: right, permission
Forbidden synonyms: identity, address, capability
Rationale: Authority is the right to perform an operation; capability is the token that grants it. These are distinct per I-13.
Notes: None
```
**Definition:** The right to perform an operation on a resource. Orthogonal to address. *See also* Capability, I-13.

## B

### Bootstrap path
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0007
Introduced by RFC: RFC-0007
Last modified by RFC: RFC-0007
Related invariants: I-12
Allowed synonyms: boot sequence, initialization sequence
Forbidden synonyms: startup, power-on
Rationale: Bootstrap path is the specific ordered sequence of authority construction per I-12, not general initialization.
Notes: None
```
**Definition:** The ordered sequence of stages through which the platform's initial authority is constructed, attested, and delegated downward. Specified in `RFC-0007`.

### Bundle (capability)
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0037
Introduced by RFC: RFC-0037
Last modified by RFC: RFC-0037
Related invariants: I-7
Allowed synonyms: wedge, capability bundle
Forbidden synonyms: composite capability, nested capability
Rationale: Bundle is the defined term per RFC-0037 and RFC-0038. Wedge is a legacy synonym from earlier drafts.
Notes: None
```
**Definition:** A capability-mediated grouping of co-located or co-versioned sub-capabilities used for transfer and revocation efficiency. Defined in `RFC-0037` and `RFC-0038`.

## C

### Capability token
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0037
Introduced by RFC: RFC-0037
Last modified by RFC: RFC-0037
Related invariants: I-2, I-3, I-7
Allowed synonyms: capability, handle, reference
Forbidden synonyms: pointer, authority itself
Rationale: The token is the handle, not the authority. This distinction is load-bearing for the unforgeability property.
Notes: None
```
**Definition:** An unforgeable, opaque, transferable handle that authorizes operations on a specific resource within a specific rights set. The unit of authority in this platform.

### Capability transfer
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0038
Introduced by RFC: RFC-0038
Last modified by RFC: RFC-0038
Related invariants: I-3, I-7
Allowed synonyms: capability transmission, rights delegation
Forbidden synonyms: capability copy, capability clone
Rationale: Transfer is the defined semantics — capability moves, it is not duplicated. Copy would violate I-3.
Notes: None
```
**Definition:** The protocol by which a capability moves from one holder to another without changing its underlying rights. Defined in `RFC-0038`.

### Cohort (versioned)
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0042
Introduced by RFC: RFC-0042
Last modified by RFC: RFC-0042
Related invariants: I-9
Allowed synonyms: versioned cohort, atomic update group
Forbidden synonyms: release, rollback unit
Rationale: Cohort groups components for atomic coordinated updates per I-9. The term distinguishes this from ad-hoc release sets.
Notes: RFC-0042 is not yet written; this entry is reserved pending its acceptance.
```
**Definition:** A set of components sharing a single versioned commit identifier, generated and pinned via the cohort protocol. Used for atomic coordinated updates. Defined in `RFC-0042`.

### Cold state
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0018
Introduced by RFC: RFC-0018
Last modified by RFC: RFC-0018
Related invariants: I-10
Allowed synonyms: cold, tier-2 storage
Forbidden synonyms: archived, dormant
Rationale: Cold is a lifecycle state, not a storage tier. Distinction matters for the lifecycle FSM semantics.
Notes: None
```
**Definition:** A lifecycle state representing persistent state whose active use has declined below a configurable threshold; eligible for tier movement. *See also* Lifecycle FSM.

### Communication Architecture
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-1 through I-13
Allowed synonyms: peer architecture, messaging layer
Forbidden synonyms: network layer, transport layer
Rationale: Communication Architecture is explicitly asymmetric to Execution Architecture. The term must not conflate them.
Notes: None
```
**Definition:** The peer architecture concerned with message transfer between components and execution domains. Defined in `Platform-Architecture-Specification-v1.1.md` Section 3.4 and `RFCs/03-Communication/`.

### Component
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0002
Introduced by RFC: RFC-0002
Last modified by RFC: RFC-0002
Related invariants: I-9, I-11
Allowed synonyms: software component, deployed component
Forbidden synonyms: module, service, microservice
Rationale: Component is the unit of Composition Architecture with specific manifest and lifecycle semantics. Module/service are too overloaded.
Notes: None
```
**Definition:** The unit of Composition Architecture; a versioned, manifest-described bundle of capabilities and contracts.

### Component manifest
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0002
Introduced by RFC: RFC-0002
Last modified by RFC: RFC-0002
Related invariants: I-2, I-9, I-11
Allowed synonyms: manifest, component descriptor
Forbidden synonyms: configuration, spec file
Rationale: Component manifest is a specific declarative document type per RFC-0002, not a general config file.
Notes: None
```
**Definition:** A declarative document describing a component's identity, dependencies, capabilities, and contracts. Defined in `RFC-0002`.

### Composition Architecture
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-1 through I-13
Allowed synonyms: peer architecture, composition layer
Forbidden synonyms: module system, linking
Rationale: One of the five peer architectures. The term distinguishes it from Execution, State, Communication, and Observability.
Notes: None
```
**Definition:** One of the five peer architectures, concerned with how components are declared, bound, versioned, composed, and governed. Defined in `Platform-Architecture-Specification-v1.1.md` Section 3.1 and `RFCs/00-Composition/`.

### Conformance declaration
```yaml
Status: Active
Normative: Yes
Owner RFC: Conformance-Test-Suite-Methodology.md
Introduced by RFC: Conformance-Test-Suite-Methodology.md
Last modified by RFC: Conformance-Test-Suite-Methodology.md
Related invariants: I-1
Allowed synonyms: conformance statement, profile declaration
Forbidden synonyms: certification, attestation
Rationale: Declaration is a specific document type produced by the implementer. Certification implies third-party assessment.
Notes: None
```
**Definition:** A document, produced by an implementation, declaring which profiles, mechanisms, and verification tiers it satisfies.

### Conformance Test Suite (CTS)
```yaml
Status: Active
Normative: Yes
Owner RFC: Conformance-Test-Suite-Methodology.md
Introduced by RFC: Conformance-Test-Suite-Methodology.md
Last modified by RFC: Conformance-Test-Suite-Methodology.md
Related invariants: I-1
Allowed synonyms: CTS, conformance tests
Forbidden synonyms: unit tests, integration tests, QA tests
Rationale: CTS is the executable specification against which conformance is measured. Unit/integration tests are implementation-level.
Notes: None
```
**Definition:** The executable tests through which a candidate implementation is judged conformant or non-conformant. Methodology in `Conformance-Test-Suite-Methodology.md`.

### Conformance violation
```yaml
Status: Active
Normative: Yes
Owner RFC: Conformance-Test-Suite-Methodology.md
Introduced by RFC: Conformance-Test-Suite-Methodology.md
Last modified by RFC: Conformance-Test-Suite-Methodology.md
Related invariants: I-1
Allowed synonyms: non-conformance, conformance failure
Forbidden synonyms: bug, error, defect
Rationale: A conformance violation is a measurable deviation from a normative requirement. Bug/error/defect imply implementation fault rather than specification deviation.
Notes: None
```
**Definition:** Any measurable deviation from a normative invariant, RFC requirement, or declared profile. Discovered through conformance testing.

### Contract
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0002
Introduced by RFC: RFC-0002
Last modified by RFC: RFC-0002
Related invariants: I-1
Allowed synonyms: interface contract, behavioral contract
Forbidden synonyms: API, specification, promise
Rationale: Contract is a predicate, not prose. The term distinguishes this from general API specifications.
Notes: None
```
**Definition:** A formally stated obligation between a provider and a consumer; in this platform, a contract is a predicate, not prose. Defined throughout the RFCs.

### Cross-cutting invariant
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-1 through I-13
Allowed synonyms: cross-cutting concern, shared invariant
Forbidden synonyms: global invariant
Rationale: Cross-cutting invariants are mediated by multiple peer architectures, not necessarily all of them. Global implies universality.
Notes: None
```
**Definition:** An invariant such as security that is mediated by every peer architecture rather than belonging to any one of them.

## D

### Deterministic core (scheduler Layer 1)
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0011
Introduced by RFC: RFC-0011
Last modified by RFC: RFC-0011
Related invariants: I-8
Allowed synonyms: scheduler layer 1, L1 core, deterministic scheduler
Forbidden synonyms: real-time scheduler (overly narrow), ML scheduler
Rationale: I-8 requires deterministic worst-case latency. The deterministic core satisfies this. ML is Layer 3, not Layer 1.
Notes: None
```
**Definition:** A scheduler subsystem whose decisions are bounded by provable worst-case latency and which implements a documented class of scheduling algorithms. *See also* Advisory tuning, Offline learning.

### Distributed execution domain
```yaml
Status: Reserved
Normative: No
Owner RFC: RFC-0007
Introduced by RFC: RFC-0007
Last modified by RFC: RFC-0007
Related invariants: None in v1
Allowed synonyms: federation domain, multi-node domain
Forbidden synonyms: cluster, network
Rationale: Distributed execution is v2 scope, deferred from v1. Reserved for future RFC.
Notes: v1 = single execution domain; distributed = v2.
```
**Definition:** An execution domain potentially spanning multiple physical nodes, governed by the v2 federation rules. Not in v1.

### Dispatch
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0008
Introduced by RFC: RFC-0008
Last modified by RFC: RFC-0008
Related invariants: I-8
Allowed synonyms: thread dispatch, context switch
Forbidden synonyms: scheduling, execution
Rationale: Dispatch is the act of moving a thread to an execution unit. Scheduling is the decision of which thread. These are distinct.
Notes: None
```
**Definition:** The act of moving a thread or task to an execution unit in response to a scheduling decision. Defined in `RFC-0008`.

## E

### Execution Architecture
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-1 through I-13
Allowed synonyms: peer architecture, execution layer
Forbidden synonyms: kernel, microkernel
Rationale: Execution Architecture is one of five peers. It is not the kernel; the kernel is one possible implementation of it.
Notes: None
```
**Definition:** One of the five peer architectures, concerned with dispatch, scheduling, address-space management, interrupt handling, and capability mediation. Defined in `Platform-Architecture-Specification-v1.1.md` Section 3.2 and `RFCs/01-Execution/`.

### Execution domain
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0007
Introduced by RFC: RFC-0007
Last modified by RFC: RFC-0007
Related invariants: I-2, I-13
Allowed synonyms: domain, trust domain
Forbidden synonyms: address space, process, partition
Rationale: Execution domain is the boundary at which capability enforcement is performed. Process/address space are specific implementations.
Notes: None
```
**Definition:** The boundary at which capability enforcement is performed and within which references are valid. Defined in `RFC-0007`.

### Execution provider
```yaml
Status: Active
Normative: Yes
Owner RFC: Hardware-Support-RFC.md
Introduced by RFC: Hardware-Support-RFC.md
Last modified by RFC: Hardware-Support-RFC.md
Related invariants: I-1
Allowed synonyms: hardware provider, execution substrate
Forbidden synonyms: CPU, processor
Rationale: Execution provider is the substrate-neutral contract. CPU is one possible provider. GPU/FPGA accelerators are also providers.
Notes: None
```
**Definition:** The hardware-or-managed-runtime implementation of the execution-provider contract.

## F

### Five-peer architecture
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-1 through I-13
Allowed synonyms: five-peer model, peer decomposition
Forbidden synonyms: layered architecture, modular kernel
Rationale: The five peers (Composition, Execution, State, Communication, Observability) are co-equal and communicate through contracts. Not a layer stack.
Notes: None
```
**Definition:** The decomposition of the platform into Composition, Execution, State, Communication, and Observability Architecture. Defined in `Platform-Architecture-Specification-v1.1.md` Section 3.

## I

### Identity (persistent)
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0015
Introduced by RFC: RFC-0015
Last modified by RFC: RFC-0015
Related invariants: I-10
Allowed synonyms: persistent identity, object identity
Forbidden synonyms: address, pointer, handle
Rationale: Identity is content-resolved and immutable. Address is a locator. These are orthogonal per I-13.
Notes: None
```
**Definition:** An immutable, content-resolved pointer to the bytes defining a persistent object. Orthogonal to address.

### Invariant
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-1 through I-13
Allowed synonyms: architectural invariant, conformance invariant
Forbidden synonyms: requirement, property, guarantee
Rationale: Invariant specifically means a property no conforming implementation may violate. Requirement is broader.
Notes: None
```
**Definition:** A normative architectural property that no conforming implementation may violate. Listed in `Platform-Architecture-Specification-v1.1.md` Section 2.

## L

### Layered extension
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-9
Allowed synonyms: optional extension, removable layer
Forbidden synonyms: core component, base layer
Rationale: Layered extensions exist above the verified core but are optional and replaceable. Core components are mandatory.
Notes: None
```
**Definition:** A facility that exists above the verified core but is optional, may be removed, and may be replaced without architectural violation. Example: capability-aware storage above the Merkle + CoW persistence core.

### Lifecycle FSM
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0018
Introduced by RFC: RFC-0018
Last modified by RFC: RFC-0018
Related invariants: I-10
Allowed synonyms: object lifecycle, state machine
Forbidden synonyms: object graph, reference counting
Rationale: Lifecycle FSM is a specific finite state machine with defined states and transitions per RFC-0018.
Notes: None
```
**Definition:** The finite state machine governing persistent-object lifecycle, with states Created, Validated, Referenced, Active, Cold, Archived, Expired, Destroyed. Defined in `RFC-0018`.

### Lifecycle states
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0018
Introduced by RFC: RFC-0018
Last modified by RFC: RFC-0018
Related invariants: I-10
Allowed synonyms: lifecycle stages, object states
Forbidden synonyms: object types, data states
Rationale: States are specific FSM states, not general data classifications.
Notes: See Lifecycle FSM.
```
**Definition:** The enumerated states of the Lifecycle FSM. *See* Lifecycle FSM.

## M

### Manifest (component)
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0002
Introduced by RFC: RFC-0002
Last modified by RFC: RFC-0002
Related invariants: I-2, I-9, I-11
Allowed synonyms: component manifest
Forbidden synonyms: configuration, spec
Rationale: Synonym for Component manifest. Primary term in RFC-0002.
Notes: None
```
**Definition:** *See* Component manifest.

### Merkle integrity
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0015
Introduced by RFC: RFC-0015
Last modified by RFC: RFC-0015
Related invariants: I-10
Allowed synonyms: Merkle tree, hash tree integrity
Forbidden synonyms: checksumming, CRC integrity
Rationale: Merkle integrity is a specific recursive hash structure, not general checksumming.
Notes: None
```
**Definition:** The persistence integrity model in which every persistent object is verifiable against a recursive hash structure over its content and children. The recommended default for the State core.

### Minimum executor
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-12
Allowed synonyms: minimum bootstrap executor, min executor
Forbidden synonyms: init, kernel, first process
Rationale: The minimum executor is the bootstrap stage at which the first capability is constructed. Init/kernel are implementation concepts.
Notes: None
```
**Definition:** The bootstrap stage at which the platform's first capability is constructed and bound to the hardware root of trust. Defined in `Platform-Architecture-Specification-v1.1.md` Section 5.

## N

### Namespace
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0015
Introduced by RFC: RFC-0015
Last modified by RFC: RFC-0015
Related invariants: I-13
Allowed synonyms: address namespace, object namespace
Forbidden synonyms: directory, file system
Rationale: Namespace is a projection, not an underlying primitive. File system is one possible projection.
Notes: None
```
**Definition:** A structured view over a set of addresses and identity-resolved objects. A namespace is a projection, not an underlying primitive.

### Non-conformant
```yaml
Status: Active
Normative: Yes
Owner RFC: Conformance-Test-Suite-Methodology.md
Introduced by RFC: Conformance-Test-Suite-Methodology.md
Last modified by RFC: Conformance-Test-Suite-Methodology.md
Related invariants: I-1
Allowed synonyms: conformance violation, non-compliant
Forbidden synonyms: broken, buggy, wrong
Rationale: Non-conformant is a preciseCTS term. Broken/buggy imply implementation fault rather than specification deviation.
Notes: None
```
**Definition:** Failing to satisfy a normative requirement. Equivalent to conformance violation.

### Normative
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-1
Allowed synonyms: required, mandatory
Forbidden synonyms: recommended, should
Rationale: Normative has a specific meaning in this suite — required for conformance. Recommended is SHOULD, not MUST.
Notes: None
```
**Definition:** Required for conformance.

## O

### Object identity
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0015
Introduced by RFC: RFC-0015
Last modified by RFC: RFC-0015
Related invariants: I-10
Allowed synonyms: persistent object identity, content identity
Forbidden synonyms: address, pointer
Rationale: Object identity is content-addressed and immutable per RFC-0015. Pointer/address are neither.
Notes: None
```
**Definition:** The content-addressed identifier that uniquely names a persistent object across time, location, and replicas. Defined in `RFC-0015`.

### Offline learning (scheduler Layer 3)
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0011
Introduced by RFC: RFC-0011
Last modified by RFC: RFC-0011
Related invariants: I-8
Allowed synonyms: scheduler layer 3, L3 learning, workload analysis
Forbidden synonyms: online ML, hot-path ML, real-time ML
Rationale: Offline learning has no real-time influence on scheduling decisions per I-8. Online/real-time ML on the hot path is prohibited.
Notes: None
```
**Definition:** A scheduler subsystem that analyzes workload patterns offline and produces versioned, auditable recommendations. Has no real-time influence. *See also* Deterministic core, Advisory tuning.

### Origin of authority
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-12
Allowed synonyms: authority origin, root of authority
Forbidden synonyms: root of trust (distinct concept)
Rationale: Origin of authority is the source of the first capability. Root of trust is the hardware anchoring. These are related but distinct.
Notes: None
```
**Definition:** The hardware or actor from which the platform's first authorized capability is constructed at bootstrap. Specified in `Platform-Architecture-Specification-v1.1.md` Section 5 and `RFC-0007`.

## P

### Peer architecture
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-1 through I-13
Allowed synonyms: peer, architectural peer
Forbidden synonyms: subsystem, component, module
Rationale: The five peers are co-equal in authority and communicate through explicit contracts. Subsystem/component imply hierarchy.
Notes: None
```
**Definition:** One of the five peer architectures. Co-equal in authority; operate through explicit contracts.

### Persistent object
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0015
Introduced by RFC: RFC-0015
Last modified by RFC: RFC-0015
Related invariants: I-10
Allowed synonyms: state object, persisted entity
Forbidden synonyms: file, record
Rationale: Persistent object has immutable identity, transactional commit, and observable lifecycle per RFC-0015. File/record are storage projections.
Notes: None
```
**Definition:** A state-resident entity with immutable identity, transactional commit, and observable lifecycle. Defined in `RFC-0015`.

### Profile
```yaml
Status: Active
Normative: Yes
Owner RFC: Profiles-Specification.md
Introduced by RFC: Profiles-Specification.md
Last modified by RFC: Profiles-Specification.md
Related invariants: I-11
Allowed synonyms: deployment profile, execution profile
Forbidden synonyms: configuration, tier
Rationale: Profile is a named subset of mechanisms for a deployment class per Profiles-Specification.md. Configuration is too general.
Notes: None
```
**Definition:** A named subset of mechanisms selected for a deployment class. Specified in `Profiles-Specification.md`. Profiles include: Embedded, Desktop, Cloud, HPC, Safety-Critical.

### Proof-fragility class
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-6
Allowed synonyms: fragility class, proof fragility
Forbidden synonyms: verification tier (confusingly similar)
Rationale: Proof-fragility class estimates re-proof cost per change surface. Tier is the verification assurance level. These are related but distinct.
Notes: None
```
**Definition:** A categorical estimate of how much surface change triggers re-proof and how re-proof effort scales. Specified in `Platform-Architecture-Specification-v1.1.md` Section 6.1.

## R

### Reachability
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0015
Introduced by RFC: RFC-0015
Last modified by RFC: RFC-0015
Related invariants: I-10
Allowed synonyms: object reachability, reference reachability
Forbidden synonyms: connectivity, liveness
Rationale: Reachability is the property by which an object exists and is addressable through at least one live reference. Part of the State invariant set.
Notes: None
```
**Definition:** The property by which an object exists and is addressable through at least one live reference. Part of the State invariant set.

### RFC
```yaml
Status: Active
Normative: Yes
Owner RFC: 00-RFC-Index.md
Introduced by RFC: Governance-RFC.md
Last modified by RFC: RFC-GOV-002
Related invariants: None
Allowed synonyms: specification, request for comments
Forbidden synonyms: standard, specification document (vague)
Rationale: RFC is the unit of normative specification in this suite. Standard is too weighted; specification document is too vague.
Notes: None
```
**Definition:** A *Request for Comments* specification. The unit of normative specification in this suite. Forty-one are listed in `00-RFC-Index.md`.

### Root of trust
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-12
Allowed synonyms: hardware root of trust, trusted anchor
Forbidden synonyms: origin of authority (distinct concept)
Rationale: Root of trust is the hardware boundary. Origin of authority is the first capability constructed. These are related but distinct.
Notes: None
```
**Definition:** The boundary at which the platform's first authorized capability originates. Necessarily non-arbitrary; rooted in hardware attestation or a user-controlled physical act.

## S

### Scheduler (three-layer)
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0011
Introduced by RFC: RFC-0011
Last modified by RFC: RFC-0011
Related invariants: I-8
Allowed synonyms: three-layer scheduler, layered scheduler
Forbidden synonyms: single-layer scheduler, priority scheduler
Rationale: Three-layer scheduler = deterministic core + advisory tuning + offline learning per RFC-0011 and Platform-Architecture-Specification-v1.1.md Section 8.
Notes: None
```
**Definition:** The scheduling model consisting of deterministic core, advisory tuning, and offline learning. Defined in `Platform-Architecture-Specification-v1.1.md` Section 8.

### State Architecture
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-1 through I-13
Allowed synonyms: peer architecture, state layer
Forbidden synonyms: storage system, database
Rationale: State Architecture is one of five peers, concerned with persistent identity and lifecycle. Storage system/database are implementations.
Notes: None
```
**Definition:** One of the five peer architectures, concerned with persistent identity, transactional mutation, integrity verification, lifecycle management, and storage-provider abstraction. Defined in `Platform-Architecture-Specification-v1.1.md` Section 3.3 and `RFCs/02-State/`.

### Storage provider
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0019
Introduced by RFC: RFC-0019
Last modified by RFC: RFC-0019
Related invariants: I-10
Allowed synonyms: storage backend, persistence provider
Forbidden synonyms: disk, filesystem
Rationale: Storage provider is the substrate-neutral contract. Disk/filesystem are specific implementations.
Notes: None
```
**Definition:** The backend that physically realizes a storage contract. Defined in `RFC-0019`.

### Substrate (computational)
```yaml
Status: Active
Normative: Yes
Owner RFC: Hardware-Support-RFC.md
Introduced by RFC: Hardware-Support-RFC.md
Last modified by RFC: Hardware-Support-RFC.md
Related invariants: I-1
Allowed synonyms: computation substrate, hardware substrate
Forbidden synonyms: CPU only, processor only
Rationale: Substrate includes CPU, GPU, FPGA, and accelerators. CPU-only is insufficient.
Notes: None
```
**Definition:** A specific implementation technology (CPU, GPU, FPGA, accelerator). The architecture defines an execution-provider contract that substrates may implement where they can; substrates that cannot are documented as out of contract scope.

## T

### Tier A / B / C / D
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-6
Allowed synonyms: verification tier, assurance tier
Forbidden synonyms: trust level, security level
Rationale: Tier A/B/C/D are specific verification assurance classes. Trust level/security level are too vague.
Notes: None
```
**Definition:** Verification assurance classes. Tier A: machine-verified. Tier B: model-checked. Tier C: property-tested. Tier D: conventional. Defined in `Platform-Architecture-Specification-v1.1.md` Section 6.

### Tier A admission
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-6
Allowed synonyms: core admission, TCB admission
Forbidden synonyms: Tier B admission, verification admission
Rationale: Tier A admission specifically means placement in the verified core. Subject to the Critical-Path Admission Test plus verification-economics justification.
Notes: None
```
**Definition:** The decision to place a component in the verified core. Subject to the Critical-Path Admission Test plus verification-economics justification.

### Transition (lifecycle)
```yaml
Status: Active
Normative: Yes
Owner RFC: RFC-0018
Introduced by RFC: RFC-0018
Last modified by RFC: RFC-0018
Related invariants: I-10
Allowed synonyms: lifecycle transition, state transition
Forbidden synonyms: function call, method invocation
Rationale: Lifecycle transition is a specific FSM movement per RFC-0018 with declared preconditions, atomic effects, observability events, and rollback semantics.
Notes: None
```
**Definition:** A movement in the Lifecycle FSM from one state to another, with declared preconditions, atomic effects, observability events, and rollback semantics. Defined in `RFC-0018`.

## V

### Verification tier
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-6
Allowed synonyms: tier, assurance class
Forbidden synonyms: security level, trust tier
Rationale: Synonym for Tier A/B/C/D.
Notes: See Tier A/B/C/D.
```
**Definition:** *See* Tier A/B/C/D.

### Verified core
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-6
Allowed synonyms: TCB, trusted computing base
Forbidden synonyms: kernel, microkernel
Rationale: Verified core is the set of Tier A/B components in the TCB. Kernel is one possible implementation of it.
Notes: None
```
**Definition:** The set of components satisfying Tier A or Tier B that are part of the trusted computing base.

### v1 / v2
```yaml
Status: Active
Normative: Yes
Owner RFC: Platform-Architecture-Specification-v1.1.md
Introduced by RFC: Platform-Architecture-Specification-v1.1.md
Last modified by RFC: Platform-Architecture-Specification-v1.1.md
Related invariants: I-1
Allowed synonyms: version 1, version 2, v1 spec, v2 spec
Forbidden synonyms: release 1, major version
Rationale: v1 and v2 are specific versioning milestones. v1 = single-domain; v2 = multi-domain federation.
Notes: None
```
**Definition:** Versions of the specification. v1 is single-domain; v2 introduces multi-domain federation.

## W

### Wedge (capability)
```yaml
Status: Deprecated
Normative: No
Owner RFC: RFC-0037
Introduced by RFC: RFC-0037
Last modified by RFC: RFC-0037
Related invariants: I-7
Allowed synonyms: bundle (preferred)
Forbidden synonyms: None
Rationale: Wedge was used in early drafts. Bundle is the preferred term per RFC-0037 and RFC-0038.
Notes: Superseded By: Bundle.
```
**Definition:** *See* Bundle.