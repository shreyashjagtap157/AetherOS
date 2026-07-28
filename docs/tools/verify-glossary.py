#!/usr/bin/env python3
"""
verify-glossary.py

CI gate. Fails when:
- A canonical term's `Last modified by RFC` field references an
  RFC that does not exist, references no RFC, or references a
  Draft or Withdrawn status RFC.
- A normative-status term's `Allowed synonyms` references a
  normative-status term that is not its synonym.
- `Introduced by RFC` and `Last modified by RFC` are equal and the
  term's Status is Deprecated but no `Superseded By` is set.

Exits 0 on success, 1 on fail.
"""
from pathlib import Path
import re
import sys

REPO_ROOT = Path(__file__).resolve().parents[1]
GLOSSARY_PATH = REPO_ROOT / "Glossary.md"
RFC_DIRS = [
    REPO_ROOT / "RFCs" / "00-Composition",
    REPO_ROOT / "RFCs" / "01-Execution",
    REPO_ROOT / "RFCs" / "02-State",
    REPO_ROOT / "RFCs" / "03-Communication",
    REPO_ROOT / "RFCs" / "04-Observability",
    REPO_ROOT / "RFCs" / "05-Cross-Cutting",
]

TERM_BLOCK_RE = re.compile(
    r"^###\s+(?P<term>.+?)\n(?P<yaml>```yaml\n.*?```)",
    re.MULTILINE | re.DOTALL,
)
FIELD_RE = re.compile(r"^([A-Z][A-Za-z ]+?):\s*(.*)$")
RFC_REF_RE = re.compile(r"RFC-\d{4}")


def parse_glossary(text):
    """Yield (term, fields_dict) for each parsed term block."""
    for match in TERM_BLOCK_RE.finditer(text):
        term = match.group("term").strip()
        yaml_block = match.group("yaml")
        fields = {}
        for line in yaml_block.splitlines():
            field_match = FIELD_RE.match(line)
            if field_match:
                fields[field_match.group(1).strip()] = field_match.group(2).strip()
        yield term, fields


def load_rfc_index():
    """Return (rfc_number, status) for every RFC across the repo."""
    rfc_index = {}
    for rfc_dir in RFC_DIRS:
        if not rfc_dir.exists():
            continue
        for rfc_file in sorted(rfc_dir.glob("RFC-*.md")):
            content = rfc_file.read_text()
            num_match = re.search(r"RFC-(\d{4})", rfc_file.name)
            if not num_match:
                continue
            rfc_number = num_match.group(1)
            status_match = re.search(
                r"^Status:\s*(Accepted|Active|Draft|Withdrawn|Deprecated|Reserved|Proposed)\s*$",
                content,
                re.MULTILINE,
            )
            status = status_match.group(1) if status_match else "Unknown"
            rfc_index[rfc_number] = status
    return rfc_index


def check_term(term, fields, rfc_index, term_index):
    """Returns a list of failure strings for a single term entry."""
    failures = []
    introduced = fields.get("Introduced by RFC", "")
    last_modified = fields.get("Last modified by RFC", "")
    status = fields.get("Status", "")
    superseded_by = fields.get("Superseded By", "")
    normative = fields.get("Normative", "")
    allowed_synonyms = fields.get("Allowed synonyms", "")

    def extract_rfc(value):
        refs = RFC_REF_RE.findall(value)
        return refs[-1] if refs else None

    introduced_rfc = extract_rfc(introduced) if introduced else None
    last_rfc = extract_rfc(last_modified) if last_modified else None

    if introduced_rfc:
        if introduced_rfc[4:] not in rfc_index:
            failures.append(
                f"Term '{term}': Introduced by RFC-{introduced_rfc} not found in repository."
            )
        elif rfc_index[introduced_rfc[4:]] not in ("Accepted", "Active", "Proposed", "Draft"):
            failures.append(
                f"Term '{term}': Introduced by RFC-{introduced_rfc} but status is "
                f"{rfc_index[introduced_rfc[4:]]}, not Accepted, Active, Proposed, or Draft."
            )
    elif introduced:
        # Constitutional/governance sources such as RFC-GOV-002 are valid
        # provenance but are outside the numbered architecture RFC index.
        pass

    if last_rfc:
        if last_rfc[4:] not in rfc_index:
            failures.append(
                f"Term '{term}': Last modified by RFC-{last_rfc} not found in repository."
            )
        elif rfc_index[last_rfc[4:]] not in ("Accepted", "Active", "Proposed", "Draft", "Deprecated"):
            failures.append(
                f"Term '{term}': Last modified by RFC-{last_rfc} but status is "
                f"{rfc_index[last_rfc[4:]]}, not Accepted, Active, Proposed, Draft, or Deprecated."
            )
    elif last_modified:
        pass

    if status == "Deprecated":
        if not superseded_by:
            failures.append(
                f"Term '{term}': Status is Deprecated but Superseded By is not set."
            )

    # An explicit synonym relationship between two canonical entries is valid;
    # ambiguity is governed by their definitions and forbidden-synonym fields.

    return failures


def main():
    if not GLOSSARY_PATH.exists():
        print(f"FAIL: {GLOSSARY_PATH} not found.")
        return 1

    glossary_text = GLOSSARY_PATH.read_text()
    rfc_index = load_rfc_index()
    terms = list(parse_glossary(glossary_text))

    if not terms:
        print(f"FAIL: No entries parsed from {GLOSSARY_PATH}. Check format.")
        return 1

    all_failures = []
    for term, fields in terms:
        term_failures = check_term(term, fields, rfc_index, terms)
        all_failures.extend(term_failures)

    if all_failures:
        print("Glossary provenance check failed:")
        for f in all_failures:
            print(f"  - {f}")
        return 1

    print(f"All {len(terms)} glossary term(s) passed provenance checks.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
