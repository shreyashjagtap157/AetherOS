# RFC-HW-001: Hardware Support Architecture

Classification: Normative
Authoritative Source: Hardware-Support-RFC.md
Requirement-ID: ARCH-HW-001

**Status:** Active.
**Implements invariants:** I-1, I-2, I-3, I-9, I-10, I-13.
**Depends on:** RFC-0007 (Execution Domain), RFC-0009 (Address Space), RFC-0010 (Interrupt Routing), RFC-0011 (Scheduler), RFC-0013 (Capability Mediation), RFC-0040 (Verification Tier Methodology).

---

## 1. Scope

This RFC specifies the hardware support architecture: how the platform abstracts, supports, and integrates diverse computational substrates ranging from classical CPUs to future quantum processors. It defines the execution-provider contract that all hardware must satisfy, the hardware adaptation layer (HAL) that bridges hardware to the platform, and the concrete support matrix for all targeted hardware families.

This RFC is a **companion to the Execution Architecture** (`RFC-0007` through `RFC-0014`). It does not modify any invariant in `Platform-Architecture-Specification-v1.1.md`.

---

## 2. Hardware Philosophy

### 2.1 The Stable Contract Principle

The platform makes a strict distinction between what is **stable** and what is **hardware-specific**:

**Stable (never changes per hardware family):**
- The execution-provider contract interface (Section 3)
- The capability token format (`RFC-0037`)
- The address space API (`RFC-0009`)
- The interrupt routing API (`RFC-0010`)
- The dispatch protocol (`RFC-0008`)
- The scheduler interface (`RFC-0011`)

**Hardware-specific (varies per hardware family):**
- MMU page table format and management
- Interrupt controller programming model
- Cache-coherency protocol implementation
- Memory bus characteristics (latency, bandwidth, NUMA topology)
- Vector/SIMD instruction availability
- Clock and timer hardware

The stable contract is what allows the platform core to be hardware-agnostic. Every new hardware target requires a new HAL that implements the stable contract over that hardware's specific characteristics.

### 2.2 Hardware Families

The platform categorizes hardware into **execution provider classes**. Each class represents a fundamentally different computational model:

| Class | Computational Model | Examples |
|-------|--------------------|----------|
| **Classical CPU** | Sequential/parallel imperative execution | x86-64, ARM64, RISC-V64, POWER |
| **GPU/Accelerator** | Massively parallel SIMT/SIMD | NVIDIA CUDA, AMD RDNA, Intel Xe |
| **TPU/AI-Accelerator** | Tensor/matrix operation offload | Google TPU, AWS Inferentia |
| **FPGA** | User-defined circuitry, partial reconfiguration | Xilinx/AMD Versal, Intel Agilex |
| **DPU/SmartNIC** | Distributed storage and network offload | NVIDIA BlueField, AMD Pensando |
| **Neuromorphic** | Spiking neural network, event-driven | Intel Loihi, IBM TrueNorth |
| **Quantum** | Quantum circuit execution, gate-model | IBM Quantum, Google Sycamore |

A single platform deployment may have multiple provider classes active simultaneously. The scheduler (`RFC-0011`) treats all execution providers uniformly for scheduling purposes, but each class implements the execution-provider contract differently under the hood.

---

## 3. The Execution-Provider Contract

Every execution provider, regardless of hardware class, implements the following contract:

### 3.1 Thread Representation

`RFC-HW-001.1`: The execution provider exposes a **hardware thread** (hart) abstraction: a schedulable execution context with its own program counter, register set, and execution stack.

`RFC-HW-001.2`: Each hart has a fixed-capacity **register file** with declared width (bits) and depth (number of architectural registers). The provider declares these at registration.

### 3.2 Address Space

`RFC-HW-001.3`: The execution provider manages an address space via an MMU or equivalent. It supports:
- Virtual-to-physical address translation
- Memory protection bits (read/write/execute)
- At least two page-size classes (base and large)
- Optional huge page support

