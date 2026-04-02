#!/usr/bin/env bash
# scripts/smoke-test-binary.sh — Minimal smoke tests for a release binary.
# Usage: ./scripts/smoke-test-binary.sh [path-to-arukellt]
#
# Exits 0 if all checks pass, non-zero on first failure.
set -euo pipefail

BIN="${1:-target/release/arukellt}"

if [ ! -x "$BIN" ]; then
  echo "FAIL: binary not found or not executable: $BIN"
  exit 1
fi

PASS=0
FAIL=0

check() {
  local desc="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    echo "  ✓ $desc"
    PASS=$((PASS + 1))
  else
    echo "  ✗ $desc"
    FAIL=$((FAIL + 1))
  fi
}

check_output() {
  local desc="$1"
  local expected="$2"
  shift 2
  local actual
  actual=$("$@" 2>/dev/null) || true
  if [ "$actual" = "$expected" ]; then
    echo "  ✓ $desc"
    PASS=$((PASS + 1))
  else
    echo "  ✗ $desc (expected '$expected', got '$actual')"
    FAIL=$((FAIL + 1))
  fi
}

check_fail() {
  local desc="$1"
  shift
  if ! "$@" >/dev/null 2>&1; then
    echo "  ✓ $desc"
    PASS=$((PASS + 1))
  else
    echo "  ✗ $desc (expected non-zero exit)"
    FAIL=$((FAIL + 1))
  fi
}

echo "Smoke testing: $BIN"
echo "  version: $("$BIN" --version 2>/dev/null || echo 'unknown')"
echo ""

# Basic CLI
check "--version exits 0" "$BIN" --version
check "--help exits 0" "$BIN" --help

# Compile and run hello world
if [ -f tests/fixtures/hello/hello.ark ]; then
  check_output "run hello.ark" "Hello, world!" "$BIN" run tests/fixtures/hello/hello.ark
fi

# Check command on valid source
VALID_FIXTURE=$(find tests/fixtures -name '*.ark' -path '*/stdlib_string/*' | head -1)
if [ -n "$VALID_FIXTURE" ]; then
  check "check valid source" "$BIN" check "$VALID_FIXTURE"
fi

# Compile-fail: type error should exit non-zero
if [ -f tests/fixtures/modules/pub_private/main.ark ]; then
  check_fail "compile-fail on private access" "$BIN" run tests/fixtures/modules/pub_private/main.ark
fi

# Fmt runs without crashing
if [ -f tests/fixtures/hello/hello.ark ]; then
  check "fmt runs on hello.ark" "$BIN" fmt tests/fixtures/hello/hello.ark
fi

echo ""
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
