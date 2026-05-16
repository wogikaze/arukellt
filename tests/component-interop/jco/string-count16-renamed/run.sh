#!/usr/bin/env bash
# Component interop smoke test: String parameter to u16 result canonical ABI is name-independent.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="tests/component-interop/jco/string-count16-renamed/string_count16_renamed.component.wasm"
SOURCE_REL="tests/component-interop/jco/string-count16-renamed/string_count16_renamed.ark"
cd "$REPO_ROOT"

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling string_count16_renamed.ark -> component wasm"
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

echo "[2/3] Running string u16 invocations"
run_test 'wide_code16("") = 21' "21" 'wide-code16("")'
run_test 'wide_code16("hello") = 2100' "2100" 'wide-code16("hello")'

echo "[3/3] Results: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
