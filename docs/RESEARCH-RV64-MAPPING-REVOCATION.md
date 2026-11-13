# RV64 Memory Mapping and Capability-Revocation Research

Classification: Operational
Authoritative Source: RFC-0009, RFC-0013, RFC-0037, RFC-0039 and the accepted INTF-000 capability-enforcement substrate decision
Status: Experimental host-reference implementation decision; per-access authority revalidation and serialized runtime implemented on current development branch
Last reviewed: 2026-11-12

## 1. Scope and decision boundary

This work implements a host-side semantic model for the Software Capability Provider (SCP) and the address-space enforcement boundary on RV64/Sv39. It does not amend any normative RFC, implement a kernel page-table walker, or claim hardware conformance.

The model addresses a concrete gap between the capability reference provider and the execution architecture: a capability check can authorize the creation of a mapping, but on conventional RV64 hardware the CPU enforces memory accesses using page-table entries and translation caches. The host model therefore treats mapping authorization, PTE permissions, and translation invalidation as distinct mechanisms.

## 2. Normative source constraints

### RFC-0009 — Address spaces

The current RFC requires a MAP capability for mapping, declared READ/WRITE/EXECUTE permissions, enforcement at every memory access, atomic map/unmap updates relative to execution, and invalidation before unmap returns (RFC-0009.3–.7 and .12). It intentionally does not specify the hardware MMU format or physical page-table structure.

### RFC-0013 — Capability mediation

The current RFC requires resource authority and rights to be checked at the earliest non-bypassable boundary, with address resolution distinct from authorization (RFC-0013.1–.5 and .10). The reference model keeps the SCP handle/authority check separate from PTE translation and page permissions.

### RFC-0037 and INTF-000 — Capability representation

RFC-0037's canonical serialized rights mask is not the live host provider's Rights bit layout. The SCP decision reserves canonical records for mediated import/export and uses compact local handles in the hot path. The host model therefore makes the distinction explicit:

- Rights represents provider-side capability authority labels, including MAP and EXECUTE.
- PtePermissions represents only hardware leaf-page R/W/X permissions.
- Conversion from a mapping request to the required capability-right set is explicit and includes MAP plus each requested access right.

Neither Rust type is a byte-for-byte representation of the RFC-0037 canonical token.

### RFC-0039 — Revocation

The indexed capability lineage graph propagates revocation through derivation and cross-domain transfer. The memory manager finds mappings whose authority is no longer valid, removes their PTE model entries, publishes the updates, and requires invalidation on the executing and known resident harts before completion is reported.

## 3. Hardware research — RISC-V Privileged Architecture

The official RISC-V Privileged Architecture specification identifies Sv39 as a 39-bit virtual-address scheme. A 64-bit effective address must have bits 63–39 equal to bit 38. Sv39 uses a three-level page table and supports 4 KiB base pages, 2 MiB megapages and 1 GiB gigapages; superpage virtual and physical addresses must be aligned to their page-size boundaries.

The RISC-V PTE R/W/X bits independently describe readable, writable and executable pages. A leaf PTE with R=0 and W=1 is reserved and faults. The reference type therefore rejects a write-only PTE permission set and requires READ together with WRITE.

SFENCE.VMA orders the relevant page-table accesses and invalidates address-translation cache entries on the local hart only. The privileged specification describes a remote shootdown sequence in which page-table writes are made globally visible, other harts are notified, each remote hart executes its local SFENCE.VMA, and the initiating hart is notified when completion has occurred.

### SBI RFENCE is not itself a completion acknowledgement

The current SBI v3.0 RFENCE specification defines range-targeted remote SFENCE.VMA and ASID-targeted remote SFENCE.VMA calls. Its documented SBI_SUCCESS result says the IPI was sent successfully to the targeted harts. That result alone is not proof that every target hart has executed the fence and acknowledged completion.

The host backend trait is deliberately stronger: invalidate_hart_and_wait must not return success until the target hart has executed the matching local fence and acknowledged it. A real implementation must add a completion protocol over and above merely observing that a remote-fence request was dispatched.

## 4. Implemented host-reference model

### 4.1 Address-space identity and ranges

- A domain is registered to one unique host-model ASID; the registration is immutable within the manager.
- The model validates Sv39 canonical addresses, including both low and high canonical regions.
- Virtual and resource ranges are non-empty, half-open ranges with equal lengths.
- Both ranges must be aligned to the explicitly requested page size. The model never rounds a mapping outward to a hardware page boundary.

### 4.2 Capability-authorized mapping

Before any PTE is installed, map checks:

1. The domain is registered to the requested ASID.
2. The virtual and resource ranges are valid, canonical, aligned and equal in size.
3. The requested PTE permission set is supported and obeys the RISC-V W-implies-R rule.
4. The capability resolves to a live Memory object and grants MAP plus every requested READ/WRITE/EXECUTE permission over the exact resource range.
5. The virtual range does not overlap any existing or pending mapping.

