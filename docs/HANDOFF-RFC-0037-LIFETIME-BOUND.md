# Handoff: RFC-0037 Capability Lifetime and Domain Binding (Host Reference)

Classification: Operational
Authoritative Source: HANDOFF-RFC-0037-LIFETIME-BOUND.md

**Date:** 2026-11-19
**Purpose:** Snapshot of the uncommitted working-tree milestone so a new session can resume.
**Classification:** Operational

---

## 1. What this milestone is

Closes the gap between the historical "aether-cap-model milestone" lead (CapabilityFlags,
CapabilityLifecycle, RFC-0037.14 expiry, RFC-0037.15 domain binding, six unit tests) and the
actual checkout. The historical milestone does **not** exist in this repository: there is no
`CapabilityFlags`, `CapabilityLifecycle`, `expires_at_ns` or scheduling-time-aware operation
anywhere at `main`. RFC-0037.14 and RFC-0037.15 exist only as normative text. This milestone
implements the missing semantics in the host reference prototype.

Scope is the **host reference model only**. It is not a kernel, not a capability hardware
provider, not canonical token serialization, and not a product release.

## 2. Baseline (before changes)

- Workspace: `C:\Users\siddh\Downloads\ABC\AetherOS` (git root; remote `origin` =
  https://github.com/shreyashjagtap157/AetherOS.git; branch `main` at `e8dbc0f`,
  merge of PR #10). Only pre-existing untracked file: `AetherOS_OpenCode_Master_Goal_Prompt.txt`.
- `cargo test --workspace`: 46 tests passed. CTS runner: 17 cases passed.
- Unmodified `main` fails `./check.sh` at `tools/verify-reproducible-reference.sh` on this
  host (two consecutive release builds differ at char 257, line 2 — the MSVC PE
  `TimeDateStamp`). Confirmed pre-existing by cloning the pristine `main` into a temp
  directory and reproducing the identical failure. **Resolved in this milestone** (see
  section 4).

## 3. Changes (uncommitted, working tree)

`aether-types` (`reference/crates/aether-types/src/lib.rs`)
- New `CapabilityFlags` bitflag set per RFC-0037.7: TRANSIENT 0x01, REVOCABLE 0x02, NAMED 0x04,
  BOUND 0x08, SEALED 0x10; `from_bits` rejects unknown bits; Debug redacts like Rights.
- `Capability` gains `flags: CapabilityFlags` and `expires_at_ns: u64` (0 = never expires).
- `CapError` gains `Expired` and `DomainBound`.

`aether-cap-model` (`reference/crates/aether-cap-model/src/lib.rs`) and
`aether-capspace` (`reference/crates/aether-capspace/src/lib.rs`)
- `derive`, `transfer`, `authorize` take `now_ns: u64` as the final parameter (scheduling time
  is supplied by the caller; the model never reads a clock).
- Expiry enforced per RFC-0037.14 with the strict comparison `now_ns > expires_at_ns`
  (`expires_at_ns != 0`): equality is still valid; the first instant after expiry is rejected
  at every enforcement boundary, including revocation resolution (`resolve_live`) used by
  derive/transfer, `authorize`, and hence every memory-access revalidation path.
- Derived and transferred children inherit the parent's `expires_at_ns` (no lifetime
  amplification).
- `CAPF_BOUND` capabilities cannot be transferred (`CapError::DomainBound`); same-domain
  authorize/derive remain valid; cross-domain presentation fails closed with `WrongDomain`
  (domain-local handles already bind decode to one domain — RFC-0037.15's domain scope is
  enforced by construction plus the transfer refusal).
- `aether-capspace::memory::MemoryManager` (map, access, unmap, revoke_capability) and
  `MemoryRuntime` (derive, transfer, map, access, unmap, revoke_capability) thread `now_ns`
  to the capability check. All in-repo callers (tests, CTS) pass an explicit scheduling time.

`aether-cts-runner` (`reference/crates/aether-cts-runner/src/main.rs`)
- Two new differential cases: `CTS-CAP-EXPIRY-001` (RFC-0037.14 boundary, equality valid,
  +1 ns rejected, expired parent cannot derive/transfer; oracle and provider must agree) and
  `CTS-CAP-BOUND-001` (CAPF_BOUND: same-domain use valid, transfer refused by both).
- Existing helper `pair_derive`/`pair_transfer` extracted to keep functions under the
  Clippy `too_many_lines` limit (no allow/suppression used).

