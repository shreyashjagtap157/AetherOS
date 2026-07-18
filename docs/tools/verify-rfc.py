#!/usr/bin/env python3
"""
verify-rfc.py

CI gate. Per-RFC structural checks:

- Every RFC carries `Classification:` and `Authoritative Source:` headers.
- Every RFC has a `Status:` field with one of:
  Draft, Proposed, Accepted, Deprecated, Withdrawn, Reserved.
- Every Accepted RFC has a populated Decision Record appendix.
- RFC filenames match the RFC number declared within.

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

ACCEPTABLE_STATUSES = {
    "Draft", "Proposed", "Accepted", "Deprecated", "Withdrawn", "Reserved"
}

CLASS_HEADER_RE = re.compile(
    r"^Classification:\s*([A-Z][A-Za-z]+)\s*$", re.MULTILINE
)
SOURCE_HEADER_RE = re.compile(
    r"^Authoritative Source:\s*([^\n]+)\s*$", re.MULTILINE
)
STATUS_FIELD_RE = re.compile(
    r"^Status:\s*(Draft|Proposed|Accepted|Deprecated|Withdrawn|Reserved)\s*$",
    re.MULTILINE,
)
DECISION_RECORD_HEADER_RE = re.compile(
    r"^##\s+Decision\s+Record\s*$", re.MULTILINE
)
RFC_NUMBER_RE = re.compile(r"^#\s+RFC-(\d{4}):", re.MULTILINE)


def main():
    failures = []
    rfc_count = 0
    for rfc_dir in RFC_DIRS:
        if not rfc_dir.exists():
            continue
        for rfc_file in sorted(rfc_dir.glob("RFC-*.md")):
            rfc_count += 1
            content = rfc_file.read_text()

            if not CLASS_HEADER_RE.search(content):
                failures.append(
                    f"{rfc_file.relative_to(REPO_ROOT)}: missing Classification header."
                )
            if not SOURCE_HEADER_RE.search(content):
                failures.append(
                    f"{rfc_file.relative_to(REPO_ROOT)}: missing Authoritative Source header."
                )

            status_match = STATUS_FIELD_RE.search(content)
            if not status_match:
                failures.append(
                    f"{rfc_file.relative_to(REPO_ROOT)}: missing or invalid Status field "
                    f"(must be one of: {sorted(ACCEPTABLE_STATUSES)})."
                )
                continue
            status = status_match.group(1)
            if status not in ACCEPTABLE_STATUSES:
                failures.append(
                    f"{rfc_file.relative_to(REPO_ROOT)}: Status '{status}' not in accepted set."
                )

            if status == "Accepted" and not DECISION_RECORD_HEADER_RE.search(content):
                failures.append(
                    f"{rfc_file.relative_to(REPO_ROOT)}: Status is Accepted but no "
                    f"`## Decision Record` appendix is present."
                )

            # Verify filename matches RFC number in content
            filename_num = re.search(r"RFC-(\d{4})", rfc_file.name)
            if filename_num:
                content_num = RFC_NUMBER_RE.search(content)
                if content_num and content_num.group(1) != filename_num.group(1):
                    failures.append(
                        f"{rfc_file.relative_to(REPO_ROOT)}: filename declares RFC-{filename_num.group(1)} "
                        f"but content declares RFC-{content_num.group(1)}."
                    )

    if failures:
        print(f"RFC structural check failed across {rfc_count} RFC(s):")
        for f in failures:
            print(f"  - {f}")
        return 1

    print(f"All {rfc_count} RFCs passed structural checks.")
    return 0


if __name__ == "__main__":
    sys.exit(main())