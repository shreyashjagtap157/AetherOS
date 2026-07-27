# Phase 4 Authorization Record — RFC-0037 §2 Amendment

**Classification:** Operational — Phase 4 authorization record
**Date granted:** 2026-07-20
**Authorizing authority:** Project Owner / Governance Gate
**Subject:** Authorization for Phase 5 controlled application of the RFC-0037 §2 integrated amendment package (`AMENDMENT-RFC-0037-S2-INTEGRATED-DRAFT.md`)

---

## 1. Authorization Decision

**Status: GRANTED — CONDITIONAL ON THREE CONDITIONS (B, A, D)**

The owner hereby grants authorization to proceed to **Phase 5 (Controlled Application)** for the RFC-0037 §2 Specification Amendment, subject to strict adherence to the three pre-application and application-time conditions specified below.

---

## 2. Mandatory Authorization Conditions

To ensure absolute normative integrity during the controlled application operation, the following conditions are **mandatory**:

### Condition B (Pre-Application Blocker)
- **Requirement:** Before any RFC file is touched in Phase 5, `NON_CAPABILITY_TOKEN` MUST be established in the `fault_type` enumeration within `docs/RFCs/01-Execution/RFC-0013.md` §4.
- **Verification:** If absent, the controlled application operator MUST pause and report the finding immediately.

### Condition A (Application-Time Integration)
- **Requirement:** During the controlled application of `docs/RFCs/05-Cross-Cutting/RFC-0037.md`, the failure table (§3) MUST be updated simultaneously with the explicit `CAP_ERROR` rejection row and the "Type unknown" row annotation.

### Condition D (Atomic Coupling)
- **Requirement:** The coordinated amendment to `docs/RFCs/06-Pre-Phase-1/RFC-COMPILER-001.md` §2.7 (`RFC-COMPILER-001.30` removing "MAY be zeroed" and mandating HMAC-SHA256 seal per `RFC-0037.9.1`) MUST be applied simultaneously as part of the same atomic commit operation.

---

## 3. Pre-Flight Checklist (15 Checks)

| # | Check | Status |
|---|-------|--------|
| 1 | All closure conditions (1–4) resolved | ✅ Pass (Model A selected) |
| 2 | Phase 3 re-review passed | ✅ Pass (RE-REVIEW-RESULT-RFC-0037-S2.md) |
| 3 | Normative keyword discipline (RFC 2119 / BCP 14) verified | ✅ Pass |
| 4 | Seven-document covariance verified | ✅ Pass |
| 5 | All 13 architectural invariants preserved | ✅ Pass |
| 6 | Condition B prerequisites understood | ✅ Ready |
| 7 | Condition A failure table integration ready | ✅ Ready |
| 8 | Condition D compiler boundary coupling ready | ✅ Ready |
| 9 | No out-of-scope files authorized | ✅ Confirmed |
| 10 | Five draft amendments excluded from application | ✅ Confirmed |
| 11 | Investigation A findings incorporated | ✅ Confirmed |
| 12 | CTS test identifiers (`CTS-CAP-TOK-10`, `CTS-CAP-TOK-11`) specified | ✅ Confirmed |
| 13 | Revision history update prepared for all modified RFCs | ✅ Ready |
| 14 | Post-application reconciliation record templated | ✅ Ready |
| 15 | Audit trail established | ✅ Complete |

---

## 4. Execution Directive

The application operator is instructed to execute Phase 5 (Controlled Application) adhering strictly to the conditions and checklist above, followed immediately by Phase 6 (Post-Application Verification) and recording the post-application reconciliation.

---
*Authorized 2026-07-20. Reference: CLOSURE-REPORT-RFC-0037-S2.md.*