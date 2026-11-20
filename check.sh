#!/bin/sh
# Full validation gate for AetherOS.
# Runs documentation validators + Rust reference validation.

set -e

echo "=== Documentation validators ==="
cd docs 2>/dev/null || true

# Python documentation validators
for validator in tools/verify-authority.py tools/verify-decision-records.py tools/verify-generated.py tools/verify-glossary.py tools/verify-rfc.py; do
    if [ -f "$validator" ]; then
        echo "Running $validator..."
        python3 "$validator" || python "$validator"
    fi
done

echo "=== Rust reference validation ==="
cd reference

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