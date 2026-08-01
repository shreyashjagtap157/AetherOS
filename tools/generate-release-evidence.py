#!/usr/bin/env python3
"""Generate deterministic release evidence and an SPDX 2.3 source SBOM."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
CHECKS = [
    "repository-policy",
    "python-bytecode",
    "rust-format",
    "rust-check-locked",
    "rust-test-debug-locked",
    "rust-test-release-locked",
    "rust-clippy-deny-warnings",
    "rustdoc-deny-warnings",
    "reference-cts",
    "supply-chain-policy",
    "reproducibility",
]


def command(*args: str) -> str:
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def source_files() -> list[Path]:
    raw = command(
        "git",
        "ls-files",
        "--cached",
        "--others",
        "--exclude-standard",
        "-z",
    )
    paths = sorted(
        (ROOT / name for name in raw.split("\0") if name), key=lambda p: p.as_posix()
    )
    for path in paths:
        if path.is_symlink() or not path.resolve().is_relative_to(ROOT):
            raise ValueError(f"unsafe evidence input path: {path}")
    return paths


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def git_dirty() -> bool:
    return bool(command("git", "status", "--porcelain=v1"))


def cargo_packages() -> list[dict[str, Any]]:
    metadata = json.loads(
        command(
            "cargo",
            "metadata",
            "--manifest-path",
            "reference/Cargo.toml",
            "--locked",
            "--offline",
            "--format-version",
            "1",
        )
    )
    packages = []
    for package in sorted(metadata["packages"], key=lambda item: item["name"]):
        packages.append(
            {
                "SPDXID": f"SPDXRef-Package-{package['name']}",
                "name": package["name"],
                "versionInfo": package["version"],
                "downloadLocation": "NOASSERTION",
                "filesAnalyzed": False,
                "licenseConcluded": package.get("license") or "NOASSERTION",
                "licenseDeclared": package.get("license") or "NOASSERTION",
                "copyrightText": "NOASSERTION",
                "supplier": "Organization: AetherOS project",
            }
        )
    return packages


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", default="0.0.1")
    parser.add_argument("--output", type=Path, default=ROOT / "artifacts" / "evidence")
    parser.add_argument(
        "--checks-completed",
        action="store_true",
        help="assert that the caller completed every check recorded in the manifest",
    )
    args = parser.parse_args()
    if not args.checks_completed:
        parser.error("--checks-completed is required before emitting passing evidence")

    revision = command("git", "rev-parse", "HEAD")
    epoch = int(
        os.environ.get(
            "SOURCE_DATE_EPOCH", command("git", "show", "-s", "--format=%ct", "HEAD")
        )
    )
    files = [
        {
            "path": path.relative_to(ROOT).as_posix(),
            "sha256": digest(path),
            "bytes": path.stat().st_size,
        }
        for path in source_files()
        if path.is_file()
    ]
    evidence = {
        "schema_version": "1.0.0",
        "product_version": args.version,
        "source": {
            "revision": revision,
            "source_date_epoch": epoch,
            "dirty": git_dirty(),
        },
        "toolchain": {
            "rustc": command("rustc", "--version"),
            "cargo": command("cargo", "--version"),
            "python": platform.python_version(),
        },
        "checks": CHECKS,
        "files": files,
    }
    namespace = f"https://aetheros.invalid/spdx/{revision}/{args.version}"
    created = datetime.fromtimestamp(epoch, tz=timezone.utc).strftime(
        "%Y-%m-%dT%H:%M:%SZ"
    )
    sbom = {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": f"AetherOS-reference-{args.version}",
        "documentNamespace": namespace,
        "creationInfo": {
            "created": created,
            "creators": ["Tool: AetherOS-generate-release-evidence-1.0.0"],
        },
        "packages": cargo_packages(),
    }
    write_json(args.output / "release-evidence.json", evidence)
    write_json(args.output / "sbom.spdx.json", sbom)
    print(args.output.resolve())
    return 0


if __name__ == "__main__":
    sys.exit(main())
