#!/usr/bin/env bash
# Component interop smoke test: result<s32, string> parameter canonical ABI.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
# shellcheck source=../../common.sh
source "$REPO_ROOT/tests/component-interop/common.sh"
interop_setup_s2_compiler


WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="tests/component-interop/jco/result-string-param/result_string_param.component.wasm"
SOURCE_REL="tests/component-interop/jco/result-string-param/result_string_param.ark"
cd "$REPO_ROOT"


if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling result_string_param.ark -> component wasm"
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

echo "[2/3] Running result<string> parameter invocations"
run_test "result_or_default(ok(21)) = 21" "21" "result-or-default(ok(21))"
run_test "result_or_default(err(\"bad\")) = -5" "-5" 'result-or-default(err("bad"))'

echo "[3/3] Results: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
