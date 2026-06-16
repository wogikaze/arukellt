#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$REPO_ROOT"
# shellcheck source=../common.sh
source "$REPO_ROOT/tests/component-interop/common.sh"
interop_setup_s2_compiler
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
WT="${WASM_TOOLS_BIN:-$HOME/.cargo/bin/wasm-tools}"
FIXTURE="tests/component-interop/compose"
OUT=".build/compose-smoke"
ADAPTER="$OUT/wasi_snapshot_preview1.reactor.wasm"
SOCKET="$FIXTURE/runner-cargo/target/wasm32-wasip1/release/compose_runner_guest.wasm"
if [[ ! -x "$WT" ]] || [[ -z "$WASMTIME" ]] || ! command -v wac >/dev/null || ! command -v cargo >/dev/null; then
  echo "SKIP: toolchain missing"; exit 0
fi
mkdir -p "$OUT"
[[ -f "$ADAPTER" ]] || curl -fsSL -o "$ADAPTER" \
  "https://github.com/bytecodealliance/wasmtime/releases/download/v39.0.1/wasi_snapshot_preview1.reactor.wasm"
bash "$ARUKELLT" compile --target wasm32-wasi-p1 --emit wasm "$FIXTURE/math_lib.ark" -o "$OUT/math-lib.core.wasm"
"$WT" component embed "$FIXTURE/wit/math-lib.wit" "$OUT/math-lib.core.wasm" -o "$OUT/math-lib.embed.wasm"
"$WT" component new "$OUT/math-lib.embed.wasm" --adapt "wasi_snapshot_preview1=$ADAPTER" -o "$OUT/math-lib-component.wasm"
[[ "$(wasmtime run --wasm gc --wasm component-model --invoke 'add(40,2)' "$OUT/math-lib-component.wasm")" == "42" ]]
( cd "$FIXTURE/runner-cargo" && cargo component build --release )
wac plug --plug "$OUT/math-lib-component.wasm" "$SOCKET" -o "$OUT/composed-component.wasm"
[[ "$(wasmtime run --wasm gc --wasm component-model --invoke 'run()' "$OUT/composed-component.wasm")" == "42" ]]
echo PASS compose smoke
