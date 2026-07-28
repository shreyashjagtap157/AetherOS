# RFC-COMPILER-001: Compiler and Runtime Boundary

Classification: Normative
Authoritative Source: RFC-COMPILER-001.md
Requirement-ID: RFC-COMPILER-001-000
Status: Draft

**Lifecycle note:** Draft.
**Implements invariants:** I-1, I-2, I-12, I-13.
**Depends on:** RFC-0037 (Capability Token Format), RFC-0038 (Capability Transfer Protocol), RFC-0025 (Endpoint Model), RFC-0007 (Execution Domain), RFC-ALLOC-001 (Allocator Hierarchy).
**Resolves:** INTF-002 (Capability Register Convention), INTF-003 (Component Entry Protocol), INTF-006 (Error Propagation at ABI).

---

## 1. Scope

This RFC specifies the compiler and runtime boundary: the ABI (Application Binary Interface) contract that governs how compiled components interact with the AetherOS capability system. It defines how capability tokens cross function boundaries, how components enter and exit, how errors are represented at the ABI level, how position-independent code interacts with capability-relative addressing, and how symbols are resolved across component boundaries.

The compiler/runtime boundary is the contract between the architecture's capability semantics and the executable code that components produce. Every capability check, every function call that crosses a trust boundary, and every component lifecycle event passes through this boundary.

This RFC implements I-1 (capability required for resource access at ABI level), I-2 (no ambient authority — capabilities are never implicitly created), I-12 (origin of authority — initial capability environment), and I-13 (address–authority orthogonality at the ABI level).

This RFC does not specify the allocator hierarchy (`RFC-ALLOC-001`), the driver lifecycle (`RFC-DRIVER-001`), or the HAL implementation (`Hardware-Support-RFC.md`). It defines the interfaces that driver components and the driver framework consume.

---

## 2. Normative Requirements

### 2.1 Architecture Overview

`RFC-COMPILER-001.1`: The compiler/runtime boundary consists of four layers:

1. **ABI Layer** — calling conventions, register preservation, stack frame layout, and capability register convention.
2. **Capability Passing Layer** — how capability tokens are passed across function boundaries and trust boundaries.
3. **Component Lifecycle Layer** — entry, exit, and error propagation for compiled components.
4. **Linkage Layer** — position-independent code, symbol resolution, and load-time relocations.

`RFC-COMPILER-001.2`: The ABI is architecture-neutral where possible. RISC-V-specific elements are confined to Section 2.9. The architecture-neutral ABI describes the *logical* contract; the RISC-V-specific section describes the *physical* implementation on the Phase 1 reference platform.

### 2.2 Live Capability Convention (INTF-002)

`RFC-COMPILER-001.3`: Under the Software Capability Provider, a live capability is a pointer-width, domain-local, generational handle into a provider-owned capability space. The canonical `RFC-0037` record is used only for explicit import, export, persistence, and cross-trust transfer.

`RFC-COMPILER-001.4`: Handles use ordinary integer argument registers but are typed distinctly in IDL and generated bindings. Integer construction, arithmetic, memory copying, or use in another domain grants no authority.

`RFC-COMPILER-001.5`: Handles are caller-saved unless the architecture ABI says otherwise. Preserving a handle preserves only a reference; provider validation of domain, slot generation, type, rights, and revocation state occurs on mediated use.

`RFC-COMPILER-001.6`: Zero is the null handle. Freed slots advance their generation before reuse. Truncated, stale, out-of-domain, or type-confused handles fail closed in bounded time.

`RFC-COMPILER-001.7`: Native Capability Providers MAY use tagged capability registers and their standard ABI. Generated stubs preserve the same semantic contract without forcing the software-handle bit layout onto native hardware.

`RFC-COMPILER-001.8`: A compiler cannot mint authority. Live handles originate only from the initial environment, a mediated return, explicit transfer/derivation, or validated canonical import.

### 2.3 Capability Passing Protocol

