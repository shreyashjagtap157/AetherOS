# AetherOS Agent Guide

Classification: Informative
Authoritative Source: docs/README.md

This file documents the development environment, git workflow, and validation
gate for anyone contributing to AetherOS — human or agent.

## Owner working preferences and repository expectations

Before a non-trivial task, read [the owner preferences and repository expectations](docs/project/OWNER_WORKING_PREFERENCES_AND_REPOSITORY_EXPECTATIONS.md), [the active v4.0 product goal](docs/project/AetherOS_Complete_Platform_Goal_v4_2026-11-21.md), and the [requirements suite](docs/requirements/README.md).

- Inspect and synchronize the actual remote/repository state before acting: branch, HEAD, history, staged/unstaged/untracked/ignored files, nested repositories, instructions and CI.
- Follow the Git branch/PR/squash policy documented in this file. Preserve history; never force-push, rewrite, hard-reset, clean, overwrite or silently stash user work.
- Implement authorized work directly: research, design, code, real consumer integration, tests, debugging, documentation and evidence. Do not stop at an audit or plan.
- Continue through dependency-ready tasks without asking the owner to say “continue” repeatedly. Escalate only true owner-only decisions, unavailable hardware/credentials, paid dependencies, destructive real-device/host actions or fundamental contract changes.
- Research with current official specifications, standards, maintained implementations and peer-reviewed work; assess maturity, licensing, security, compatibility, performance, power and thermal cost.
- Use one specialist subagent at a time; close and independently verify its output before assigning the next. The primary agent remains the accountable architecture/integration lead.
- Fix root causes. Never weaken tests, accept vacuous evidence, hide failures, or call a scaffold a functioning driver/service.
- Distinguish designed, scaffold-only, partially implemented, implemented, integrated, verified, qualified and certified states. Tie all claims to exact commit, target, environment, command, CI run and artifacts.
- Update all impacted requirements, architecture/ADR/RFC references, roadmap, status, risk/dependency records, tests and handoffs.
- Build this repository into the complete, integrated AetherOS platform, not merely a reference model, boot checkpoint or first installable RISC-V image. The first usable release is an intermediate milestone.

Project entry points: [project index](docs/project/README.md), [requirements suite](docs/requirements/README.md), [current implementation status](docs/IMPLEMENTATION-STATUS.md), and [complete v4.0 mandate](docs/project/AetherOS_Complete_Platform_Goal_v4_2026-11-21.md).

---

## Environment Setup

```sh
git clone https://github.com/shreyashjagtap157/AetherOS.git
cd AetherOS
./bootstrap.sh           # activates hooks and git config
rustup toolchain install --profile minimal --component rustfmt,clippy
```

The pinned toolchain is declared in `rust-toolchain.toml`. Docker users can
build the reproducible image from `.devcontainer/Dockerfile`.

> **`.git/config` is local state.** A fresh clone does not inherit another
> developer's `.git/config`. Run `./bootstrap.sh` after cloning to activate
> hooks (`core.hooksPath`) and apply the project's recommended git settings.

## Git Workflow

### Branch Naming

| Prefix     | Purpose                          | Protection |
|------------|----------------------------------|------------|
| `main`     | Stable, passing-validation state | Protected  |
| `kilo/*`   | Agent work branches              | Unprotected|
| `codex/*`  | Architecture/codex work          | Unprotected|
| `agent/*`  | Foundation/reference work      | Unprotected|
| `feature/*`| Short-lived feature work          | PR-gated    |

### Commit Messages

Enforced by `.githooks/commit-msg`. Format:

```
<type>(<scope>): <description>
```

| Type     | Scope examples                  | Notes                       |
|----------|---------------------------------|-----------------------------|
| `docs`   | *(any)*                         | Documentation only          |
| `rfc`    | `0037`, `ALLOC-001`             | RFC addition/amendment      |
| `impl`   | `capspace`, `cts-runner`        | Implementation              |
| `cts`    | *(any)*                         | Conformance tests           |
| `arch`   | `invariants`, `rfc-0037`        | Architecture amendment      |
| `gov`    | *(any)*                         | Governance/process          |
| `fix`    | *(component name)*              | Bug fix                     |
| `refactor`| *(component name)*              | Code restructuring          |
| `chore`  | `deps`, `ci`                    | Maintenance                 |
| `release`| `0.0.1`                         | Release tag                 |

Commit messages must have a description of at least 10 characters and must
not be generic ("update", "fix stuff", "wip", etc.).

### Pre-Commit Gate

`.githooks/pre-commit` runs the full validation gate before every commit:

1. Python documentation validators (`docs/tools/verify-authority.py`,
   `docs/tools/verify-decision-records.py`, `docs/tools/verify-generated.py`,
   `docs/tools/verify-glossary.py`, `docs/tools/verify-rfc.py`)
2. Rust reference validation (`cargo fmt`, `cargo check`, `cargo test`,
   `cargo clippy`, `cargo doc`, `cargo run -p aether-cts-runner`)

Run the complete gate manually at any time:

```sh
./check.sh
```

### Merge Strategy

**Squash-and-merge only.** The remote is configured to reject merge commits
and rebase merges. Every PR produces a single squashed commit on `main`.

Before merging, ensure your branch is rebased on `main`:

```sh
git checkout main
git pull --rebase
git checkout my-branch
git rebase main
```

Use `git commit --amend` or `git rebase -i` to fix commit messages if the
hook rejects a commit.

## Enterprise Git Configuration

A fresh clone does **not** inherit `.git/config`. Run `./bootstrap.sh` once
after cloning to activate hooks and apply all required and recommended settings
in one step. A fully annotated reference template lives at `.gitconfig.example`,
with each setting classified as REQUIRED, RECOMMENDED, OPTIONAL, or LOCAL-ONLY
so contributors can adopt selectively rather than importing a monolithic blob.

### Configuration Layers

```text
Repository policy          →  versioned files (.gitattributes, CODEOWNERS,
                                    .githooks/, .github/)
Developer preferences      →  ~/.gitconfig / includeIf / .gitconfig.example
Repository-local activation →  .git/config (per-clone; set by bootstrap.sh)
Remote enforcement         →  GitHub branch protection (via setup-branch-protection.sh)
```

## Branch Protection

> **Current state: NOT ENFORCED on remote.** No branch protection rules exist on the
> remote `main` branch. The script below provides the automation, but requires
> an admin to execute it. Until then, `main` is unprotected.

Branch protection rules are managed as code via:

```sh
.github/setup-branch-protection.sh          # apply
.github/setup-branch-protection.sh --dry-run  # preview
```

Each control has an identified rationale in the script's governance table.
The script is **idempotent** and **self-verifying**: it applies the configuration,
reads it back from the GitHub API, and asserts the expected state.

## Useful Commands

| Command | What it does |
|---------|-------------|
| `./check.sh` | Full validation gate (docs + Rust + reproducible) |
| `git log --oneline --graph` | Visual history |
| `git rerere` | Reapply previously resolved conflicts automatically |
| `git diff --check` | Check for whitespace errors |
