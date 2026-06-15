#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

cd "$ROOT"

if ! command -v node >/dev/null; then
    echo "SKIP: node not in PATH"
    exit 0
fi
if ! command -v wasmtime >/dev/null && [[ -z "${WASMTIME_BIN:-}" ]]; then
    echo "SKIP: wasmtime not in PATH"
    exit 0
fi

bash "$ROOT/examples/ark/export-library/run.sh"
node "$SCRIPT_DIR/run.mjs"
