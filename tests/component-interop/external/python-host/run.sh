#!/usr/bin/env bash
set -euo pipefail
if [[ -z "${ARUKELLT_COMPONENT:-}" ]]; then
  echo "SKIP: ARUKELLT_COMPONENT not set"
  exit 0
fi
if ! command -v wasmtime >/dev/null; then
  echo "SKIP: wasmtime unavailable"
  exit 0
fi
ARUKELLT_COMPONENT="$ARUKELLT_COMPONENT" python3 - <<'PY'
import os
import subprocess
component = os.environ["ARUKELLT_COMPONENT"]
run = subprocess.run(
    ["wasmtime", "run", "--wasm", "gc", "--wasm", "component-model", component],
    capture_output=True,
    text=True,
)
if run.returncode != 0:
    raise SystemExit(f"Python host failed to run Arukellt component: {run.stderr or run.stdout}")
print("PASS: Python host ran Arukellt component")
PY
