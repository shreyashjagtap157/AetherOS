# Pre-Implementation Dependency Matrix

Classification: Operational
Authoritative Source: `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md`
Requirement-ID: `GOV-PRE-001`

**Status:** Active. Revision 2 incorporating architectural review feedback: added INTF-000 (Capability Enforcement Substrate), revised formal verification sequencing, revised boot ordering, added Implementation Contract layer, strengthened DMA semantics, updated blocking relationships, and revised invariant language.

This document is the last major planning artifact before drafting RFC-ALLOC-001, RFC-COMPILER-001, and RFC-DRIVER-001. It must be updated if the dependency chain changes.

---

## Purpose

This matrix maps every requirement that blocks Phase 1 implementation to:

1. The RFC that must specify it
2. The architectural invariant(s) it realises
3. The implementation component(s) it governs
4. The CTS test that will verify it
5. The formal verification obligation (Tier A/B/C/D)

The matrix exists to make the dependency chain between the three missing RFCs **explicit and verifiable** before any of them are drafted. Writing them in the wrong order, without this map, risks defining interfaces that later conflict.

---

## Legend

| Symbol | Meaning |
|--------|---------|
| **I-N** | Invariant from `Platform-Architecture-Specification-v1.1.md` |
| **RFC-NNNN** | Normative RFC in `RFCs/` subdirectories |
| **RFC-ALLOC** | RFC-ALLOC-001 (Allocator Hierarchy) |
| **RFC-COMP** | RFC-COMPILER-001 (Compiler and Runtime Boundary) |
| **RFC-DRIVER** | RFC-DRIVER-001 (Driver Framework Architecture) |
| **INTF-000** | Capability Enforcement Substrate (new — see Section 3.0) |
| **INTF-N** | Cross-RFC interface (see Section 3) |
| **CTX-N** | CTS test identifier pattern |
| **VT-N** | Verification tier (A/B/C/D) |

---

## 1. Requirements Blocking Phase 1

These are drawn from `IMPLEMENTATION-ROADMAP.md` Section 2 and from `00-RFC-Index.md` "Future RFCs Required Before Implementation" table.

---

### REQ-INTF-000: Capability Enforcement Substrate (Blocker for the three RFCs)

**Statement:** Before any of the three blocker RFCs can be drafted, the architecture must specify the capability enforcement substrate — the mechanism by which capability tokens (per `RFC-0037`) are made unforgeable, provenance-tracked, and verificable at the mediation boundary. This decision is recorded separately in `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md`.

**Owning decision record:** `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md`
**Cross-referenced by:** `RFC-ALLOC-001`, `RFC-COMPILER-001`, `RFC-DRIVER-001`

**Realised invariants:**
- I-1 (Authority mediation — every check yields capability-mediated outcome)
- I-2 (No ambient authority — unforgeability prevents ambient construction)
- I-13 (Address–authority orthogonality — provider does not infer one from the other)

**Why this is a precondition, not a downstream concern:**
- `RFC-ALLOC-001` (memory / DMA / authority allocation) needs to know how allocation-capability tokens are sealed and verified.
- `RFC-COMPILER-001` (ABI / capability passing / register convention) needs to know whether capabilities are architectural hardware entities or software-mediated 96-byte tokens. The decision drives register convention, faulting representation, and entry/exit protocol.
- `RFC-DRIVER-001` (MMIO / DMA / device capability) needs to know whether MMIO/DMA capabilities are mediated by hardware tags or by a software-mediator boundary. The decision also drives IOMMU ownership semantics.

**Resolved decision (see `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md`):**

The capability system has an architecture-neutral semantic model. Enforcement is delegated to one of two providers selected per deployment:
- **Native Capability Provider (NCP)** — hardware capability architectures (CHERI-RISC-V when available). Tier A.
- **Software Capability Provider (SCP)** — software mediation on conventional RV64. Tier B.

Phase 1 implements the SCP, retained as a removable layered extension when NCP becomes available in Phase 3+. The capability token format (`RFC-0037`) is unchanged.

**Implementation components:**
- Capability-mediator shim (the trust anchor above which AetherOS code is substrate-neutral)
- Provider-selection registry (declared per I-11 in conformance declaration)
- Seal verifier (HMAC-SHA-256 truncated to 16 bytes per `RFC-0037.8`)
- Domain-bound check (per `RFC-0037.15`)
- Cross-provider semantic-equivalence tests

**CTS tests:**
- `CTX-INTF-000-1`: Active provider declared in conformance declaration
- `CTX-INTF-000-2`: Capability check completes within declared bound for the active provider
- `CTX-INTF-000-3`: Native-provider and software-provider capability checks produce identical semantic outcome for the input token
- `CTX-INTF-000-4`: Provider swap without capability-token shape change

**Verification tier:** VT-A (capability semantics) **with provider-stratified sub-tiers**: VT-A under NCP; VT-B under SCP. Conformance declaration declares which is in force.

**Formal verification obligations:**
- Proof that capability path cannot create capabilities without explicit grant (I-2) — VT-A
- Proof that software provider's check sequence produces same outcome as the hardware provider for any well-formed token — VT-B
- Proof that capability semantics at the AetherOS-code boundary are identical regardless of provider — VT-A

---

### REQ-ALLOC-01: Physical memory allocation with authority tracking

**Statement:** The platform must specify how physical memory is allocated, by whom, under whose authority, and with what capability representation.

**Owning RFC:** `RFC-ALLOC-001`

**Realised invariants:**
- I-1 (Authority mediation — every resource access authorised via capability)
- I-2 (No ambient authority — allocation authority must be explicitly granted)
- I-3 (Least privilege — newly created allocation capabilities grant minimum rights)
- I-13 (Address–authority orthogonality — allocation creates address; capability grants authority)

**Feeds into:**
- `RFC-COMPILER-001` (§ ABI memory model, § TLS model, § stack model)
- `RFC-DRIVER-001` (§ DMA capability, § driver memory ownership)
- `RFC-0009` (Address Space — HAL interface for page-table operations)
- `RFC-0011` (Scheduler — memory for scheduler structures)
- `RFC-0037` (Capability Token Format — storage of allocation capabilities)

**Implementation components:**
- Page allocator (buddy system, fragmentation threshold)
- Slab allocator (kernel object cache)
- NUMA policy engine
- Huge-page promotion/split logic
- DMA-capable memory region tracker

**CTS tests:**
- `CTX-ALLOC-001`: Allocator grants no capability without explicit request
- `CTX-ALLOC-002`: Least-privilege allocation capability derivation
- `CTX-ALLOC-003`: NUMA allocation respects capability-specified node constraints
- `CTX-ALLOC-004`: Fragmentation triggers do not exceed declared budget

**Verification tier:** VT-A (capability mediation layer); VT-C (slab/page allocator algorithms); VT-D (NUMA policy)

**Formal verification obligations:**
- Formal proof that allocation capabilities cannot be silently created (I-2)
- Formal proof that right-narrowing on derived allocation capabilities holds (I-3)

---

### REQ-ALLOC-02: Capability-addressable memory regions

**Statement:** Memory regions that are capability-addressable must have a defined representation, lifetime, and revocation semantics distinct from raw physical pages.

**Owning RFC:** `RFC-ALLOC-001`

**Realised invariants:**
- I-1, I-2, I-3, I-13