The provider exposes this via the `address_space_operations` interface, which is hardware-specific but wrapped by the platform's address space model (`RFC-0009`).

### 3.3 Time and Interrupts

`RFC-HW-001.4`: The execution provider exposes:
- A monotonic counter for duration measurement
- A timer facility for scheduled interrupts
- An interrupt controller with programmable priority and masking

The interrupt routing (`RFC-0010`) operates over this abstraction regardless of the specific controller (APIC, GIC, RISC-V AIA, etc.).

### 3.4 Capability Enforcement

`RFC-HW-001.5`: The execution provider **enforces capability mediation** at the hardware boundary. This is not optional. If the hardware does not natively support capability-based memory protection, the provider implements it in the HAL with acceptable performance overhead. Acceptable overhead for software-emulated capability enforcement is declared at provider registration.

### 3.5 Dispatch

`RFC-HW-001.6`: The execution provider implements the dispatch protocol (`RFC-0008`): moving a thread from scheduler decision to execution unit. The provider implements this through its specific instruction fetch/decode/execute pipeline.

---

## 4. Hardware Adaptation Layer (HAL)

### 4.1 HAL Structure

The HAL is the hardware-specific shim between raw hardware and the execution-provider contract. It is implemented per hardware family.

```c
// The HAL operations structure — all execution providers implement this
struct hal_operations {
    // Hart management
    int (*hart_start)(uint32_t hart_id, uintptr_t entry_pc, uintptr_t sp);
    int (*hart_stop)(uint32_t hart_id);
    int (*hart_suspend)(uint32_t hart_id);
    int (*hart_resume)(uint32_t hart_id);

    // Address space
    int (*mmu_init)(struct mmu_config *cfg);
    int (*map_range)(uintptr_t vaddr, uintptr_t paddr, uint64_t size, uint8_t rights);
    int (*unmap_range)(uintptr_t vaddr, uint64_t size);

    // Interrupts
    int (*irq_enable)(uint32_t irq_id, uint8_t priority);
    int (*irq_disable)(uint32_t irq_id);
    int (*irq_complete)(uint32_t irq_id);

    // Time
    uint64 (*get_time_ns)(void);
    int (*set_timer)(uint64_t deadline_ns);

    // Capability enforcement (hardware-assisted or HAL-emulated)
    int (*cap_init)(void);
    int (*cap_check)(struct capability_token *cap, uintptr_t address, uint8_t operation);

    // Provider info
    void (*get_provider_info)(struct provider_info *info);
};
```

### 4.2 Provider Registration

`RFC-HW-001.7`: At platform boot, each active execution provider registers with the Execution Architecture by calling `execution_provider_register()`. The registration includes:
- Provider class (Classical CPU, GPU, TPU, etc.)
- HAL operations structure
- Provider characteristics (number of harts, MMU capabilities, timer resolution, etc.)
- Provider-specific configuration

`RFC-HW-001.8`: The platform supports **multiple active providers simultaneously**. A system with an x86-64 CPU, an NVIDIA GPU, and a TPU has three registered providers. The scheduler (`RFC-0011`) dispatches across all of them using the same algorithm.

### 4.3 Provider Isolation

`RFC-HW-001.9`: Each execution provider operates within its own security domain. A capability token for a GPU work object is meaningful only within the GPU provider's namespace. Cross-provider capability use (e.g., a CPU component accessing a GPU tensor) requires an explicit capability granted by the component that owns the other provider's object.

---

## 5. Classical CPU Support (Phase 1 Targets)

### 5.1 x86-64 (Intel/AMD)

**Architectures supported:** x86-64 (Intel/AMD 64-bit), including:
- Intel Skylake and later (AVX-512)
- AMD Zen 2 and later (AVX-512)
- Virtualization: Intel VT-x and AMD AMD-V for microVMs

