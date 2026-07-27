# Cross-Impact Gate 001 Decision Record

**Classification:** Governance — cross-impact gate decision
**Gate ID:** CROSS-IMPACT-GATE-001
**Subject:** RFC-0037 §2 Amendment & Model A Provenance Selection
**Date established:** 2026-07-19
**Status:** Active / Phase 5 Application Complete

---

## 1. Gate Purpose

Cross-Impact Gate 001 governs the cross-cutting architectural impact of adding types 8–12 to RFC-0037 (Capability Token Format) and ensuring full covariance across the 7-document core suite (RFC-0013, RFC-0037, RFC-0039, INTF-000, RFC-ALLOC-001, RFC-COMPILER-001, RFC-DRIVER-001).

---

## 2. Core Decisions Recorded

### Decision 1: Type Expansion & Target Object Mapping
- **Status:** Approved & Applied (2026-07-20)
- **Summary:** Types 8–12 (`CAP_DMA_MEMORY`, `CAP_IOMMU_CONFIG`, `CAP_MMIO_REGION`, `CAP_ERROR`, `CAP_DEVICE_CONTROL`) added to RFC-0037 §2.2 type table with owning RFC citations. Explicit `target_object` interpretation table added for types 0–12.

### Decision 2: CAP_ERROR Provenance — Model A Selection
- **Status:** Approved & Applied (2026-07-19 / 2026-07-20)
- **Summary:** Owner selected Model A (mandatory HMAC-SHA256 seal for `CAP_ERROR` tokens per `RFC-0037.9.1`). Coordinated change applied to `RFC-COMPILER-001.30` removing the provisional "MAY be zeroed" permission.

### Decision 3: Mediation Rejection & Fault Integration
- **Status:** Approved & Applied (2026-07-20)
- **Summary:** `RFC-0037.3.1` and `RFC-0013.4.1` integrated, establishing mandatory `CHECK_REQUEST` rejection of `CAP_ERROR` when presented for resource access, emitting `ECAPSTALE` and `FAULT_NON_CAPABILITY_TOKEN = 6`.

---

## 3. Revision History

| Date | Change |
|------|--------|
| 2026-07-19 | Gate established. Evaluated Models A, B, C for `CAP_ERROR` provenance. Selected Model A. |
| 2026-07-20 | Phase 4 authorization recorded (`PHASE-4-AUTHORIZATION-RFC-0037-S2.md`). Phase 5 controlled application completed. Gate status updated to Active / Applied. |

---
*Cross-Impact Gate 001 Decision Record. AetherOS Governance.*