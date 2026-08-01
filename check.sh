#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")"

if command -v python3 >/dev/null 2>&1 && python3 -c '' >/dev/null 2>&1; then
    PYTHON=python3
elif command -v python >/dev/null 2>&1 && python -c '' >/dev/null 2>&1; then
    PYTHON=python
else
    echo "A working Python 3 interpreter is required." >&2
    exit 1
fi

"$PYTHON" docs/tools/verify-repository.py
"$PYTHON" -m py_compile docs/tools/*.py tools/*.py
"$PYTHON" tools/verify-supply-chain.py
./reference/check.sh
./tools/verify-reproducible-reference.sh
"$PYTHON" tools/generate-release-evidence.py --version 0.0.1 --checks-completed
"$PYTHON" tools/verify-release-evidence.py
git diff --check