`RFC-COMPILER-001.9`: Intra-domain calls pass live handles by value using the platform's normal pointer-width argument convention.

`RFC-COMPILER-001.10`: Functions MAY return one handle directly and return additional handles through an explicitly typed result structure. Error status is separate from capability values.

`RFC-COMPILER-001.11`: Stack and heap storage may contain live handles, but such bytes have meaning only inside the owning capability domain and generation. They are never a portable serialized authority format.

`RFC-COMPILER-001.12`: Crossing a trust/domain boundary invokes a generated mediator stub that checks transfer authority, attenuates rights, allocates a destination entry, and returns a destination-local handle. Raw handle copying across the boundary is invalid.

### 2.4 Object Model at ABI Level (REQ-COMP-02)

`RFC-COMPILER-001.13`: Every capability-addressable object in memory has a defined ABI-level representation: an **object header** that precedes the object's data region.

```c
struct object_header {
    struct object_id    identity;         // RFC-0015: immutable content-resolved identity
    uint32              header_size;      // size of this header (for alignment)
    uint32              object_size;      // size of the data region
    uint8               capability_slots; // number of capability slots following header
    uint8               flags;            // object flags (see below)
    uint16              reserved;
    // Followed by: capability_slots × provider-width live_cap_handle
    // Followed by: object data
};
```

Object header flags:

| Flag | Name | Meaning |
|------|------|---------|
| 0x01 | `OBJ_SEALED` | Object identity is sealed and immutable |
| 0x02 | `OBJ_TRANSIENT` | Object is not persisted to State |
| 0x04 | `OBJ_DOMAIN_BOUND` | Object is bound to its creating domain |

`RFC-COMPILER-001.14`: The `identity` field in the object header is immutable after creation. Attempts to modify it produce undefined behaviour.

`RFC-COMPILER-001.15`: The `capability_slots` field declares how many capability tokens immediately follow the header. These slots are the object's exported capabilities. The capability slots are part of the object's in-memory representation and are accessed through capability tokens (not raw pointers).

`RFC-COMPILER-001.16`: A **null capability handle** is zero and yields `ECAPSTALE` when used. It is not a canonical capability record.

`RFC-COMPILER-001.17`: An invalid or stale live handle yields `ECAPSTALE`. Canonical seal failures occur only at import and never allocate a live handle.

### 2.5 Component Entry Protocol (INTF-003)

`RFC-COMPILER-001.18`: When a component is instantiated and receives control for the first time, the **component entry stub** executes before the component's application code. The entry stub is provided by the runtime and is architecture-specific.

`RFC-COMPILER-001.19`: The component entry stub performs the following initialisation in order:

1. **Stack pointer validation.** The stack pointer is set to the component's pre-allocated stack region (per RFC-ALLOC-001.23). The stack region's bounds are validated against the stack capability.
2. **TLS pointer initialisation.** The thread-local storage (TLS) pointer is set to the component's TLS region. The TLS region is a pre-allocated capability-gated region containing the component's static thread-local data.
3. **Capability environment establishment.** The component's initial capability set is loaded from the capability slots declared in the component's manifest (RFC-0002). Each capability is verified by the capability-mediator before being placed in the component's capability namespace.
4. **Root capability grant.** The component receives a root allocation capability (per RFC-ALLOC-001.25) authorising heap allocation within its declared bounds.
5. **Event emission.** The entry stub emits a `ComponentEntered` event per RFC-0031.

`RFC-COMPILER-001.20`: The component entry stub is set up by the Composition Root (RFC-0007 Stage IV) or by the Minimum Executor (RFC-0007 Stage II for bootstrap components). The component itself does NOT set up its own entry environment.

`RFC-COMPILER-001.21`: A component's initial capability set is EXACTLY the set declared in its manifest (RFC-0002). No ambient capability is available. The component cannot create capabilities except through derivation from its initial set or through explicit capability transfer from another component.

### 2.6 Component Exit Protocol

`RFC-COMPILER-001.22`: When a component terminates (voluntarily or due to fault), the **component exit stub** executes after the component's application code.

