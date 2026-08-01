#!/usr/bin/env python3
"""Validate generated evidence without a third-party JSON Schema runtime."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from pathlib import PurePosixPath
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
SHA256 = re.compile(r"^[0-9a-f]{64}$")
SEMVER = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")


def fail(message: str) -> None:
    raise ValueError(message)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "path",
        type=Path,
        nargs="?",
        default=ROOT / "artifacts/evidence/release-evidence.json",
    )
    args = parser.parse_args()
    document = json.loads(args.path.read_text(encoding="utf-8"))
    expected = {
        "schema_version",
        "product_version",
        "source",
        "toolchain",
        "checks",
        "files",
    }
    if set(document) != expected:
        fail("unexpected top-level evidence fields")
    if document["schema_version"] != "1.0.0" or not SEMVER.fullmatch(
        document["product_version"]
    ):
        fail("invalid schema or product version")
    source = document["source"]
    if set(source) != {"revision", "source_date_epoch", "dirty"}:
        fail("invalid source identity")
    if not re.fullmatch(r"[0-9a-f]{40}", source["revision"]):
        fail("invalid source revision")
    paths: list[str] = []
    for entry in document["files"]:
        if set(entry) != {"path", "sha256", "bytes"} or not SHA256.fullmatch(
            entry["sha256"]
        ):
            fail("invalid file evidence entry")
        relative = PurePosixPath(entry["path"])
        if relative.is_absolute() or ".." in relative.parts:
            fail(f"unsafe file evidence path: {entry['path']}")
        path = ROOT.joinpath(*relative.parts)
        if path.is_symlink() or not path.resolve().is_relative_to(ROOT):
            fail(f"unsafe file evidence path: {entry['path']}")
        if not path.is_file() or path.stat().st_size != entry["bytes"]:
            fail(f"file missing or size changed: {entry['path']}")
        if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
            fail(f"file digest changed: {entry['path']}")
        paths.append(entry["path"])
    if paths != sorted(set(paths)):
        fail("file evidence is not sorted and unique")
    print(f"Release evidence passed for {len(paths)} file(s).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
