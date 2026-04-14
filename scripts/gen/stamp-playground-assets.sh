#!/usr/bin/env bash
# scripts/gen/stamp-playground-assets.sh — Content-hash stamp for playground assets.
#
# Creates a content-addressed copy of the playground Wasm binary and writes an
# asset-manifest.json that the browser entrypoint reads to load the correct
# versioned Wasm URL.
#
# The Wasm binary is the dominant playground asset (≈247 KB). Content-hashed
# filenames ensure browsers cache it by hash: a Rust-source change produces a
# new hash, a new filename, and automatic cache invalidation — with no explicit
# cache purge needed. JS files continue to be served under their original names
# (short GitHub Pages TTL provides adequate freshness until a proper bundler is
# wired in; tracked separately).
#
# Usage:
#   stamp-playground-assets.sh [--wasm-dir <dir>] [--manifest <path>]
#
# Defaults:
#   --wasm-dir   docs/playground/wasm
#   --manifest   docs/playground/wasm/asset-manifest.json
#
# Called by `npm run build:app` (playground/package.json) after wasm assets are
# copied into docs/playground/. Safe to run when wasm is absent — prints a skip
# notice and exits 0.
#
# See docs/playground/deployment-strategy.md §3.2–§3.3 for strategy rationale.

set -euo pipefail

# ─── Defaults ──────────────────────────────────────────────────────────────
WASM_DIR="docs/playground/wasm"
MANIFEST=""

# ─── Argument parsing ──────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --wasm-dir)  WASM_DIR="$2"; shift 2 ;;
    --manifest)  MANIFEST="$2"; shift 2 ;;
    --help|-h)
      sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "[stamp-assets] Unknown argument: $1" >&2
      echo "[stamp-assets] Run with --help for usage." >&2
      exit 1
      ;;
  esac
done

if [[ -z "$MANIFEST" ]]; then
  MANIFEST="${WASM_DIR}/asset-manifest.json"
fi

WASM_FILE="${WASM_DIR}/ark_playground_wasm_bg.wasm"

# ─── Skip gracefully when wasm has not been built ──────────────────────────
if [[ ! -f "$WASM_FILE" ]]; then
  echo "[stamp-assets] SKIP: wasm file not found: $WASM_FILE"
  echo "[stamp-assets]   Run 'npm run build:wasm' in playground/ to build wasm."
  echo "[stamp-assets]   stamp-playground-assets.sh will run automatically on next build:app."
  exit 0
fi

# ─── Compute SHA-256; use first 12 hex chars as the content hash ───────────
# sha256sum (GNU coreutils) or shasum (macOS) — try both.
if command -v sha256sum &>/dev/null; then
  HASH=$(sha256sum "$WASM_FILE" | awk '{print $1}')
elif command -v shasum &>/dev/null; then
  HASH=$(shasum -a 256 "$WASM_FILE" | awk '{print $1}')
else
  echo "[stamp-assets] ERROR: neither sha256sum nor shasum found in PATH" >&2
  exit 1
fi

HASH12="${HASH:0:12}"
HASHED_WASM="ark_playground_wasm_bg-${HASH12}.wasm"

# ─── Copy wasm to content-hashed filename ──────────────────────────────────
cp "$WASM_FILE" "${WASM_DIR}/${HASHED_WASM}"

# ─── Write asset manifest ──────────────────────────────────────────────────
# The browser entrypoint (docs/playground/index.html) fetches this manifest to
# resolve the content-hashed Wasm URL at runtime.
cat > "$MANIFEST" <<EOF
{
  "wasmUrl": "./wasm/${HASHED_WASM}",
  "wasmHash": "${HASH12}"
}
EOF

echo "[stamp-assets] Wasm content-hash: ${HASH12}"
echo "[stamp-assets] Hashed copy: ${WASM_DIR}/${HASHED_WASM}"
echo "[stamp-assets] Manifest written: ${MANIFEST}"