`RFC-COMPILER-001.23`: The component exit stub performs the following cleanup in order:

1. **Capability namespace release.** All capability tokens in the component's namespace that are bound to this domain (`CAPF_BOUND`) are revoked per RFC-0039.
2. **Memory release.** All heap and stack allocations owned by the component are returned to the allocator via capability revocation.
3. **Event emission.** The exit stub emits a `ComponentExited` event per RFC-0031 with the exit reason code.
4. **Control transfer.** Control is returned to the Composition Root or to the parent domain.

`RFC-COMPILER-001.24`: A component exit stub MUST execute even if the component terminates due to an uncaught exception. The exit stub is the last code that runs in the component's address space before destruction.

### 2.7 Error Propagation (INTF-006)

`RFC-COMPILER-001.25`: ABI operations return a fixed-width status code separately from any capability result. An error value is data and never authority.

`RFC-COMPILER-001.26`: Generated bindings use a result structure or the platform language's checked result type; output capability handles are zeroed on failure.

`RFC-COMPILER-001.27`: The initial error namespace includes `ERR_NOMEM`, `ERR_NOCAP`, `ERR_FAULT`, `ERR_RANGE`, `ERR_PERM`, `ERR_DEPEND`, `ERR_TIMEOUT`, `ERR_BUSY`, `ERR_CANCELLED`, `ERR_UNCERTAIN`, and `ERR_VERSION`. Numeric assignments belong to the ABI implementation contract.

`RFC-COMPILER-001.28`: Applications handle synchronous errors through normal control flow. Error values cannot be presented to the mediator as capabilities.

`RFC-COMPILER-001.29`: Hardware and asynchronous faults are delivered as bounded fault records to a registered handler endpoint. If no handler exists or the handler violates policy, the component is terminated and cleanup runs from a trusted supervisor context.

`RFC-COMPILER-001.30`: Error detail follows explicit disclosure policy and may carry an opaque diagnostic correlation ID. It never contains undelegated resource identity or authority.

### 2.8 Position-Independent Code and Address Space Layout (REQ-COMP-04)

`RFC-COMPILER-001.31`: Components MUST be compiled as position-independent code (PIC). Component load addresses are determined at load time by the Composition Root, not at compile time.

`RFC-COMPILER-001.32`: The ABI defines a **Global Offset Table (GOT)** for capability-relative relocations. Each capability reference in the component's code is resolved through a GOT entry that contains the capability token's in-memory address. The GOT is populated at load time.

`RFC-COMPILER-001.33`: The ABI supports load-time relocations for the following reference types:

| Relocation Type | Description |
|----------------|-------------|
| `REL_CAPABILITY` | Provider-width live capability handle slot |
| `REL_CAP_GOT` | GOT entry for a capability reference |
| `REL_DATA` | Data pointer (address-relative) |
| `REL_FUNC` | Function pointer (address-relative) |

`RFC-COMPILER-001.34`: Component address space layout is:

```
Low address
    ├── Guard region (unmapped, for stack overflow detection)
    ├── Stack region (capability-gated)
    ├── TLS region (capability-gated)
    ├── Code region (PIC, mapped with RIGHT_EXECUTE)
    ├── Data region (mapped with RIGHT_READ+RIGHT_WRITE)
    ├── GOT region (capability slots, mapped with RIGHT_READ+RIGHT_WRITE)
    ├── Heap region (capability-gated, expandable)
    └── Guard region (unmapped, for heap overflow detection)
High address
```

`RFC-COMPILER-001.35`: Address space layout MAY include ASLR (Address Space Layout Randomisation) for the component base address. ASLR is optional per profile; the Security-Critical profile REQUIRES ASLR.

### 2.9 RISC-V Specific ABI Elements (Phase 1)

`RFC-COMPILER-001.36`: Phase 1 follows the standard RV64 integer calling convention wherever possible. A Software Capability Provider handle occupies one XLEN register.

