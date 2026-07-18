# RFC-COMPILER-001: Compiler and Runtime Boundary

**Status:** Draft.
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

### 2.2 Capability Register Convention (INTF-002)

`RFC-COMPILER-001.3`: In Phase 1 (Software Capability Provider per INTF-000), capability tokens are represented in conventional integer registers. There is no dedicated hardware capability register class. A 96-byte capability token occupies multiple consecutive registers (see Section 2.9 for RISC-V).

`RFC-COMPILER-001.4`: The ABI defines a set of **capability-carrying registers**: registers that MAY contain a valid capability token during a function call. Capability-carrying registers are a subset of the architecture's general-purpose register file.

`RFC-COMPILER-001.5`: Capability-carrying registers are **caller-saved** by convention. A callee that wishes to preserve a capability across a call MUST copy it to a callee-saved location (stack slot or callee-saved register pair) before making any call. The caller MUST NOT assume that capability-carrying registers are preserved across a call.

`RFC-COMPILER-001.6`: When a capability token is loaded into a capability-carrying register, the register pair MUST be loaded atomically (both halves of the 96-byte token loaded before any use). A capability-carrying register that contains only one half of a token is in a **faulting representation**.

`RFC-COMPILER-001.7`: A faulting representation is a register state that, when presented to the capability-mediator, yields `ECAPSTALE`. The faulting representation MUST be detectable by the capability-mediator in O(1) time. Two common faulting representations are: (a) a register pair where the seal field has been zeroed, and (b) a register pair where the high word contains a distinguished invalid-marker value (platform-defined constant).

`RFC-COMPILER-001.8`: The ABI MUST NOT create capability tokens implicitly. Every capability in a capability-carrying register MUST originate from: (a) a function parameter, (b) a return value, (c) a capability load from memory, or (d) the component's initial capability environment (Section 2.6). A capability that appears in a register without one of these origins is non-conformant (I-2).

### 2.3 Capability Passing Protocol

`RFC-COMPILER-001.9`: When a component function call passes a capability token as an argument, the token is passed in consecutive capability-carrying registers. The callee receives the token and MAY pass it to the capability-mediator for verification before use.

`RFC-COMPILER-001.10`: A function MAY return at most one capability token as a return value. Return values that include a capability use the same register pair as the primary return. Multi-capability return values are NOT supported; they MUST be communicated through memory or through sequential calls.

`RFC-COMPILER-001.11`: Capability tokens are never passed by value in the stack frame as part of the calling convention. Stack-passed arguments are always raw data (integers, pointers to data, or pointers to capability slots in memory). Capability tokens in stack frames are permitted only as explicit save/restore by the callee for preservation across calls.

`RFC-COMPILER-001.12`: When a capability token crosses a trust boundary (user→kernel, user→device, kernel→user), the boundary-crossing MUST be mediated by the capability-mediator per RFC-0013. The ABI provides no mechanism for trust-boundary bypass.

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
    // Followed by: capability_slots × 96-byte capability_token
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

`RFC-COMPILER-001.16`: A **null capability** is a 96-byte zeroed token. A null capability is a valid representation that yields `ECAPSTALE` when presented to the capability-mediator (per RFC-0037.9: no capability and invalid capability are indistinguishable).

`RFC-COMPILER-001.17`: An **invalid capability** is any token whose seal verification fails. The ABI MUST NOT produce invalid capabilities as a normal outcome. An invalid capability in a capability-carrying register is a programming error that yields `ECAPSTALE`.

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

`RFC-COMPILER-001.25`: Errors at the ABI level are represented by a distinguished **error token**: a 96-byte capability token with `type = CAP_ERROR` (type 11) and a zeroed `target_object` field. The error token is NOT a valid capability for any resource access; presenting it to the capability-mediator for resource access yields `ECAPSTALE`.

`RFC-COMPILER-001.26`: A function that encounters an error returns the error token instead of a success capability. The caller distinguishes success from error by checking whether the returned value is the error token (comparison against the canonical error token constant).

`RFC-COMPILER-001.27`: The error token carries an error code in the `rights` field (repurposed as error code for `CAP_ERROR` tokens). The error code is one of a fixed set defined per profile:

| Code | Name | Meaning |
|------|------|---------|
| 0x01 | `ERR_NOMEM` | Allocation failed |
| 0x02 | `ERR_NOCAP` | Required capability not held |
| 0x03 | `ERR_FAULT` | Capability fault (seal invalid, expired, revoked) |
| 0x04 | `ERR_RANGE` | Address or size out of bounds |
| 0x05 | `ERR_PERM` | Insufficient rights for operation |
| 0x06 | `ERR_DEPEND` | Dependency not satisfied |
| 0x07 | `ERR_TIMEOUT` | Operation timed out |
| 0x08 | `ERR_BUSY` | Resource temporarily unavailable |

