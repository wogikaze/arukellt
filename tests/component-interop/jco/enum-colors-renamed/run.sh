#!/usr/bin/env bash
# Component interop test: multi-export enum adapters are name-independent.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="tests/component-interop/jco/enum-colors-renamed/colors_renamed.component.wasm"
SOURCE_REL="tests/component-interop/jco/enum-colors-renamed/colors_renamed.ark"
cd "$REPO_ROOT"

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling colors_renamed.ark -> colors_renamed.component.wasm"
"$ARUKELLT" compile \
    --emit component \
    --target wasm32-wasi-p2 \
    "$SOURCE_REL" \
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
        echo "      FAIL: $desc - expected '$expected', got '$actual'"
        ((FAIL++)) || true
    fi
}

echo "[2/3] Running component-model invocations via wasmtime"
run_test "color-rank(red) = 1"       "1" 'color-rank(red)'
run_test "color-rank(green) = 2"     "2" 'color-rank(green)'
run_test "color-rank(blue) = 3"      "3" 'color-rank(blue)'
run_test "rotate-color(red) = green" "green" 'rotate-color(red)'
run_test "rotate-color(green) = blue" "blue" 'rotate-color(green)'
run_test "rotate-color(blue) = red"  "red" 'rotate-color(blue)'

echo "[3/3] Results: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
