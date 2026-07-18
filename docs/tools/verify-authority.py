#!/usr/bin/env python3
"""
verify-authority.py

CI gate. Validates document authority hierarchy and Requirement-ID uniqueness.

Reads policy from `repository-policy.yaml` in the tools directory.
Fails when:
- A normative or constitutional document is missing its
  `Classification:` or `Authoritative Source:` header.
- A document's `Authoritative Source:` references a document whose
  Classification rank is lower than its own (Monotonicity).
- A normative or constitutional document is missing at least one
  explicit `Requirement-ID:` declaration.
- The same `Requirement-ID:` is declared by more than one normative
  or constitutional document (Completeness violation).
- A document declares a `Requirement-ID` whose namespace prefix is
  not in the accepted set defined in repository-policy.yaml.

Exits 0 on success, 1 on fail.
"""
from pathlib import Path
import re
import sys

import yaml

REPO_ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = REPO_ROOT / "tools" / "repository-policy.yaml"

CLASS_ORDER = {
    "Constitutional": 5,
    "Normative": 4,
    "Operational": 3,
    "Generated": 2,
    "Informative": 1,
}

CLASS_HEADER_RE = re.compile(
    r"^Classification:\s*([A-Z][A-Za-z]+)\s*$",
    re.MULTILINE,
)
SOURCE_HEADER_RE = re.compile(
    r"^Authoritative Source:\s*([^\n]+)\s*$",
    re.MULTILINE,
)
REQ_ID_RE = re.compile(r"^Requirement-ID:\s*([A-Z][A-Z0-9\-]+)\s*$", re.MULTILINE)
NAMESPACE_RE = re.compile(r"^([A-Z]+)-")


def load_policy():
    if not POLICY_PATH.exists():
        print(f"FAIL: {POLICY_PATH} not found.")
        sys.exit(1)
    with open(POLICY_PATH) as f:
        return yaml.safe_load(f)


def main():
    policy = load_policy()
    accepted_namespaces = set(policy.get("requirement_namespaces", []))
    if not accepted_namespaces:
        print("FAIL: repository-policy.yaml has no requirement_namespaces.")
        sys.exit(1)

    documents = {}
    for path in sorted(REPO_ROOT.rglob("*.md")):
        if any(part.startswith(".") for part in path.parts):
            continue
        documents[path] = path.read_text()

    failures = []
    name_index = {p.name: p for p in documents}
    id_to_owners = {}

    for path, text in documents.items():
        rel = path.relative_to(REPO_ROOT)
        classification_match = CLASS_HEADER_RE.search(text)
        if not classification_match:
            failures.append(f"{rel}: missing Classification header.")
            continue
        classification = classification_match.group(1).strip()
        if classification not in CLASS_ORDER:
            failures.append(
                f"{rel}: Classification '{classification}' is not a recognized value."
            )
            continue
        if classification in ("Constitutional", "Normative"):
            if not SOURCE_HEADER_RE.search(text):
                failures.append(f"{rel}: missing Authoritative Source header.")

        cls_rank = CLASS_ORDER[classification]
        source_match = SOURCE_HEADER_RE.search(text)
        if source_match:
            source_text = source_match.group(1)
            for source_ref in re.findall(r"[\w\-./]+\.md", source_text):
                target_name = source_ref.split("/")[-1]
                target = name_index.get(target_name)
                if target is None:
                    continue
                target_cls_match = CLASS_HEADER_RE.search(documents[target])
                if not target_cls_match:
                    continue
                target_cls = target_cls_match.group(1).strip()
                target_rank = CLASS_ORDER.get(target_cls, 0)
                if target_rank > cls_rank:
                    failures.append(
                        f"{rel} (Classification {classification}) references lower-classified "
                        f"document {target.name} (Classification {target_cls}) as Authoritative "
                        f"Source (Monotonicity violation)."
                    )

        if classification in ("Constitutional", "Normative"):
            ids = REQ_ID_RE.findall(text)
            if not ids:
                failures.append(
                    f"{rel}: classification is {classification} but no Requirement-ID declared. "
                    f"At least one explicit ID is required for completeness tracking."
                )
            for req_id in ids:
                ns_match = NAMESPACE_RE.match(req_id)
                if not ns_match:
                    failures.append(
                        f"{rel}: Requirement-ID '{req_id}' has no <NAMESPACE>- prefix."
                    )
                    continue
                ns = ns_match.group(1)
                if ns not in accepted_namespaces:
                    failures.append(
                        f"{rel}: Requirement-ID '{req_id}' namespace '{ns}' is not in the "
                        f"accepted set {sorted(accepted_namespaces)}."
                    )
                if req_id in id_to_owners and id_to_owners[req_id] != rel:
                    failures.append(
                        f"{rel}: Requirement-ID '{req_id}' is shared with "
                        f"{id_to_owners[req_id]} (Completeness violation)."
                    )
                else:
                    id_to_owners[req_id] = rel

    failures = sorted({f for f in failures})
    if failures:
        print(f"Authority check failed across {len(failures)} issue(s):")
        for f in failures:
            print(f"  - {f}")
        return 1

    print(
        f"All {len(documents)} documents passed Authority checks. "
        f"{len(id_to_owners)} unique Requirement-ID declarations tracked."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())