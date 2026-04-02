#!/usr/bin/env bash
# scripts/pre-push-verify.sh — Lightweight pre-push gate (target: 2-5 min).
#
# Runs fast checks only. Heavy CI (release build, selfhost, component interop,
# extension tests, determinism, T1 fixtures) lives in scripts/ci-full-local.sh.
#
# Change-based filtering skips irrelevant checks when only docs/issues changed.

set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

export RUSTFLAGS="-D warnings"
export CARGO_TERM_COLOR=always

YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

echo -e "${YELLOW}=== arukellt Pre-Push (lightweight gate) ===${NC}"

# Detect what changed: use merge-base for accuracy with complex histories
UPSTREAM_REF=$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || true)
if [ -n "${UPSTREAM_REF:-}" ]; then
    BASE=$(git merge-base HEAD "$UPSTREAM_REF")
    CHANGED=$(git diff --name-only "$BASE"...HEAD)
else
    CHANGED=$(git diff --name-only HEAD~1...HEAD 2>/dev/null || true)
fi

has_rust_changes() {
    echo "$CHANGED" | grep -qE '^(crates/|src/|tests/|benches/|examples/|build\.rs|Cargo\.toml|Cargo\.lock)' 2>/dev/null
}
has_doc_changes() {
    echo "$CHANGED" | grep -qE '^(docs/|issues/|scripts/generate-docs\.py|scripts/generate-issue-index\.sh|std/manifest\.toml|README\.md)' 2>/dev/null
}
has_fixture_changes() {
    echo "$CHANGED" | grep -qE '^(tests/fixtures/|std/)' 2>/dev/null
}
has_extension_changes() {
    echo "$CHANGED" | grep -qE '^extensions/' 2>/dev/null
}

# ── 1. Rust: fmt + clippy + test (skip if only docs/issues changed) ──────────
if has_rust_changes || has_fixture_changes || [ -z "$CHANGED" ]; then
    echo -e "\n${YELLOW}── Rust checks ──${NC}"
    cargo fmt --check --all
    cargo clippy --workspace --exclude ark-llvm --exclude ark-lsp --all-targets -- -D warnings
    cargo test --workspace --exclude ark-llvm --exclude ark-lsp
else
    echo -e "\n⊙ No Rust changes — skipping cargo fmt/clippy/test"
fi

# ── 2. Docs freshness (skip if no doc/script changes) ────────────────────────
if has_doc_changes || has_rust_changes || [ -z "$CHANGED" ]; then
    echo -e "\n${YELLOW}── Docs freshness ──${NC}"
    python3 scripts/generate-docs.py --check
    bash scripts/generate-issue-index.sh
    git diff --exit-code -- docs/ issues/ README.md
else
    echo -e "\n⊙ No doc changes — skipping docs freshness"
fi

# ── 3. T3 fixture suite (skip if no Rust/fixture changes) ────────────────────
if has_rust_changes || has_fixture_changes || [ -z "$CHANGED" ]; then
    echo -e "\n${YELLOW}── T3 fixtures (wasm32-wasi-p2) ──${NC}"
    ARUKELLT_TARGET=wasm32-wasi-p2 bash scripts/verify-harness.sh --fixtures
else
    echo -e "\n⊙ No Rust/fixture changes — skipping T3 fixtures"
fi

# ── 4. Extension syntax check (only when extension files changed) ─────────────
if has_extension_changes; then
    echo -e "\n${YELLOW}── Extension syntax check ──${NC}"
    if [ -f extensions/arukellt-all-in-one/src/extension.js ]; then
        node --check extensions/arukellt-all-in-one/src/extension.js
    fi
    if [ -f extensions/arukellt-all-in-one/package.json ]; then
        python3 -c "import json; json.load(open('extensions/arukellt-all-in-one/package.json'))" \
            && echo "  ✓ package.json valid JSON"
    fi
fi

echo -e "\n${GREEN}=== Pre-push passed ===${NC}"
echo -e "${GREEN}For full CI checks (release, selfhost, component, extension): bash scripts/ci-full-local.sh${NC}"
