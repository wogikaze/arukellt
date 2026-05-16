#!/usr/bin/env bash
# Component interop test: multi-export record adapters are name-independent.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="tests/component-interop/jco/record-point-renamed/point_renamed.component.wasm"
SOURCE_REL="tests/component-interop/jco/record-point-renamed/point_renamed.ark"
cd "$REPO_ROOT"

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling point_renamed.ark -> point_renamed.component.wasm"
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
run_test "length-sq({3,4}) = 25"    "25"  'length-sq({x: 3, y: 4})'
run_test "length-sq({0,0}) = 0"     "0"   'length-sq({x: 0, y: 0})'
run_test "length-sq({1,1}) = 2"     "2"   'length-sq({x: 1, y: 1})'
run_test "translate({1,2},{3,4})"   "{x: 4, y: 6}"  'translate({x: 1, y: 2}, {x: 3, y: 4})'
run_test "translate({0,0},{5,5})"   "{x: 5, y: 5}"  'translate({x: 0, y: 0}, {x: 5, y: 5})'

echo "[3/3] Results: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
