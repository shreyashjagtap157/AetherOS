# Revision History — Platform Architecture Suite

Classification: Informative
Authoritative Source: Git history

This document records the revision history of `Platform-Architecture-Specification-v1.1.md` and the surrounding suite. Future changes to RFCs are tracked inside each RFC's own revision history section.

---

## Suite-level history

| Date                | Change                                                                                   |
|---------------------|------------------------------------------------------------------------------------------|
| (this cycle)        | Initial release: Platform Architecture v1.1 + 41 RFCs + governance + profiles + CTS methodology + glossary |

---

## Platform Architecture Specification

| Version | Date       | Material changes                                                                                                                                                                                                                                                                                                                                                                                                            |
|---------|------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1.0     | this cycle | Initial frozen architecture. Established 11 invariants (I-1 through I-11), five-peer decomposition, capability model, verification tiers, scheduling three-layer model, persistence–state boundary, distributed boundary, hardware universality.                                                                                                                                                                          |
| 1.1     | this cycle | Added **I-12 (origin of authority)** and **I-13 (address–authority orthogonality)** in response to critique that the architecture lacked both a bootstrap story and a rigorous separation of authority from addressing. Added Section 5 (Origin of Authority and Bootstrap Model) declaring the five-stage bootstrap (Firmware Root of Trust → Minimum Executor → Capability Root → Composition Root → User Components). Added Section 6.1 (Economic cost of verification) declaring the verification-tier cost commitment as architectural. Revised Section 4 with explicit authority–address orthogonality. Revised Section 7 to quantitative metrics (operation count, distinct state count, dependency-graph diameter, proof-fragility class, concept set size). Refined Section 3.5 with explicit Communication–Execution layering dependency. Added Section 7.1 (recommended failure-mode coverage budget metric). |

---

## Why v1.1 stopped here

The architectural phase is complete once edits cease to discover invariants and start to rediscover them. The v1.0 → v1.1 transition added **two normative invariants** plus **three substantive specification sections** in direct response to peer-review critique. Every added item either resolves a decision that subsequent RFCs cannot make on their own, or formalizes a property that was implicit and at risk of being lost.

After v1.1, no further architectural revision is expected. Future changes will be RFC-level and confined to specific subsystems.

---

## RFC-level revisions

Once an RFC is published, its own revision history is recorded in a `## Revision History` block at its end. No central index is maintained.