**Feeds into:**
- `RFC-COMPILER-001` (§ object representation, § capability passing)
- `RFC-DRIVER-001` (§ MMIO capability representation)
- `RFC-0009` (Address Space — which allocations are mappable)
- `RFC-0013` (Capability Mediation — what memory capabilities can be mediated)

**Implementation components:**
- Typed memory region descriptor
- Capability-backed allocation handle
- Revocation chain for allocated regions

**CTS tests:**
- `CTX-ALLOC-005`: Allocated region identity is content-resolved and immutable
- `CTX-ALLOC-006`: Revocation of allocation capability removes reachability

**Verification tier:** VT-A

---

### REQ-ALLOC-03: DMA ownership and memory region isolation

**Statement:** Device DMA access to memory must be explicitly capability-authorised. DMA without a valid DMA capability is impossible.

**Owning RFC:** `RFC-DRIVER-001` (primary — DMA ownership is a driver-framework concern)
**Second:** `RFC-ALLOC-001` (DMA-safe allocation pools)

**Realised invariants:**
- I-1 (DMA is a resource access)
- I-2 (no ambient DMA authority)
- I-10 (explicit failure semantics — DMA without authority must fail predictably)

**Feeds into:**
- `RFC-ALLOC-001` (DMA-safe pool management)
- `RFC-COMPILER-001` (device memory in address space model)
- `RFC-0010` (Interrupt Routing — DMA completion interrupts)
- `RFC-0009` (Address Space — IOMMU integration)

**Implementation components:**
- IOMMU page-table programming (via HAL)
- DMA window descriptor
- Device-to-memory capability mapping

**CTS tests:**
- `CTX-DRIVER-001`: DMA access fails without valid DMA window capability
- `CTX-DRIVER-002`: DMA capability revocation revokes IOMMU window

**Verification tier:** VT-A

---

### REQ-ALLOC-04: Deterministic memory allocation for scheduler

**Statement:** Layer 1 (deterministic core scheduler) memory allocation decisions must be bounded and deterministic. No dynamic allocation on the scheduler hot path.

**Owning RFC:** `RFC-ALLOC-001`
**References:** `RFC-0011`

**Realised invariants:**
- I-8 (deterministic core verifiable)

**Feeds into:**
- `RFC-0011` (Scheduler — Layer 1 must not allocate on hot path)

**Implementation components:**
- Pre-allocated scheduler control blocks
- Static priority queue structures
- Lock-free pre-allocated dispatch queues

**CTS tests:**
- `CTX-ALLOC-007`: Scheduler hot path completes without heap allocation
- `CTX-SCHED-001`: Scheduler dispatch latency is bounded by constant, not workload

**Verification tier:** VT-A (hot-path allocation bounds); VT-C (scheduler correctness proofs)

---

### REQ-COMP-01: ABI calling conventions including capability register usage

**Statement:** The platform ABI must specify how capability tokens cross function boundaries — which registers carry them, how they are preserved, and what happens when a capability crosses a trust boundary.

**Owning RFC:** `RFC-COMPILER-001`

**Realised invariants:**
- I-1 (capability required for resource access)
- I-2 (no ambient authority — capability passing must be explicit)
- I-13 (address and authority are separate; both may cross ABI boundary)

**Feeds into:**
- `RFC-DRIVER-001` (driver IPC crosses capability ABI boundary)
- `RFC-0025` (Endpoint Model — capabilities as message arguments)
- `RFC-0027` (Capability Transfer — transfer semantics at ABI level)
- `RFC-0037` (Capability Token Format — register representation)
- `RFC-0007` (Execution Domain — capability crossing domain boundary)

**Implementation components:**
- Calling convention definition (RISC-V: a0–a7 for capabilities, or a dedicated capability register file)
- Callee-saved capability registers
- Trust-boundary crossing convention (user↔kernel, user↔device)
- Capability crossing assembly sequences

**CTS tests:**
- `CTX-COMP-001`: Capability passed to function is identical on entry and exit
- `CTX-COMP-002`: Capability not explicitly passed cannot be derived by callee
- `CTX-COMP-003`: Invalid capability register state triggers fault, not silent drop

**Verification tier:** VT-A (capability boundary enforcement); VT-C (ABI specification)

**Formal verification obligations:**
- Proof that capability registers are never implicitly created (I-2)
- Proof that ABI cannot be violated without hardware fault

---

### REQ-COMP-02: Object model at ABI level

**Statement:** The representation of capability-addressable objects in memory must be defined at the ABI level, independently of any specific compiler.

**Owning RFC:** `RFC-COMPILER-001`
**References:** `RFC-0015` (Object Identity), `RFC-0021` (Object Projection)

**Realised invariants:**
- I-6 (cryptographic integrity — object identity must be verifiable)
- I-10 (explicit failure semantics)
- I-13 (address–authority orthogonality)

**Feeds into:**
- `RFC-ALLOC-001` (object backing store)
- `RFC-DRIVER-001` (device object representation)

**Implementation components:**
- Object header format (identity field, capability slot vector, Merkle anchor)
- Object reference representation in registers
- Null capability representation
- Invalid capability representation (faulting vs. non-faulting)

**CTS tests:**
- `CTX-COMP-004`: Object identity field is immutable post-creation
- `CTX-COMP-005`: Invalid object reference triggers defined fault

**Verification tier:** VT-A

---

### REQ-COMP-03: Component entry, exit, and error propagation

**Statement:** The contract for how a compiled component enters, exits, propagates errors, and handles exceptions must be defined before the first component can be linked.

**Owning RFC:** `RFC-COMPILER-001`

**Realised invariants:**
- I-4 (observable state transitions — component entry/exit must emit events)
- I-10 (explicit failure semantics)

**Feeds into:**
- `RFC-0007` (Execution Domain — entry point contract)
- `RFC-0031` (Event Model — required events at component boundary)
- `RFC-DRIVER-001` (driver component lifecycle)

**Implementation components:**
- Component entry stub (capability environment setup, TLS pointer, stack pointer validation)
- Error return convention (error capability vs. error code)
- Exception delivery to component (synchronous fault model)
- Component exit stub (capability cleanup, finalisation events)

**CTS tests:**
- `CTX-COMP-006`: Every component exit emits a lifecycle event before state is reclaimed
- `CTX-COMP-007`: Uncaught exception propagates as defined capability fault

**Verification tier:** VT-C (entry/exit protocol); VT-A (error capability semantics)

---

### REQ-COMP-04: Position-independent code and address space layout

**Statement:** Component address spaces are established at load time, not compile time. The compiler must emit position-independent code, and the ABI must define the load-time layout contract.

**Owning RFC:** `RFC-COMPILER-001`
**References:** `RFC-0009` (Address Space)

**Realised invariants:**
- I-9 (replaceable mechanisms — components must be relocatable)

**Feeds into:**
- `RFC-0009` (address space establishment)
- `RFC-DRIVER-001` (driver loading into dynamic address space)
- `RFC-0002` (Component Manifest — load-time relocations)

**Implementation components:**
- GOT/PLT convention for capability-relative relocations
- Load-time relocations table format
- ASLR-compatible entry point resolution
- Address space layout randomisation (component base)

**CTS tests:**
- `CTX-COMP-008`: Compiled component runs correctly at any load address
- `CTX-COMP-009`: Capability-relative relocation resolves to correct object

**Verification tier:** VT-C

---

### REQ-COMP-05: Symbol resolution and component linking

**Statement:** How components import and export symbols via the Composition Architecture must be defined at the ABI level.

