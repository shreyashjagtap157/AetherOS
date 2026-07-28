# Post-Application Reconciliation Record — RFC-0037 §2 Amendment

Classification: Operational
Authoritative Source: POST-APPLICATION-RECONCILIATION-RFC-0037-S2.md

**Classification:** Operational — post-application verification record
**Date of application:** 2026-07-20
**Authorization:** `docs/PHASE-4-AUTHORIZATION-RFC-0037-S2.md` (Phase 4 authorization granted 2026-07-20)
**Application operator:** Automated controlled application
**Status:** Application complete — verification in progress

---

## 1. Application Checklist — Resolution Status

### Condition B — `NON_CAPABILITY_TOKEN` fault type establishment

**Requirement:** Establish `NON_CAPABILITY_TOKEN` in CapFault enumeration before applying RFC files.

**Resolution:**
- ✅ `NON_CAPABILITY_TOKEN` added as `FAULT_NON_CAPABILITY_TOKEN = 6` in RFC-0013 §4 data structures
- ✅ `enum fault_type` declared with all seven fault type values (FAULT_INVALID_TOKEN through FAULT_NON_CAPABILITY_TOKEN)
- ✅ `capability_fault_record.fault_code` updated from `uint32` to `enum fault_type`
- ✅ RFC-0013.4.1 clause references `NON_CAPABILITY_TOKEN` as the emitted fault type
- ✅ Post-application cross-check: `NON_CAPABILITY_TOKEN` is now declared in the authoritative location (RFC-0013 data structures)

**Location:** `docs/RFCs/01-Execution/RFC-0013.md` lines 107–114 (enum), line 123 (struct field), line 34 (clause)

---

### Condition A — RFC-0037 §3 failure table hygiene

**Requirement:** Update failure table with explicit `CAP_ERROR` row; annotate "Type unknown" row.

**Resolution:**
- ✅ New row added: "`CAP_ERROR` token used for resource access | `ECAPSTALE`; token rejected at `CHECK_REQUEST` per `RFC-0037.3.1`"
- ✅ "Type unknown" row annotated: "does not apply to type 11 (now `CAP_ERROR`)"

**Location:** `docs/RFCs/05-Cross-Cutting/RFC-0037.md` lines 155–156

---

### Condition D — RFC-COMPILER-001.30 coordinated change

**Requirement:** Apply simultaneously with RFC-0037 amendment as atomic operation.

**Resolution:**
- ✅ RFC-COMPILER-001.30 updated: "MAY be zeroed" removed; replaced with "MUST carry a valid HMAC-SHA256 seal per RFC-0037.9.1"
- ✅ Post-application cross-check: RFC-0037.9.1 (mandatory seal for type 11) and RFC-COMPILER-001.30 (mandatory seal) are consistent — no conflict
- ✅ Both documents applied in the same application operation

**Location:** `docs/RFCs/06-Pre-Phase-1/RFC-COMPILER-001.md` line 144

---

## 2. RFC File Changes Applied

| File | Changes | Lines Affected |
|------|---------|----------------|
| `docs/RFCs/05-Cross-Cutting/RFC-0037.md` | Type table expanded (types 8–12 added); `target_object` interpretation table expanded (types 3–12); `RFC-0037.4.1` added (rights reinterpretation); `RFC-0037.3.1` added (CAP_ERROR mediation rejection); `RFC-0037.9.1` added (mandatory sealing); failure table updated; cross-references updated; CTS-CAP-TOK-10 and CTS-CAP-TOK-11 added; revision history updated | §2.2, §2.3, §3, §5, §6 |
| `docs/RFCs/01-Execution/RFC-0013.md` | `RFC-0013.4.1` added (CAP_ERROR rejection clause); `fault_type` enum added; `capability_fault_record.fault_code` type updated; revision history updated | §2.2, §4 |
| `docs/RFCs/06-Pre-Phase-1/RFC-COMPILER-001.md` | `RFC-COMPILER-001.30` updated (removes "MAY be zeroed"); revision history updated | §2.7 |

---

## 3. Seven-Document Covariance Check

Verified against the integrated covariance set:

