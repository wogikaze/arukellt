#!/usr/bin/env bash
# Component interop test: enum exports via canonical ABI adapters.
#
# Verifies that Arukellt enum types are correctly exported through the
# Component Model using canonical ABI adapter functions.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="$SCRIPT_DIR/colors.component.wasm"

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling colors.ark -> colors.component.wasm"
"$ARUKELLT" compile \
    --emit component \
    --target wasm32-wasi-p2 \
    "$SCRIPT_DIR/colors.ark" \
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

echo "[2/3] Running component-model invocations via wasmtime"
# Enum params use variant names in wasmtime invoke syntax
run_test "color-to-value(red) = 1"       "1" 'color-to-value(red)'
run_test "color-to-value(green) = 2"     "2" 'color-to-value(green)'
run_test "color-to-value(blue) = 3"      "3" 'color-to-value(blue)'

# next-color cycles: Red→Green→Blue→Red (returns enum name)
run_test "next-color(red) = green"       "green" 'next-color(red)'
run_test "next-color(green) = blue"      "blue"  'next-color(green)'
run_test "next-color(blue) = red"        "red"   'next-color(blue)'

echo "[3/3] Results: $PASS passed, $FAIL failed"

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
