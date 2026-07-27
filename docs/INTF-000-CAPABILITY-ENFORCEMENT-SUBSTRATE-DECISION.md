# Decision Record: INTF-000 — Capability Enforcement Substrate

Classification: Normative
Authoritative Source: `Platform-Architecture-Specification-v1.1.md` (Invariants I-1, I-2, I-3, I-13)
Requirement-ID: ARCH-INTF-000
Status: Accepted

**Purpose.** This document records the resolved architectural decision for INTF-000 (Capability Enforcement Substrate). The decision was identified as the single most critical unresolved question in the pre-implementation dependency matrix and must be closed before `RFC-ALLOC-001`, `RFC-COMPILER-001`, and `RFC-DRIVER-001` are finalised.

This is the last planning artifact before drafting the three pre-Phase-1 RFCs.

---

## 1. Problem Statement

`Platform-Architecture-Specification-v1.1.md` invariants I-1 (Authority mediation), I-2 (No ambient authority), I-3 (Least privilege by default), and I-13 (Address–authority orthogonality) jointly require that every resource access — memory, MMIO, DMA, capability-transfer, interrupt, storage, and endpoint — be authorised through a capability token whose validity derives from an unforgeable, provenance-tracked record of how the capability came to exist.

RISC-V64 — the Phase 1 reference platform per `IMPLEMENTATION-ROADMAP.md` — has **no native hardware capability enforcement** comparable to CHERI's tagged capability registers, sealed metadata, and architectural derivation/fault semantics. The architecture must therefore decide **how capability tokens are unforgeable and provably validated** at runtime on a substrate that does not provide hardware capability semantics, without contradicting the existing invariants.

This decision affects all three pre-Phase-1 RFCs (ALLOC, COMPILER, DRIVER), the Minimum Executor, the HAL, the formal verification strategy, the compiler ABI, and the security model. It cannot be deferred past RFC-ALLOC-001 drafting.

---

## 2. Alternatives Considered

### Option A — Native CHERI-RISC-V (hardware-only enforcement)

The AetherOS capability model is realised directly by RISC-V hardware capabilities (CHERI-CRV or CHERI-RISC-V variant). Capability tokens map onto architectural capability registers; tagged memory; sealed entries; and architecturally-enforced derivation, narrowing, sealing, and revocation.

- **Pros.** Hardware-enforced unforgeability and provenance. Strongest Tier-A properties with simplest reasoning. Sealing and revocation have architectural support.
- **Cons.** As of v1.1 freeze, RISC-V CHERI silicon availability is limited (research prototypes: CHERI-Toooba, CHERI-RISC-V FPGA). Phase 1 timing aligned with hardware silicon availability is uncertain. Reference portability constrained to CHERI-capable platforms. Hardware assumption would contradict Hardware-Support-RFC.md's intent that the capability model is **architecture-neutral**.

### Option B — Software capability emulation only

Capability tokens are represented as ordinary 96-byte structures (per RFC-0037) in conventional integer registers. Every capability use passes through a trusted software mediator (the Minimum Executor and Capability Root) which validates the seal and rights before any operation.

- **Pros.** No hardware dependency; runs on any RV64 platform. Most portable.
- **Cons.** Security properties fundamentally different from hardware enforcement: a confused-compiler bug, MMIO probe, or kernel-implementation defect that exposes capability bytes to untrusted code produces forgeable capabilities. Tier-A claims of "never combinable address+authority in a single integer," — true at the seal-verification ABI level — become accident-of-implementation rather than architectural guarantee. Formal verification of capability propagation requires reasoning about every software path that touches capability bytes, dramatically expanding the verified-surface area.

### Option C — Two-layer architecture-neutral substrate (recommended)

The AetherOS **capability semantic model is architecture-neutral** and **does not assume a particular enforcement substrate**. The Execution-Provider contract (per `Hardware-Support-RFC.md`) admits **two capability enforcement providers**, each satisfying the same invariants:

```
                AetherOS Capability Model
                          │
            ┌─────────────┴─────────────┐
            ▼                           ▼
    Native Hardware Provider     Software Capability Provider
    (CHERI-RISC-V, future x86/   (Pure software validation
    ARM hardware extensions)     for non-CHERI hardware)
            │                           │
        Tagged-capability            Sealed 96-byte
        register semantics           token, software-checked
```

- **Pros.** Capability semantics remain architecture-neutral; AetherOS code that holds and passes tokens is identical across providers. Verification tier for capability-mediated components can be Tier-A when native hardware provider is active and Tier-B when software provider is active (declared in conformance declaration per I-11). Phase 1 implementation feasible in QEMU and any RV64 silicon, **without** sacrificing the long-term path to native enforcement. Aligns with Hardware-Support-RFC.md's substrate-neutrality principle, the five-peer architecture's separation of concerns, and `RFC-0037.10`–`.11`'s orthogonal address/authority encoding (which is preserved regardless of enforcement substrate).
- **Cons.** The capability enforcement provider must be declared per-deployment. Two provider implementations mean two enforcement codebases until hardware capability RISC-V becomes routine. Software provider's Tier-B limits the formal reach of Tier-A claims in Phase 1.

