#!/usr/bin/env python3
"""Read and output all key files in the AetherOS specification suite (docs/)."""
import os
# import sys

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

files_to_read = [
    "Platform-Architecture-Specification-v1.1.md",
    "PRE-IMPLEMENTATION-DEPENDENCY-MATRIX.md",
    "IMPLEMENTATION-ROADMAP.md",
    "IMPLEMENTATION-STATUS.md",
    "ARCHITECTURE-GOVERNANCE.md",
    "RFC-GOV-002.md",
    "REVIEWER-GUIDE.md",
    "REPOSITORY-PRINCIPLES.md",
    "CHANGELOG.md",
    "Revision-History.md",
    "OBSERVATION-RECORD.md",
    "Conformance-Test-Suite-Methodology.md",
    "Profiles-Specification.md",
    "Hardware-Support-RFC.md",
    "README.md",
    "generated/DECISION-INDEX.md",
]

for fname in files_to_read:
    fpath = os.path.join(BASE, fname)
    if os.path.exists(fpath):
        print(f"\n{'='*80}")
        print(f"FILE: {fname}")
        print(f"{'='*80}")
        with open(fpath, 'r', encoding='utf-8') as f:
            content = f.read()
        # Truncate very large files
        if len(content) > 15000:
            print(content[:7500])
            print(f"\n... [TRUNCATED - {len(content)} total chars, showing first 7500] ...\n")
            print(content[-3000:])
        else:
            print(content)
    else:
        print(f"\nFILE NOT FOUND: {fname}")

# Also read all RFCs
rfc_dirs = [
    "RFCs/00-Composition",
    "RFCs/01-Execution",
    "RFCs/02-State",
    "RFCs/03-Communication",
    "RFCs/04-Observability",
    "RFCs/05-Cross-Cutting",
]

for rfc_dir in rfc_dirs:
    dirpath = os.path.join(BASE, rfc_dir)
    if os.path.isdir(dirpath):
        for fname in sorted(os.listdir(dirpath)):
            if fname.endswith('.md'):
                fpath = os.path.join(dirpath, fname)
                print(f"\n{'='*80}")
                print(f"FILE: {rfc_dir}/{fname}")
                print(f"{'='*80}")
                with open(fpath, 'r', encoding='utf-8') as f:
                    content = f.read()
                if len(content) > 10000:
                    print(content[:5000])
                    print(f"\n... [TRUNCATED - {len(content)} total chars] ...\n")
                    print(content[-2000:])
                else:
                    print(content)
