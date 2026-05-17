#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
MANIFEST="${ROOT}/docs/playground/wasm/asset-manifest.json"

rm -f "${MANIFEST}"
echo "playground asset stamp: retired; browser engine is bundled in docs/playground/dist"
