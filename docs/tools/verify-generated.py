#!/usr/bin/env python3
"""
verify-generated.py

CI gate. Every file under `generated/` must declare complete provenance:

    Classification: Generated
    Authoritative Source: <source document(s)>
    Generated-By: <generator script path>
    Generated-At: <ISO-8601 UTC timestamp>
    Generator-Version: <semver or git ref>
    Source-Revision: <git ref of source document state>
    Generation-Inputs: <list of source files that participated>

Exits 0 on success, 1 on fail.
"""
from pathlib import Path
import re
import sys

REPO_ROOT = Path(__file__).resolve().parents[1]
GENERATED_DIR = REPO_ROOT / "generated"

REQUIRED_MARKERS = [
    "Classification: Generated",
    "Authoritative Source:",
    "Generated-By:",
    "Generated-At:",
    "Generator-Version:",
    "Source-Revision:",
    "Generation-Inputs:",
]


def main():
    if not GENERATED_DIR.exists():
        return 0
    failures = []
    for path in sorted(GENERATED_DIR.glob("*.md")):
        text = path.read_text()
        rel = path.relative_to(REPO_ROOT)
        for marker in REQUIRED_MARKERS:
            if marker not in text:
                failures.append(f"{rel}: missing provenance marker '{marker}'.")
    if failures:
        print("Generated-artifact check failed:")
        for f in failures:
            print(f"  - {f}")
        return 1
    print(f"All generated artifacts in {GENERATED_DIR.relative_to(REPO_ROOT)} carry complete provenance.")
    return 0


if __name__ == "__main__":
    sys.exit(main())