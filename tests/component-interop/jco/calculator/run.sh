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

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/scripts/run/arukellt-selfhost.sh}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="tests/component-interop/jco/calculator/calculator.component.wasm"
SOURCE_REL="tests/component-interop/jco/calculator/calculator.ark"
cd "$REPO_ROOT"

# Library scalar exports require s2 selfhost (#666); bootstrap overlay stub is empty.
if [[ -z "${ARUKELLT_SELFHOST_WASM:-}" ]]; then
    if [[ -f "$REPO_ROOT/.build/selfhost/arukellt-s2.wasm" ]]; then
        export ARUKELLT_SELFHOST_WASM="$REPO_ROOT/.build/selfhost/arukellt-s2.wasm"
    elif [[ -f "$REPO_ROOT/.bootstrap-build/arukellt-s2.wasm" ]]; then
        export ARUKELLT_SELFHOST_WASM="$REPO_ROOT/.bootstrap-build/arukellt-s2.wasm"
    fi
fi

# ── Dependency checks ──────────────────────────────────────────────────────

if [[ ! -f "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

if [[ "$(basename "$ARUKELLT")" == "arukellt-selfhost.sh" ]]; then
    if [[ -z "${ARUKELLT_SELFHOST_WASM:-}" ]] || [[ "${ARUKELLT_SELFHOST_WASM}" != *"arukellt-s2"* ]]; then
        echo "SKIP: calculator library exports require s2 selfhost (set ARUKELLT_SELFHOST_WASM)"
        exit 0
    fi
fi

# ── Build ──────────────────────────────────────────────────────────────────

echo "[1/3] Compiling calculator.ark -> calculator.component.wasm"
if [[ "$(basename "$ARUKELLT")" == "arukellt-selfhost.sh" ]]; then
    bash "$ARUKELLT" compile \
        --emit component \
        --target wasm32-wasi-p2 \
        "$SOURCE_REL" \
        -o "$COMPONENT_WASM"
else
    "$ARUKELLT" compile \
        --emit component \
        --target wasm32-wasi-p2 \
        "$SOURCE_REL" \
        -o "$COMPONENT_WASM"
fi
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