---

## 3. Why Option C Over A and B

- **vs. Option A:** Phase 1 timing is incompatible with current CHERI-RISC-V silicon availability; locking AetherOS to a single hardware substrate contradicts the Hardware-Support-RFC's intent.
- **vs. Option B:** Pure software emulation collapses `RFC-0037.10`–`.11` orthogonality from architectural property to implementation discipline, and expands the verified surface area to include every byte-touching code path. It does not satisfy the intent of I-13 as an architectural primitive.
- **Over A and B:** Capability M-Model semantics are substrate-neutral. Application code holds, derives, transfers, and revokes tokens identically regardless of provider. The enforcement contract is a capability-enforcement-provider interface within the HAL. **This aligns with the five-peer architecture, Hardware-Support-RFC's separation of execution-provider contract from hardware specifics, and the existing term `Execution Provider` (per RFC-0037 / Glossary) which already accommodates multiple substrate types.**

---

## 4. Accepted Decision

**Option C is accepted.** The AetherOS capability model is architecture-neutral; capability enforcement is delegated to a substrate-specific provider selected per deployment. `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md` INTF-000 interface resolves as follows.

### 4.1 Capability Enforcement Substrate Interface (INTF-000)

| Property | Specification |
|----------|--------------|
| **Unforgeability model** | A capability token cannot be created except by a correctly sealed derivation chain rooted in the platform master seal key. The seal-verification primitive is the only path by which a byte sequence becomes a valid capability. Whether seal verification is performed by hardware (tagged register, dedicated instruction) or software (trusted mediator on token dereference) is the provider's choice; the interface presented to AetherOS code is identical. |
| **Provenance model** | Every new capability has a recorded parent (`rights_derived` counter per `RFC-0037.12`). Revocation (`RFC-0039`) walks the lineage. Provenance is mandatory and enforced by the provider. |
| **Representation** | The `RFC-0037` canonical record is a versioned serialization for persistence, audit, and transfer; it is **not** the mandatory live or hot-path representation. Conventional providers use compact, domain-local, generational capability-space handles whose table entries hold authority and provenance. Native providers may use tagged hardware capabilities. Serialization/deserialization is an explicit mediated operation and never implies that arbitrary bytes become live authority. |
| **Derivation** | Right-narrowing is performed by a seal-signed operation; the resulting token has incremented `rights_derived` counter (per `RFC-0037.12`). The narrowing invariant — derived rights are a subset of parent rights — is the provider's hard guarantee. |
| **Revocation** | Driven by `RFC-0039`. The provider must invalidate tokens whose lineage contains a revoked ancestor within the declared latency bound. |
| **Address interaction** | I-13 is semantic rather than a bit-layout prohibition: an address without validated authority grants nothing, and authority does not invent an undeclared locator. CHERI-style tagged representations are conformant when independent rights/bounds/tag validation preserves those properties. Canonical serialization keeps locator and authorization fields independently validateable. |
| **Fault semantics** | Any failed capability check (invalid seal, expired, insufficient rights, revoked, cross-domain) yields a single observable fault (`ECAPSTALE` per `RFC-0025.8`). The fault MUST NOT corrupt any resource. The provider MUST NOT silently drop; it MUST record the attempt in observability (per `RFC-0034.1` capability events). |
| **Trust boundary** | The provider's seal-verification primitive is the trust boundary. Above the primitive, AetherOS code may assume capability semantics. Below the primitive, the provider's internal implementation is software-trusted only if the deployment uses the software provider. |
| **Performance contract** | The live check is an O(1) tagged-capability validation or bounded capability-space lookup plus generation, rights, type, and domain checks. HMAC/canonical-record verification occurs at explicit import, persistence, or cross-trust serialization boundaries—not on every intra-domain dereference. Bounds and benchmark distributions are declared per `RFC-0040.13`. |
| **Verification boundary** | Tier A applies to the capability semantics visible at the provider boundary. The hardware provider's internal mechanism is hardware-trusted; the software provider's check sequence is verified. **Provider choice is declared per I-11.** |

### 4.2 Implementation Providers Defined by This Decision

1. **Native Capability Provider (Provider NCP).** Targets hardware capability architectures. First concrete provider: CHERI-RISC-V (when available). Implementation maps `RFC-0037` token onto architectural register + tag semantics. Tier A.
2. **Software Capability Provider (Provider SCP).** Targets conventional RV64, x86-64, and ARM64. A live capability is a compact unforgeable handle into a kernel/provider-owned capability space. Importing a canonical record validates its seal, provenance, type, rights, freshness, and domain once and creates a local entry; use validates the entry and generation. Tier B initially, eligible for Tier A after refinement proof.
3. **Hybrid Provider (Provider HCP, future).** May use hardware-assisted sealing (e.g., AES-NI, RISC-V vector crypto extensions) where available, with software fallback. Future RFC.