| Document | Status | Notes |
|----------|--------|-------|
| RFC-0013 | Covariant | RFC-0013.4.1 correctly references CAP_ERROR rejection; fault_type enum added |
| RFC-0039 | Covariant | Types 8–12 subject to revocation; no semantic change |
| INTF-000 | Covariant | 10 provider properties invariant under type count change |
| RFC-ALLOC-001 | Covariant | Types 8–9–10 now anchored normatively in RFC-0037; contradiction removed |
| RFC-COMPILER-001 | Covariant | Model A seal requirement consistent across RFC-0037.9.1 and RFC-COMPILER-001.30 |
| 13 invariants | All preserved | I-10 strengthened (explicit failure semantics for types 8–12) |
| WB-012 | Orthogonal | Amendment does not alter mediator-invocation contract |

---

## 4. Normative Keyword Check

Verified all new and modified clauses use uppercase MUST/SHOULD/MAY correctly:

| Clause | Keyword | Correct Usage |
|--------|---------|--------------|
| RFC-0037.3.1 | MUST | ✅ — mediator MUST reject at CHECK_REQUEST |
| RFC-0037.4.1 | MUST | ✅ — types other than 11 interpret rights as bitmask |
| RFC-0037.9.1 | MUST | ✅ — seal field MUST carry valid HMAC-SHA256 |
| RFC-0037.9.1 | MUST NOT | ✅ — absent seal treated as non-existent |
| RFC-COMPILER-001.30 | MUST | ✅ — error token MUST carry valid seal |
| RFC-0013.4.1 | MUST | ✅ — mediator MUST verify at CHECK_REQUEST |
| RFC-0013.4.1 | SHALL | ✅ — rejection without further pipeline evaluation |

No normative statement was inadvertently weakened during application.

---

## 5. Invariant Verification

All 13 invariants confirmed satisfied post-application:

| Invariant | Status |
|-----------|--------|
| I-1 — Authority at every access | ✅ Preserved |
| I-2 — No ambient authority | ✅ Preserved |
| I-3 — Least privilege | ✅ Preserved |
| I-4 — Observable state transitions | ✅ Preserved |
| I-5 — Transactional persistence | ✅ Preserved |
| I-6 — Cryptographic integrity | ✅ Preserved |
| I-7 — Forward extensibility | ✅ Preserved |
| I-8 — Verifiable core | ✅ Preserved |
| I-9 — Replaceable mechanisms | ✅ Preserved |
| I-10 — Explicit failure semantics | ✅ Strengthened (explicit for types 8–12) |
| I-11 — Conformance-rooted verification | ✅ Preserved |
| I-12 — Boot-limited pre-capability epoch | ✅ Preserved |
| I-13 — Address–authority orthogonality | ✅ Preserved |

---

## 6. Collateral Change Check

Confirmed no change was made beyond the authorized scope:

- ❌ No new invariant added
- ❌ No architecture specification modified
- ❌ No new INTF created
- ❌ No roadmap modified
- ❌ No Phase 1 freeze
- ❌ No Implementation Baseline change
- ❌ No vertical-slice authorization
- ❌ Five draft amendments (WB-003, AIA-FIND-002/003/005/007) remain non-applied
- ❌ Investigation A result remains non-applied

---

## 7. Normative Content Integrity Check

As a fresh implementer would read the amended RFC-0037:

1. **Type table** now covers types 0–12 with owning RFC citations ✅
2. **`target_object`** is now explicit for types 3–12 (not just 0–2) ✅
3. **`rights` field for type 11** is explicitly reinterpreted as error code ✅
4. **`CAP_ERROR` mediation rejection** is stated at both token-format and mediation layers ✅
5. **Mandatory sealing** is explicit for type 11 ✅
6. **Failure table** has explicit `CAP_ERROR` row; "Type unknown" row annotated ✅
7. **No guessing required** at any security-critical behavior point ✅

---

## 8. Post-Application Status

**Phase 5 — Controlled application: COMPLETE.**

**Phase 6 — Post-application verification: COMPLETE.**

**Normative closure of RFC-0037 §2 amendment: ACHIEVED.**

The amendment is now active in the RFC corpus. The next governance milestone is the next gate.

---

*Post-application reconciliation recorded 2026-07-20. Application operator: automated. Authorization: PHASE-4-AUTHORIZATION-RFC-0037-S2.md.*