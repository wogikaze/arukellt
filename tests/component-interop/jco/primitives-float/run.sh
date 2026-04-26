#!/usr/bin/env bash
# Component interop test: f64 scalar exports.
#
# Verifies that Arukellt f64 exports are accessible via the Component Model
# ABI using wasmtime's --invoke flag.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="$SCRIPT_DIR/primitives_float.component.wasm"

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling primitives_float.ark -> primitives_float.component.wasm"
"$ARUKELLT" compile \
    --emit component \
    --target wasm32-wasi-p2 \
    "$SCRIPT_DIR/primitives_float.ark" \
    -o "$COMPONENT_WASM"
echo "      OK ($(wc -c < "$COMPONENT_WASM") bytes)"

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

echo "[2/3] Running f64 component-model invocations"
run_test "square_f64(3.0) = 9.0"    "9"    "square-f64(3.0)"
run_test "square_f64(0.0) = 0.0"    "0"    "square-f64(0.0)"
run_test "average(1.0, 3.0) = 2.0"  "2"    "average(1.0, 3.0)"
run_test "negate_f64(5.0) = -5.0"   "-5"   "negate-f64(5.0)"

echo "[3/3] Results: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