**Owning RFC:** `RFC-COMPILER-001`
**References:** `RFC-0002` (Component Manifest), `RFC-0003` (Capability Routing)

**Feeds into:**
- `RFC-0002` (component manifest specifies exported symbols as capabilities)
- `RFC-0003` (capability routing uses exported symbols as endpoints)
- `RFC-DRIVER-001` (driver symbol export for service registration)

**Implementation components:**
- Symbol table format (capability symbols vs. data symbols distinguished)
- Weak symbol semantics
- Symbol resolution algorithm (load-time vs. runtime)
- Dynamic symbol import/export via capability namespace

**CTS tests:**
- `CTX-COMP-010`: Capability symbol resolves to same object across component instances
- `CTX-COMP-011`: Symbol not in manifest cannot be resolved (isolation enforcement)

**Verification tier:** VT-C

---

### REQ-DRIVER-01: Driver lifecycle (discovery, load, initialise, fault, unload)

**Statement:** The driver framework must specify the complete lifecycle of a driver — from hardware discovery through capability-bound operation to unload — as a first-class capability lifecycle.

**Owning RFC:** `RFC-DRIVER-001`

**Realised invariants:**
- I-1 (every driver operation authorised via capability)
- I-2 (no ambient driver authority)
- I-9 (replaceable mechanisms — drivers are replaceable)
- I-10 (explicit failure modes at every lifecycle transition)
- I-4 (lifecycle transitions are observable)

**Feeds into:**
- `RFC-0018` (Lifecycle FSM — driver as a managed component)
- `RFC-0031` (Event Model — driver lifecycle events required)
- `RFC-0010` (Interrupt Routing — interrupt routing to driver components)

**Implementation components:**
- Driver manifest (capabilities required, hardware IDs, version)
- Driver registry (capability-addressable)
- Driver load protocol (capability grant sequence)
- Driver fault containment (driver runs in isolated address space)
- Driver unload protocol (capability revocation sequence)

**CTS tests:**
- `CTX-DRIVER-003`: Driver cannot operate without declared capabilities
- `CTX-DRIVER-004`: Driver fault does not leak capability to other drivers
- `CTX-DRIVER-005`: Driver unload fully revokes all driver capabilities

**Verification tier:** VT-A (capability lifecycle enforcement); VT-B (driver isolation)

**Formal verification obligations:**
- Proof that driver cannot acquire capability not declared in manifest
- Proof that driver fault cannot extend capability scope beyond granted set

---

### REQ-DRIVER-02: Hot-plug capability renegotiation

**Statement:** Device add/remove must be modelled as a capability renegotiation event, not a physical event.

**Owning RFC:** `RFC-DRIVER-001`
**References:** `RFC-0039` (Capability Revocation Protocol)

**Realised invariants:**
- I-1, I-2, I-3, I-9, I-10

**Feeds into:**
- `RFC-0039` (revocation protocol used for device removal)
- `RFC-0018` (Lifecycle FSM — device removal as lifecycle transition)

**Implementation components:**
- Device add event (capability grant sequence for new device)
- Device remove event (capability revocation cascade)
- Capability renegotiation protocol
- Graceful degradation contract

**CTS tests:**
- `CTX-DRIVER-006`: Device removal revokes all capabilities held by removed driver
- `CTX-DRIVER-007`: New device capability is isolated from existing drivers until explicitly granted

**Verification tier:** VT-A

---

### REQ-DRIVER-03: Driver-to-driver dependency and ordering

**Statement:** Drivers may depend on other drivers. Dependency ordering must be declared in manifests and enforced through capability gating.

**Owning RFC:** `RFC-DRIVER-001`
**References:** `RFC-0002` (Component Manifest), `RFC-0004` (Dependency Resolution)

**Realised invariants:**
- I-2, I-3, I-9, I-10

**Feeds into:**
- `RFC-0004` (Dependency Resolution — drivers are components with declared dependencies)
- `RFC-0002` (Component Manifest — driver manifest extends component manifest)

**Implementation components:**
- Driver dependency declaration (in manifest)
- Capability-gated initialisation order
- Dependency cycle detection
- Failed dependency fault containment

**CTS tests:**
- `CTX-DRIVER-008`: Cyclic driver dependencies detected at load time
- `CTX-DRIVER-009`: Driver initialises only after all dependency capabilities are granted

**Verification tier:** VT-B

---

### REQ-DRIVER-04: Power management integration

**Statement:** Device suspend/resume must be modelled as a capability suspension and restoration event.

**Owning RFC:** `RFC-DRIVER-001`

**Realised invariants:**
- I-10 (explicit failure semantics — what happens to in-flight operations during suspend)

**Feeds into:**
- `RFC-0012` (Time — timer state preservation across suspend)
- `RFC-0010` (Interrupt Routing — interrupt routing across suspend/resume)

**Implementation components:**
- Device suspend protocol (capability suspension, in-flight operation completion)
- Device resume protocol (capability restoration, capability validity verification)
- System suspend coordinator

**CTS tests:**
- `CTX-DRIVER-010`: Suspended device capability cannot be used for new operations
- `CTX-DRIVER-011`: Resume restores exactly the capability set present before suspend

**Verification tier:** VT-B

---

### REQ-DRIVER-05: MMIO capability representation

**Statement:** Memory-mapped I/O regions must be represented as capability-addressable resources, not raw pointer values.

**Owning RFC:** `RFC-DRIVER-001`
**References:** `RFC-ALLOC-001` (§ DMA-capable memory), `RFC-COMP-01` (§ capability passing)

**Realised invariants:**
- I-1, I-2, I-13 (MMIO region is an address; capability grants authority to access it)

**Feeds into:**
- `RFC-COMPILER-001` (MMIO access through capability-typed pointers)
- `RFC-ALLOC-001` (MMIO regions are allocated from a separate pool, not generic heap)
- `RFC-0009` (Address Space — MMIO mapping in component address space)

**Implementation components:**
- MMIO capability type (distinct from memory allocation capability)
- MMIO region descriptor (base address, size, access rights)
- MMIO capability validation at access point

**CTS tests:**
- `CTX-DRIVER-012`: MMIO access without MMIO capability triggers fault
- `CTX-DRIVER-013`: MMIO capability grants only declared access rights (read/write/expanding)

**Verification tier:** VT-A

---

## 2. Dependency Graph (RFC-by-RFC)

### RFC-ALLOC-001 dependencies

```
Requires from Architecture:
  - I-1, I-2, I-3, I-8, I-13 (Platform-Architecture-Specification-v1.1.md)
  - INTF-000 (Capability Enforcement Substrate — records substrate decision; SCP active in Phase 1)
  - RFC-0037 (Capability Token Format) — for capability representation of allocations
  - RFC-0009 (Address Space) — for page allocator interface
  - RFC-0011 (Scheduler) — for deterministic allocator requirements

Requires from RFC-COMPILER-001: NONE
  (allocator is below the compiler boundary)

Requires from RFC-DRIVER-001:
  - None (driver depends on allocator, not vice versa)

Delivers to:
  → RFC-COMPILER-001: memory model, TLS model, stack allocation
  → RFC-DRIVER-001: DMA memory pools, MMIO region pools
  → RFC-0007: bootstrap memory allocator
  → RFC-0011: scheduler memory (pre-allocated, no hot-path allocation)
  → INTF-001 (Memory region capability type) — defined here, blocks downstream
  → INTF-004 (DMA capability semantics) — co-defined here with RFC-DRIVER-001, blocks it
  → INTF-005 (Stack/heap allocation capability) — defined here, blocks RFC-COMPILER-001
```

