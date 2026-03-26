#!/bin/bash
# Root verification and completion gate for the repository harness.
# This script defines what "done" means for this project.

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse arguments
QUICK_MODE=false
if [[ "${1:-}" == "--quick" ]]; then
    QUICK_MODE=true
fi

echo -e "${YELLOW}Running harness verification...${NC}"
if [ "$QUICK_MODE" = true ]; then
    echo -e "${YELLOW}Quick mode: skipping slower cargo verification steps${NC}"
fi

# Counter for checks
TOTAL_CHECKS=0
PASSED_CHECKS=0
SKIPPED_CHECKS=0

check_pass() {
    echo -e "${GREEN}✓ $1${NC}"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
}

check_skip() {
    echo -e "${YELLOW}⊙ $1 (skipped)${NC}"
    SKIPPED_CHECKS=$((SKIPPED_CHECKS + 1))
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
}

check_fail() {
    echo -e "${RED}✗ $1${NC}"
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
}

run_check() {
    local label="$1"
    local command="$2"

    if bash -lc "$command"; then
        check_pass "$label"
    else
        check_fail "$label"
    fi
}

# 1. Check documentation structure
echo -e "\n${YELLOW}[1/10] Checking documentation structure...${NC}"
doc_ok=true
for doc in "AGENTS.md" "docs/process/agent-harness.md"; do
    if [ ! -f "$doc" ]; then
        check_fail "$doc not found"
        doc_ok=false
    fi
done
for dir in "docs/adr" "issues/open" "issues/done" "docs/language" "docs/platform" "docs/stdlib" "docs/process"; do
    if [ ! -d "$dir" ]; then
        check_fail "$dir/ directory not found"
        doc_ok=false
    fi
done
if [ "$doc_ok" = true ]; then
    check_pass "Documentation structure OK"
fi

# 2. Check ADR decisions
echo -e "\n${YELLOW}[2/10] Checking ADR decisions...${NC}"
adr_ok=true
for adr in "docs/adr/ADR-002-memory-model.md" "docs/adr/ADR-003-generics-strategy.md" "docs/adr/ADR-004-trait-strategy.md" "docs/adr/ADR-005-llvm-scope.md" "docs/adr/ADR-006-abi-policy.md"; do
    if [ ! -f "$adr" ]; then
        check_fail "Missing: $adr"
        adr_ok=false
    elif ! grep -q "DECIDED\|決定" "$adr"; then
        check_fail "Not decided: $adr"
        adr_ok=false
    fi
done
if [ "$adr_ok" = true ]; then
    check_pass "All required ADRs decided"
fi

# 3. Check language spec documents
echo -e "\n${YELLOW}[3/10] Checking language specification...${NC}"
spec_ok=true
for spec in "docs/language/memory-model.md" "docs/language/type-system.md" "docs/language/syntax.md"; do
    if [ ! -f "$spec" ]; then
        check_fail "Missing: $spec"
        spec_ok=false
    fi
done
if [ "$spec_ok" = true ]; then
    check_pass "Language specification OK"
fi

# 4. Check platform documents
echo -e "\n${YELLOW}[4/10] Checking platform specification...${NC}"
platform_ok=true
for pdoc in "docs/platform/wasm-features.md" "docs/platform/abi.md" "docs/platform/wasi-resource-model.md"; do
    if [ ! -f "$pdoc" ]; then
        check_fail "Missing: $pdoc"
        platform_ok=false
    fi
done
if [ "$platform_ok" = true ]; then
    check_pass "Platform specification OK"
fi

# 5. Check stdlib documents
echo -e "\n${YELLOW}[5/10] Checking stdlib specification...${NC}"
stdlib_ok=true
for sdoc in "docs/stdlib/README.md" "docs/stdlib/core.md" "docs/stdlib/io.md"; do
    if [ ! -f "$sdoc" ]; then
        check_fail "Missing: $sdoc"
        stdlib_ok=false
    fi
done
if [ "$stdlib_ok" = true ]; then
    check_pass "Stdlib specification OK"
fi

# 6. Check markdown lint
echo -e "\n${YELLOW}[6/11] Checking markdown lint...${NC}"
run_check "markdownlint-cli2 **/*.md" "npx markdownlint-cli2 '**/*.md'"

# 7. Check formatting
echo -e "\n${YELLOW}[7/11] Checking formatting...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo fmt --all --check"
else
    run_check "cargo fmt --all --check" "cargo fmt --all --check"
fi

# 8. Check clippy
echo -e "\n${YELLOW}[8/11] Running clippy...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo clippy --workspace -- -D warnings"
else
    run_check "cargo clippy --workspace -- -D warnings" "cargo clippy --workspace -- -D warnings"
fi

# 9. Check build
echo -e "\n${YELLOW}[9/11] Building workspace...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo build --workspace"
else
    run_check "cargo build --workspace" "cargo build --workspace"
fi

# 10. Run tests
echo -e "\n${YELLOW}[10/11] Running workspace tests...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo test --workspace"
else
    run_check "cargo test --workspace" "cargo test --workspace"
fi

# 11. Run fixture harness discovery test
echo -e "\n${YELLOW}[11/11] Running fixture harness test...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo test -p arukellt --test harness -- --nocapture"
else
    run_check "cargo test --test harness -- --nocapture" "cargo test -p arukellt --test harness -- --nocapture"
fi

# Summary
echo -e "\n${YELLOW}========================================${NC}"
echo -e "${YELLOW}Summary${NC}"
echo -e "${YELLOW}========================================${NC}"
echo -e "Total checks: $TOTAL_CHECKS"
echo -e "Passed: ${GREEN}$PASSED_CHECKS${NC}"
echo -e "Skipped: ${YELLOW}$SKIPPED_CHECKS${NC}"
echo -e "Failed: ${RED}$((TOTAL_CHECKS - PASSED_CHECKS - SKIPPED_CHECKS))${NC}"

if [ $PASSED_CHECKS -eq $TOTAL_CHECKS ] || [ $((PASSED_CHECKS + SKIPPED_CHECKS)) -eq $TOTAL_CHECKS ]; then
    echo -e "\n${GREEN}✓ All harness checks passed${NC}"
    exit 0
else
    echo -e "\n${RED}✗ Some harness checks failed${NC}"
    exit 1
fi

# NOTE:
# The current fixture harness test only verifies fixture discovery.
# It does not yet execute compile/run assertions for every fixture.
# See tests/harness.rs:52-55 and tests/harness.rs:61-65.
