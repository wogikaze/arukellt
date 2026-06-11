#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
ASSETS_DIR="${ROOT}/docs/playground/assets"
MANIFEST="${ASSETS_DIR}/compiler-asset.json"
DEST_WASM="${ASSETS_DIR}/arukellt-selfhost.wasm"

mkdir -p "${ASSETS_DIR}"

pick_compiler_wasm() {
  if [[ -f "${ROOT}/.build/selfhost/arukellt-s3.wasm" ]]; then
    echo "${ROOT}/.build/selfhost/arukellt-s3.wasm"
    return 0
  fi
  if [[ -f "${ROOT}/.build/selfhost/arukellt-s2.wasm" ]]; then
    echo "${ROOT}/.build/selfhost/arukellt-s2.wasm"
    return 0
  fi
  if [[ -f "${ROOT}/bootstrap/arukellt-selfhost.wasm" ]]; then
    echo "${ROOT}/bootstrap/arukellt-selfhost.wasm"
    return 0
  fi
  return 1
}

if ! SRC_WASM="$(pick_compiler_wasm)"; then
  echo "stamp-playground-assets: no compiler wasm source found" >&2
  exit 1
fi

cp "${SRC_WASM}" "${DEST_WASM}"
SIZE="$(wc -c < "${DEST_WASM}" | tr -d ' ')"
SHA256="$(sha256sum "${DEST_WASM}" | awk '{print $1}')"
SOURCE="$(realpath --relative-to="${ROOT}" "${SRC_WASM}")"

cat > "${MANIFEST}" <<EOF
{
  "path": "assets/arukellt-selfhost.wasm",
  "source": "${SOURCE}",
  "size_bytes": ${SIZE},
  "sha256": "${SHA256}"
}
EOF

echo "playground asset stamp: ${DEST_WASM} (${SIZE} bytes, sha256=${SHA256})"
