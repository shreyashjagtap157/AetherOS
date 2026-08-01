#!/usr/bin/env python3
"""
generate-decision-index.py

Walks the RFC repository, extracts the Decision Record appendix
from each normative RFC, and emits a non-authoritative
`generated/DECISION-INDEX.md`.

The output is generated. It MUST NOT be edited directly.
The authoritative sources are the per-RFC Decision Record appendices.

Generation-Inputs lists all RFC files that were scanned so that
regenerators can verify which sources actually participated.
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
OUT_PATH = REPO_ROOT / "generated" / "DECISION-INDEX.md"
SCRIPT_PATH = Path(__file__).relative_to(REPO_ROOT)

DECISION_RECORD_RE = re.compile(r"^##\s+Decision\s+Record\s*$", re.MULTILINE)
FIELD_RE = re.compile(r"^([A-Za-z ]+?):\s*(.*)$")


def collect_records():
    records = []
    sources = []
    for rfc_dir in RFC_DIRS:
        if not rfc_dir.exists():
            continue
        for rfc_file in sorted(rfc_dir.glob("RFC-*.md")):
            content = rfc_file.read_text(encoding="utf-8")
            sources.append(str(rfc_file.relative_to(REPO_ROOT)))
            match = DECISION_RECORD_RE.search(content)
            if not match:
                continue
            block = content[match.end():]
            record = {"rfc": rfc_file.stem, "fields": {}}
            for line in block.splitlines():
                field_match = FIELD_RE.match(line)
                if field_match:
                    key, value = field_match.group(1).strip(), field_match.group(2).strip()
                    record["fields"][key.strip()] = value.strip()
            records.append(record)
    return records, sources


def emit(records, sources):
    lines = [
        "Classification: Generated",
        "Authoritative Source: Per-RFC Decision Record appendices",
        f"Generated-By: {SCRIPT_PATH}",
        "Generated-At: (regenerate to update)",
        "Generator-Version: 1.0.0",
        "Source-Revision: (regenerate to update)",
        "Generation-Inputs:",
    ]
    for src in sorted(sources):
        lines.append(f"  - {src}")
    lines.extend([
        "",
        "# Decision Index",
        "",
        "This document is generated. Do not edit it directly.",
        "The authoritative sources are the per-RFC Decision Record appendices.",
        "",
    ])
    for record in records:
        rfc = record["rfc"]
        fields = record["fields"]
        lines.append(f"## {rfc}")
        lines.append("")
        if not fields:
            lines.append("> Decision Record not yet populated.")
            lines.append("")
            continue
        if "Accepted Decision" in fields:
            lines.append(f"> Decision: {fields['Accepted Decision']}")
        if "Status" in fields:
            lines.append(f"> Status: {fields['Status']}")
        if "Problem" in fields:
            lines.append(f"> Problem: {fields['Problem']}")
        if "Architectural Consequences" in fields:
            lines.append(f"> Architectural Consequences: {fields['Architectural Consequences']}")
        if "Supersedes" in fields and fields["Supersedes"]:
            lines.append(f"> Supersedes: {fields['Supersedes']}")
        if "Superseded By" in fields and fields["Superseded By"]:
            lines.append(f"> Superseded By: {fields['Superseded By']}")
        lines.append("")
    return "\n".join(lines)


def main():
    records, sources = collect_records()
    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUT_PATH.write_text(emit(records, sources), encoding="utf-8")
    print(f"Wrote {OUT_PATH} with {len(records)} decisions from {len(sources)} RFCs.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
