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
rustup toolchain install --profile minimal --component rustfmt,clippy
```

The pinned toolchain is declared in `rust-toolchain.toml`. Docker users can
build the reproducible image from `.devcontainer/Dockerfile`.

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

The repository ships with enterprise-grade defaults already applied locally.
If you clone on a new machine, apply the recommended local settings:

```sh
git config core.hooksPath .githooks
git config push.default current
git config push.autoSetupRemote true
git config push.followTags true
git config tag.sort version:refname
git config init.defaultBranch main
git config pull.rebase true
git config core.longpaths true
git config core.whitespace "blank-at-eol,blank-at-eof,space-before-tab,trailing-space,indent-with-non-tab"
git config http.sslVerify true
git config rerere.enabled true
git config rerere.autoupdate true
git config diff.renames true
git config diff.compactionHeuristic true
git config checkout.autoSetupRemote true
git config status.short true
git config gc.auto 256
```

A reference `.gitconfig` template is provided at `.gitconfig.example`.

## Branch Protection

Branch protection rules are managed as code via:

```sh
.github/setup-branch-protection.sh
```

This script (run by a repo admin) enforces on `main`:

- Require PR with 2 approvals + CODEOWNERS review
- Require status checks to pass (strict, up-to-date)
- Require linear history
- Require signed commits
- Enforce rules for administrators
- Auto-delete merged branches

## Useful Commands

| Command | What it does |
|---------|-------------|
| `./check.sh` | Full validation gate (docs + Rust + reproducible) |
| `git log --oneline --graph` | Visual history |
| `git rerere` | Reapply previously resolved conflicts automatically |
| `git diff --check` | Check for whitespace errors |
