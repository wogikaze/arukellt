#!/usr/bin/env bash
# test-opt-equivalence.sh — Verify that optimization passes preserve semantics.
#
# Runs all `run:` and `module-run:` fixtures at --opt-level 0 and --opt-level 1
# and asserts identical stdout. Failures indicate an optimization pass that
# changes program semantics.
#
# Usage:
#   bash scripts/test-opt-equivalence.sh              # all run fixtures
#   bash scripts/test-opt-equivalence.sh --quick       # first 50 fixtures only
#   bash scripts/test-opt-equivalence.sh --fixture X   # single fixture
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ARUKELLT="${REPO_ROOT}/target/debug/arukellt"
FIXTURE_DIR="${REPO_ROOT}/tests/fixtures"
MANIFEST="${FIXTURE_DIR}/manifest.txt"

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
NC=$'\033[0m'

QUICK=false
SINGLE_FIXTURE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --quick) QUICK=true ;;
        --fixture) shift; SINGLE_FIXTURE="$1" ;;
        --help|-h)
            echo "Usage: bash scripts/test-opt-equivalence.sh [--quick] [--fixture path]"
            exit 0
            ;;
    esac
    shift
done

if [[ ! -x "$ARUKELLT" ]]; then
    echo "${RED}error: arukellt not found at $ARUKELLT (run cargo build -p arukellt first)${NC}"
    exit 1
fi

PASS=0
FAIL=0
SKIP=0
FAILURES=""

run_one() {
    local fixture="$1"
    local full_path="${FIXTURE_DIR}/${fixture}"
    local expected_path="${full_path%.ark}.expected"

    if [[ ! -f "$full_path" ]]; then
        ((SKIP++)) || true
        return
    fi
    if [[ ! -f "$expected_path" ]]; then
        ((SKIP++)) || true
        return
    fi

    # Run at O0 (no optimizations)
    local out_o0
    out_o0=$("$ARUKELLT" run --opt-level 0 "$full_path" 2>/dev/null) || true

    # Run at O1 (safe optimizations)
    local out_o1
    out_o1=$("$ARUKELLT" run --opt-level 1 "$full_path" 2>/dev/null) || true

    if [[ "$out_o0" == "$out_o1" ]]; then
        ((PASS++)) || true
    else
        ((FAIL++)) || true
        FAILURES="${FAILURES}  FAIL: ${fixture}\n    O0: $(echo "$out_o0" | head -1)\n    O1: $(echo "$out_o1" | head -1)\n"
    fi
}

echo "${YELLOW}Optimization equivalence test: O0 vs O1${NC}"

if [[ -n "$SINGLE_FIXTURE" ]]; then
    run_one "$SINGLE_FIXTURE"
else
    COUNT=0
    while IFS= read -r line; do
        line=$(echo "$line" | xargs)
        [[ -z "$line" || "$line" == \#* ]] && continue
        kind="${line%%:*}"
        path="${line#*:}"
        # Only test run/module-run fixtures (those that produce stdout)
        [[ "$kind" == "run" || "$kind" == "module-run" ]] || continue
        run_one "$path"
        ((COUNT++)) || true
        if [[ "$QUICK" == "true" && $COUNT -ge 50 ]]; then
            break
        fi
    done < "$MANIFEST"
fi

echo ""
echo "${GREEN}Passed: $PASS${NC}  ${RED}Failed: $FAIL${NC}  Skipped: $SKIP"
if [[ $FAIL -gt 0 ]]; then
    echo ""
    printf "%b" "$FAILURES"
    exit 1
fi
echo "${GREEN}All fixtures produce identical output at O0 and O1.${NC}"