**HAL specifics:**
- **MMU:** x86-64 long mode with 4-level paging (IA-32e). Base page: 4 KiB. Large page: 2 MiB. Huge page: 1 GiB. The HAL manages CR3 and processes page-table walks.
- **Interrupts:** APIC (local APIC + IOAPIC). HAL abstracts the local APIC register file. MSI (Message Signaled Interrupts) for PCIe devices.
- **Capability enforcement:** Intel CET (Control-flow Enforcement Technology) where available; HAL-emulated shadow stack where not.
- **Vector units:** AVX-512 exposed to user components as SIMD capability tokens. Scheduler is unaware of vector unit topology.
- **Timer:** HPET (High Precision Event Timer) + local APIC timer. Resolution declared at provider registration.

### 5.2 ARM64 (ARMv8-A)

**Architectures supported:** ARMv8-A, ARMv8.2-A (with FEAT_LSE atomic instructions), including:
- Apple Silicon (M-series)
- ARM Neoverse (N1, N2, V1)
- Qualcomm Snapdragon
- Virtualization: ARMv8.1-A hvc for hypervisor

**HAL specifics:**
- **MMU:** ARMv8-A long format with 4-level translation tables. Base page: 4 KiB (granule 4K, 16K, or 64K per Linux kernel conventions). Large: 2 MiB (contiguous hint). Huge: 1 GiB.
- **Interrupts:** GICv3 or GICv4. Redistributor and ITS (Interrupt Translation Service) for MSI equivalents. HAL manages ICC_*_EL1 system registers.
- **Capability enforcement:** ARM Pointer Authentication (PAuth) + BTI (Branch Target Identification) where available; HAL-emulated on earlier hardware.
- **Vector units:** NEON (64-bit and 128-bit) and SVE/SVE2 (variable length). Declared as vector capability classes.
- **Timer:** ARM Generic Timer (CNTFRQ_EL0) and EL1 physical timer. Architecturally defined resolution.

### 5.3 RISC-V64 (Future — Phase 3)

**Architectures supported:** RISC-V64 (RV64GC, with M, A, F, D, C extensions), including:
- SiFive FU740
- StarFive JH7110
-阿里巴巴 T-Head C910

**HAL specifics:**
- **MMU:** Sv39 (39-bit virtual address, 56-bit physical) or Sv48 (48-bit virtual). HAL manages satp CSR. Base page: 4 KiB.
- **Interrupts:** RISC-V Advanced Interrupt Architecture (AIA). MSISC extensions for PCIe-style interrupts. HAL manages clint or aia-msic depending on hart capability.
- **Capability enforcement:** RISC-V does not yet have a mature hardware capability extension comparable to CHERI; HAL-emulated capability mediation is used. H extension (hypervisor) for virtualization.
- **Timer:** clint (Core Local Interruptor) for timer interrupts. MTIME/MTIMECMP registers.

### 5.4 IBM POWER (Future — Phase 3)

**Architectures supported:** POWER9, POWER10.

**HAL specifics:**
- **MMU:** PowerPC-segmented MMU with variable page size (4K, 64K, 16M). Hash table for legacy, radix tree for POWER9+. HAL manages HPT (Hash Page Table).
- **Interrupts:** XIVE (EXtreme Interrupt Environment) for POWER9/10. HAL manages interrupt server registers.
- **Capability enforcement:** HAL-emulated (POWER has no native capability extension comparable to CHERI).
- **Vector units:** AltiVec/VSX (128-bit). Declared as vector capability class.

---

## 6. GPU Support (Phase 3–4 Targets)

### 6.1 GPU as Execution Provider

A GPU is treated as a **massively parallel execution provider** with hundreds or thousands of small execution contexts (workitems/threads/warps). The scheduler dispatches to GPU harts (SIMT lanes) the same way it dispatches to CPU harts — but GPU dispatch uses the GPU provider's HAL, not the CPU provider's.

### 6.2 GPU Memory Model