### 4.3 Phase 1 Provider Assignment

**Phase 1 implements Provider SCP.** Rationale:

- QEMU RISC-V and current RISC-V silicon (SiFive FU740, StarFive JH7110) do not include CHERI capability extensions. Phase 1 must boot on these platforms.
- Capabilities, derivation, revocation, and I-13 invariants are realised through protected capability spaces and verified initially at Tier B. The semantic model is unchanged.
- **Canonical bytes are accepted only by a small import/export mediator; ordinary calls carry compact handles**, keeping cryptography and 96-byte copies off the hot path and preventing ordinary memory bytes from becoming authority.

### 4.4 Migration Path to Native Provider

When a Native Provider is available, the migration sequence is:
1. Native Provider implementation accepted (Phase 3+).
2. Conformance declaration per I-11 declares the new provider.
3. Hybrid deployment simultaneously uses both providers, exercising capability tokens across provider boundaries; CTS cross-provider tests verify semantic identity (per `RFC-0041.2`).
4. Software Provider retained as a removable layered extension (per I-9), not as a TCB component.

The migration does NOT require changes to `RFC-0037` through `RFC-0040` — the capability semantic model is unchanged.

---

## 5. Architectural Consequences

- **`RFC-0037` (Capability Token Format) and `RFC-0038`/`RFC-0039` (Transfer/Revocation) are unmodified.** The Capability Token Format operates above the provider boundary.
- **`RFC-0013` (Capability Mediation) gains a parameterisation** — it dispatches to the active provider per deployment. This is implementation-level and does not change the contract.
- **`INTF-000` becomes a new normative interface** in `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md`. RFC-ALLOC-001, RFC-COMPILER-001, and RFC-DRIVER-001 inherit from it; none of them define capability semantics themselves.
- **The Tier A -> Tier B distinction** for capability-related invariants becomes deployment-relative. Conformance declarations must declare the provider and tier.

## 6. Verification Consequences

- **Phase 1 Tier B for capability path.** Software provider requires verified seal-check, rights-check, and lineage-check correctness. This is reachable through model checking per `RFC-0040` Tier B.
- **Tier A properties move with native provider.** When Provider NCP is active, capability check correctness is hardware-trusted; formal verification can target the AetherOS code that *creates / passes / narrows* tokens, which is much smaller than the mediation code itself.
- **Formal obligation: hardware / software path equivalence.** A formal model must prove that the two providers accept the same set of well-formed capabilities. CTS cross-provider tests provide empirical validation on top.

## 7. Implementation Consequences

- **First Phase 1 substrate-neutral deliverable.** The capability-space lookup and canonical import/export mediator form the boundary between provider and the rest of AetherOS. No source-line-count claim is made before implementation; size, cycles, cache behavior, and proof surface are measured by the Phase B gate.
- **No CHERI hardware emulator required for Phase 1.** The Software Provider runs on every RISC-V emulation target.
- **Compiler ABI (§ `RFC-COMPILER-001`) passes a pointer-width local handle on conventional ISAs and a tagged native capability where an approved ABI supports it.** Cross-domain calls invoke generated stubs that transfer/attenuate capabilities through the mediator. Canonical records never occupy a fixed set of argument registers.

## 8. CTS Impact

A new CTS class is declared, `CTS-INTF-000-N`:

- **`CTS-INTF-000-1`:** Provider declared in conformance declaration (I-11).
- **`CTS-INTF-000-2`:** Capability check completes within declared bound for the active provider.
- **`CTS-INTF-000-3`:** Native-provider capability check produces identical semantic outcome to software-provider capability check for the input token.
- **`CTS-INTF-000-4`:** Provider swap without capability-token shape change.

## 9. Supersedes

- `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md` open question: "Is capability enforcement hardware (CHERI) or software (mediator)? Not yet decided." Resolved as Option C above.

## 10. Superseded By

None.

---

## Acceptance Signature

This revised decision is the prototype baseline for Phase A modeling and Phase B implementation. It is normative input to `RFC-ALLOC-001`, `RFC-COMPILER-001`, and `RFC-DRIVER-001`, but handle layout, revocation structure, canonical cryptography, and provider equivalence remain evidence gates. I-13 is semantic orthogonality, permitting tagged CHERI-style packing without treating a raw address as authority.

A future Architecture-Amendment is required only if a new invariant is introduced (e.g., making native hardware capability mandatory). No such amendment is planned.

---

## Revision History — INTF-000 Decision

| Date       | Change |
|------------|--------|
| this cycle | Initial INTF-000 decision accepted. Option C (two-layer architecture-neutral substrate) adopted. Phase 1 implements Software Capability Provider. Native Provider reserved for Phase 3+ when hardware is available. |
