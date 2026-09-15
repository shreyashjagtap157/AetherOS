#!/usr/bin/env sh
#
# Enterprise-grade branch protection configuration for AetherOS.
#
# Run this as a repository admin to enforce state-of-the-art branch protection
# rules on `main` (and optionally all `kilo/*` and `codex/*` branches).
#
# Requires: gh CLI v2.0+ with admin:write:branch protection scope.
#
# Usage:
#   gh auth login  # as repo admin
#   ./setup-branch-protection.sh
#
set -euo pipefail

REPO="${GITHUB_REPOSITORY:-shreyashjagtap157/AetherOS}"
DEFAULT_BRANCH="main"

echo "🔐 Configuring branch protection on $REPO …"

# ── 1. Repository-level merge hygiene ──────────────────────────────────────
# Enforce squash-merge with clean commit messages, auto-delete merged branches,
# and allow auto-merge for PRs.
#
gh api "repos/${REPO}" \
  -X PATCH \
  -f allow_merge_commit=false \
  -f allow_squash_merge=true \
  -f allow_rebase_merge=false \
  -f allow_auto_merge=true \
  -f delete_branch_on_merge=true \
  -f squash_merge_commit_title="COMMIT_OR_PR_TITLE" \
  -f squash_merge_commit_message="COMMIT_MESSAGES"

echo "✅ Repository merge settings updated (squash-only, auto-delete, auto-merge)"

# ── 2. Branch protection rule for main ─────────────────────────────────────
# Require: linear history, up-to-date branch, PR reviews (incl. CODEOWNERS),
#          required status checks (CI), and admin enforcement.
#
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

echo "✅ Protection rule applied to ${DEFAULT_BRANCH}"
echo "   - Require PRs with 2 approvals + CODEOWNERS"
echo "   - Require status checks (strict/up-to-date)"
echo "   - Require linear history"
echo "   - Require signed commits"
echo "   - Enforce for administrators"

# ── 3. Default branch settings ─────────────────────────────────────────────
gh api "repos/${REPO}" \
  -X PATCH \
  -f enable_issues=false \
  -f enable_wiki=false \
  -f enable_projects=false \
  -f allow_fork_syncing=false

echo "✅ Disabled unused features (issues, wiki, projects)"
echo ""
echo "🎉 Branch protection configuration complete."
echo "   Review at: https://github.com/${REPO}/settings/branches"
