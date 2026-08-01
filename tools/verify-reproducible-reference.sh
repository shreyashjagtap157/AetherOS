#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
TMP=${TMPDIR:-/tmp}/aether-repro-$$
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
mkdir -p "$TMP/one" "$TMP/two"

export CARGO_INCREMENTAL=0
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(git -C "$ROOT" show -s --format=%ct HEAD)}
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$ROOT=. -C strip=symbols"

(
  cd "$ROOT/reference"
  CARGO_TARGET_DIR="$TMP/one" cargo build --locked --offline --release -p aether-cts-runner
  CARGO_TARGET_DIR="$TMP/two" cargo build --locked --offline --release -p aether-cts-runner
)
cmp "$TMP/one/release/aether-cts-runner" "$TMP/two/release/aether-cts-runner"
"$TMP/one/release/aether-cts-runner" > "$TMP/one.jsonl"
"$TMP/two/release/aether-cts-runner" > "$TMP/two.jsonl"
cmp "$TMP/one.jsonl" "$TMP/two.jsonl"
sha256sum "$TMP/one/release/aether-cts-runner"
