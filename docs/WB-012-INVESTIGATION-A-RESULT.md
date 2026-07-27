# WB-012 Investigation A Result — Mediation Invocation & CAP_ERROR Provenance

**Classification:** Investigation Result — Workstream WB-012
**Investigation:** Investigation A (Mediation Invocation & Provenance)
**Date:** 2026-07-19
**Status:** Complete

---

## 1. Executive Summary

This investigation analyzed 17 mediation call sites across the architecture and evaluated three provenance models (Model A: mandatory seal, Model B: dynamic context trust, Model C: consumer-constraint restricted) for error tokens (`CAP_ERROR`, type 11). 

**Recommendation:** **Model A (mandatory HMAC-SHA256 seal)** is strongly recommended and has been selected by owner disposition. It integrates natively with the architecture's existing cryptographic root of trust without introducing new trust assumptions or runtime overhead.

---

## 2. Mediation Call Site Analysis

The 17 mediation call sites evaluated across execution, memory, communication, and driver subsystems confirm that `CHECK_REQUEST` (`RFC-0013` §3) is the universal gateway. Ensuring `CAP_ERROR` is intercepted at `CHECK_REQUEST` prevents any error token from being misrouted as a resource-access capability.

---

## 3. Provenance Model Evaluation

- **Model A (Mandatory Seal):** Reuses the 16-byte HMAC-SHA256 seal (`RFC-0037.8`). Guarantees integrity and attestation of the error code in the `rights` field. **Selected.**
- **Model B (Dynamic Context):** Requires new sideband state tracking. Rejected due to lack of foundational mechanism.
- **Model C (Consumer Constraints):** Relies on layered restrictions. Rejected as unnecessarily complex.

---

## 4. What Investigation A Does NOT Require

- Does not require a new hardware instruction.
- Does not require altering the 96-byte capability token format.
- Does not require modifying the 13 architectural invariants.

---
*WB-012 Investigation A Result. AetherOS Technical Workstream.*