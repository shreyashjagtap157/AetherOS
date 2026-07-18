#!/usr/bin/env python3
"""
verify-repository.py

CI orchestrator. Auto-discovers and runs all validators in the tools directory.

Each validator MUST be named `verify-*.py` (excluding this file itself).
Validators must be standalone-runnable via `python verify-*.py`.

Exits 0 only if all validators pass.
"""
from pathlib import Path
import subprocess
import sys

REPO_ROOT = Path(__file__).resolve().parents[1]
TOOLS_DIR = REPO_ROOT / "tools"


def main():
    failures = 0
    validators = sorted(TOOLS_DIR.glob("verify-*.py"))
    for path in validators:
        if path.name == __file__:
            continue
        result = subprocess.run(
            [sys.executable, str(path)],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
        )
        print(f"--- {path.name}")
        if result.stdout:
            print(result.stdout.strip())
        if result.stderr:
            print(result.stderr.strip())
        if result.returncode != 0:
            failures += 1
            print(f"FAIL: {path.name} exited {result.returncode}")
    if failures:
        print(f"\n{failures} validator(s) failed.")
        return 1
    print("\nAll validators passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())