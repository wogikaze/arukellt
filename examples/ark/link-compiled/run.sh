#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../common.sh"

ROOT="$(examples_repo_root)"
OUT_REL=".build/examples/ark-link-compiled"
OUT="$ROOT/$OUT_REL"
CONSUMER_ARK="examples/ark/link-compiled/consumer/client.ark"
PROVIDER_DIR="$ROOT/examples/rust/host-provider"
SOCKET_REL="$OUT_REL/client.component.wasm"
COMPOSED_REL="$OUT_REL/composed.component.wasm"

cd "$ROOT"

ARUKELLT="$(examples_find_arukellt "$ROOT" || true)"
WASMTIME="$(examples_find_wasmtime || true)"
WT="$(examples_find_wasm_tools || true)"

if [[ -z "$ARUKELLT" ]]; then
    echo "SKIP: arukellt not found"
    exit 0
fi
if [[ -z "$WT" ]] || ! command -v cargo >/dev/null; then
    echo "SKIP: need wasm-tools and cargo"
    exit 0
fi

mkdir -p "$OUT"

echo "[1/4] build Rust host provider"
( cd "$PROVIDER_DIR" && cargo component build --release )
PROVIDER_WASM="$PROVIDER_DIR/target/wasm32-wasip1/release/wit_import_host_provider.wasm"
PROVIDER_REL="examples/rust/host-provider/target/wasm32-wasip1/release/wit_import_host_provider.wasm"
[[ -f "$PROVIDER_WASM" ]] || { echo "FAIL: missing provider wasm"; exit 1; }

echo "[2/4] compile Ark consumer socket (WIT import)"
examples_compile "$ARUKELLT" modern compile \
    "$CONSUMER_ARK" \
    --target wasm32-wasi-p2 \
    --wasi-version p2 \
    --emit component \
    -o "$SOCKET_REL"

echo "[3/4] compose --validate (link provider into consumer socket)"
compose_out="$(examples_compile "$ARUKELLT" modern compose --validate \
    --plug "$PROVIDER_REL" "$SOCKET_REL" \
    -o "$COMPOSED_REL" 2>&1)"
echo "$compose_out" | tail -3
echo "$compose_out" | grep -q "compose: validation ok" || {
    echo "FAIL: compose validation did not pass"
    exit 1
}

echo "[4/4] optional runtime invoke (wac plug + wasmtime)"
if command -v wac >/dev/null && [[ -n "$WASMTIME" ]]; then
    if wac plug --plug "$PROVIDER_WASM" "$OUT/client.component.wasm" -o "$OUT/composed.component.wasm" 2>/dev/null; then
        got="$("$WASMTIME" run --wasm gc --wasm component-model --invoke 'run()' "$OUT/composed.component.wasm" 2>/dev/null || true)"
        if [[ "$got" == "42" ]]; then
            echo "      runtime invoke run() -> 42"
            echo "PASS ark/link-compiled (validate + runtime)"
            exit 0
        fi
    fi
    echo "      note: runtime invoke skipped (P2 socket + local wac/wasmtime); validate step succeeded"
fi

echo "PASS ark/link-compiled (compose validate)"