`RFC-COMPILER-001.28`: Error tokens are NOT catchable by the component that caused the error. An error token returned to a component indicates that the operation failed and the component MUST handle it through normal control flow (e.g., branching on the error token). Unhandled error tokens propagate upward through the call stack until caught or until the component exits.

`RFC-COMPILER-001.29`: Uncaught exceptions (hardware faults, capability-mediator faults, asynchronous faults) are delivered to the component as an error token via the fault handler registered in the component's manifest. If no fault handler is registered, the component is terminated and the exit stub (RFC-COMPILER-001.23) executes.

`RFC-COMPILER-001.30`: An error token MUST NOT carry information about the resource that caused the error beyond the error code. Error tokens are not capabilities; they grant no authority. The error token's `seal` field MAY be zeroed (it is not a valid capability seal).

### 2.8 Position-Independent Code and Address Space Layout (REQ-COMP-04)

`RFC-COMPILER-001.31`: Components MUST be compiled as position-independent code (PIC). Component load addresses are determined at load time by the Composition Root, not at compile time.

`RFC-COMPILER-001.32`: The ABI defines a **Global Offset Table (GOT)** for capability-relative relocations. Each capability reference in the component's code is resolved through a GOT entry that contains the capability token's in-memory address. The GOT is populated at load time.

`RFC-COMPILER-001.33`: The ABI supports load-time relocations for the following reference types:

| Relocation Type | Description |
|----------------|-------------|
| `REL_CAPABILITY` | Capability token reference (96-byte aligned) |
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

`RFC-COMPILER-001.36`: The following are RISC-V-specific ABI elements for the Phase 1 reference platform. These are NOT architecture-neutral; they apply only when the execution provider is a RISC-V hart.

**Register allocation for capabilities:**

| Register pair | Role | Caller-saved? |
|---------------|------|---------------|
| `a0`–`a7` (8 registers) | Capability arguments (up to 4 capabilities in argument registers; each capability uses 2 registers on RV64) | Yes |
| `t0`–`t6` (7 registers) | Capability temporaries | Yes |
| `s0`–`s11` (12 registers) | Data only; MUST NOT contain capability tokens | N/A |

`RFC-COMPILER-001.37`: On RV64, a 96-byte capability token occupies 12 general-purpose registers (each 8 bytes). The token is divided into three 32-byte sections, each occupying 4 registers:

| Register group | Token section |
|----------------|---------------|
| `a0`–`a3` (or `t0`–`t3`) | Identity section (bytes 0–31) |
| `a4`–`a7` (or `t4`–`t6` + 1 temp) | Reference section (bytes 32–63) |
| `t0`–`t3` (reused or stack) | Integrity section (bytes 64–95) |

`RFC-COMPILER-001.38`: Because RV64 has only 32 general-purpose registers, passing more than two 96-byte capabilities simultaneously via registers is NOT feasible. The ABI limits capability arguments to a maximum of **two capability tokens** passed in registers. Additional capabilities are passed via memory (stack-passed capability slots).

`RFC-COMPILER-001.39`: The RISC-V ABI uses the following calling convention extensions for capability passing:

- **Capability return value:** The first capability return value occupies `a0`–`a11` (12 registers). A function returning a capability uses the full `a` register file.
- **Capability arguments:** The first capability argument occupies `a0`–`a11`. The second capability argument (if any) is passed via a pointer to a stack-allocated capability slot.
- **Capability preservation:** Callee-saved capability preservation uses the stack. The callee saves the capability's 96 bytes to a known stack offset before making any call.

`RFC-COMPILER-001.40`: The capability-mediator performs seal verification on the capability's integrity section (bytes 64–95) when the capability is presented for a resource operation. The mediator does NOT require the capability to reside in specific registers; the register convention is a software contract, not a hardware enforcement.

### 2.10 Symbol Resolution and Component Linking (REQ-COMP-05)

`RFC-COMPILER-001.41`: Component symbols are classified as **capability symbols** or **data symbols**. Capability symbols represent capability tokens exported by a component. Data symbols represent data addresses.

`RFC-COMPILER-001.42`: The symbol table format distinguishes capability symbols from data symbols using a type field. A capability symbol's value is an offset into the component's GOT where the capability token is stored. A data symbol's value is an offset into the component's data region.

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
