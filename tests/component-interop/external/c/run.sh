#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
if [[ -z "${ARUKELLT_C_COMPONENT:-}" || -z "${ARUKELLT_SOCKET_COMPONENT:-}" ]]; then
  echo "SKIP: set ARUKELLT_C_COMPONENT and ARUKELLT_SOCKET_COMPONENT"
  exit 0
fi
if ! command -v wasm-tools >/dev/null || ! command -v wac >/dev/null; then
  echo "SKIP: wasm-tools/wac unavailable"
  exit 0
fi
wasm-tools component wit "$ARUKELLT_C_COMPONENT" >/dev/null
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
cat >"$tmp/ark.toml" <<EOF
[dependencies]
provider = { component = "$ARUKELLT_C_COMPONENT" }
EOF
python3 "$ROOT/scripts/component-deps.py" compose --manifest "$tmp/ark.toml" --socket "$ARUKELLT_SOCKET_COMPONENT" -o "$tmp/composed.component.wasm"
wasm-tools validate "$tmp/composed.component.wasm"
echo "PASS: Arukellt socket composed with C provider"
