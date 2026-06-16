#!/usr/bin/env bash
# Component interop smoke test: option<s32> parameter/result canonical ABI is name-independent.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
# shellcheck source=../../common.sh
source "$REPO_ROOT/tests/component-interop/common.sh"
interop_setup_s2_compiler


WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="tests/component-interop/jco/option-roundtrip-renamed/option_roundtrip_renamed.component.wasm"
SOURCE_REL="tests/component-interop/jco/option-roundtrip-renamed/option_roundtrip_renamed.ark"
cd "$REPO_ROOT"


if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling option_roundtrip_renamed.ark -> component wasm"
interop_compile_component "$SOURCE_REL" "$COMPONENT_WASM"
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
run_test "adjust_optional(some(9)) = some(8)" "some(8)" "adjust-optional(some(9))"
run_test "adjust_optional(some(1)) = none" "none" "adjust-optional(some(1))"
run_test "adjust_optional(none) = none" "none" "adjust-optional(none)"

echo "[3/3] Results: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