`RFC-HW-001.10`: GPU device memory is exposed as a **separate NUMA node** within the address space model (`RFC-0009`). GPU memory allocations use a NUMA hint; the address space HAL manages PCIe ATS (Address Translation Services) for GPU memory access from the CPU.

`RFC-HW-001.11`: Unified memory (where the GPU and CPU share a virtual address space) is a provider feature: the GPU provider MAY expose a unified address range, and the HAL guarantees cache-coherency across the CPU and GPU caches for that range.

### 6.3 GPU Capability Model

`RFC-HW-001.12`: GPU kernels and tensor operations are **capability-addressable objects**. A component that holds a `CAP_OBJECT` capability for a compiled GPU kernel can dispatch it to the GPU provider. The GPU provider verifies the capability before accepting the dispatch.

### 6.4 Supported GPU APIs

| GPU Provider | API Compatibility | Notes |
|-------------|-----------------|-------|
| NVIDIA (CUDA) | CUDA driver API | cuBLAS, cuDNN as provider services |
| AMD (ROCm) | HIP, OpenCL 3.0 | ROCm stack as provider |
| Intel (Xe) | Level Zero, OpenCL | Data Center GPU Max Series |
| Cross-platform | Vulkan Compute, OpenCL 3.0 | Provider-agnostic compute |

---

## 7. TPU and AI Accelerator Support (Phase 4)

### 7.1 TPU as Execution Provider

A TPU is a **domain-specific coprocessor** that performs matrix multiply operations at extreme throughput. The platform treats it as an execution provider with:
- Tensor workloads as capability-addressable objects
- Host memory mapped into the TPU's address space via PCIe
- TPU harts treated as specialized scheduler entities with a distinct Layer 1 algorithm class (dataflow scheduling rather than thread scheduling)

### 7.2 Tensor Workload Lifecycle

```
1. Host component creates tensor operation as a persistent object (RFC-0015)
2. Component acquires CAP_OBJECT capability for the tensor op
3. Component submits tensor op to TPU provider via capability transfer (RFC-0027)
4. TPU provider schedules tensor op on its execution units
5. Completion triggers an interrupt (RFC-0010) to host
6. Result written to tensor output object; capability returned
```

### 7.3 Supported AI Accelerators

| Accelerator | Interface | Notes |
|-------------|-----------|-------|
| Google TPU | TPU runtime API | Cloud TPU emulator available |
| AWS Inferentia | Neuron SDK | 1x/4x/16x neuron cores |
| Graphcore IPU | Poplar SDK | Graph compilation as capability management |
| Tesla Dojo | Custom API | As a specialized vector provider |

---

## 8. FPGA Support (Phase 4)

### 8.1 FPGA as Execution Provider

An FPGA is a **user-programmable circuit** that can be partially reconfigured at runtime. The platform treats an FPGA as an execution provider with:

- Partial bitstreams as capability-addressable persistent objects
- FPGA acceleration contexts as capability-addressable execution domains
- PCIe BAR-mapped memory as a storage/memory provider (RFC-0019)

### 8.2 FPGA Acceleration Context

`RFC-HW-001.13`: An FPGA acceleration context is an isolated region of the FPGA fabric, created by loading a partial bitstream. The acceleration context is represented as an execution domain (`RFC-0007`) with its own address space. The host CPU communicates with the FPGA via memory-mapped I/O and doorbell interrupts (RFC-0010).

### 8.3 Supported FPGA Platforms

| Platform | Toolchain | Partial Reconfiguration |
|----------|----------|----------------------|
| Xilinx/AMD Versal | Vitis | Yes — runtime partial reconfiguration |
| Intel Agilex | OneAPI / OpenCL | Yes — PR via Intel FPGA SDK |
| Intel Stratix 10 | OpenCL | Yes — partial reconfiguration via SDK |

---

## 9. DPU/SmartNIC Support (Phase 4)

