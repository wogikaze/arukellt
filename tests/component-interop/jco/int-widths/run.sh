#!/usr/bin/env bash
# Component interop smoke test: narrow integer WIT types.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="tests/component-interop/jco/int-widths/int_widths.component.wasm"
SOURCE_REL="tests/component-interop/jco/int-widths/int_widths.ark"
cd "$REPO_ROOT"

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling int_widths.ark -> component wasm"
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

echo "[2/3] Running narrow integer invocations"
run_test "inc_u8(41) = 42" "42" "inc-u8(41)"
run_test "inc_u16(999) = 1000" "1000" "inc-u16(999)"
run_test "echo_u32(123456) = 123456" "123456" "echo-u32(123456)"
run_test "echo_u64(5000000000) = 5000000000" "5000000000" "echo-u64(5000000000)"
run_test "echo_i64(-5000000000) = -5000000000" "-5000000000" "echo-i64(-5000000000)"
run_test "echo_i8(-7) = -7" "-7" "echo-i8(-7)"
run_test "dec_i16(17) = 16" "16" "dec-i16(17)"

echo "[3/3] Results: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