### RFC-COMPILER-001 dependencies

```
Requires from Architecture:
  - I-1, I-2, I-13 (capability passing at ABI boundary)
  - INTF-000 (Capability Enforcement Substrate — drives register convention, faulting representation)
  - RFC-0037 (Capability Token Format) — register representation
  - RFC-0038 (Capability Transfer Protocol) — transfer semantics
  - RFC-0025 (Endpoint Model) — capability as message argument
  - RFC-0007 (Execution Domain) — entry/exit protocol

Requires from RFC-ALLOC-001:
  - Memory region descriptor format (for global/local/TLS allocation)
  - DMA-safe memory pool identifier (for device-accessible allocations)
  - Stack lower-bound capability (for stack overflow detection)
  - INTF-005 (Stack/heap allocation capability) — consumed here

Delivers to:
  → RFC-DRIVER-001: ABI boundary for driver components, capability register convention
  → RFC-0002 (Component Manifest): symbol export format, load-time relocations
  → RFC-0027 (Capability Transfer): transfer at ABI boundary
  → INTF-002 (Capability register convention) — defined here, blocks RFC-DRIVER-001
  → INTF-003 (Component entry protocol) — defined here, blocks RFC-DRIVER-001
  → INTF-006 (Error propagation at ABI) — defined here, blocks RFC-DRIVER-001
```

### RFC-DRIVER-001 dependencies

```
Requires from Architecture:
  - I-1, I-2, I-3, I-9, I-10 (driver lifecycle model)
  - INTF-000 (Capability Enforcement Substrate — drives MMIO/DMA capability representation)
  - RFC-0039 (Capability Revocation Protocol) — for hot-unplug
  - RFC-0018 (Lifecycle FSM) — driver as component with defined states
  - RFC-0031 (Event Model) — lifecycle events

Requires from RFC-ALLOC-001:
  - DMA-capable memory pool (REQ-ALLOC-03)
  - MMIO region pool (REQ-DRIVER-05 via allocator)
  - Capability representation for memory regions
  - INTF-001 (Memory region capability type) — consumed here
  - INTF-004 (DMA capability semantics) — consumed/co-defined here

Requires from RFC-COMPILER-001:
  - Component entry/exit convention (REQ-COMP-03)
  - Capability register ABI (REQ-COMP-01)
  - Object representation at ABI (REQ-COMP-02)
  - Symbol resolution (REQ-COMP-05)
  - INTF-002 (Capability register convention) — consumed here
  - INTF-003 (Component entry protocol) — consumed here
  - INTF-006 (Error propagation at ABI) — consumed here

Delivers to:
  → RFC-0007: driver as component type with hardware capability bindings
  → RFC-0010: interrupt routing to driver components
  → All hardware provider HALs (x86-64, ARM64, RISC-V): driver model is HAL-agnostic
```

**Specification dependency chain** (this is one relationship; engineering may parallelise — see Section 8):

```
INTF-000 (Capability Enforcement Substrate) [resolved]
       │
       └────┬───────┬───────┐
            ▼       ▼       ▼
       ALLOC      COMP    DRIVER
                │     │       │
                │     │       │
       ALLOC -->│     │       │
                ▼     │       │
              COMP -->│       │
                      ▼       ▼
              COMP  --> DRIVER
              ALLOC  --> DRIVER
              COMP   --> DRIVER
```

Engineers may parallelise the *drafting* of subsections after INTF-000 is resolved and the cross-RFC interfaces they need are stable, but the dependency chain above is normative for *interface agreements* — an interface defined by RFC-ALLOC-001 cannot be second-guessed by RFC-COMPILER-001.

---

## 3. Cross-RFC Interface Inventory

Every interface that must be agreed upon between the three RFCs before any of them can be finalised. **INTF-000 must be resolved before any section below**, as it drives the architecture choice that subsequent interfaces assume.

### INTF-000: Capability Enforcement Substrate

**Defined by:** `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md` (accepted decision record)
**Consumed by:** `RFC-ALLOC-001`, `RFC-COMPILER-001`, `RFC-DRIVER-001`, all RFCs that reason about capability tokens above the mediation layer

This interface specifies how capability tokens (`RFC-0037`) are made unforgeable, provenance-tracked, and verifiable at the mediation boundary. The decision is **accepted** and documented; this section captures the interface properties that the three blocker RFCs inherit.

**Properties preserved across all providers:**

- **Unforgeability.** A capability cannot be constructed except by a correctly sealed derivation chain rooted in the platform master seal key. The seal-verification primitive is the only path that turns a byte sequence into a valid capability.
- **Provenance.** Every new capability has a recorded parent (`rights_derived` counter per `RFC-0037.12`). Revocation walks lineage.
- **Representation.** AetherOS code holds and passes the canonical 96-byte capability token (`RFC-0037.1`) regardless of provider. The active provider may use a different in-register representation (e.g., 128-bit architectural capability register under NCP) as long as canonical-form round-trip is available.
- **Derivation.** Right-narrowing is sealed-signed; resulting token has incremented `rights_derived` counter. Derived rights must be a subset of parent rights.
- **Revocation.** Per `RFC-0039`. Provider must invalidate tokens whose lineage contains a revoked ancestor.
- **Address interaction.** Provider does not derive authority from address bits; provider does not derive address from authority bits. I-13 must hold at the provider boundary.
- **Fault semantics.** Any failed capability check yields the single observable `ECAPSTALE` (`RFC-0025.8`). The fault must not corrupt any resource.
- **Trust boundary.** Provider seal-verification is the trust anchor. Above it, AetherOS code may assume capability semantics.

**Provider choices (provider-selected per deployment per I-11):**

- **Native Capability Provider (NCP):** hardware-tagged capability architectures. First concrete: CHERI-RISC-V. Tier A.
- **Software Capability Provider (SCP):** software-mediated capability checks on conventional RV64 / x86-64 / ARM64. Tier B.
- **Hybrid Capability Provider (HCP, future):** hardware-assisted sealing with software fallback. Deferred.

**Phase 1 substrate assignment:** SCP. Rationale: QEMU RISC-V and SiFive FU740 / StarFive JH7110 lack CHERI extensions; Phase 1 must boot on these platforms; the SCP is the smallest path to a Tier-B-conformant capability system in Phase 1.

**Questions this interface resolves for downstream RFCs:**
- How does a freshly minted allocation-capability token reach AetherOS code? — Via sealed derivation, regardless of provider.
- What is the protection against a confused compiler that leaks capability bytes to user code? — Seal verification at the provider boundary; the bytes are useless until checked.
- Is there a hardware-tagged capability in Phase 1? — No. The architecture models a tag; the implementation realises it in software for Phase 1.

**Sub-questions deferred to specific blocker RFCs:**
- Which provider-active control register / sys-call gate? — `RFC-COMPILER-001` (INTF-002).
- Which allocator-call sites construct the first token of a kind? — `RFC-ALLOC-001`.
- Which provider token is presented to a driver? — `RFC-DRIVER-001`.

**Status:** RESOLVED. Recorded in `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md`. No further interface negotiation required at this level.

---

### INTF-001: Memory region capability type

**Defined by:** `RFC-ALLOC-001`
**Consumed by:** `RFC-COMPILER-001`, `RFC-DRIVER-001`

