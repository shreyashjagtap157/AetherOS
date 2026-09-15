# AetherOS Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| v1.x    | ✅ Active development |
| v0.x    | ❌ Not supported   |

## Reporting a Vulnerability

**Do not report security vulnerabilities via public GitHub issues.**

Instead, report them via:

1. **GitHub Security Advisories** (preferred): Use the "Report a vulnerability" button on the Security tab
2. **Email**: security@aetheros.example.com (PGP key available on request)

### What to Include

- Description of the vulnerability
- Steps to reproduce or proof-of-concept
- Affected components (RFCs, crates, invariants)
- Potential impact assessment
- Suggested mitigation (if any)

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 7 days
- **Fix development**: Priority based on severity and exploitability
- **Disclosure coordination**: Per standard responsible disclosure practices

## Security Model

AetherOS is a **capability-only authority** architecture with these security properties:

- **No ambient authority** (I-2): Every operation requires explicit capability
- **Least privilege by default** (I-3): Capabilities grant minimum rights
- **Address–authority orthogonality** (I-13): Capability ≠ address
- **Observable state transitions** (I-4): All mutations audited before commit
- **Cryptographic integrity** (I-6): Content-addressed object identity
- **Verifiable core** (I-8): Tier A components admit machine-checked proofs

## Threat Model

See `docs/THREAT-MODEL.md` for the complete threat model covering:
- Capability forgery and confused deputy attacks
- Bootstrap authority compromise
- Cross-domain capability transfer
- Revocation bypass
- State rollback and replay
- Supply chain integrity

## Cryptographic Standards

- **Checksums**: Non-cryptographic (FNV-1a) in reference only; production uses BLAKE3
- **Audit commitments**: SHA-256 in reference; production uses BLAKE3 or SHA-3
- **Capability sealing**: Production uses Ed25519 or equivalent (RFC-0037)
- **Key management**: Hardware-backed where available; HSM for root keys

## Supply Chain Security

- All dependencies pinned via `Cargo.lock`
- `tools/verify-supply-chain.py` enforces license and source validation
- No network access during build
- Reproducible builds via pinned toolchain (`rust-toolchain.toml`)

## Verification Tiers

Per `RFC-0040`:
- **Tier A**: Machine-checked formal proofs (kernel paths, capability mediation)
- **Tier B**: Model-checked traces (lifecycle, transactions)
- **Tier C**: Property-based testing (invariants under random operations)
- **Tier D**: Standard test coverage

Security-critical paths target Tier A.

## Incident Response

1. **Contain**: Disable affected capability/provider paths
2. **Assess**: Determine blast radius via authority provenance
3. **Mitigate**: Deploy targeted revocation or profile restriction
4. **Recover**: Rebuild from verified state + audit log
5. **Learn**: File Observation Record, amend invariants if needed

## Contacts

- **Security Team**: @security-team (CODEOWNERS)
- **Architecture Team**: @architecture-team
- **Project Maintainer**: @shreyashjagtap157

---

*This policy applies to the AetherOS specification suite and reference implementation. Production deployments inherit this baseline plus deployment-specific controls.*