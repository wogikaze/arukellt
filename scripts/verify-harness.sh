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

# 1. Check documentation structure
echo -e "\n${YELLOW}[1/5] Checking documentation structure...${NC}"
doc_ok=true
for doc in "AGENTS.md" "docs/agent-harness.md"; do
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
echo -e "\n${YELLOW}[2/5] Checking ADR decisions...${NC}"
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
echo -e "\n${YELLOW}[3/5] Checking language specification...${NC}"
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
echo -e "\n${YELLOW}[4/5] Checking platform specification...${NC}"
platform_ok=true
for pdoc in "docs/platform/wasm-features.md" "docs/abi.md" "docs/wasi-resource-model.md"; do
    if [ ! -f "$pdoc" ]; then
        check_fail "Missing: $pdoc"
        platform_ok=false
    fi
done
if [ "$platform_ok" = true ]; then
    check_pass "Platform specification OK"
fi

# 5. Check stdlib documents
echo -e "\n${YELLOW}[5/5] Checking stdlib specification...${NC}"
stdlib_ok=true
for sdoc in "docs/stdlib/README.md" "docs/core.md" "docs/io.md"; do
    if [ ! -f "$sdoc" ]; then
        check_fail "Missing: $sdoc"
        stdlib_ok=false
    fi
done
if [ "$stdlib_ok" = true ]; then
    check_pass "Stdlib specification OK"
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
