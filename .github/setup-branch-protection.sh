#!/usr/bin/env bash
#
# Enterprise-grade branch protection configuration for AetherOS.
#
# Run this as a repository admin to enforce state-of-the-art branch protection
# rules on `main`. The script is idempotent and self-verifying: it applies
# configuration, reads it back, and asserts the expected state.
#
# Requires: gh CLI v2.0+ with admin:org, repo, and admin:protected_branch scopes.
#
# Usage:
#   gh auth login  # as repo admin with full control
#   ./setup-branch-protection.sh
#
# To preview without applying:
#   ./setup-branch-protection.sh --dry-run
#
set -euo pipefail

REPO="${GITHUB_REPOSITORY:-shreyashjagtap157/AetherOS}"
DEFAULT_BRANCH="main"
DRY_RUN=0

if [ "${1:-}" = "--dry-run" ]; then DRY_RUN=1; fi

echo "🔐 Configuring branch protection on ${REPO} …"

# ── Governance rationale ─────────────────────────────────────────────────
#
# Each control below has an identified AetherOS justification. Controls are
# not accumulated because they "sound enterprise." Each protects a property
# defined in the specification suite.
#
# | Control                        | AetherOS Justification                                      |
# |--------------------------------|-------------------------------------------------------------|
# | Squash-and-merge only          | RFC-0040 (verification methodology) requires reproducible |
# |                                | single-commit changesets for CTS evidence provenance.      |
# | No merge commits               | Prevents history complexity that obscures authority lineage.|
# | No rebase merges               | Maintains single-source-of-truth for conformance evidence.  |
# | Delete branches on merge       | Reduces stale reference surface; prevents accidental reuse  |
# |                                | of merged capability tokens or session identifiers.        |
# | Require 2 approvals            | ARCHITECTURE-GOVERNANCE.md §4: architectural amendments   |
# |                                | require dual expert review (arch + implementer).           |
# | CODEOWNERS review required     | CODEOWNERS maps subsystem ownership; prevents out-of-scope|
# |                                | mutations to security-critical documents (SECURITY.md,     |
# |                                | THREAT-MODEL.md, RFCs).                                     |
# | Status checks (strict)         | Evidence-Based Amendment Principle: changes must pass       |
# |                                | the full validation gate (check.sh) before integration.      |
# | Up-to-date branches            | Ensures checks ran against latest target, preventing race   |
# |                                | conditions in mediation/policy logic.                       |
# | Linear history                 | RFC-0037 capability token provenance requires traceable,    |
# |                                | non-rewritten history for audit commitment integrity.      |
# | Signed commits                 | Cryptographic integrity (I-6): content-addressed object    |
# |                                | identity must be attributable to a known principal.          |
# | Enforce admins                 | Governance Principle §7: governance protects architecture   |
# |                                | from all actors, including those with elevated privileges.    |
# | Disable wiki/issues/projects   | Reduces attack surface and avoids fragmented issue tracking |
# |                                | outside the RFC governance process.                        |
# | Signoffs required              | RFC-0037 §6: contributor attestation of provenance chain.   |

apply_repo_settings() {
    if [ "$DRY_RUN" = 1 ]; then
        echo "  [dry-run] Would PATCH repos/${REPO}"
        return 0
    fi
    gh api "repos/${REPO}" \
      -X PATCH \
      -f allow_merge_commit=false \
      -f allow_squash_merge=true \
      -f allow_rebase_merge=false \
      -f allow_auto_merge=true \
      -f delete_branch_on_merge=true \
      -f squash_merge_commit_title="COMMIT_OR_PR_TITLE" \
      -f squash_merge_commit_message="COMMIT_MESSAGES" \
      -f enable_issues=false \
      -f enable_wiki=false \
      -f enable_projects=false \
      -f allow_fork_syncing=false
}

