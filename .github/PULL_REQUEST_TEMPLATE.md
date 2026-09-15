---
name: Pull Request
about: Standard PR template for AetherOS
title: ''
labels: ''
assignees: ''
---

## Description

<!-- What does this PR do? Reference the issue/RFC it addresses -->

## Type

<!-- Check one -->
- [ ] **Architecture Amendment** (requires RFC-GOV-002 process)
- [ ] **RFC Addition/Amendment** (new or modified RFC)
- [ ] **Reference Implementation** (code in `reference/`)
- [ ] **CTS Test Addition** (new conformance test cases)
- [ ] **Documentation** (non-normative)
- [ ] **Governance/Process**
- [ ] **Build/CI**

## Checklist

### All Changes
- [ ] Commit messages follow `<type>(<scope>): <description>` format
- [ ] No generic commit messages ("fix stuff", "update", "wip")
- [ ] `./check.sh` passes locally (or CI will run it)
- [ ] No `.orig`, `.bak`, or temporary files committed

### Architecture/RFC Changes
- [ ] Decision Record created/updated (`docs/generated/DECISION-INDEX.md`)
- [ ] Observation Record filed if from review (`docs/OBSERVATION-RECORD.md` template)
- [ ] Invariant cross-reference updated in `00-RFC-Index.md` if applicable
- [ ] `CHANGELOG.md` updated under `[Unreleased]`

### Implementation Changes
- [ ] Rust code compiles: `cargo check --workspace --all-targets --all-features`
- [ ] Tests pass: `cargo test --workspace --all-features`
- [ ] Clippy clean: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] Docs build: `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`
- [ ] CTS runner passes: `cargo run -p aether-cts-runner`
- [ ] New CTS cases added for new MUST requirements
- [ ] Differential tests pass (model vs provider)

### Documentation Changes
- [ ] No generated files edited by hand (`docs/generated/`)
- [ ] Glossary updated if new terms introduced
- [ ] Cross-references valid

## Evidence

### Conformance Evidence
<!-- CTS output, test results -->
```
# paste relevant CTS JSON output here
```

### Verification Evidence
<!-- Proof artifacts, model-checking traces -->
```
# reference proof files or describe approach
```

### Security Evidence
<!-- Review reports, red-team results -->
```
# reference review artifacts
```

## Review Requirements

Per `REVIEWER-GUIDE.md`, reviewers must:
1. **Reconstruct** the intended design from the PR
2. **Validate** against invariants and RFCs
3. **Assess Impact** via Impact Matrix
4. **Confidence Level**: High / Medium / Low

## Breaking Changes

<!-- List any breaking changes and migration path -->

## Screenshots / Logs

<!-- If applicable -->