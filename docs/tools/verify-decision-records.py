#!/usr/bin/env python3
"""
verify-decision-records.py

CI gate. Fails when:
- An RFC marked `Status: Accepted` lacks a populated Decision Record appendix.

Exits 0 on success, 1 on fail.
"""
from pathlib import Path
import re
import sys

REPO_ROOT = Path(__file__).resolve().parents[1]
RFC_DIRS = [
    REPO_ROOT / "RFCs" / "00-Composition",
    REPO_ROOT / "RFCs" / "01-Execution",
    REPO_ROOT / "RFCs" / "02-State",
    REPO_ROOT / "RFCs" / "03-Communication",
    REPO_ROOT / "RFCs" / "04-Observability",
    REPO_ROOT / "RFCs" / "05-Cross-Cutting",
]

STATUS_RE = re.compile(r"^Status:\s*Accepted\s*$", re.MULTILINE)
DECISION_RECORD_RE = re.compile(r"^##\s+Decision\s+Record\s*$", re.MULTILINE)


def main():
    failures = []
    for rfc_dir in RFC_DIRS:
        if not rfc_dir.exists():
            continue
        for rfc_file in sorted(rfc_dir.glob("RFC-*.md")):
            content = rfc_file.read_text(encoding="utf-8")
            if STATUS_RE.search(content):
                if not DECISION_RECORD_RE.search(content):
                    failures.append(str(rfc_file.relative_to(REPO_ROOT)))
    if failures:
        for failure in failures:
            print(f"FAIL: {failure} is Accepted without a Decision Record.")
        return 1
    print("All accepted RFCs have populated Decision Records.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
