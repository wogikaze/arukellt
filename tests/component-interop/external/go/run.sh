#!/usr/bin/env bash
set -euo pipefail
if [[ -z "${ARUKELLT_GO_COMPONENT:-}" ]]; then echo "SKIP: ARUKELLT_GO_COMPONENT not set"; exit 0; fi
wasm-tools component wit "$ARUKELLT_GO_COMPONENT" >/dev/null
