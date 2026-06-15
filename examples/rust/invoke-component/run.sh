#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../common.sh"

ROOT="$(examples_repo_root)"
EXPORT_RUN="$ROOT/examples/ark/export-library/run.sh"
COMPONENT="$ROOT/.build/examples/ark-export/calculator.component.wasm"

cd "$ROOT"

WASMTIME="$(examples_find_wasmtime || true)"
if [[ -z "$WASMTIME" ]]; then
    echo "SKIP: wasmtime not in PATH"
    exit 0
fi

echo "[1/2] ensure Ark calculator component exists"
bash "$EXPORT_RUN"

echo "[2/2] Rust host (wasmtime) invokes Ark export add(10, 32)"
got="$("$WASMTIME" run --wasm gc --wasm component-model --invoke 'add(10, 32)' "$COMPONENT")"
[[ "$got" == "42" ]] || { echo "FAIL: expected 42, got $got"; exit 1; }

echo "PASS rust/invoke-component"
