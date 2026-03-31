#!/usr/bin/env bash
# Package-workspace manifest validation tests.
#
# Verifies that ark-manifest handles valid and invalid ark.toml correctly.
# These are integration-level tests that exercise the manifest loading,
# project root discovery, and diagnostic output paths.
#
# Usage:
#   bash scripts/test-package-workspace.sh
#   ARUKELLT_BIN=/path/to/arukellt bash scripts/test-package-workspace.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
FIXTURE_DIR="$REPO_ROOT/tests/package-workspace"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASS=0
FAIL=0
SKIP=0

check() {
    if [[ ! -x "$ARUKELLT" ]]; then
        echo -e "${YELLOW}SKIP${NC} all tests (arukellt not found at $ARUKELLT)"
        SKIP=$((SKIP + 1))
        return
    fi
}

run_test() {
    local desc="$1"
    local cmd="$2"
    local expected_exit="${3:-0}"

    local actual_exit=0
    eval "$cmd" > /tmp/pw_test_out.txt 2>&1 || actual_exit=$?

    if [[ "$actual_exit" -eq "$expected_exit" ]]; then
        echo -e "  ${GREEN}PASS${NC}  $desc"
        PASS=$((PASS + 1))
    else
        echo -e "  ${RED}FAIL${NC}  $desc"
        echo "       Expected exit $expected_exit, got $actual_exit"
        cat /tmp/pw_test_out.txt | head -5 | sed 's/^/       /'
        FAIL=$((FAIL + 1))
    fi
}

check

echo "Package/workspace manifest tests"
echo

echo "── Basic project ──"
BASIC="$FIXTURE_DIR/basic-project"

# Test 1: build compiles the basic project
run_test "arukellt build in basic-project succeeds" \
    "cd '$BASIC' && '$ARUKELLT' build 2>&1 | grep -q ''" \
    0

# Test 2: arukellt build a second time also succeeds (idempotent)
run_test "arukellt build is idempotent" \
    "cd '$BASIC' && '$ARUKELLT' build 2>&1 | grep -q ''" \
    0

echo
echo "── Manifest discovery from subdirectory ──"

# Test 3: manifest discovery from a subdirectory
run_test "manifest discovered from src/ subdirectory" \
    "cd '$BASIC/src' && '$ARUKELLT' build 2>&1 | grep -q ''" \
    0

echo
echo "── Manifest error diagnostics ──"

# Test 4: missing ark.toml gives actionable error
TMPDIR_TEST="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_TEST"' EXIT
run_test "missing ark.toml gives clear error message" \
    "cd '$TMPDIR_TEST' && '$ARUKELLT' build 2>&1 | grep -qi 'ark.toml\|manifest\|not found'" \
    1

echo
echo "── Results ──"
echo "  PASS: $PASS  FAIL: $FAIL  SKIP: $SKIP"

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
