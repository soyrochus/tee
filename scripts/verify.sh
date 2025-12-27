#!/usr/bin/env bash
set -euo pipefail

export SQLX_OFFLINE=true

echo "==> rustfmt"
cargo fmt --all -- --check

echo "==> build"
cargo build --locked

echo "==> clippy"
cargo clippy --all-targets --all-features --locked -- -D warnings

echo "==> test"
cargo test --all --locked

echo "All checks passed."
