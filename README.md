# AetherOS

Classification: Informative

AetherOS is the implementation and specification project for one coherent, general-purpose operating system spanning desktop, laptop, tablet, foldable and mobile profiles. The initial qualification target is RISC-V64 on QEMU `virt`; that is the first reference platform, not the limit of the long-term hardware strategy.

## Current verified repository state — 23 November 2026

**Canonical branch:** `main`  
**Inspected HEAD:** `91478e592be1077bd79a0996f4207faf53d9fdc8`

- **QEMU boot qualification:** succeeded on this exact commit in [RISC-V64 Boot Qualification run 131](https://github.com/shreyashjagtap157/AetherOS/actions/runs/45012374998).
- **Reference/documentation validation:** historical main [run 184](https://github.com/shreyashjagtap157/AetherOS/actions/runs/45012375003) failed at the early documentation-authority gate because the historical v3 goal lacked its required `Classification:` header. PR #12 fixes it. The latest PR-head [reference validation run](https://github.com/shreyashjagtap157/AetherOS/actions/runs/45231381272) passed 104 document/authority checks, 59 glossary terms, 42 RFC structural checks, 179 Rust tests and all 38 CTS cases; evidence artifact 14847087634 was uploaded.
- The latest commit summary records VirtIO block/network driver work, TCP data transfer and protected process execution. Commit summaries are not acceptance evidence for every behavior; inspect implementation and its end-to-end tests.
- The corrected PR-head [QEMU boot run 133](https://github.com/shreyashjagtap157/AetherOS/actions/runs/45231380667) validated the ELF, observed the expected `hello` boot marker and captured exit status `124` as the expected 60-second timeout. The validator accepted this bounded-run result and rejects other unexpected exit codes; artifact 14848115771 was uploaded. This is not clean-shutdown, long-duration stability or full-OS qualification.
- The successful QEMU boot is a real milestone. It does **not** prove the first usable desktop release, live-media safety, installation, physical-board support, complete networking, the entire driver ecosystem or ultimate platform completion.

The active product mandate is **AetherOS Complete Platform Goal v4.0**. The first usable RISC-V64 release is an intermediate milestone; the actual project goal is the fully integrated platform and all required profile, service, runtime, security, user-experience and ecosystem capabilities. Unimplemented requirements must remain visible and Open.

## Project goals and authoritative documentation

Start with [the project index](docs/project/README.md), then read:

1. [Complete Platform Goal v4.0](docs/project/AetherOS_Complete_Platform_Goal_v4_2026-11-21.md) — active user goal and execution mandate.
2. [Reconstructed user requirements](docs/requirements/USER_REQUIREMENTS_BASELINE.md) — complete traceable product baseline.
3. [Source reconciliation](docs/requirements/SOURCE_RECONCILIATION.md) — provenance and authority rules.
4. [Platform Architecture Specification v1.1](docs/Platform-Architecture-Specification-v1.1.md) — current constitutional architecture baseline; Validation Candidate, not yet frozen.
5. [Architecture Governance](docs/ARCHITECTURE-GOVERNANCE.md), [RFC Index](docs/00-RFC-Index.md), applicable RFCs and accepted ADRs.
6. [Implementation Roadmap](docs/IMPLEMENTATION-ROADMAP.md) and [Implementation Status](docs/IMPLEMENTATION-STATUS.md) — always reconcile against live source and exact CI commit.

The earlier v3 goal remains available as an informative historical checkpoint. The complete v1.0 convergence source is preserved under `docs/archive/architecture/`; neither historical source overrides the current v1.1 constitutional architecture, governance, accepted ADRs or active RFC state.

## Repository map

- `docs/project/` — active product goal, navigation and architecture/source reconciliation.
- `docs/requirements/` — reconstructed user requirements and traceability.
- `docs/archive/` — preserved historical source documents, explicitly non-normative.
- `docs/RFCs/`, `docs/Platform-Architecture-Specification-v1.1.md` — architectural contracts and governance.
- `reference/` — host semantic/reference models and the RISC-V64 kernel/architecture implementation; inspect each crate's actual status.
- `schemas/`, `tools/`, `.github/workflows/` — machine-readable records, validators and CI qualification.

## Validation

Run the documented repository gate from the repository root:

```sh
./check.sh
```

Use the pinned toolchain from `rust-toolchain.toml`. For target work, also run the relevant RISC-V build and QEMU boot workflow. Report exact command, target, commit, environment, workflow/run ID and result. A documentation gate, host-model suite, kernel build and QEMU boot are different evidence classes and must not be conflated.

## Development and safety rules

- Preserve the canonical `main` history; do not force-push or rewrite history.
- Inspect the real worktree, remote, staged/unstaged/untracked/ignored files and nested repositories before modifying a local checkout.
- Do not discard user work, hard-reset, clean or overwrite a workspace.
- Prefer disposable QEMU disks for install/recovery testing. Never write to a real target disk, alter firmware/boot state or make privileged host changes without the applicable explicit authorization.
- Keep private signing keys, credentials and tokens outside Git and build artifacts.
- Work continuously through dependency-ready research, implementation, integration, testing and documentation. Do not stop at a plan, audit, boot checkpoint or first release.

## License

See [LICENSE](LICENSE).
