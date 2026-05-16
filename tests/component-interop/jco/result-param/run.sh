#!/usr/bin/env bash
# Component interop smoke test: result<s32, s32> parameter canonical ABI is name-independent.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="tests/component-interop/jco/result-param/result_param.component.wasm"
SOURCE_REL="tests/component-interop/jco/result-param/result_param.ark"
cd "$REPO_ROOT"

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling result_param.ark -> component wasm"
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

echo "[2/3] Running result parameter invocations"
run_test "ok_or_zero(ok(12)) = 12" "12" "ok-or-zero(ok(12))"
run_test "ok_or_zero(err(9)) = 0" "0" "ok-or-zero(err(9))"

echo "[3/3] Results: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