A capability that authorises access to a memory region must have a defined representation. This is not just a data structure — it is the fundamental capability type for physical resources.

**Questions that must be answered by `RFC-ALLOC-001`:**
- Is a memory region capability a capability token (`RFC-0037` format) or a derived capability within that format?
- What rights are representable? (read, write, execute, DMA, MMIO — orthogonal or combined?)
- Can a memory region capability be narrowed without creating a new token?
- What is the format of the address part when a memory capability is dereferenced?

**Impact if answered wrong:**
- `RFC-COMPILER-001` will define an ABI that cannot represent memory capabilities correctly
- `RFC-DRIVER-001` will specify MMIO access semantics that conflict with the allocation model

**Status:** BLOCKING — must be resolved in `RFC-ALLOC-001` before both other RFCs proceed

---

### INTF-002: Capability register convention at trust boundary

**Defined by:** RFC-COMPILER-001
**Consumed by:** RFC-DRIVER-001

When a driver component (user space) calls into a driver service or when any component crosses a kernel/user boundary, which registers carry capability tokens?

**Questions that must be answered by RFC-COMPILER-001:**
- Dedicated capability register(s) or capability encoded in general registers?
- RISC-V has no native CHERI yet — is this emulated in the Minimum Executor?
- What is the faulting representation (invalid capability value vs. uninitialised register)?
- Are capability registers callee-saved or caller-saved?

**Impact if answered wrong:**
- RFC-DRIVER-001 will specify IPC and capability transfer that cannot be implemented
- seL4-style capability passing at function boundaries cannot be realised

**Status:** BLOCKING — must be resolved in RFC-COMPILER-001 before RFC-DRIVER-001 can specify IPC

---

### INTF-003: Component entry protocol (capability environment setup)

**Defined by:** RFC-COMPILER-001
**Consumed by:** RFC-DRIVER-001

When a component (including a driver component) starts, what capability environment is established before the first instruction of user code runs?

