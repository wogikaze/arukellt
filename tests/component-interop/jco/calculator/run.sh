#!/usr/bin/env bash
# Component interop smoke test for the calculator component.
#
# Uses wasmtime CLI (--wasm gc --wasm component-model --invoke) to verify
# that Arukellt scalar exports are accessible via the Component Model ABI.
#
# jco (JavaScript component toolchain) is NOT used here because jco 1.x does
# not support Wasm GC types. All Arukellt T3 components embed GC type
# definitions in their core module, which causes jco transpile to fail with:
#   "array indexed types not supported without the gc feature"
# This is a jco limitation, not an Arukellt limitation. When jco gains GC
# support, a Node.js test (test.mjs) should be added alongside this script.
# Track: https://github.com/bytecodealliance/jco/issues (search "gc")
#
# Usage:
#   ./run.sh                        # run from repo root or this directory
#   ARUKELLT_BIN=path/to/arukellt ./run.sh
#   WASMTIME_BIN=path/to/wasmtime ./run.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="$SCRIPT_DIR/calculator.component.wasm"

# ── Dependency checks ──────────────────────────────────────────────────────

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT (run cargo build -p arukellt first)"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

# ── Build ──────────────────────────────────────────────────────────────────

echo "[1/3] Compiling calculator.ark -> calculator.component.wasm"
"$ARUKELLT" compile \
    --emit component \
    --target wasm32-wasi-p2 \
    "$SCRIPT_DIR/calculator.ark" \
    -o "$COMPONENT_WASM"
echo "      OK ($(wc -c < "$COMPONENT_WASM") bytes)"

# ── Run tests ─────────────────────────────────────────────────────────────

PASS=0
FAIL=0

run_test() {
    local desc="$1"
    local expected="$2"
    local invocation="$3"
    local actual
    actual="$("$WASMTIME" run --wasm gc --wasm component-model --invoke "$invocation" "$COMPONENT_WASM" 2>&1)"
    if [[ "$actual" == "$expected" ]]; then
        echo "      PASS: $desc"
        ((PASS++)) || true
    else
        echo "      FAIL: $desc — expected '$expected', got '$actual'"
        ((FAIL++)) || true
    fi
}

echo "[2/3] Running component-model invocations via wasmtime"
run_test "add(3, 4) = 7"      "7"   "add(3, 4)"
run_test "add(0, 0) = 0"      "0"   "add(0, 0)"
run_test "add(-1, 1) = 0"     "0"   "add(-1, 1)"
run_test "mul(6, 7) = 42"     "42"  "mul(6, 7)"
run_test "mul(0, 100) = 0"    "0"   "mul(0, 100)"
run_test "negate(5) = -5"     "-5"  "negate(5)"
run_test "negate(-3) = 3"     "3"   "negate(-3)"

echo "[3/3] Results: $PASS passed, $FAIL failed"

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
