#!/usr/bin/env bash
set -euo pipefail
if [[ -z "${ARUKELLT_C_COMPONENT:-}" ]]; then echo "SKIP: ARUKELLT_C_COMPONENT not set"; exit 0; fi
wasm-tools component wit "$ARUKELLT_C_COMPONENT" >/dev/null
