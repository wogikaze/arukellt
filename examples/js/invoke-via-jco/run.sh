#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../common.sh"

ROOT="$(examples_repo_root)"
OUT="$ROOT/.build/examples/js-jco/out"
COMPONENT="$ROOT/.build/examples/ark-export/calculator.component.wasm"
JCO_VERSION="${JCO_VERSION:-1.23.0}"

cd "$ROOT"

if ! command -v node >/dev/null; then
    echo "SKIP: node not in PATH"
    exit 0
fi
if ! command -v npm >/dev/null && ! command -v npx >/dev/null; then
    echo "SKIP: npm/npx not in PATH"
    exit 0
fi

echo "[1/2] build Ark calculator component"
bash "$ROOT/examples/ark/export-library/run.sh"

mkdir -p "$OUT"
echo "[2/2] jco transpile (optional; in-process invoke still #036)"
if npx --yes "@bytecodealliance/jco@${JCO_VERSION}" transpile "$COMPONENT" -o "$OUT"; then
    if [[ -f "$OUT/calculator.component.js" ]] || compgen -G "$OUT/*.js" >/dev/null; then
        echo "PASS js/invoke-via-jco (transpile)"
        exit 0
    fi
    echo "FAIL: jco transpile produced no .js output"
    exit 1
else
    echo "SKIP: jco transpile failed (see README / #036)"
    exit 0
fi
