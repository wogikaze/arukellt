#!/usr/bin/env bash
# scripts/ci-full-local.sh — Full local CI gate.
#
# Equivalent to the complete GitHub Actions CI pipeline.
# Run before releases or after large changes. NOT run on every push.
#
# Usage:
#   bash scripts/ci-full-local.sh             # all layers
#   bash scripts/ci-full-local.sh --skip-ext  # skip extension tests (no xvfb)
#
# Layer structure (matches .github/workflows/ci.yml):
#   1. unit         cargo fmt + clippy + test
#   2. docs         freshness + consistency + links
#   3. fixtures-t3  wasm32-wasi-p2 (primary)
#   4. fixtures-t1  wasm32-wasi-p1 (supported, non-blocking)
#   5. release      cargo build --release
#   6. integration  smoke tests + packaging
#   7. determinism  same input → same output (T1 + T3)
#   8. selfhost     verify-bootstrap --stage1-only
#   9. component    component interop + size + wat
#  10. extension    VS Code extension tests

set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

export RUSTFLAGS="-D warnings"
export CARGO_TERM_COLOR=always

SKIP_EXT=0
for arg in "$@"; do
    [[ "$arg" == "--skip-ext" ]] && SKIP_EXT=1
done

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASS=0
FAIL=0
SKIP=0

step() { echo -e "\n${YELLOW}══ Layer $1: $2 ══${NC}"; }
ok()   { echo -e "  ${GREEN}✓ $1${NC}"; PASS=$((PASS+1)); }
skip() { echo -e "  ${YELLOW}⊙ $1 (skipped)${NC}"; SKIP=$((SKIP+1)); }
fail() { echo -e "  ${RED}✗ $1${NC}"; FAIL=$((FAIL+1)); }

echo -e "${YELLOW}=== arukellt Full Local CI ===${NC}"

# ── 1. Unit ──────────────────────────────────────────────────────────────────
step 1 "Unit (fmt + clippy + test)"
cargo fmt --check --all
cargo clippy --workspace --exclude ark-llvm --exclude ark-lsp --all-targets -- -D warnings
cargo test --workspace --exclude ark-llvm --exclude ark-lsp
ok "Unit checks"

# ── 2. Docs ──────────────────────────────────────────────────────────────────
step 2 "Docs (freshness + consistency)"
bash scripts/run/verify-harness.sh --docs
python3 scripts/check-docs-consistency.py
ok "Docs checks"

# ── 3. T3 Fixtures ───────────────────────────────────────────────────────────
step 3 "Fixtures T3 (wasm32-wasi-p2)"
ARUKELLT_TARGET=wasm32-wasi-p2 bash scripts/run/verify-harness.sh --fixtures
ok "T3 fixtures"

# ── 4. T1 Fixtures (non-blocking) ────────────────────────────────────────────
step 4 "Fixtures T1 (wasm32-wasi-p1, non-blocking)"
if ARUKELLT_TARGET=wasm32-wasi-p1 bash scripts/run/verify-harness.sh --fixtures 2>&1; then
    ok "T1 fixtures"
else
    fail "T1 fixtures (non-blocking — recorded but not fatal)"
fi

# ── 5. Release build ─────────────────────────────────────────────────────────
step 5 "Release build"
cargo build --release -p arukellt
ok "Release build"

# ── 6. Integration & Packaging ───────────────────────────────────────────────
step 6 "Integration & Packaging"
if [ -x scripts/smoke-test-binary.sh ]; then
    bash scripts/run/smoke-test-binary.sh ./target/release/arukellt
fi
if [ -x scripts/test-package-workspace.sh ]; then
    bash scripts/run/test-package-workspace.sh
fi
ok "Integration & packaging"

# ── 7. Determinism ───────────────────────────────────────────────────────────
step 7 "Determinism"
HELLO_ARK="docs/examples/hello.ark"
if [ -f "$HELLO_ARK" ]; then
    TMP_A=$(mktemp) TMP_B=$(mktemp)
    ./target/release/arukellt compile --target wasm32-wasi-p2 --output "$TMP_A" "$HELLO_ARK"
    ./target/release/arukellt compile --target wasm32-wasi-p2 --output "$TMP_B" "$HELLO_ARK"
    diff "$TMP_A" "$TMP_B" && ok "T3 deterministic"
    ./target/release/arukellt compile --target wasm32-wasi-p1 --output "$TMP_A" "$HELLO_ARK"
    ./target/release/arukellt compile --target wasm32-wasi-p1 --output "$TMP_B" "$HELLO_ARK"
    diff "$TMP_A" "$TMP_B" && ok "T1 deterministic"
    rm -f "$TMP_A" "$TMP_B"
else
    skip "Determinism (hello.ark not found)"
fi

# ── 8. Selfhost Stage 0 ──────────────────────────────────────────────────────
step 8 "Selfhost Stage 0"
bash scripts/run/verify-bootstrap.sh --stage1-only
ok "Selfhost stage 0"

# ── 9. Component Interop + Size + WAT ────────────────────────────────────────
step 9 "Component interop + size + WAT"
bash scripts/run/verify-harness.sh --component
bash scripts/run/verify-harness.sh --size --wat
ok "Component + size + WAT"

# ── 10. Extension Tests ───────────────────────────────────────────────────────
step 10 "VS Code Extension Tests"
if [ "$SKIP_EXT" -eq 1 ]; then
    skip "Extension tests (--skip-ext)"
elif command -v npm >/dev/null 2>&1 && command -v xvfb-run >/dev/null 2>&1; then
    (cd extensions/arukellt-all-in-one && npm ci --quiet && xvfb-run -a npm test)
    ok "Extension tests"
else
    skip "Extension tests (npm or xvfb-run not found)"
fi

# ── Summary ──────────────────────────────────────────────────────────────────
echo -e "\n${YELLOW}══════════════════════════════════════════${NC}"
echo -e "  Passed: ${GREEN}${PASS}${NC}  Failed: ${RED}${FAIL}${NC}  Skipped: ${YELLOW}${SKIP}${NC}"
if [ "$FAIL" -gt 0 ]; then
    echo -e "${RED}✗ Full CI failed${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Full CI passed${NC}"
fi
