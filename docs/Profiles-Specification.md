# Profiles Specification

Classification: Normative
Authoritative Source: Profiles-Specification.md

**Status:** Active. This document defines deployment profiles referenced by I-11 and by the Composition Architecture. Profiles never relax architectural invariants; they select mechanism subsets, declare defaults, and constrain instance topology.

---

## 1. Profiles and Their Purpose

Profiles exist to allow the same architecture to be deployed in radically different environments without violating invariants and without proliferating incompatible forks. Each profile declares:

- Which mechanisms are normative required
- Which mechanisms are normative optional
- Which mechanisms are forbidden in that profile
- The default verification tier for profile-mandatory components
- The minimum conformance-test derivation requirements

Profiles never modify architectural invariants and never modify Conformance Test Suite obligations. Profiles modify **the configuration state of o在任何** which conformance is tested.

---

## 2. The Five Profiles

### 2.1 Embedded Profile

**Target.** Devices with constrained resources: storage, memory, compute.

**Mandatory mechanisms:**
- Capability-mediated address spaces (`RFC-0009`)
- Single execution domain (`RFC-0007`)
- Static component manifests (`RFC-0002`)
- Compressed state storage with at least one storage provider
- Layer 1 deterministic scheduler (`RFC-0011`)

**Optional mechanisms:**
- Layer 2 advisory tuning
- Multi-process components
- POSIX projection
- Key-value projection
- Lifecycle FSM cold-tier migration

**Forbidden mechanisms:**
- Multi-domain communication
- Distributed execution domain configuration
- AI Layer 3 recommendations (offline learning), unless the embedding host has a documented budget

**Default verification tier:**
- Capability mediation: Tier A
- Identity: Tier A
- Lifecycle FSM: Tier A
- Scheduler core: Tier A
- All other components: Tier D unless profile compliance requires otherwise

**Minimum conformance test derivation requirements:**
- All Core Conformance tests MUST pass.
- Resource-budget tests MUST be specified at component level.
- Storage-providers MUST demonstrate low-storage operation mode.

### 2.2 Desktop Profile

**Target.** Interactive user-facing devices with general-purpose workloads.

**Mandatory mechanisms:**
- All embedded-profile mandatory mechanisms
- POSIX projection (`RFC-0020`)
- Key-value projection (`RFC-0022`)
- Layer 2 advisory tuning
- Bundle capability operations for GUI session management
- Update governor with atomic rollback

**Optional mechanisms:**
- Layer 3 offline learning (advisory)
- Object projection (`RFC-0021`)
- Lifecycle cold and archive tier migration

**Forbidden mechanisms:**
- Multi-domain federation primitives
- Cluster-tier tiering

**Default verification tier:**
- Capability mediation: Tier A
- Identity: Tier A
- Lifecycle FSM: Tier A
- POSIX projection: Tier C
- Everything else: Tier D

**Minimum conformance test derivation requirements:**
- All Desktop Conformance tests MUST pass.
- Application-compatibility projection conformance MUST be documented for any runtime claimed to back an arbitrary legacy application.

### 2.3 Cloud Profile

**Target.** Cloud-resident workloads and service hosting.

**Mandatory mechanisms:**
- Single execution-domain operation declared as part of platform conformance.
- Multi-tenant isolation
- Compressed + replicated state with erasure coding or equivalent
- Lifecycle FSM with active and cold tiers
- Capability mediation for tenant boundaries
- Diagnostic query interface (`RFC-0035`)
- Audit log (`RFC-0034`)
- Storage provider interface with at least one cloud provider
- Replicated or erasure-coded storage for Tier A durable objects

**Optional mechanisms:**
- Multi-domain federation primitives (provided v2 has shipped; in v1 these are documented as restricted to single-domain operation)
- Layer 3 offline learning
- Object projection

**Forbidden mechanisms:**
- Single-tenant assumptions
- Direct hardware-root-of-trust reliance for tenant rights

**Default verification tier:**
- Capability mediation: Tier A
- Identity: Tier A
- Lifecycle FSM: Tier A
- Audit log: Tier B
- Tenant enforcement boundary: Tier A
- Everything else: Tier D

**Minimum conformance test derivation requirements:**
- All Cloud Conformance tests MUST pass.
- Multi-tenant capability ingress MUST be verified to limit blast radius across tenants.
- Audit-log integrity MUST be verifiable end-to-end.

### 2.4 HPC Profile

**Target.** High-performance compute workloads where latency and throughput dominate.

**Mandatory mechanisms:**
- All desktop-profile mandatory mechanisms except where HPC explicitly differs.
- NUMA-aware scheduling extensions to Layer 1 and 2
- High-throughput storage provider with deterministic bandwidth characteristics
- Stream-oriented communication usage (`RFC-0029`)
- Capability-bundle execution-context propagation rules
- Replicated storage required for Tier A durable objects.

**Optional mechanisms:**
- Layer 3 offline learning if the deployment agrees to no-runtime-influence semantics.
- Lifecycle cold-tier migration.

