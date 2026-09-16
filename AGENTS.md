# AetherOS Agent Guide

Classification: Informative
Authoritative Source: docs/README.md

This file documents the development environment, git workflow, and validation
gate for anyone contributing to AetherOS — human or agent.

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
