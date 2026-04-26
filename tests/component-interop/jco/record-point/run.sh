#!/usr/bin/env bash
# Component interop test: struct (record) exports via canonical ABI adapters.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="$SCRIPT_DIR/point.component.wasm"

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling point.ark -> point.component.wasm"
"$ARUKELLT" compile \
    --emit component \
    --target wasm32-wasi-p2 \
    "$SCRIPT_DIR/point.ark" \
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
# distance-sq with record param {x, y}
run_test "distance-sq({3,4}) = 25"    "25"  'distance-sq({x: 3, y: 4})'
run_test "distance-sq({0,0}) = 0"     "0"   'distance-sq({x: 0, y: 0})'
run_test "distance-sq({1,1}) = 2"     "2"   'distance-sq({x: 1, y: 1})'

# add-points returns a record
run_test "add-points({1,2},{3,4})"    "{x: 4, y: 6}"  'add-points({x: 1, y: 2}, {x: 3, y: 4})'
run_test "add-points({0,0},{5,5})"    "{x: 5, y: 5}"  'add-points({x: 0, y: 0}, {x: 5, y: 5})'

echo "[3/3] Results: $PASS passed, $FAIL failed"

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