`RFC-COMPILER-001.37`: Handle arguments use ordinary `a0`–`a7` positions according to generated function signatures; status and primary result follow the approved psABI-compatible result convention. No twelve-register token convention exists.

`RFC-COMPILER-001.38`: Cross-domain entry points are generated stubs. They copy ordinary data only after bounds validation and transfer capabilities through the provider, which returns destination-local handles.

`RFC-COMPILER-001.39`: Canonical capability records are passed by pointer and length only to explicit import/export calls. The mediator validates version and the entire record before creating a live entry.

`RFC-COMPILER-001.40`: A future CHERI-RISC-V ABI uses tagged capability registers and approved CHERI calling conventions behind the same IDL semantics; it need not preserve the software handle representation.

### 2.10 Symbol Resolution and Component Linking (REQ-COMP-05)

`RFC-COMPILER-001.41`: Component symbols are classified as **capability symbols** or **data symbols**. Capability symbols represent capability tokens exported by a component. Data symbols represent data addresses.

`RFC-COMPILER-001.42`: The symbol table format distinguishes capability symbols from data symbols using a type field. A capability symbol's value is an offset into the component's GOT where the live capability handle is stored. A data symbol's value is an offset into the component's data region.

`RFC-COMPILER-001.43`: Symbol resolution occurs at load time. The Composition Root resolves imported symbols by matching the symbol name and type against exported symbols in the component's manifest (RFC-0002). Capability symbols are resolved by granting the importing component a derived capability over the exporting component's capability slot.

`RFC-COMPILER-001.44`: A symbol not declared in the manifest CANNOT be resolved. Attempting to import an undeclared symbol is a load-time error that prevents the component from being instantiated.

`RFC-COMPILER-001.45`: Weak symbol semantics are permitted for data symbols. Weak capability symbols are NOT permitted; all capability symbol bindings are strong and must be resolved at load time.

`RFC-COMPILER-001.46`: The minimum viable symbol resolution for Phase 1 is **static, load-time resolution**. Full dynamic linking (runtime symbol resolution) is deferred to Phase 3.

### 2.11 Capability-Mediator Integration at ABI

`RFC-COMPILER-001.47`: The capability-mediator (per INTF-000) is the trust boundary. Above the mediator, AetherOS code assumes capability semantics. Below the mediator, the provider's implementation is software-trusted (SCP) or hardware-trusted (NCP).

`RFC-COMPILER-001.48`: Every ABI-level capability check (seal verification, rights check, domain bound check) is performed by the capability-mediator. The ABI itself does NOT perform capability checks; the ABI provides the register convention and calling convention that allow the mediator to be invoked.

`RFC-COMPILER-001.49`: The capability-mediator MAY be invoked implicitly (at every memory access via the MMU boundary) or explicitly (at designated check points in the code). The choice is provider-specific. In Phase 1 (SCP), the mediator is invoked explicitly at designated check points to minimise overhead.

---

## 3. ABI State Machine

```
COMPONENT_NOT_LOADED
    │ manifest loaded, symbols resolved
    ▼
COMPONENT_LOADED — GOT populated, relocations applied
    │ entry stub invoked
    ▼
COMPONENT_ENTERING — capability environment established
    │ application code executing
    ▼
COMPONENT_RUNNING — normal execution
    │ component calls function with capability argument
    ▼
CAPABILITY_PASSED — token in capability-carrying registers
    │ callee invokes capability-mediator
    ▼
CAPABILITY_CHECKED — seal + rights + domain verified
    │ operation proceeds or fault
    ▼
OPERATION_COMPLETE or CAPABILITY_FAULT
    │ component terminates
    ▼
COMPONENT_EXITING — exit stub executing
    │ capabilities revoked, memory released
    ▼
COMPONENT_DESTROYED — address space released
```

`RFC-COMPILER-001.50`: The `COMPONENT_ENTERING → COMPONENT_RUNNING` transition MUST emit a `ComponentEntered` event per RFC-0031. The `COMPONENT_EXITING → COMPONENT_DESTROYED` transition MUST emit a `ComponentExited` event per RFC-0031.

