#!/usr/bin/env bash
# Component interop smoke test: multi-type exports.
#
# Verifies that a single component can export functions with different
# scalar types (i32, i64, f64, bool) through the Component Model ABI.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
# shellcheck source=../../common.sh
source "$REPO_ROOT/tests/component-interop/common.sh"
interop_setup_s2_compiler


WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="tests/component-interop/jco/multi-type-exports/multi_type_exports.component.wasm"
SOURCE_REL="tests/component-interop/jco/multi-type-exports/multi_type_exports.ark"
cd "$REPO_ROOT"


if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling multi_type_exports.ark -> component wasm"
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
        echo "      FAIL: $desc — expected '$expected', got '$actual'"
        ((FAIL++)) || true
    fi
}

echo "[2/3] Running multi-type export invocations via wasmtime"

# i32 functions
run_test "add_i32(10, 20) = 30"     "30"    "add-i32(10, 20)"
run_test "add_i32(-5, 5) = 0"      "0"     "add-i32(-5, 5)"
run_test "abs_i32(-7) = 7"         "7"     "abs-i32(-7)"
run_test "abs_i32(3) = 3"          "3"     "abs-i32(3)"

# i64 functions
run_test "add_i64(100, 200) = 300"  "300"   "add-i64(100, 200)"

# f64 functions
run_test "mul_f64(2.5, 4.0) = 10.0" "10"   "mul-f64(2.5, 4.0)"

# bool functions
run_test "is_positive(5) = true"    "true"  "is-positive(5)"
run_test "is_positive(-1) = false"  "false" "is-positive(-1)"
run_test "is_positive(0) = false"   "false" "is-positive(0)"

echo "[3/3] Results: $PASS passed, $FAIL failed"

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
