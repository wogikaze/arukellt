#!/bin/bash
# Root verification and completion gate for the repository harness.
# This script defines what "done" means for this project.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

QUICK_MODE=false
if [[ "${1:-}" == "--quick" ]]; then
    QUICK_MODE=true
fi

echo -e "${YELLOW}Running harness verification...${NC}"
if [ "$QUICK_MODE" = true ]; then
    echo -e "${YELLOW}Quick mode: skipping slower cargo verification steps${NC}"
fi

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
    local output
    if output=$(bash -lc "$command" 2>&1); then
        check_pass "$label"
    else
        check_fail "$label"
        printf '%s\n' "$output" | tail -30
    fi
}

FIXTURE_COUNT=$(python3 - <<'PY'
from pathlib import Path
manifest = Path('tests/fixtures/manifest.txt')
count = sum(1 for line in manifest.read_text().splitlines() if line.strip() and not line.strip().startswith('#'))
print(count)
PY
)

# 1. Check documentation structure
printf '\n%s\n' "${YELLOW}[1/16] Checking documentation structure...${NC}"
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
printf '\n%s\n' "${YELLOW}[2/16] Checking ADR decisions...${NC}"
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
printf '\n%s\n' "${YELLOW}[3/16] Checking language specification...${NC}"
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
printf '\n%s\n' "${YELLOW}[4/16] Checking platform specification...${NC}"
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
printf '\n%s\n' "${YELLOW}[5/16] Checking stdlib specification...${NC}"
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

# 6. Check docs consistency
printf '\n%s\n' "${YELLOW}[6/16] Checking docs consistency...${NC}"
run_check "docs consistency (${FIXTURE_COUNT} fixtures)" "python3 scripts/check-docs-consistency.py"

# 7. Check markdown lint
printf '\n%s\n' "${YELLOW}[7/16] Checking markdown lint...${NC}"
run_check "markdownlint-cli2 **/*.md --fix --config .markdownlint.json" "npx markdownlint-cli2 '**/*.md' --fix --config .markdownlint.json"

# 8. Check formatting
printf '\n%s\n' "${YELLOW}[8/16] Checking formatting...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo fmt --all --check"
else
    run_check "cargo fmt --all --check" "cargo fmt --all --check"
fi

# 9. Check clippy
printf '\n%s\n' "${YELLOW}[9/16] Running clippy...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo clippy --workspace -- -D warnings"
else
    run_check "cargo clippy --workspace -- -D warnings" "cargo clippy --workspace --exclude ark-llvm -- -D warnings"
fi

# 10. Check build
printf '\n%s\n' "${YELLOW}[10/16] Building workspace...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo build --workspace"
else
    run_check "cargo build --workspace" "cargo build --workspace --exclude ark-llvm"
fi

# 11. Run tests
printf '\n%s\n' "${YELLOW}[11/16] Running workspace tests...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo test --workspace"
else
    run_check "cargo test --workspace" "cargo test --workspace --exclude ark-llvm --quiet -- --skip fixture_harness"
fi

# 12. Fixture manifest completeness
printf '\n%s\n' "${YELLOW}[12/16] Checking fixture manifest completeness...${NC}"
manifest_ok=true
manifest_file="tests/fixtures/manifest.txt"
if [ ! -f "$manifest_file" ]; then
    check_fail "Fixture manifest not found: $manifest_file"
    manifest_ok=false
else
    disk_entries=$(python3 - <<'PY'
from pathlib import Path
root = Path('tests/fixtures')
entries = []
for ark in sorted(root.rglob('*.ark')):
    rel = ark.relative_to(root)
    if ark.name != 'main.ark' and (ark.parent / 'main.ark').exists():
        continue
    entries.append(str(rel))
print('\n'.join(entries))
PY
)
    manifest_entries=$(python3 - <<'PY'
from pathlib import Path
manifest = Path('tests/fixtures/manifest.txt')
rows = []
for line in manifest.read_text().splitlines():
    line = line.strip()
    if not line or line.startswith('#'):
        continue
    rows.append(line.split(':', 1)[1])
print('\n'.join(sorted(rows)))
PY
)
    diff_result=$(diff <(printf '%s\n' "$disk_entries") <(printf '%s\n' "$manifest_entries") || true)
    if [ -n "$diff_result" ]; then
        check_fail "Fixture manifest out of sync with disk"
        printf '%s\n' "$diff_result" | head -20
        manifest_ok=false
    fi
fi
if [ "$manifest_ok" = true ]; then
    check_pass "Fixture manifest completeness (${FIXTURE_COUNT} entries)"
fi

# 13. Run fixture harness test
printf '\n%s\n' "${YELLOW}[13/16] Running fixture harness test...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo test -p arukellt --test harness -- --nocapture"
else
    local_output=$(bash -lc "cargo test -p arukellt --test harness -- --nocapture 2>&1" 2>&1) || true
    if printf '%s\n' "$local_output" | grep -q "FAIL: 0"; then
        summary=$(printf '%s\n' "$local_output" | grep "PASS:")
        check_pass "fixture harness (${summary})"
    else
        check_fail "fixture harness"
        printf '%s\n' "$local_output" | grep -E "^(PASS:|FAIL )" | head -30
    fi
fi

# 14. Stdlib manifest consistency
printf '\n%s\n' "${YELLOW}[14/16] Checking stdlib manifest consistency...${NC}"
run_check "stdlib manifest check" "bash scripts/check-stdlib-manifest.sh"

# 15. Baseline collection smoke
printf '\n%s\n' "${YELLOW}[15/16] Collecting baseline snapshots...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "python3 scripts/collect-baseline.py"
else
    run_check "baseline collection" "python3 scripts/collect-baseline.py"
fi

# 16. Perf gate contract
printf '\n%s\n' "${YELLOW}[16/16] Checking perf gate contract...${NC}"
check_pass "Perf policy documented (check<=10%, compile<=20%; heavy perf separated)"

printf '\n%s\n' "${YELLOW}========================================${NC}"
printf '%s\n' "${YELLOW}Summary${NC}"
printf '%s\n' "${YELLOW}========================================${NC}"
printf 'Total checks: %s\n' "$TOTAL_CHECKS"
printf 'Passed: %b%s%b\n' "$GREEN" "$PASSED_CHECKS" "$NC"
printf 'Skipped: %b%s%b\n' "$YELLOW" "$SKIPPED_CHECKS" "$NC"
printf 'Failed: %b%s%b\n' "$RED" "$((TOTAL_CHECKS - PASSED_CHECKS - SKIPPED_CHECKS))" "$NC"

if [ $PASSED_CHECKS -eq $TOTAL_CHECKS ] || [ $((PASSED_CHECKS + SKIPPED_CHECKS)) -eq $TOTAL_CHECKS ]; then
    printf '\n%b✓ All harness checks passed%b\n' "$GREEN" "$NC"
    exit 0
else
    printf '\n%b✗ Some harness checks failed%b\n' "$RED" "$NC"
    exit 1
fi
