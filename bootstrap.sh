#!/usr/bin/env sh
#
# AetherOS Repository Bootstrap
# ---------------------------------------------------------------------------
# Run this once after cloning to activate git hooks and apply the
# enterprise-grade git configuration that the project expects.
#
# A fresh clone does NOT inherit .git/config from another developer's machine.
# This script provides the reproducible bootstrap that .git/config cannot.
#
# Usage:
#   git clone https://github.com/shreyashjagtap157/AetherOS.git
#   cd AetherOS
#   ./bootstrap.sh
#
set -eu

echo "Bootstrapping AetherOS development environment..."

# 1. Activate git hooks
echo "  Activating hooks via core.hooksPath..."
git config core.hooksPath .githooks

# 2. Apply project git configuration
echo "  Applying git configuration..."

# REQUIRED for AetherOS hooks and workflow
git config push.default current
git config push.autoSetupRemote true
git config push.followTags true
git config tag.sort version:refname
git config init.defaultBranch main
git config pull.rebase true
git config http.sslVerify true
git config core.longpaths true

# RECOMMENDED for all contributors
git config core.whitespace "blank-at-eol,blank-at-eof,space-before-tab,trailing-space"
git config rerere.enabled true
git config rerere.autoupdate true
git config diff.renames true
git config diff.compactionHeuristic true
git config checkout.autoSetupRemote true

# 3. Verify
echo ""
echo "  Verifying configuration..."

check() {
    actual=$(git config "$1")
    if [ "$actual" = "$2" ]; then
        echo "    OK: $1 = $actual"
    else
        echo "    FAIL: $1: expected '$2', got '$actual'"
        exit 1
    fi
}

check core.hooksPath ".githooks"
check push.default current
check pull.rebase true
check init.defaultbranch main
check tag.sort version:refname

echo ""
echo "Bootstrap complete."
echo ""
echo "Hooks active:"
ls -1 .githooks/ | sed 's/^/  /'
echo ""
echo "Next steps:"
echo "  - Run ./check.sh for the full validation gate"
echo "  - Make changes, then git add and git commit"