### 9.1 DPU as Storage and Network Provider

A DPU is an ARM-based system-on-chip embedded in a network adapter. It runs its own operating system and exposes storage and network services. The platform treats a DPU as:
- A **storage provider** (`RFC-0019`) — DPU-attached NVMe exposed to the platform
- A **network provider** — DPU offload for the Communication Architecture
- A **capability-sealed execution domain** — DPU firmware is isolated and interacts via capability tokens

### 9.2 DPU Capability Model

`RFC-HW-001.14`: DPU resources (network queues, storage namespaces) are capability-addressable objects. The DPU firmware validates capabilities at its own boundary. The host platform's capability subsystem (`RFC-0013`) is not responsible for DPU enforcement — only the DPU's capability implementation is.

### 9.3 Supported DPUs

| DPU | Storage Provider | Network Provider |
|-----|-----------------|-----------------|
| NVIDIA BlueField-2/3 | Yes (NVMe-oF) | Yes (DOCA) |
| AMD Pensando | Yes (NVMe) | Yes |
| Intel IPU | Yes | Yes |

---

## 10. Quantum Processor Support (Phase 5 — Deferred)

### 10.1 Architectural Position

Quantum processors present a fundamentally different computational model. The platform does not attempt to unify classical and quantum execution under a single abstraction. Instead, the quantum processor is a **specialized execution provider with a distinct contract extension**.

### 10.2 Quantum Execution Domain

`RFC-HW-001.15`: A quantum processor is represented as a **quantum execution domain** — a distinct class of execution domain (`RFC-0007`) with:
- Qubit allocation and deallocation as lifecycle-managed resources (RFC-0018)
- Quantum gates as capability-addressable operations
- Classical/quantum co-execution via shared capability namespace
- Quantum error correction as a background lifecycle process

### 10.3 Quantum-Classical Boundary

`RFC-HW-001.16`: Quantum processors operate **alongside** classical CPUs, not instead of them. Classical CPUs manage the quantum processor's control plane via capability-addressable quantum control objects. The quantum processor executes quantum circuits dispatched from classical components.

Classical programs dispatch quantum circuits via:
1. Creating a quantum circuit as a capability-addressable object
2. Transferring the circuit capability to the quantum provider
3. Receiving completion notification via interrupt (RFC-0010)

### 10.4 Supported Quantum Systems

The platform will target:
- IBM Quantum (Qiskit Runtime API)
- Google Cirq (Cirq API)
- IonQ (Aria API)

Formal definitions of the quantum execution-provider contract extension are deferred to RFC-Q1.

---

## 11. Verification per Hardware Family

### 11.1 Tier Assignments

| Hardware Family | Verification Tier | Notes |
|----------------|-------------------|-------|
| x86-64 | Tier A (scheduler, cap enforcement) | Hardware CET support enables Tier A |
| ARM64 | Tier A (scheduler, cap enforcement) | PAuth + BTI enable Tier A |
| RISC-V64 | Tier B (no hardware cap enforcement) | HAL-emulated; formal proof of correctness |
| POWER | Tier B | HAL-emulated; formal proof of correctness |
| GPU | Tier C | Large state space; property-testing approach |
| TPU | Tier C | Domain-specific; conformance-tested |
| FPGA | Tier C | Bitstream integrity via Merkle proofs |
| DPU | Tier A at boundary, Tier B inside | Capability boundary is Tier A |
| Quantum | Tier B | Formal model of quantum circuit semantics |

### 11.2 Cross-Provider Testing

`RFC-HW-001.17`: Any capability token used across multiple providers (e.g., a storage object accessed from both a CPU and a GPU) MUST be verified by both providers. The CTS includes cross-provider capability tests.

---

## 12. Data Bus and Interconnect Architecture

### 12.1 Supported Buses and Interconnects

The platform's storage, network, and memory providers interact over well-defined bus architectures:

