#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
WASM_DIR="${ROOT}/docs/playground/wasm"
WASM_FIXED="${WASM_DIR}/ark_playground_wasm_bg.wasm"
MANIFEST="${WASM_DIR}/asset-manifest.json"

if [[ ! -f "${WASM_FIXED}" ]]; then
  rm -f "${MANIFEST}"
  echo "playground asset stamp: ${WASM_FIXED} not found; skipping wasm cache stamp"
  exit 0
fi

hash="$(sha256sum "${WASM_FIXED}" | awk '{print substr($1, 1, 12)}')"
hashed_name="ark_playground_wasm_bg-${hash}.wasm"
hashed_path="${WASM_DIR}/${hashed_name}"

cp "${WASM_FIXED}" "${hashed_path}"

cat > "${MANIFEST}" <<EOF
{
  "wasmUrl": "./wasm/${hashed_name}",
  "wasmHash": "${hash}",
  "source": "./wasm/ark_playground_wasm_bg.wasm"
}
EOF

echo "playground asset stamp: wrote ${MANIFEST}"
