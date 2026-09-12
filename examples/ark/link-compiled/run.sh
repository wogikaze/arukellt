#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../common.sh"

ROOT="$(examples_repo_root)"
OUT_REL=".build/examples/ark-link-compiled"
OUT="$ROOT/$OUT_REL"
CONSUMER_ARK="examples/ark/link-compiled/consumer/client.ark"
PROVIDER_WIT="examples/ark/link-compiled/provider.wit"
SOCKET_REL="$OUT_REL/client.component.wasm"
PROVIDER_REL="$OUT_REL/provider.component.wasm"
COMPOSED_REL="$OUT_REL/composed.component.wasm"

cd "$ROOT"

ARUKELLT="$(examples_find_arukellt "$ROOT" || true)"
WT="$(examples_find_wasm_tools || true)"
WAC="$(command -v wac 2>/dev/null || true)"

if [[ -z "$ARUKELLT" ]]; then
    echo "SKIP: arukellt selfhost wrapper not found"
    exit 0
fi
if [[ -z "$WT" || -z "$WAC" ]]; then
    echo "SKIP: need wasm-tools and wac"
    exit 0
fi

mkdir -p "$OUT"

echo "[1/3] package official WIT provider"
"$WT" component embed "$PROVIDER_WIT" --world provider \
    --dummy-names standard32 -o "$OUT/provider.core.wasm"
"$WT" component new "$OUT/provider.core.wasm" \
    --reject-legacy-names --realloc-via-memory-grow \
    -o "$OUT/provider.component.wasm"

echo "[2/3] compile Ark consumer socket (WIT import)"
examples_compile "$ARUKELLT" modern compile \
    "$CONSUMER_ARK" \
    --target wasm32-gc \
    --wasi-version wasi-p2 \
    --emit component \
    -o "$SOCKET_REL"

echo "[3/3] compose and validate with wac"
examples_compile "$ARUKELLT" modern compose \
    --validate --plug "$PROVIDER_REL" "$SOCKET_REL" \
    -o "$COMPOSED_REL"
wac plug --plug "$OUT/provider.component.wasm" \
    "$OUT/client.component.wasm" -o "$OUT/composed.component.wasm"
"$WT" validate "$OUT/composed.component.wasm"

echo "PASS ark/link-compiled (official WIT provider + Ark consumer)"
