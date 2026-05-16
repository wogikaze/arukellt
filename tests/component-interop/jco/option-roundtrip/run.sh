#!/usr/bin/env bash
# Component interop smoke test: option<s32> parameter/result canonical ABI.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="tests/component-interop/jco/option-roundtrip/option_roundtrip.component.wasm"
SOURCE_REL="tests/component-interop/jco/option-roundtrip/option_roundtrip.ark"
cd "$REPO_ROOT"

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling option_roundtrip.ark -> component wasm"
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

echo "[2/3] Running option roundtrip invocations"
run_test "keep_positive(some(9)) = some(10)" "some(10)" "keep-positive(some(9))"
run_test "keep_positive(some(-2)) = none" "none" "keep-positive(some(-2))"
run_test "keep_positive(none) = none" "none" "keep-positive(none)"

echo "[3/3] Results: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
