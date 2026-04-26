#!/usr/bin/env bash
# scripts/run/arukellt-selfhost.sh — Selfhost-only arukellt entrypoint (#559, #583).
#
# This wrapper executes the **selfhost wasm** for every invocation. Per #583
# (and ADR-029, #585) the legacy `ARUKELLT_USE_RUST=1` opt-in has been
# RETIRED — the Rust legacy CLI no longer exists. Setting `ARUKELLT_USE_RUST`
# now hard-fails with a pointer to this notice.
#
# Resolution order:
#   1. `$ARUKELLT_SELFHOST_WASM`
#   2. `.build/selfhost/arukellt-s2.wasm`
#   3. `.bootstrap-build/arukellt-s2.wasm`
#   4. `bootstrap/arukellt-selfhost.wasm` (committed pinned reference;
#      see `bootstrap/PROVENANCE.md`)
# Then `wasmtime run --dir=<repo_root> <wasm> -- "$@"`.
#
# Exit codes are forwarded from the underlying selfhost process.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

is_truthy() {
  case "${1:-}" in
    1|true|TRUE|True|yes|YES|on|ON) return 0 ;;
    *) return 1 ;;
  esac
}

resolve_selfhost_wasm() {
  if [[ -n "${ARUKELLT_SELFHOST_WASM:-}" ]] && [[ -f "$ARUKELLT_SELFHOST_WASM" ]]; then
    echo "$ARUKELLT_SELFHOST_WASM"; return 0
  fi
  for cand in \
    "$REPO_ROOT/.build/selfhost/arukellt-s2.wasm" \
    "$REPO_ROOT/.bootstrap-build/arukellt-s2.wasm" \
    "$REPO_ROOT/bootstrap/arukellt-selfhost.wasm"; do
    if [[ -f "$cand" ]]; then echo "$cand"; return 0; fi
  done
  return 1
}

# Retired opt-in: hard-fail with a clear pointer rather than silently
# fall back. Per #583 (ADR-029) the Rust legacy CLI has been removed.
if is_truthy "${ARUKELLT_USE_RUST:-}"; then
  cat >&2 <<'EOF'
arukellt-selfhost: ARUKELLT_USE_RUST is set, but the legacy Rust CLI has
been retired (#583, ADR-029). Selfhost is now the only execution path.
Unset ARUKELLT_USE_RUST and re-run, or invoke the selfhost wasm directly
via `wasmtime run bootstrap/arukellt-selfhost.wasm -- <args>`.
EOF
  exit 2
fi

if ! command -v wasmtime >/dev/null 2>&1; then
  echo "arukellt-selfhost: error — wasmtime not found in PATH; install wasmtime ≥ 30" >&2
  exit 127
fi

if ! wasm="$(resolve_selfhost_wasm)"; then
  echo "arukellt-selfhost: error — no selfhost wasm available." >&2
  echo "  Tried: \$ARUKELLT_SELFHOST_WASM, .build/selfhost/arukellt-s2.wasm," >&2
  echo "         .bootstrap-build/arukellt-s2.wasm, bootstrap/arukellt-selfhost.wasm" >&2
  echo "  Build one with: python3 scripts/manager.py selfhost fixpoint --build" >&2
  exit 127
fi

exec wasmtime run --dir="$REPO_ROOT" "$wasm" -- "$@"