**Questions that must be answered by RFC-COMPILER-001:**
- Who sets up the initial capability environment? (Bootstrapper, kernel, loader?)
- Is there a primary capability (root of the component's authority tree)?
- How is the initial memory allocation capability granted?
- What is the initial address space — who programmes the MMU?

**Impact if answered wrong:**
- RFC-DRIVER-001 cannot specify how driver components are instantiated
- The bootstrap model (Section 5 of Platform Architecture) cannot be realised

**Status:** BLOCKING — must be resolved in RFC-COMPILER-001 before RFC-DRIVER-001

---

### INTF-004: DMA capability semantics

**Defined by:** RFC-ALLOC-001 (DMA memory pool)
**Consumed by:** RFC-DRIVER-001

Does a DMA capability authorise direct memory access by a device, or does it authorise the driver to configure an IOMMU?

**Questions that must be answered by RFC-ALLOC-001:**
- Is DMA a separate capability right on a memory region, or a separate capability type?
- Who programmes the IOMMU — the kernel, the driver, or the HAL?
- Can a driver hold a DMA capability without holding the underlying memory capability?
- What is the lifecycle of a DMA window capability vs. the underlying allocation capability?

**Impact if answered wrong:**
- RFC-DRIVER-001 will specify DMA ownership inconsistently with allocation semantics
- I/O MMU isolation guarantees (I-1, I-2) cannot be formally verified

**Status:** BLOCKING — must be resolved in RFC-ALLOC-001 before RFC-DRIVER-001

---

### INTF-005: Stack and heap allocation capability

**Defined by:** RFC-ALLOC-001
**Consumed by:** RFC-COMPILER-001

The compiler must emit code that allocates stack frames and heap objects. What capability governs those allocations?

**Questions that must be answered by RFC-ALLOC-001:**
- Is there a distinguished stack allocation capability per thread?
- Is the heap allocator accessed through a capability, or through a system call that is itself capability-gated?
- Can a component derive a narrower heap allocation capability for a subsystem?

**Impact if answered wrong:**
- The ABI cannot specify TLS model, stack lower bound, or heap allocation convention
- Formal verification of the allocator cannot be scoped

**Status:** BLOCKING — must be resolved in RFC-ALLOC-001 before RFC-COMPILER-001 is complete

---

### INTF-006: Error capability and error propagation at ABI

**Defined by:** RFC-COMPILER-001
**Consumed by:** RFC-DRIVER-001

When a driver operation fails, how is the error represented at the ABI level?

**Questions that must be answered by RFC-COMPILER-001:**
- Is there a distinguished error capability (per seL4's error token)?
- Or is error a capability type alongside success capabilities?
- Can a component catch a capability fault and recover, or must it propagate?

**Impact if answered wrong:**
- RFC-DRIVER-001 cannot specify driver error handling
- The failure-mode coverage budget cannot be realised

**Status:** BLOCKING — must be resolved in RFC-COMPILER-001 before RFC-DRIVER-001 specifies error handling

---

## 4. Invariant Coverage Map

Which requirements and RFCs are responsible for which invariants.

| Invariant | Primary requirement | Primary RFC | Supporting RFCs |
|-----------|-------------------|-------------|-----------------|
| I-1 (Authority mediation) | REQ-ALLOC-01, REQ-COMP-01, REQ-DRIVER-01 | All three | All 41 |
| I-2 (No ambient authority) | REQ-ALLOC-01, REQ-COMP-01, REQ-DRIVER-01 | All three | All 41 |
| I-3 (Least privilege) | REQ-ALLOC-02, REQ-DRIVER-01 | RFC-ALLOC-001, RFC-DRIVER-001 | RFC-0003, RFC-0037 |
| I-4 (Observable transitions) | REQ-COMP-03, REQ-DRIVER-01 | RFC-COMPILER-001, RFC-DRIVER-001 | RFC-0031 |
| I-5 (Transactional persistence) | REQ-ALLOC-01 (indirectly) | RFC-ALLOC-001 (page allocator stability) | RFC-0016 |
| I-6 (Cryptographic integrity) | REQ-COMP-02, REQ-ALLOC-01 | RFC-COMPILER-001, RFC-ALLOC-001 | RFC-0017 |
| I-7 (Forward extensibility) | REQ-DRIVER-01 (replaceable drivers) | RFC-DRIVER-001 | RFC-0002, RFC-0005 |
| I-8 (Verifiable core) | REQ-ALLOC-04 (scheduler allocation) | RFC-ALLOC-001 | RFC-0011 |
| I-9 (Replaceable mechanisms) | REQ-DRIVER-01 | RFC-DRIVER-001 | RFC-0002, RFC-0004 |
| I-10 (Explicit failure semantics) | REQ-DRIVER-01, REQ-COMP-03, REQ-ALLOC-01 | All three | All 41 |
| I-11 (Declarable profiles) | REQ-DRIVER-01 (driver manifest) | RFC-DRIVER-001 | RFC-0005, RFC-0041 |
| I-12 (Origin of authority) | REQ-COMP-03 (initial environment) | RFC-COMPILER-001 | RFC-0007, RFC-0037 |
| I-13 (Address-authority orthogonality) | REQ-ALLOC-01, REQ-COMP-01, REQ-DRIVER-05 | RFC-ALLOC-001, RFC-COMPILER-001, RFC-DRIVER-001 | RFC-0009, RFC-0037 |

**Key observation:** Every invariant has at least one dependency relationship with the three missing RFCs. This confirms that RFCs participate in the Phase 1 conformance surface. It does not by itself establish that every aspect of each RFC is a prerequisite for the first executable boot milestone.

For example, I-6 (cryptographic integrity) being indirectly touched by `RFC-COMPILER-001` does not necessarily mean the full compiler-boundary RFC must be accepted before the Minimum Executor boots. The dependency matrix identifies **what interfaces must be agreed**, not that any single interface must be implemented to the full normative specification before any code runs.

The matrix's strict-sequential interpretation (in the dependency-graph at the end of Section 7) reflects normative-agreements ordering, not implementation-unblocking ordering. The implementation-side ordering may be parallelised where CTS scaffolding or a layered prototype only depends on a subset of a given RFC's normative statements.

---

## 5. CTS Coverage (by RFC)

### RFC-ALLOC-001 CTS obligations

| CTS ID | Tests | Tier |
|--------|-------|------|
| CTX-ALLOC-001 to 004 | Allocation authority tests | VT-A |
| CTX-ALLOC-005 to 006 | Capability-addressable region tests | VT-A |
| CTX-ALLOC-007 | Scheduler hot-path allocation test | VT-A |
| CTX-ALLOC-008 to 009 | NUMA and fragmentation tests | VT-C |

### RFC-COMPILER-001 CTS obligations

| CTS ID | Tests | Tier |
|--------|-------|------|
| CTX-COMP-001 to 003 | Capability boundary tests | VT-A |
| CTX-COMP-004 to 005 | Object representation tests | VT-A |
| CTX-COMP-006 to 007 | Entry/exit and error propagation tests | VT-A/VT-C |
| CTX-COMP-008 to 011 | PIC, relocations, symbol resolution tests | VT-C |

### RFC-DRIVER-001 CTS obligations

| CTS ID | Tests | Tier |
|--------|-------|------|
| CTX-DRIVER-001 to 002 | DMA capability tests | VT-A |
| CTX-DRIVER-003 to 005 | Driver lifecycle tests | VT-A/VT-B |
| CTX-DRIVER-006 to 007 | Hot-plug tests | VT-A |
| CTX-DRIVER-008 to 009 | Dependency ordering tests | VT-B |
| CTX-DRIVER-010 to 011 | Power management tests | VT-B |
| CTX-DRIVER-012 to 013 | MMIO capability tests | VT-A |

---

## 6. Formal Verification Obligations

| Invariant | Component | Tier | Proof obligation |
|-----------|-----------|------|-----------------|
| I-1, I-2 | Capability mediation at allocation boundary | VT-A | Allocation capabilities cannot be created without grant |
| I-2, I-3 | Capability derivation at ABI boundary | VT-A | Right-narrowing holds across function call |
| I-8 | Scheduler hot-path allocator | VT-A | No heap allocation on dispatch path |
| I-12 | Bootstrap Minimum Executor | VT-A | First capability is legitimately constructed, not ambient |
| I-13 | Address/authority separation | VT-A | Capability token never contains address; address never implies authority |
| I-1, I-13 | MMIO capability enforcement | VT-A | MMIO access requires MMIO capability distinct from memory capability |

**When formal work begins — split by tier:**

* **Tier A formal models begin at specification freeze.** Properties that depend only on the architecture invariants themselves may be modelled and partially proved as soon as the relevant RFC is normative, without waiting for any implementation. The Capability System (Section 4 of the architecture specification) and bootstrap model (Section 5) are conducive to this, since they are normative on a frozen architecture.
* **Tier A machine-checked implementation refinement begins when the corresponding implementation boundary is stable enough to admit proof.** The implementation must pass CTS for consecutive test cycles AND the proof-refinement target must be isolated from churn.
* **Tier B obligations begin alongside implementation.** Model-checked properties of lifecycle safety, driver isolation, and revocation are developed alongside the code that realises them; the formal model and the implementation are co-developed after the relevant RFCs land.
* **Tier C / Tier D obligations do not require formal proof.** Property tests and conventional test coverage are within ordinary test discipline from RFC draft acceptance onwards.

**Tier stratification for the three blocker RFCs:**

- **Tier A** (formal models begin immediately at RFC draft freeze, full machine proofs after implementation stabilisation): capability mediation at allocation boundary (I-1, I-2), capability derivation at ABI boundary (I-2, I-3), bootstrap minimum-executor construction (I-12), address/authority separation (I-13), MMIO capability enforcement (I-1, I-13).
- **Tier A** (formal models begin at RFC draft freeze, but stable proof is gated on **stability** of the implementation passes for two consecutive CTS cycles): scheduler hot-path allocator non-allocation proof (I-8), proof that ABI cannot be violated without hardware fault.
- **Tier B** (formal models begin alongside implementation): driver lifecycle safety, hot-plug revocation safety, power-management capability suspension safety, critical driver-isolation consequences.
- **Tier C / D** (testing initially, formal later if justified by evidence): NUMA policy, fragmentation budget, hot-plug graceful degradation, allocation composability for subsystems.

**Operational consequence:** engineering parallelisation of formal modelling with implementation is permitted for the *first* formal model. Implementation-refinement proofs are gated on stability. Both gates must pass before Tier A components can be declared in the conformance declaration per `RFC-0040`.

---

## 7. Blocking Relationships Summary

```
INTF-000 (Capability Enforcement Substrate) RESOLVED — [decision record]
                                            blocks ALL 3 RFCs
INTF-001 (Memory capability type)          BLOCKS → RFC-COMPILER-001, RFC-DRIVER-001
INTF-002 (Capability register ABI)         BLOCKS → RFC-DRIVER-001
INTF-003 (Component entry protocol)        BLOCKS → RFC-DRIVER-001
INTF-004 (DMA capability semantics)        BLOCKS → RFC-DRIVER-001
INTF-005 (Stack/heap allocation cap)       BLOCKS → RFC-COMPILER-001
INTF-006 (Error propagation at ABI)        BLOCKS → RFC-DRIVER-001

RFC-ALLOC-001     ──→ RFC-COMPILER-001     (INTF-005)
RFC-ALLOC-001     ──→ RFC-DRIVER-001       (INTF-001, INTF-004)
RFC-COMPILER-001  ──→ RFC-DRIVER-001       (INTF-002, INTF-003, INTF-006)
```

**Confirmed dependency chain:**

- INTF-000 is resolved and provides the substrate decision. All three RFCs assume its choice (Option C: Software Capability Provider for Phase 1, Native Capability Provider reserved for Phase 3+).
- RFC-ALLOC-001 has no inputs from the other two — it is the pure foundation.
- RFC-COMPILER-001 depends on RFC-ALLOC-001.
- RFC-DRIVER-001 depends on both RFC-ALLOC-001 and RFC-COMPILER-001.

**Three tracks distinguished (per seL4-style layered dependency reasoning):**

| Track | Purpose | Ordering rule |
|-------|---------|---------------|
| **Specification** | RFC normative-acceptance dependency | Strictly serial: ALLOC → COMPILER → DRIVER, with INTF-000 above all three |
| **Engineering** | Implementation-component drafting | Parallel where interfaces allow; ALLOC and COMPILER cross-RFC interfaces stabilising enables component scaffolding before full RFC acceptance |
| **Verification** | Formal model + proof-refinement dependency | Formal models in parallel with RFC draft freeze; machine-checked proofs after stabilisation |

**Specification track:** the dependency graph above — the formal normative contract ordering.

**Engineering track:** after INTF-000 is resolved and the interfaces that ALLOC/COMP own are stable, component scaffolding (CTS fixtures with `SPEC_PENDING` markers, prototype dispatches) may proceed; this is implementation scaffolding, not normative acceptance.

**Verification track:** the formal-model obligations above (Tier A models at RFC freeze, machine-checked refinement at implementation stability) run in parallel with engineering. CTS converges with formal-model acceptance on the same set of properties, not as a one-way prerequisite.

These three tracks are **not three independent codebases** — they are **three views of the same work**. The integration point is at every cross-boundary interface, where CTS tests verify the implementation matches the formal model matches the normative RFC.

---

## 8. CTS and Verification Parallelism

CTS and formal verification work can begin concurrently with RFC drafting, not after.

| Workstream | Can start when | Notes |
|------------|---------------|-------|
| CTS harness infrastructure | Architecture v1.1 frozen | Already possible |
| CTS fixtures for RFC-ALLOC-001 | RFC-ALLOC-001 drafted | Not before — fixtures depend on spec |
| CTS fixtures for RFC-COMPILER-001 | RFC-COMPILER-001 drafted | Not before |
| CTS fixtures for RFC-DRIVER-001 | RFC-DRIVER-001 drafted | Not before |
| Formal invariants for capability mediation | Architecture v1.1 frozen | Already in existing RFCs |
| Formal proof of allocation capability non-creation | RFC-ALLOC-001 drafted + stable impl | Requires spec + CTS confirmation of stability |
| Formal proof of ABI boundary enforcement | RFC-COMPILER-001 drafted + stable impl | Same |

---

## 9. Updated Critical Path

The critical path below is the **first executable-boot-to-run-user-component** sequence. It is *not* the entire 42-month roadmap; the implementation-critical milestone that verifies the architecture can instantiate its most fundamental execution and authority model on real hardware or a faithful emulator.

```
Architecture Baseline v1.1                  ALREADY COMPLETE
         │
         ▼
INTF-000 Capability Enforcement Decision     ALREADY COMPLETE (this matrix + decision record)
         │
         ▼
Pre-Implementation Dependency Matrix         THIS DOCUMENT
         │
         ▼
       RFC-ALLOC-001                          START HERE
       (Allocator Hierarchy)
         │
    INTF-001, INTF-004, INTF-005
    resolved in this RFC
         │
         ▼
       RFC-COMPILER-001                       SECOND
       (Compiler and Runtime Boundary)
         │
    INTF-002, INTF-003, INTF-006
    resolved in this RFC
         │
         ▼
       RFC-DRIVER-001                         THIRD
       (Driver Framework Architecture)
         │
         ▼
Implementation Baseline v1.2                 NORMATIVE COMPLETION MILESTONE
         │
         ▼
   ┌────────────────────┐
   │ RISC-V Platform    │ — the hardware target declaration, fetched from BIOS/DTBL.
   │  Contract          │   Stable before anything physical runs.
   │  (Pre-existing,    │
   │  in Hw-Support-RFC)│
   └──────────┬──────────┘
              ▼
   ┌────────────────────┐
   │ Minimal RISC-V     │ — *before* the Minimum Executor; the HAL is the
   │ HAL (HAL-BOOT)     │   substrate the executor runs on. NOT the full HAL.
   │ • hart init        │   Awaits below: capability mediator, MMU,
   │ • privilege cyl    │   time, interrupt controller, physical memory
   │ • trap entry       │   discovery, cache/fence primitives, device
   │ • clint/MTIME      │   discovery.
   │ • MMU init (Sv39)  │
   │ • phys mem disc    │   (Full HAL comes later, in RFC-DRIVER-001 stage.)
   │ • device discover  │
   └──────────┬──────────┘
              ▼
   Minimum Executor (RISC-V)              — constructs the origin capability;
         │                                  binds to hardware identity; deliver
         │                                  the capability to Capability Root.
         ▼                                  Has the documented boundary: the
   Capability Root                        first capability's seal is computed
         │                                  from the platform master seal key.
         ▼
   Composition Root                       — invokes RFC-0002 manifests; first
         │                                  userspace components instantiated.
         ▼
   First isolated component               — receives only what its manifest
         │                                  declares. Cold boots under CTS.
         ▼
   First real CTS run                     — executes INV-I-1..13 invariants;
         │                                  tests His-only allocator / ABI /
         ▼                                  driver boundaries.
   First Observation Record               — first empirical evidence driving
         │                                  possible Architecture-Amendment
         ▼                                  via RFC-GOV-002.
   Evidence-driven evolution
```

**Boot ordering correction (essential):** The Minimum Executor cannot meaningfully be implemented on bare silicon without *any* hardware abstraction. The previous ordering (Minimum Executor → RISC-V HAL) inverted the dependency. The corrected ordering separates a deliberately tiny **HAL-BOOT** (hart init, privilege transitions, trap entry, timer/interrupt primitive, MMU, memory discovery, device discovery — described in `Hardware-Support-RFC.md` as the boot substrate of any execution provider) from the full **HAL-RISCV** (full interrupt abstraction, IOMMU, DMA, power management, performance counters, advanced extensions — described in the same file as the full substrate-neutral contract). HAL-BOOT is necessary before the Minimum Executor; HAL-RISCV (full) is an outcome of `RFC-DRIVER-001` and later driver work.

**Parallel workstreams:**

```
RFC drafting
  ‖
CTS development (fixture scaffolding marked SPEC_PENDING until corresponding
  RFC is normative; CTS-INTF-000-N tests can be written immediately since
  INTF-000 is resolved)
  ‖
Formal modelling
  ‖
Toolchain investigation
  ‖
Hardware validation (QEMU + targeted RISC-V boards)
```

All five workstreams run concurrently once INTF-000 is settled. They converge on integration testing at the boundary.

---

## 10. Open Questions (Must Resolve Within Each RFC)

### For RFC-ALLOC-001

1. Is "memory region capability" a base capability type (instantiated directly from RFC-0037) or a derived capability?
2. Are DMA rights a separate capability right or a separate capability type? (Per `INTF-000` decision: a DMA capability is a *separately typable* capability that grants DMA access to a memory region, distinct from `CAP_MEMORY`. Three authorities must be distinguishable: a memory capability (CPU access), a DMA-memory capability (device-originated access), and an IOMMU-config capability (programming the IOMMU's translation tables).)
3. What is the minimum formal proof that allocation creates no ambient capability? (Required for Tier A declaration)
4. Does the page allocator interface to the HAL, or does the HAL call the page allocator? (The page allocator is privileged; the HAL provides physical-region discovery; the capability-mediator gates every allocation. Concretely: **HAL provides physical-region descriptors → Allocator consumes them → Allocator emits allocation-capability tokens via the mediator.**)
5. Who owns the boot-time allocator before the capability system exists? (Bootstrap tension with I-12. **Resolution:** a static, pre-capability byte pool is used by the Minimum Executor for the capability mediator itself; once the first capability is constructed and sealed, the boot allocator is converted from a no-capability code path to a capability-gated one. The boot allocator is single-use, register-traced, and not exposed after Capability Root is initialised. Documented in `RFC-0007` Section 3.3 and tied to `INTF-000`.)
6. Can a component derive a narrower allocation capability for a subsystem, or does every allocation require the root allocation capability?

### For RFC-COMPILER-001

1. On RISC-V without CHERI: is capability register emulation in hardware or software? **RESOLVED** by `INTF-000` decision: software emulation via the capability-mediator (SCP). Capability registers are conventional integer registers. The register-convention answer to "which registers carry tokens" is: caller-method specific, defined by the ABI, validated against the capability-mediator at unseal points. The capability-mediator does NOT require dedicated register classes.
2. Is the component entry protocol set up by the Minimum Executor or by the component itself?
3. What is the error capability representation — error token or error capability type?
4. Which parts of the ABI are RISC-V-specific vs. architecture-neutral?
5. How does position-independent code interact with capability-relative addressing?
6. What is the minimum viable symbol resolution for Phase 1? (Full dynamic linking or static + fixed-memory?)

### For RFC-DRIVER-001

1. Does a driver hold a memory region capability for MMIO, or is MMIO a distinct capability type? **Per `INTF-000`:** MMIO is a separate capability type, distinct from `CAP_MEMORY`. A device-control capability, a DMA-memory capability, and an IOMMU-configuration capability are three distinct capability types. Drivers see combinations; the IOMMU-config capability is restricted to the capable-mediator.
2. Who programmes the IOMMU? **By the capability-mediated path:** the kernel/HAL programs the IOMMU in response to a request — but only when the request is gated by a `CAP_IOMMU_CONFIG` capability held by the requesting component. A driver receives device-control + DMA-memory capability; not IOMMU-config. The capable-mediator translates the former into the latter as part of the technical effect. Three authorities are distinguishable per the `INTF-000` sharpening.
3. Is driver fault containment enforced by the Execution Architecture (address space isolation) or by the Driver Framework (capability revocation)?
4. Can drivers be updated without rebooting (cohort update)? (Likely defer to RFC-0042)
5. What is the maximum driver fault blast radius — can a driver corrupt another driver's capability namespace?
6. Is the driver framework responsible for device discovery, or is that an earlier bootstrap stage?

### For INTF-000 (already resolved, listed for traceability)

This matrix records the decision; the open-questions that were resolved by `INTF-000` are documented in `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md`. They are not subject to further negotiation.

---

## 11. Implementation Contracts

Implementation Contracts are the layer between normative RFCs and concrete code. They translate RFC prose to implementation-level contracts that can be referenced in pull requests and CI.

### 11.1 Why an Implementation Contract

RFCs write normative prose at the architecture level. Without an additional layer:
- Pull requests may interpret a requirement incorrectly.
- Two implementations may satisfy the same RFC in incompatible ways.
- The CTS-fixture intent is not traceable to a specific API or data structure.

The Implementation Contract layer closes these gaps. Each contract is a fixed-format document enumerating **the precise mechanism by which a normative RFC requirement lands in code**.

### 11.2 Implementation Contract Schema (per contract)

```
Implementation Contract ID: <IC-NNN>
Source Requirement: <RFC-XXXX.N OR I-N OR INTF-N>
Owner: <Implementation-component or subsystem>
Realised Invariants: <I-N list>
Implementation Components: <LOC/CU/assembly references when implemented>
ABI / Behavioral Contract: <structured API/ABI signature, in c-like pseudo-code>
CTS Test: <CTX-N list>
Formal Property: <PF-NN list, when formal model exists>
Observation Record: <OBS-N link, when observations exist>
Status: DRAFT / ACCEPTED / DEPRECATED
```

### 11.3 Implementation Contract Floor for the Three Blocker RFCs

Minimum ICs produced before the corresponding RFC is declared **Active**:

| RFC | Minimum Implementation Contracts | Coverage |
|-----|----------------------------------|----------|
| RFC-ALLOC-001 | IC-001 (memory-region-capability token), IC-002 (stack/heap capability boundaries), IC-003 (DMA-capable pool descriptor), IC-004 (boot allocator to capability transition) | All REQ-ALLOC-NN and INTF-NN5 |
| RFC-COMPILER-001 | IC-005 (capability passing register convention), IC-006 (component entry stub), IC-007 (error capability), IC-008 (PIC ABI slice), IC-009 (symbol resolution data structures) | All REQ-COMP-NN and INTF-NN2/003/006 |
| RFC-DRIVER-001 | IC-010 (driver manifest schema), IC-011 (device-control capability), IC-012 (DMA capability + IOMMU program request), IC-013 (driver lifecycle FSM), IC-014 (hot-plug renegotiation protocol), IC-015 (MMIO capability type distinct from CAP_MEMORY) | All REQ-DRIVER-NN and INTF-NN1/004 |

### 11.4 Implementation Contract Lifecycle

```
DRAFT (initial)
   │
   ▼ after requirements freeze
ACCEPTED (cited by RFCs, referenced in PRs and CTS)
   │
   ▼ if behaviour changes
DEPRECATED → SUPERSEDED (new IC supersedes old)
```

A change to an ACCEPTED IC is a Specification-Amendment to the source RFC if it changes normative behaviour, else an Editorial Amendment.

### 11.5 Where Implementation Contracts Live

Implementation Contracts are stored alongside the source RFC (e.g. `RFCs/02-State/_ic/` or a sibling directory). They are NOT normative documents; they are operational guides. The CI tooling may emit a "stale IC" warning if the corresponding RFC has been amended without IC update.

The full Implementation Contract template and authoring guide belongs in a follow-up document (`Operational-Guidance-Implementation-Contracts.md`), which is the procedure document for creating ICs. The presence of the IC layer is normative; the specific IDs and content of individual ICs are operational.

---

## 12. Cross-Reference Manifest

| Decision / Precedent | Reference | Used by |
|---------------------|-----------|---------|
| INTF-000 Capability Enforcement Substrate Decision | `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md` | All three RFCs, `RFC-0013` (Capability Mediation mechanism-driven parameter), `RFC-0037` (canonical token shape, unchanged) |
| INTF-000-A (Native Capability Provider) | future RFC | Reserved for Phase 3+ |
| INTF-000-B (Software Capability Provider) | describes in this decision record | Phase 1 implementation |
| HAL-BOOT terminology | `Hardware-Support-RFC.md` | Section 9 of this matrix |
| Three-track dependency model (Specification / Engineering / Verification) | Section 7 of this matrix | All RFC drafting teams |
| Implementation Contracts | Section 11 of this matrix | RFC-to-code traceability |

---

## 13. Revision History — Pre-Implementation Dependency Matrix

| Date       | Revision | Change |
|------------|----------|--------|
| this cycle | 1        | Initial matrix drafted. 14 blocking requirements; 6 cross-RFC interfaces; strict sequential dependency. |
| this cycle | 2        | **Architectural-Review-driven revisions:** added REQ-INTF-000 (Capability Enforcement Substrate) as a precondition for all three RFCs; added INTF-000 to Section 3 as a resolved pre-condition; updated all three dependency graphs to show INTF-000 as upstream; weakened Section 4 invariant-coverage language; restructured Section 6 formal verification sequencing (Tier A models at RFC freeze, machine proofs after implementation stability) with tier stratification; updated Section 7 blocking relationships with the three-track model (Specification / Engineering / Verification); corrected Section 9 boot ordering — HAL-BOOT precedes Minimum Executor, separating minimal boot HAL from full HAL-RISCV; added Section 10 questions sharpened by INTF-000 (DMA authority typology, software capability mediator register convention, IOMMU authorisation chain); added new Section 11 (Implementation Contracts); added new Section 12 (Cross-Reference Manifest); recorded the now-resolved INTF-000 in the cross-reference manifest. Document classified Operational. |