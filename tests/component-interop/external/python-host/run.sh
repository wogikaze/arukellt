#!/usr/bin/env bash
set -euo pipefail
if [[ -z "${ARUKELLT_COMPONENT:-}" ]]; then echo "SKIP: ARUKELLT_COMPONENT not set"; exit 0; fi
python3 - <<'PY'
try:
    import wasmtime  # noqa: F401
except ImportError:
    print("SKIP: Python wasmtime package not installed")
    raise SystemExit(0)
print("PASS: Python host can load wasmtime component API")
PY
