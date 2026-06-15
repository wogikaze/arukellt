#!/usr/bin/env bash
# Component interop smoke test: general multi-export string canonical ABI.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/scripts/run/arukellt-selfhost.sh}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
WASM_TOOLS="${WASM_TOOLS_BIN:-$(command -v wasm-tools 2>/dev/null || echo "")}"
COMPONENT_WASM="tests/component-interop/jco/string-multi/string_multi.component.wasm"
SOURCE_REL="tests/component-interop/jco/string-multi/string_multi.ark"
cd "$REPO_ROOT"

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASM_TOOLS" ]]; then
    echo "SKIP: wasm-tools not found in PATH"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/4] Compiling string_multi.ark -> component wasm"
"$ARUKELLT" compile \
    --emit component \
    --target wasm32-wasi-p2 \
    "$SOURCE_REL" \
    -o "$COMPONENT_WASM"
echo "      OK ($(wc -c < "$COMPONENT_WASM") bytes)"

echo "[2/4] Validating component wasm"
"$WASM_TOOLS" validate "$COMPONENT_WASM"
echo "      OK"

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

echo "[3/4] Running string multi-export invocations"
run_test 'echo_text("hello") = "hello"' '"hello"' 'echo_text("hello")'
run_test 'greet_text("world") = "world"' '"world"' 'greet_text("world")'

echo "[4/4] Results: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
