#!/usr/bin/env bash
# Component interop test: bool scalar exports.
#
# Verifies that Arukellt bool exports are accessible via the Component Model ABI.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/debug/arukellt}"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
COMPONENT_WASM="$SCRIPT_DIR/bool_logic.component.wasm"

if [[ ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found at $ARUKELLT"
    exit 0
fi

if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not found in PATH"
    exit 0
fi

echo "[1/3] Compiling bool_logic.ark -> bool_logic.component.wasm"
"$ARUKELLT" compile \
    --emit component \
    --target wasm32-wasi-p2 \
    "$SCRIPT_DIR/bool_logic.ark" \
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

echo "[2/3] Running bool component-model invocations"
run_test "and_bool(true, true) = true"   "true"   "and-bool(true, true)"
run_test "and_bool(true, false) = false" "false"  "and-bool(true, false)"
run_test "or_bool(false, true) = true"  "true"   "or-bool(false, true)"
run_test "or_bool(false, false) = false" "false"  "or-bool(false, false)"
run_test "not_bool(true) = false"        "false"  "not-bool(true)"
run_test "not_bool(false) = true"        "true"   "not-bool(false)"

echo "[3/3] Results: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