---

## 4. Data Structures

### 4.1 Capability-Carrying Register State (RV64)

```c
struct capability_register_state {
    uint64 regs[12];           // 12 × 8 = 96 bytes, maps to a0–a11 or t0–t11
};
```

### 4.2 Component Entry Block

```c
struct component_entry_block {
    struct capability_token  root_capability;     // component's root capability
    struct capability_token  stack_capability;     // stack region capability
    struct capability_token  heap_capability;      // heap allocation capability
    uintptr_t                tls_pointer;          // TLS region base address
    uintptr_t                entry_pc;             // component entry point
    uint32                   domain_index;         // component's domain
    uint32                   capability_count;     // number of initial capabilities
    // Followed by: capability_count × 96-byte capability tokens
};
```

### 4.3 Component Exit Record

```c
struct component_exit_record {
    struct object_id        component_id;         // component identity
    uint32                  exit_reason;           // 0=normal, 1=exception, 2=fault
    uint32                  error_code;            // error code (if exit_reason != 0)
    uint64                  timestamp_ns;          // exit timestamp
    uint32                  capabilities_revoked;  // number of capabilities revoked at exit
    uint32                  reserved;
};
```

### 4.4 Error Token (Canonical)

```c
// The canonical error token — a constant, not a resource
const struct capability_token ERROR_TOKEN = {
    .type = CAP_ERROR,                          // type 11
    .rights = 0,                                // repurposed as error code at use site
    .flags = 0,
    .seal = {0},                                // zeroed seal — not a valid capability
    .target_object = {0},                       // zeroed — no target
};
```

### 4.5 Symbol Table Entry

```c
struct symbol_entry {
    char                    name[32];            // symbol name (null-terminated)
    uint8                   type;                // 0=data, 1=capability
    uint16                  flags;               // 0=strong, 1=weak (data only)
    uint32                  got_offset;          // offset into GOT (for capability symbols)
    uint32                  data_offset;         // offset into data region (for data symbols)
    uint32                  component_id;        // exporting component
};
```

### 4.6 GOT Entry

```c
struct got_entry {
    struct capability_token capability;          // resolved capability token (96 bytes)
    uint32                  symbol_index;        // index into symbol table
    uint32                  reserved;
};
```

---

## 5. Failure Semantics

| Failure | Consequence |
|---------|-------------|
| Capability in faulting representation | `ECAPSTALE`; capability check fails at mediator |
| Capability not in manifest | Component cannot be instantiated; load-time error |
| Component entry stub fails | Component terminated; `ComponentExited` event emitted |
| Component exit stub fails (during cleanup) | Forced termination; exit record emitted with reason code |
| Error token presented for resource access | `ECAPSTALE`; error token is not a valid capability |
| Unhandled error token propagates to component root | Component terminated via exit stub |
| Uncaught hardware fault | Delivered as error token to registered fault handler |
| Symbol not in manifest | Load-time error; component not instantiated |
| GOT entry not resolved | Component not instantiated; load-time error |
| Capability argument exceeds register capacity | Second capability passed via stack slot; no fault |
| PIC relocation fails at load time | Component not instantiated; load-time error |

---

## 6. Conformance Tests

- `CTX-COMP-001`: Capability passed to a function is identical on entry and exit. The token bytes are unchanged after a round-trip through capability-carrying registers.
- `CTX-COMP-002`: Capability not explicitly passed cannot be derived by callee. A callee does not gain access to a capability that was not in its parameter set.
- `CTX-COMP-003`: Invalid capability register state (faulting representation) triggers `ECAPSTALE`, not silent propagation.
- `CTX-COMP-004`: Object identity field in the object header is immutable post-creation. Attempted modification is detected.
- `CTX-COMP-005`: Null capability (zeroed token) yields `ECAPSTALE` when presented to the capability-mediator.
- `CTX-COMP-006`: Every component exit emits a `ComponentExited` event before the component's address space is reclaimed.
- `CTX-COMP-007`: Uncaught exception propagates as an error token to the component's fault handler. If no handler is registered, the component is terminated.
- `CTX-COMP-008`: Compiled component runs correctly at any load address within the platform's address space constraints (PIC correctness).
- `CTX-COMP-009`: Capability-relative relocation (GOT entry) resolves to the correct object after load-time relocation.
- `CTX-COMP-010`: Capability symbol resolves to the same object across component instances within the same domain.
- `CTX-COMP-011`: Symbol not declared in the manifest cannot be resolved. Import of an undeclared symbol prevents component instantiation.

