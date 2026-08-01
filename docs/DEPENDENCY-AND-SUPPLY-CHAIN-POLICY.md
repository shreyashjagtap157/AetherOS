# Dependency and Supply-Chain Policy

Classification: Operational
Authoritative Source: DEPENDENCY-AND-SUPPLY-CHAIN-POLICY.md
Requirement-ID: GOV-SUPPLY-001
Status: Active Prototype Policy

## Rules

1. Target and reference builds use lockfiles and fail rather than update them implicitly.
2. Git and path-by-URL dependencies are prohibited. Registry dependencies require an explicit review of purpose, maintenance, license, transitive graph, unsafe code, build scripts, advisories, and replacement cost.
3. New privileged code prefers audited in-tree implementations only when maintaining them is safer than a mature dependency; “no dependencies” is not itself a security property.
4. Release inputs include source revision, pinned toolchain, dependency locks, container/base-image digest, build commands, SBOM, hashes, tests, and provenance.
5. CI actions and container bases use immutable digests. Network-fetched tools are not installed during a release build unless their hash and provenance are already locked.
6. Advisory databases are time-varying evidence. A release records the database identity and scan time; an offline source/license gate does not claim vulnerability absence.
7. AI-produced dependencies, code, or reports receive the same independent review as human contributions.

## Current enforcement and limitations

`tools/verify-supply-chain.py` checks Cargo metadata offline, rejects undeclared sources, and enforces the repository's dual-license expression. The current Rust workspace has no third-party packages. `tools/generate-release-evidence.py` emits a deterministic evidence manifest and SPDX 2.3 package SBOM. These are foundation controls, not SLSA certification or a complete vulnerability-management programme.

Before `0.1.0`, add a mirrored, hash-pinned advisory database; signed build provenance; release signing in an isolated environment; dependency diff review; base-image rebuild automation; and verified artifact retention.
