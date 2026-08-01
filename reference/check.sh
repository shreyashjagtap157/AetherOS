#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")"

cargo fmt --all -- --check
cargo check --locked --workspace --all-targets --all-features
cargo test --locked --workspace --all-features
cargo test --locked --workspace --all-features --release
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps
cargo run --locked -p aether-cts-runner