| Bus/Interconnect | Usage | Provider |
|-----------------|-------|---------|
| PCIe 4.0 / 5.0 | GPU, TPU, FPGA, DPU, NVMe | Storage Provider, Compute Provider |
| CXL 3.0 | CXL memory pools, CXL-attached persistent memory | Memory Provider (RFC-0009) |
| CCIX | Cache-coherent accelerator interconnect | Cross-provider memory |
| Infinity Fabric | AMD CPU-GPU interconnect | Memory coherence |
| NVLink 3.0 / 4.0 | NVIDIA GPU-GPU and GPU-CPU | Memory coherence |
| CXL + Gen-Z | Future memory and storage class memory | Next-gen provider |
| Ethernet (100GbE) | Network storage (iSCSI, NVMe-oF) | Network Provider |
| Infiniband HDR | RDMA storage | RDMA Provider |
| USB4 / Thunderbolt 4 | Peripheral devices | Device Provider |

### 12.2 CXL Memory as a Provider

`RFC-HW-001.18`: CXL-attached memory is a **memory provider** that the platform's address space HAL (`RFC-0009`) manages as a NUMA node. CXL memory provides byte-addressable persistent storage with memory-mapped I/O semantics. The Lifecycle FSM (`RFC-0018`) applies identically to CXL-resident objects.

### 12.3 Bus-Level Capability Enforcement

`RFC-HW-001.19`: PCIe devices that support PCIe IDE (Integral Data Encryption) have their memory transactions encrypted at the hardware level. The platform treats this as a capability enforcement extension at the bus level: a DMA access without a valid capability token is rejected at the PCIe controller.

---

## 13. Hardware Support Conformance Tests

The following CTS fixtures verify hardware provider conformance:

- `CTS-HW-PROV-1`: validates RFC-HW-001.7 — provider registration with complete HAL operations
- `CTS-HW-PROV-2`: validates RFC-HW-001.8 — multiple active providers
- `CTS-HW-PROV-3`: validates RFC-HW-001.9 — provider isolation
- `CTS-HW-GPU-1`: validates RFC-HW-001.10 — GPU memory as NUMA node
- `CTS-HW-GPU-2`: validates RFC-HW-001.11 — unified memory coherency
- `CTS-HW-GPU-3`: validates RFC-HW-001.12 — GPU kernel as capability-addressable object
- `CTS-HW-TPU-1`: validates RFC-HW-001.13 — TPU tensor operation lifecycle
- `CTS-HW-FPGA-1`: validates RFC-HW-001.14 — FPGA acceleration context as execution domain
- `CTS-HW-DPU-1`: validates RFC-HW-001.14 — DPU resources as capability-addressable
- `CTS-HW-CAPX-1`: validates RFC-HW-001.17 — cross-provider capability verification
- `CTS-HW-BUS-1`: validates RFC-HW-001.18 — CXL memory as NUMA node
- `CTS-HW-BUS-2`: validates RFC-HW-001.19 — PCIe IDE capability enforcement

---

## 14. Cross-References

- `RFC-0007` (Execution Domain — provider registration)
- `RFC-0008` (Dispatch — hart dispatch)
- `RFC-0009` (Address Space — NUMA, page tables)
- `RFC-0010` (Interrupt Routing — device interrupts)
- `RFC-0011` (Scheduler — multi-provider scheduling)
- `RFC-0013` (Capability Mediation — hardware-assisted enforcement)
- `RFC-0019` (Storage Provider Interface — DPU storage)
- `RFC-0037` (Capability Token Format — GPU/TPU tensor objects)
- `Platform-Architecture-Specification-v1.1.md` Section 11 (hardware universality)

---

## Revision History

| Date       | Change                                |
|------------|---------------------------------------|
| this cycle | Initial release. Execution-provider contract. HAL structure. x86-64, ARM64 detailed. GPU, TPU, FPGA, DPU specified. Quantum deferred to future RFC. CXL and bus interconnects specified. |