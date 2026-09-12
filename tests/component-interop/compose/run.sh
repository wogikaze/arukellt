#!/usr/bin/env bash
# Official Component Model composition smoke test.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$REPO_ROOT"

WASM_TOOLS="${WASM_TOOLS_BIN:-$(command -v wasm-tools 2>/dev/null || true)}"
WAC="$(command -v wac 2>/dev/null || true)"
if [[ -z "$WASM_TOOLS" || -z "$WAC" ]]; then
    echo "SKIP: need wasm-tools and wac"
    exit 0
fi

FIXTURES="tests/component-interop/compose/fixtures"
OUT=".build/compose-smoke"
mkdir -p "$OUT"

"$WASM_TOOLS" component embed "$FIXTURES/provider-match.wit" \
    --world math-lib --dummy-names standard32 \
    -o "$OUT/provider.core.wasm"
"$WASM_TOOLS" component new "$OUT/provider.core.wasm" \
    --reject-legacy-names --realloc-via-memory-grow \
    -o "$OUT/provider.component.wasm"

"$WASM_TOOLS" component embed "$FIXTURES/socket-match.wit" \
    --world runner --dummy-names standard32 \
    -o "$OUT/socket.core.wasm"
"$WASM_TOOLS" component new "$OUT/socket.core.wasm" \
    --reject-legacy-names --realloc-via-memory-grow \
    -o "$OUT/socket.component.wasm"

wac plug --plug "$OUT/provider.component.wasm" \
    "$OUT/socket.component.wasm" -o "$OUT/composed.component.wasm"
"$WASM_TOOLS" validate "$OUT/composed.component.wasm"

echo "PASS compose smoke (official WIT + wasm-tools + wac)"