---

## 7. Verification Expectations

| Component | Tier | Proof Obligation |
|-----------|------|-----------------|
| Capability register convention | VT-A | Capability registers are never implicitly created (I-2). ABI cannot be violated without hardware fault. |
| Component entry/exit stub | VT-C | Entry stub establishes capability environment exactly as declared. Exit stub revokes all bound capabilities. |
| Error token semantics | VT-A | Error token is not a valid capability. Error token carries no authority. |
| Object header immutability | VT-A | Identity field cannot be modified after creation. |
| PIC correctness | VT-C | Component runs correctly at any valid load address. |
| Symbol resolution | VT-C | Undeclared symbols cannot be resolved. Capability symbols resolve correctly. |

---

## 8. Cross-References

- `RFC-0037` (Capability Token Format — 96-byte token structure, types, seal).
- `RFC-0038` (Capability Transfer Protocol — transfer semantics at ABI level).
- `RFC-0025` (Endpoint Model — capability as message argument).
- `RFC-0007` (Execution Domain — bootstrap stages, domain binding).
- `RFC-0013` (Capability Mediation — enforcement at ABI boundary).
- `RFC-ALLOC-001` (Allocator Hierarchy — stack/heap capabilities, memory region descriptors).
- `RFC-0002` (Component Manifest — symbol export format, capability declarations).
- `RFC-0003` (Capability Routing — capability symbols as endpoints).
- `RFC-0031` (Event Model — component entry/exit events).
- `INTF-000-CAPABILITY-ENFORCEMENT-SUBSTRATE-DECISION.md` (Software Capability Provider active in Phase 1).
- `PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md` (REQ-COMP-01 through REQ-COMP-05, INTF-002, INTF-003, INTF-006).

---

## 9. Open Questions

1. **Maximum capability arguments per call.** The RV64 ABI limits register-passed capabilities to two. Is this sufficient for Phase 1 component interfaces? If not, the stack-passed capability slot mechanism must be validated for latency.
2. **Error token sealing.** Should the error token carry a seal (even if zeroed) to allow future extension to sealed error tokens with context? For Phase 1, the error token seal is zeroed.
3. **TLS model complexity.** Phase 1 uses a simple flat TLS model (one TLS block per thread, pointer in a register). A more complex TLS model (e.g., ELF TLS) may be needed for multi-threaded components. Deferred to Phase 2.
4. **Dynamic linking scope.** Phase 1 uses static, load-time resolution only. The ABI should be designed to accommodate runtime resolution in Phase 3 without breaking the calling convention. The GOT-based approach satisfies this requirement.

---

## Revision History

| Date | Change |
|------|--------|
| this cycle | Initial draft. ABI, capability passing, component lifecycle, error propagation, PIC, and symbol resolution specified. INTF-002, INTF-003, INTF-006 resolved. I-1, I-2, I-12, I-13 implemented. RISC-V-specific ABI defined for Phase 1. |
| 2026-07-20 | Coordinated Specification Amendment applied with RFC-0037 §2 amendment (Cross-Impact Gate 001 authorization; PHASE-4-AUTHORIZATION-RFC-0037-S2.md). RFC-COMPILER-001.30 updated: "MAY be zeroed" removed; error tokens now MUST carry a valid HMAC-SHA256 seal per RFC-0037.9.1 (Model A mandatory sealing). |