**Forbidden mechanisms:**
- Strict synchronous busy-waiting in components claiming to be HPC-optimized
- Profile-advertised guarantees when those guarantees depend on Layer 3

**Default verification tier:**
- Capability mediation: Tier A
- Identity: Tier A
- Lifecycle FSM: Tier A
- NUMA scheduling extensions: Tier B
- Storage provider: Tier C (pragma for known-driven acceptance of past-failure modes)
- Everything else: Tier D

**Minimum conformance test derivation requirements:**
- All HPC Conformance tests MUST pass.
- Deterministic-bandwidth tests MUST demonstrate vendor profile variance.

### 2.5 Safety-Critical Profile

**Target.** Systems whose failure can cause loss of life, environmental damage, or severe economic harm.

**Mandatory mechanisms:**
- Layer 1 deterministic scheduler proved equivalent to EDF for the documented class
- All capability-critical components at Tier A with reproducible attestation
- Locked-component manifests (`RFC-0002`) — no Layer 2 or 3 changes after certification
- Bootstrap conformance fixture tests proving reproducible attestation (`RFC-0007`)
- Static lifecycle FSM with no cold or archive migration permitted in hazardous states
- Audit-log full-fidelity retention requirement during hazardous states

**Optional mechanisms:**
- Layer 2 advisory tuning —-only in pre-operational or maintenance phases; never during hazardous states

**Forbidden mechanisms:**
- Layer 3 offline learning at any time on a deployment responsible for hazardous periods
- Lifecycle FSM Archive or Expired transitions during hazardous states
- Cold-tier migration during hazardous states
- Any operation without an explicit conformance-test path

**Default verification tier:**
- Capability mediation: Tier A
- Identity: Tier A
- Lifecycle FSM: Tier A
- Scheduler core: Tier A
- Hazard-related components: Tier A
- Everything else: Tier B minimum

**Minimum conformance test derivation requirements:**
- All Safety-Critical profile conformance tests MUST pass.
- Each hazardous-state behaviour MUST have a documented conformance fixture.
- Reproduction attestation MUST be independently verifiable.

---

## 3. Profile Combinations

A single deployment may declare multiple profiles. When combining profiles, the union of forbidden mechanisms applies, and the most restrictive verification tier applies.

A single deployment claiming to be both Cloud and HPC is acceptable; a deployment claiming both Safety-Critical and Cloud without a documented Safety-Critical Cloud role is not.

---

## 4. Profile-Specific Profile Conformance Files

Each profile's conformance tests are derived from the Conformance Test Suite methodology and tailored to that profile. Profile-specific conformance files are normative suffixes to the Conformance Test Suite.

### 4.1 Embedded Conformance Subtractive Requirements

The Embedded conformance test set is the Core set minus the tests that explicitly depend on:
- POSIX projection
- Layer 2 advisory tuning with persistent state
- Lifecycle-tier cold migration

### 4.2 Safety-Critical Conformance Additive Requirements

The Safety-Critical conformance test set is the Core set plus:
- All hazardous-state behaviour conformance fixtures
- All bootstrap attestation tests
- All deterministic-scheduler proofs

---

## 5. Profile Declaration

Each implementation MUST publish a profile declaration in its conformance declaration listing:

- The profiles the implementation fully satisfies
- The profile combinations the implementation supports
- Known profile-feature gaps and their remediation status.

---

## 6. Architecture Conformance Quantification

For each profile, implementations MUST report the following quantifiable metrics:

- Memory-ceiling achieved by Layer 1 deterministic scheduler compared to theoretical minimum
- Storage-overhead ratios across all profile-mandatory state
- Per-event latency observed at conformance-test run time
- Cold-tier migration cost observed on reference workloads

These metrics MUST be reproducible from the conformance suite run artifacts.

---

## 7. Stability of Profiles

Once declared, profile interfaces (mandatory/optional/forbidden mechanisms) MUST NOT change without an Architecture-Amendment RFC. Specific tier changes within a profile adjust the Conformance Test Suite through the Specification-Amendment process.

---

## 8. Conformance Test Suite Specifics

Each profile-level conformance test file:
- Lives next to the Conformance Test Suite methodology file
- References the same component headers
- Specifies profile-pruned and profile-extended test cases
- Specifies performance-budget expectations applicable to that profile

Profiles MAY add performance-budget conformance forensics but MUST NOT alter functional conformance expectations.

---

## 9. Architectural Justification

Profiles exist because the alternative — a single deployment model — violates the Pareto-optimal principle that justifies the architecture in the first place. Real systems fail when they attempt to simultaneously optimize for embedded and HPC, Cloud and Safety-Critical. Profiles resolve that without fragmenting the architecture.

---

## Revision History — Profiles

| Date       | Change                                |
|------------|---------------------------------------|
| this cycle | Initial release. Five profiles defined (Embedded, Desktop, Cloud, HPC, Safety-Critical). Section 3 describes combination semantics. Section 4 describes conformance-test derivation requirements. |
