# Compatibility Strategy and Tier Model

Classification: Operational
Authoritative Source: COMPATIBILITY-STRATEGY.md
Requirement-ID: GOV-COMPAT-001
Status: Active

## 1. Principle

Compatibility is a set of independently versioned products, not one kernel promise. Native AetherOS interfaces are not constrained to reproduce legacy design. Personalities translate legacy semantics; virtual machines preserve semantics that cannot safely or accurately be translated.

## 2. Claim tiers

| Tier | Claim | Minimum evidence |
|---|---|---|
| C0 | Unsupported/unknown | No claim |
| C1 | Source portable | Clean build plus declared source changes and tests |
| C2 | Portable component | WebAssembly Component/WASI contract and sandbox tests |
| C3 | Native ABI | AetherOS ELF/ABI conformance and lifecycle tests |
| C4 | POSIX source | Declared POSIX edition/options; compile and behavioral suite |
| C5 | Foreign user ABI | Named OS/architecture/version/syscall surface; differential tests |
| C6 | Container workload | Named OCI/runtime features; filesystem/network/cgroup-equivalent tests |
| C7 | Framework application | Named Android/Win32/etc. API level and application corpus |
| C8 | Whole guest OS | Named guest/version on a named virtual platform |
| C9 | Device/driver bridge | Named device, firmware, host, isolation model, stress/fault results |

No higher tier implies a lower tier. A Windows VM does not prove Win32 personality compatibility; a POSIX build does not prove Linux binary compatibility.

## 3. Implementation lanes

- **Native lane:** stable C-compatible ABI, Rust SDK, asynchronous I/O, capability-aware IDL, ELF debugging, and optional WebAssembly components.
- **Linux lane:** begin with Linux guests and virtio; then implement a versioned user-space syscall personality for high-value binaries. Never call POSIX “Linux compatibility.”
- **Windows lane:** use a lawful Wine-like user personality for supported Win32 applications and a licensed Windows guest for irreducible dependencies.
- **Android lane:** use an isolated Android framework over a Linux compatibility environment or guest VM; declare API level, graphics, binder, media, and device integration separately.
- **BSD lane:** prioritize source portability and guest VMs before foreign binary personalities.
- **macOS/iOS lane:** support portable source and standards-based components. Any guest or binary use is conditional on licensing, hardware, signing, entitlement, and distribution rules.
- **Containers:** OCI metadata is not a syscall ABI. Initially run Linux containers inside a minimal Linux VM; introduce direct hosting only when the required Linux semantic surface is measured.

## 4. Drivers and foreign kernels

Driver support order is: native memory-safe driver, isolated native legacy-language driver, protocol/device proxy, foreign driver inside a driver VM, full guest passthrough, unsupported. DMA requires an IOMMU domain and explicit capability. A foreign kernel module never loads into the AetherOS privileged core.

Initial portable devices are UART, virtio block/net/input/GPU, PCIe enumeration, NVMe, and USB xHCI. Real hardware support grows through published platform matrices, not an “all devices” assertion. Proprietary firmware is packaged separately with license and hash metadata.

## 5. Compatibility database

Every result records application/device identifier, version, architecture, tier, configuration manifest, test corpus, known deviations, performance overhead, legal prerequisites, last-tested date, maintainer, and evidence links. “Works” without this record is anecdotal.

## 6. Decision gates for compromises

When exact behavior conflicts with security, correctness, performance, or licensing, an Architecture Review SHALL present:

1. exact workload and required behavior;
2. native translation feasibility;
3. virtualization feasibility;
4. application modification feasibility;
5. security and TCB delta;
6. legal and redistribution constraints;
7. measured performance/size impact;
8. recommended compromise and explicit user decision.

No silent semantic compromise is permitted.
