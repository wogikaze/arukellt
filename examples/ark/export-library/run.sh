#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../common.sh"

ROOT="$(examples_repo_root)"
OUT_REL=".build/examples/ark-export"
OUT="$ROOT/$OUT_REL"
CORE_REL="$OUT_REL/calculator.core.wasm"
EMBED_REL="$OUT_REL/calculator.embed.wasm"
COMPONENT_REL="$OUT_REL/calculator.component.wasm"
ADAPTER="$OUT/wasi_snapshot_preview1.reactor.wasm"
SOURCE="examples/ark/export-library/calculator.ark"
WIT="examples/ark/export-library/calculator.wit"

cd "$ROOT"

ARUKELLT="$(examples_find_arukellt "$ROOT" || true)"
WASMTIME="$(examples_find_wasmtime || true)"
WT="$(examples_find_wasm_tools || true)"

if [[ -z "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found (need scripts/run/arukellt-selfhost.sh or target/release/arukellt)"
    exit 0
fi
if [[ -z "$WASMTIME" ]] || [[ -z "$WT" ]]; then
    echo "SKIP: need wasmtime and wasm-tools"
    exit 0
fi

mkdir -p "$OUT"
examples_ensure_wasi_adapter "$ADAPTER"

echo "[1/3] compile core wasm"
examples_compile "$ARUKELLT" compile "$SOURCE" \
    --target wasm32-wasi-p1 \
    --emit wasm \
    -o "$CORE_REL"

echo "[2/3] embed WIT + component new"
"$WT" component embed "$WIT" "$OUT/calculator.core.wasm" -o "$OUT/calculator.embed.wasm"
"$WT" component new "$OUT/calculator.embed.wasm" \
    --adapt "wasi_snapshot_preview1=$ADAPTER" \
    -o "$OUT/calculator.component.wasm"
echo "      wrote $COMPONENT_REL ($(wc -c < "$OUT/calculator.component.wasm") bytes)"

echo "[3/3] invoke add(3, 4) and mul(6, 7)"
got_add="$("$WASMTIME" run --wasm gc --wasm component-model --invoke 'add(3, 4)' "$OUT/calculator.component.wasm")"
got_mul="$("$WASMTIME" run --wasm gc --wasm component-model --invoke 'mul(6, 7)' "$OUT/calculator.component.wasm")"
[[ "$got_add" == "7" ]] || { echo "FAIL: add expected 7, got $got_add"; exit 1; }
[[ "$got_mul" == "42" ]] || { echo "FAIL: mul expected 42, got $got_mul"; exit 1; }

echo "PASS ark/export-library"
