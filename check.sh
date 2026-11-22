#!/bin/sh
# Full validation gate for AetherOS.
# Runs documentation validators + Rust reference validation.

set -e

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

echo "=== Documentation validators ==="
DOCS_DIR="$REPO_ROOT/docs"

# Python documentation validators
for validator in verify-authority.py verify-decision-records.py verify-generated.py verify-glossary.py verify-rfc.py; do
    VAL="$DOCS_DIR/tools/$validator"
    if [ -f "$VAL" ]; then
        echo "Running $validator..."
        python3 "$VAL" || python "$VAL"
    fi
done

echo "=== Rust reference validation ==="
REFERENCE_DIR="$REPO_ROOT/reference"
cd "$REFERENCE_DIR"

# cargo fmt
echo "Running cargo fmt..."
cargo fmt --check

# cargo check
echo "Running cargo check..."
cargo check --workspace

# cargo test
echo "Running cargo test..."
cargo test --workspace

# cargo clippy
echo "Running cargo clippy..."
cargo clippy --workspace

# cargo doc
echo "Running cargo doc..."
cargo doc --workspace --no-deps

# CTS runner
echo "Running CTS runner..."
cargo run -p aether-cts-runner

echo "=== All checks passed ==="