The complete PTE set and mapping authority record are prepared before installation. A new mapping is not returned to the caller until page-table updates have been published and the executing hart plus every known resident hart has completed translation invalidation. If activation fails, the installed PTEs are rolled back; an incomplete rollback leaves the mapping/range quarantined and returns ShootdownPending.

### 4.3 Per-access enforcement

The access operation first requires that the requested address space is the one successfully activated on the supplied hart for dispatch. It then resolves the address through the host page-table model or per-hart TLB cache, recovers the mapping's recorded domain and capability handle, computes the exact one-byte resource range, and revalidates that live handle for `MAP` plus the operation-specific `READ`, `WRITE`, or `EXECUTE` right. Only after that current authority check succeeds does the model enforce the PTE's independent R/W/X permission.

This deliberately keeps two checks separate:

- The capability provider proves that the mapping authority is still live, belongs to the mapping's domain, names the mapped Memory object, and covers the exact resource byte with the required rights.
- The PTE model proves that the mapped page itself allows the requested hardware access type.

A cached PTE is therefore not treated as sufficient authority by itself. The regression test revokes the live capability directly while leaving the simulated TLB entry cached; the next access must fail with `CapError::Revoked`. This is host-reference enforcement at the trusted access boundary, not proof that a real CPU consults AetherOS capability tables. A production provider must implement an unbypassable mediation mechanism while preserving the same authority/PTE separation.

The current host `Capability` record does not yet model token expiry or usage counters, so RFC-0013.4's lifetime and usage-bound criteria remain open. Concurrent access/revocation linearization is also not implemented; this access revalidation closes the sequential stale-authority path only.

### 4.4 Serialized host runtime

The development branch adds a MemoryRuntime façade that privately owns both the CapabilitySystem and MemoryManager behind one mutex. It exposes the supported capability mutations (root creation, derive, transfer, remove, revoke) and memory operations (map, unmap, dispatch activation, access, invalidation retry) through that shared lock.

The shared lock gives these host façade calls one total order. An access either completes before a concurrent revocation, or observes the revocation/mapping invalidation and fails. The new unit test races an access against subtree revocation and then asserts that no access can succeed after revocation returns. A machine-readable CTS case exercises the serialized runtime path as well.

This is deliberately not a production-kernel locking design. The mutex is blocking and remains held while TlbShootdown callbacks publish PTE updates and wait for target-hart acknowledgements. Production code must not hold an ordinary blocking lock across a potentially waiting dispatch/fence path; it needs an interrupt-safe synchronization and completion protocol with explicitly proved lock ordering and latency. The lower-level MemoryManager and CapabilitySystem remain available for focused tests, but callers bypassing MemoryRuntime do not receive its serialization guarantee.

### 4.5 Dispatch activation and residency

The host model exposes `activate_for_dispatch(domain, hart, fences)` as the address-space activation boundary associated with RFC-0008.4 and RFC-0008.7. The caller must not allow the target thread to execute until the activation backend reports success. The model rejects simulated access before activation and after a switch away from that address space.

A hart is recorded as potentially resident **before** the backend activation call, not after the first memory access. This is intentionally conservative: an activation attempt that fails may have partially changed hardware translation state. A failed attempt therefore removes the active designation and quarantines the hart until a later activation succeeds. Residency is retained across context switches and migrations; that may cause extra shootdowns, but avoids incorrectly assuming that a hart has discarded ASID-tagged translations merely because it stopped running a domain.

This is still a host contract and fake-backend test. No scheduler currently invokes it, no cross-thread/interrupt serialization makes dispatch atomic with map/unmap/revocation, and no real `satp` or page-table-root activation exists. RFC-0008's full dispatch protocol and RFC-0009.5 concurrent map/dispatch atomicity are not implemented by this host model.

### 4.6 Unmap and revocation

- Explicit unmap removes the PTEs, publishes the change, and waits for local and known resident-hart invalidations before reporting completion.
- Removing a capability handle invalidates mappings created directly from that handle; it does not revoke derived capabilities.
- Revoking a capability subtree also invalidates mappings whose backing authority fails afterwards, including mappings made in destination domains through transfer.
- When an invalidation is incomplete, the changed range remains blocked and pending target harts are quarantined. retry_pending_shootdowns resumes the pending protocol; access is not re-enabled merely because an IPI was sent.

## 5. Page-granularity and least-authority consequence

The host capability model permits arbitrary byte bounds, but Sv39 page permissions apply at leaf-page granularity. A software capability covering only part of a 4 KiB page cannot safely be converted into a normal page mapping if doing so exposes bytes outside its bounds.

This prototype chooses fail-closed alignment: a map request's resource bounds must cover complete pages, and must be contained by the capability. It rejects misaligned requests rather than silently rounding to a wider page. Future designs may add guarded sub-page regions, bounce buffers, memory coloring, or a separately proven software bounds-checking mechanism; none is implemented or implied here.

## 6. Validation and evidence

PR #8 was merged into `main` as `ac55bb3b6e41da75859a5f2e1ca26436e294d0d3`. The exact PR head passed the full workflow before merge; the resulting main merge commit also passed its push workflow on 2026-11-12:

- Repository validators and supply-chain policy for five packages.
- Debug and release Rust test suites: `aether-cap-model` 4, `aether-capspace` 28, `aether-state-model` 9 and `aether-types` 2 tests passed.
- All 15 machine-readable host CTS cases passed, including `CTS-CAP-REVOKE-SUBTREE-001`, `CTS-EXEC-MAP-REVOKE-001` and `CTS-EXEC-DISP-ADDRSPACE-001`.
- Formatting, compile/check, Clippy with warnings denied, Rustdoc with warnings denied, reproducibility verification and release-evidence validation (133 files).
- GitHub Actions evidence artifact retained.

PR-head workflow: https://github.com/shreyashjagtap157/AetherOS/actions/runs/43245270041

PR-head artifact: https://github.com/shreyashjagtap157/AetherOS/actions/artifacts/13956687961

Merged-main workflow: https://github.com/shreyashjagtap157/AetherOS/actions/runs/43245580188

Merged-main artifact: https://github.com/shreyashjagtap157/AetherOS/actions/artifacts/13956776653

Merged-main artifact SHA-256: `f48aa6aa2a30625e35ad2fc23af307f7c6059944dd0e41326a594ff61aed763a`

These results are host-reference evidence only. They do not prove kernel conformance, hardware page-table correctness, actual scheduler integration, concurrent map/dispatch linearization, target-hart discovery, interrupt delivery, memory ordering, worst-case shootdown latency, or formal correctness.

The live-authority access guard and serialized MemoryRuntime described above are subsequent development-branch additions. They add one unit regression for a revoked capability with a cached translation and two machine-readable CTS cases, bringing that branch to 17 cases. Their latest-head CI validation is pending; the earlier 15-case main/PR validation above does not cover them.

## 7. Explicit exclusions and unresolved engineering work

1. Scheduler/hart membership: the host model now records a hart as potentially resident at dispatch activation, before its first memory access, and retains residency across switches/migrations. A production kernel still needs a scheduler-integrated registry and serialization that prevents activation/deactivation races with map/unmap/revocation.
2. Concurrency and linearization: the lower-level manager remains single-threaded. The new host MemoryRuntime façade serializes its supported operations with one blocking mutex, but this is not a production-kernel implementation. The production design must make map, unmap, revoke, derive, transfer and dispatch atomic in accordance with RFC-0009.5, define synchronization/linearization points, and prove interrupt-safe lock ordering and bounded latency.
3. Remote completion protocol: a real backend must distinguish request dispatch from remote local-fence completion and must never release quarantined work before every required acknowledgement arrives.
4. Global translations: the host model does not model the PTE G bit or global mappings. Production domain mappings must either prohibit global PTEs or define the broader invalidation semantics required for them.
5. ASID lifecycle: this model assigns ASIDs manually and does not model hardware ASID width discovery, reuse, generation tagging, rollover or satp transitions.
6. Physical memory: object offsets stand in for backing-resource offsets. The model does not allocate physical frames, validate physical-address width, program real PTEs, account for PMP/PMA, configure an IOMMU, or model device DMA.
7. PTE A/D behavior: Accessed/Dirty bits, hardware-managed versus software-managed A/D updates, atomic PTE writes, page-table page allocation, and walk-cache behavior remain unspecified by this prototype.
8. Other mapping modes: copy-on-write, shared mappings, device/MMIO mappings, caching attributes, NUMA placement, guard pages, superpage split/merge, and memory hot-remove are not implemented.
9. Data effects: simulated access returns an object and offset; it does not read or write an actual backing byte array.
10. Bounded resources and proof: the model uses dynamically allocated host data structures and makes no worst-case execution-time or memory bound claim.

## 8. Sources

- AetherOS RFC-0009 — Address Space Model: https://github.com/shreyashjagtap157/AetherOS/blob/main/docs/RFCs/01-Execution/RFC-0009.md
- AetherOS RFC-0013 — Capability Mediation: https://github.com/shreyashjagtap157/AetherOS/blob/main/docs/RFCs/01-Execution/RFC-0013.md
- AetherOS RFC-0037 — Capability Token Format: https://github.com/shreyashjagtap157/AetherOS/blob/main/docs/RFCs/05-Cross-Cutting/RFC-0037.md
- AetherOS RFC-0039 — Capability Revocation Protocol: https://github.com/shreyashjagtap157/AetherOS/blob/main/docs/RFCs/05-Cross-Cutting/RFC-0039.md
- RISC-V Privileged Architecture, official release 2026-01-20, Sections 12.2–12.4: https://docs.riscv.org/reference/isa/_attachments/riscv-privileged.pdf
- RISC-V Supervisor Binary Interface v3.0, RFENCE extension: https://docs.riscv.org/reference/sbi/v3.0/ext-rfence.html

The RISC-V specifications are primary hardware-interface sources. The conclusions and implementation choices in this document are a host-reference interpretation of those interfaces, not normative architecture amendments.
