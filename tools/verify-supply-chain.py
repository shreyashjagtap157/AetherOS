#!/usr/bin/env python3
"""Fail closed on undeclared dependency sources or package licenses."""

from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
ALLOWED_LICENSES = {"Apache-2.0 OR MIT", "MIT OR Apache-2.0"}
ALLOWED_REGISTRIES = {"registry+https://github.com/rust-lang/crates.io-index"}


def main() -> int:
    metadata = json.loads(
        subprocess.check_output(
            [
                "cargo",
                "metadata",
                "--manifest-path",
                "reference/Cargo.toml",
                "--locked",
                "--offline",
                "--format-version",
                "1",
            ],
            cwd=ROOT,
            text=True,
        )
    )
    failures: list[str] = []
    for package in metadata["packages"]:
        source = package.get("source")
        if source is not None and source not in ALLOWED_REGISTRIES:
            failures.append(f"{package['name']}: undeclared source {source}")
        license_expression = package.get("license")
        if license_expression not in ALLOWED_LICENSES:
            failures.append(
                f"{package['name']}: disallowed or missing license {license_expression!r}"
            )
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print(f"Supply-chain policy passed for {len(metadata['packages'])} package(s).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
