#!/usr/bin/env bash
# Component interop smoke test: jco-transpiled calculator.
#
# Compiles calculator.ark to a component, runs `jco transpile` to generate
# Node-compatible ESM glue, then runs `test.mjs` to assert the exports.
#
# Requires:
#   - Node.js >= 18 (tested on 23.6; v25 may not need --experimental-wasm-memory64)
#   - npm  (for `npm exec --package=@bytecodealliance/jco@<version>`)
#   - or a local `jco` binary matching the pinned version
#
# Usage:
#   ./run.sh
#   ARUKELLT_BIN=path/to/arukellt ./run.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
# shellcheck source=../../common.sh
source "$REPO_ROOT/tests/component-interop/common.sh"
interop_setup_s2_compiler

JCO_VERSION="1.25.2"
NODE="${NODE_BIN:-$(command -v node 2>/dev/null || echo "")}"
NPM="${NPM_BIN:-$(command -v npm 2>/dev/null || echo "")}"
JCO_LOCAL="${JCO_BIN:-$(command -v jco 2>/dev/null || echo "")}"

COMPONENT_WASM="tests/component-interop/jco/calculator/calculator.component.wasm"
SOURCE_REL="tests/component-interop/jco/calculator/calculator.ark"
JCO_OUT="tests/component-interop/jco/calculator/jco-out"

cd "$REPO_ROOT"

if [[ -n "$NPM" ]]; then
    JCO="$NPM exec --package=@bytecodealliance/jco@${JCO_VERSION} -- jco"
elif [[ -n "$JCO_LOCAL" ]]; then
    installed_version="$($JCO_LOCAL --version 2>/dev/null)"
    if [[ "$installed_version" != "$JCO_VERSION" ]]; then
        echo "FAIL: jco version mismatch (expected ${JCO_VERSION}, got ${installed_version})"
        exit 1
    fi
    JCO="$JCO_LOCAL"
else
    echo "FAIL: npm or jco required"
    exit 1
fi

if [[ -z "$NODE" ]]; then
    echo "FAIL: node not found in PATH"
    exit 1
fi

echo "[1/3] Compiling calculator.ark -> calculator.component.wasm"
interop_compile_component "$SOURCE_REL" "$COMPONENT_WASM"
echo "      OK ($(wc -c < "$COMPONENT_WASM") bytes)"

echo "[2/3] Transpiling component with jco ${JCO_VERSION}"
rm -rf "$JCO_OUT"
$JCO transpile "$COMPONENT_WASM" -o "$JCO_OUT"
echo "      OK"

echo "[3/3] Running Node.js assertions"
"$NODE" --experimental-wasm-memory64 "$SCRIPT_DIR/test.mjs"

echo "      All assertions passed"