Unit tests: 10 in cap-model (was 4), 37 in capspace (was 31), 3 in aether-types (was 2),
9 in state-model, 0 elsewhere: **59 workspace tests pass**.

Independent verifier (orch-task-verifier) verdict: PASS with T5 qualified PARTIAL; no HIGH/CRITICAL
findings, no no-op implementations found. Findings resolved after review: F1 (CTS-CAP-EXPIRY-001 now
asserts actual equality-boundary validity in both models, not merely model==system agreement) and
F5 (explicit derived-child-of-BOUND-parent non-transferability test added in both models).
Noted, not blocking: F2 (REVOCABLE/SEALED/TRANSIENT/NAMED parse but have no enforcement;
`rights_derived`/`creation_timestamp_ns` unmodeled — do not cite RFC-0037.7/.12 as enforced),
F4 (scheduling time is caller-supplied; no in-repo clock binding), F6 (teardown operations may act
on expired capabilities), F7 (untracked master goal prompt is user-provided, unrelated).

## 4. Verification actually executed (2026-11-19, Windows, Rust 1.95.0)

- `reference/check.sh`: `cargo fmt --check` ✓, `cargo check --locked` ✓, debug tests ✓,
  release tests ✓, `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` ✓,
  `RUSTDOCFLAGS="-D warnings" cargo doc` ✓, CTS runner: 19/19 cases pass.
- `./check.sh` (root): passes end-to-end on Windows with exit 0, including the
  reproducibility gate, which now emits `sha256 bfdf19a6...9677` for the two byte-identical
  release builds and "Release evidence passed for 136 file(s)".
  Root cause of the pre-existing failure: MSVC `link.exe` embeds a wall-clock
  `TimeDateStamp` in the PE header regardless of `SOURCE_DATE_EPOCH` (honored only by
  GNU ld / lld). Two identical clean builds therefore always differed at offset 0x100.
  Fix: `tools/verify-reproducible-reference.sh` adds `-C link-arg=/Brepro` to `RUSTFLAGS`,
  but only on MINGW/MSYS/CYGWIN hosts (`uname -s`); Linux CI behavior is unchanged. The
  gate's assertion (byte-identical independent builds) is preserved, not weakened.
  Note `artifacts/evidence/` is gitignored and stays local.
- `git diff --check` semantics: enforced via the reference gate's fmt check; review the diff
  before committing.

## 5. Known limitations and boundaries

- Scheduling time is an input; monotonicity, source of truth, wraparound and cross-CPU time
  semantics are NOT modeled (RFC-0012 is not implemented in the host reference).
- Canonical 96-byte token layout, seal HMAC, `rights_derived` counters and creation timestamps
  are NOT implemented; flags model the RFC-0037.7 bit semantics only.
- RFC-0037.15 full domain rebinding semantics (e.g., via transfer of a non-bound capability
  carrying a bound flag in a future profile) are not defined by the RFC text this slice relies
  on; `CAPF_BOUND` refusing transfer is the conservative reading consistent with RFC-0037.7
  ("not transferable").
- No kernel/CHERI/hardware provider integration; MemoryRuntime's global mutex remains a host
  reference mechanism, not a production dispatch strategy.
- The two new CTS cases and the working-tree diff are uncommitted, unmerged, un-CI-validated.

## 6. State and next actions

- Working tree: 13 modified files (see `git status --short`), plus this document and the
  master goal prompt (untracked, user-provided). Not committed, not pushed.
- The reproducibility-gate host failure is **resolved** in this milestone: `/Brepro` on
  Windows hosts makes the PE timestamp deterministic; the full `./check.sh` gate now passes
  end-to-end on this Windows host (exit 0). Linux CI behavior unchanged.
- Independent verifier report: received and findings resolved (see above).
- Commit locally only if the repository's commit policy/authorization allows (AGENTS.md
  permits local commits only when the repo policy or explicit user decision authorizes;
  current policy: keep changes in the working tree, report them).
- Next dependency-ready roadmap item after acceptance: continue Phase A of
  docs/IMPLEMENTATION-ROADMAP.md (executable models / CTS coverage), or the highest-priority
  open risk per the risk model.
- The historical "CapabilityLifecycle" concept has no normative RFC counterpart in the suite;
  do not invent it. Lifecycle work should be derived from RFC-0018 (Lifecycle FSM) with a
  defined task, not back-filled from the old summary.
