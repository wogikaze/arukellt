#!/usr/bin/env bash
# scripts/pre-push-verify.sh — Comprehensive verification matching GitHub Actions CI.
#
# This script aligns with the CI pipeline (ADR-013) to ensure that local
# push attempts are gated by the same standards as the remote merge gates.
#
# Layer structure (matches .github/workflows/ci.yml):
# 1. unit (cargo test, clippy, fmt)
# 2. fixture/T3 (wasm32-wasi-p2 fixtures)
# 3. integration (CLI smoke tests)
# 4. packaging (binary smoke tests)
# 5. determinism (same input -> same output)
# 6. selfhost (Rust compiler stage 0)
# 7. extension (VS Code extension tests, if tools available)
# 8. heavy (size, wat, docs)
# 9. interop (component model interop)

set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

# Strict mode for CI-equivalence
export RUSTFLAGS="-D warnings"
export CARGO_TERM_COLOR=always

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}=== arukellt Pre-Push Verification (CI Equivalence Mode) ===${NC}"

step() {
    echo -e "\n${YELLOW}── Layer: $1 ──${NC}"
}

# 1. Unit tests, clippy, fmt, and docs (CI: unit-tests + heavy-checks)
step "Unit, Clippy, Fmt, and Docs"
bash scripts/verify-harness.sh --cargo --docs --size --wat

# 2. Build release binary for subsequent layers
step "Build release binary"
cargo build --release -p arukellt

# 3. Fixture suite (CI: fixture-primary + fixture-supported)
step "Fixture suite (T3 primary: wasm32-wasi-p2)"
ARUKELLT_TARGET=wasm32-wasi-p2 bash scripts/verify-harness.sh --fixtures

step "Fixture suite (T1 supported: wasm32-wasi-p1)"
ARUKELLT_TARGET=wasm32-wasi-p1 bash scripts/verify-harness.sh --fixtures || echo -e "  ${YELLOW}⚠ T1 fixtures failed (supported target, non-blocking)${NC}"

# 4. Integration & Packaging (CI: integration + packaging)
step "Integration & Packaging smoke tests"
bash scripts/smoke-test-binary.sh ./target/release/arukellt
bash scripts/test-package-workspace.sh

# 5. Determinism (CI: determinism)
step "Determinism check"
TMP_DET_A=$(mktemp)
TMP_DET_B=$(mktemp)
HELLO_ARK="docs/examples/hello.ark"

if [ -f "$HELLO_ARK" ]; then
    echo "  Checking T3 determinism..."
    ./target/release/arukellt compile --target wasm32-wasi-p2 --output "$TMP_DET_A" "$HELLO_ARK"
    ./target/release/arukellt compile --target wasm32-wasi-p2 --output "$TMP_DET_B" "$HELLO_ARK"
    diff "$TMP_DET_A" "$TMP_DET_B" && echo -e "  ${GREEN}✓ T3 deterministic${NC}"

    echo "  Checking T1 determinism..."
    ./target/release/arukellt compile --target wasm32-wasi-p1 --output "$TMP_DET_A" "$HELLO_ARK"
    ./target/release/arukellt compile --target wasm32-wasi-p1 --output "$TMP_DET_B" "$HELLO_ARK"
    diff "$TMP_DET_A" "$TMP_DET_B" && echo -e "  ${GREEN}✓ T1 deterministic${NC}"
else
    echo -e "  ${YELLOW}⊙ Skipping determinism (hello.ark not found)${NC}"
fi
rm -f "$TMP_DET_A" "$TMP_DET_B"

# 6. Selfhost Stage 0 (CI: selfhost-stage0)
step "Selfhost Stage 0 (Rust compiler → Stage 1)"
bash scripts/verify-bootstrap.sh --stage1-only

# 7. Component Interop (CI: component-interop)
step "Component Interop"
bash scripts/verify-harness.sh --component

# 8. Extension Tests (CI: extension-tests)
step "VS Code Extension Tests"
if command -v npm >/dev/null 2>&1 && command -v xvfb-run >/dev/null 2>&1; then
    if [ -d "extensions/arukellt-all-in-one" ]; then
        (cd extensions/arukellt-all-in-one && npm install --quiet && xvfb-run -a npm test)
        echo -e "  ${GREEN}✓ Extension tests passed${NC}"
    else
        echo -e "  ${YELLOW}⊙ Skipping extension tests (directory not found)${NC}"
    fi
else
    echo -e "  ${YELLOW}⊙ Skipping extension tests (npm or xvfb-run not found)${NC}"
fi

echo -e "\n${GREEN}==============================================${NC}"
echo -e "${GREEN}✓ All CI-equivalent layers passed successfully!${NC}"
echo -e "${GREEN}==============================================${NC}"