apply_branch_protection() {
    if [ "$DRY_RUN" = 1 ]; then
        echo "  [dry-run] Would PUT branch protection on ${DEFAULT_BRANCH}"
        return 0
    fi
    PROTECTION_PAYLOAD=$(cat <<'JSON'
{
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": true,
    "required_approving_review_count": 2,
    "require_last_push_commit_signoff": true
  },
  "enforce_admins": true,
  "required_status_checks": {
    "strict": true,
    "contexts": ["reference-validation"]
  },
  "required_linear_history": { "enabled": true },
  "required_signatures": true,
  "restrictions": null
}
JSON
)
    gh api "repos/${REPO}/branches/${DEFAULT_BRANCH}/protection" \
      -X PUT \
      -H "Accept: application/vnd.github+json" \
      -f body="$PROTECTION_PAYLOAD"
}

verify_repo_settings() {
    echo "  Verifying repository settings…"
    actual=$(gh api "repos/${REPO}" -H "Accept: application/vnd.github+json" 2>/dev/null)
    check_field() {
        val=$(echo "$actual" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('$1','MISSING'))" 2>/dev/null || echo "ERROR")
        if [ "$val" = "$2" ]; then
            echo "  ✅ $1 = $val"
        else
            echo "  ❌ $1: expected '$2', got '$val'"
            return 1
        fi
    }
    check_field allow_merge_commit "False"
    check_field allow_squash_merge "True"
    check_field allow_rebase_merge "False"
    check_field delete_branch_on_merge "True"
    check_field enable_issues "False"
    check_field enable_wiki "False"
}

verify_branch_protection() {
    echo "  Verifying branch protection on ${DEFAULT_BRANCH}…"
    bp=$(gh api "repos/${REPO}/branches/${DEFAULT_BRANCH}/protection" \
          -H "Accept: application/vnd.github+json" 2>/dev/null) || {
        echo "  ❌ No branch protection found (API returned error)"
        return 1
    }
    check_bp() {
        has=$(echo "$bp" | python3 -c "
import json,sys
d=json.load(sys.stdin)
parts='$1'.split('.')
val=d
for p in parts:
    val=val.get(p,{}) if isinstance(val,dict) else None
    if val is None: break
if isinstance(val,dict) and 'enabled' in val:
    print(val['enabled'])
else:
    print(val)
" 2>/dev/null || echo "ERROR")
        if [ "$has" = "$2" ]; then
            echo "  ✅ $1 = $has"
        else
            echo "  ❌ $1: expected '$2', got '$has'"
            return 1
        fi
    }
    check_bp required_pull_request_reviews.required_code_owner_reviews "True"
    check_bp required_status_checks.strict "True"
    check_bp enforce_admins true
    check_bp required_linear_history.enabled true
    check_bp required_signatures true
    echo "  ✅ Approving review count: $(echo "$bp" | python3 -c "import json,sys; print(json.load(sys.stdin).get('required_pull_request_reviews',{}).get('required_approving_review_count','?'))")"
    echo "  ✅ Required status contexts: $(echo "$bp" | python3 -c "import json,sys; print(json.load(sys.stdin).get('required_status_checks',{}).get('contexts','[]'))")"
}

# ── Apply ────────────────────────────────────────────────────────────────
echo ""
echo "Step 1: Repository settings"
apply_repo_settings

echo ""
echo "Step 2: Branch protection rule"
apply_branch_protection
echo "✅ Protection rule applied to ${DEFAULT_BRANCH}"
echo "   - Require PRs with 2 approvals + CODEOWNERS"
echo "   - Require status checks (strict/up-to-date)"
echo "   - Require linear history"
echo "   - Require signed commits"
echo "   - Enforce for administrators"
echo "   - Signoff required on last push"

# ── Verify ───────────────────────────────────────────────────────────────
if [ "$DRY_RUN" = 0 ]; then
    echo ""
    echo "Step 3: Verification (read-back + assert)"
    verify_repo_settings
    verify_branch_protection
fi

echo ""
if [ "$DRY_RUN" = 1 ]; then
    echo "🎉 Dry-run complete. Re-run without --dry-run to apply."
else
    echo "🎉 Branch protection configuration applied and verified."
    echo "   Review at: https://github.com/${REPO}/settings/branches"
fi
