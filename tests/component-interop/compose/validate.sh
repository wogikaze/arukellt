#!/usr/bin/env bash
# Issue #443 — arukellt compose validation scaffold smoke.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$REPO_ROOT"
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
if [[ -z "$WASMTIME" ]]; then
  echo "SKIP: wasmtime missing"
  exit 0
fi
RUNTIME="${ARUKELLT_SELFHOST_RUNTIME:-.build/selfhost/arukellt-s2-runtime.wasm}"
if [[ ! -f "$RUNTIME" ]]; then
  if ! python3 - <<'PY'
from pathlib import Path
from scripts.selfhost.checks import _ensure_current_selfhost, _find_pinned_wasm, _find_wasmtime
root = Path(".")
pinned = _find_pinned_wasm(root)
wasmtime = _find_wasmtime()
if pinned is None or wasmtime is None:
    raise SystemExit(1)
current, err = _ensure_current_selfhost(root, wasmtime, pinned)
if current is None:
    raise SystemExit(1)
PY
  then
    echo "SKIP: selfhost runtime wasm missing"
    exit 0
  fi
fi
OUT=".build/compose-validate"
mkdir -p "$OUT"
PROVIDER="$OUT/provider.wasm"
SOCKET="$OUT/socket.wasm"
COMPOSED="$OUT/composed.wasm"
printf '\0asm' >"$PROVIDER"
printf '\0asm' >"$SOCKET"
run_compose() {
  "$WASMTIME" run --dir "$REPO_ROOT" "$RUNTIME" -- "$@"
}
if run_compose compose 2>/dev/null; then
  echo "FAIL: compose without args should exit non-zero"
  exit 1
fi
if run_compose compose --validate --plug "$PROVIDER" "$SOCKET" -o "$COMPOSED" | grep -q "compose dependency graph"; then
  :
else
  echo "FAIL: compose --validate did not print dependency graph"
  exit 1
fi
if run_compose compose --validate --plug "$PROVIDER" "$PROVIDER" -o "$COMPOSED" 2>/dev/null; then
  echo "FAIL: compose should reject identical provider/socket paths"
  exit 1
fi
echo PASS compose validate scaffold
