#!/usr/bin/env bash
set -euo pipefail
if [[ -z "${ARUKELLT_ZIG_COMPONENT:-}" ]]; then echo "SKIP: ARUKELLT_ZIG_COMPONENT not set"; exit 0; fi
wasm-tools component wit "$ARUKELLT_ZIG_COMPONENT" >/dev/null
