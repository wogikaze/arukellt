#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "[verify-harness] cargo fmt --check"
cargo fmt --check

echo "[verify-harness] cargo clippy --workspace --lib --bins -- -D warnings"
cargo clippy --workspace --lib --bins -- -D warnings

echo "[verify-harness] cargo test -p arktc --test workboard"
cargo test -p arktc --test workboard

echo "[verify-harness] cargo test"
cargo test
