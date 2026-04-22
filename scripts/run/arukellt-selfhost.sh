#!/usr/bin/env bash
# scripts/run/arukellt-selfhost.sh — Selfhost-first arukellt entrypoint (#559).
#
# This wrapper makes the **selfhost wasm** the default execution path for the
# user-facing `arukellt` CLI. Phase 5 (#560–#564) deletes the Rust crates that
# provided the legacy compiler binary; this wrapper is the prerequisite that
# lets verify/CI/users invoke the selfhost compiler today, with the legacy
# Rust pipeline reachable only via an explicit opt-in env var.
#
# Resolution order (default = selfhost):
#   1. If $ARUKELLT_USE_RUST is set to a truthy value (1/true/yes), execute
#      the Rust binary at $ARUKELLT_RUST_BIN, target/release/arukellt, or
#      target/debug/arukellt (in that order). This is the explicit opt-in for
#      the legacy path during the Phase 5 transition.
#   2. Otherwise locate the selfhost wasm in this order:
#        $ARUKELLT_SELFHOST_WASM
#        .build/selfhost/arukellt-s2.wasm
#        .bootstrap-build/arukellt-s2.wasm
#      then exec `wasmtime run --dir=<repo_root> <wasm> -- "$@"`.
#   3. If neither wasmtime nor a selfhost wasm is available, fall back to the
#      Rust binary with a one-line warning to stderr (so CI/dev loops keep
#      working until the selfhost artifact is built).
#
# Exit codes are forwarded from the underlying process.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

is_truthy() {
  case "${1:-}" in
    1|true|TRUE|True|yes|YES|on|ON) return 0 ;;
    *) return 1 ;;
  esac
}

resolve_rust_bin() {
  if [[ -n "${ARUKELLT_RUST_BIN:-}" ]] && [[ -x "$ARUKELLT_RUST_BIN" ]]; then
    echo "$ARUKELLT_RUST_BIN"; return 0
  fi
  for cand in "$REPO_ROOT/target/release/arukellt" "$REPO_ROOT/target/debug/arukellt"; do
    if [[ -x "$cand" ]]; then echo "$cand"; return 0; fi
  done
  return 1
}

resolve_selfhost_wasm() {
  if [[ -n "${ARUKELLT_SELFHOST_WASM:-}" ]] && [[ -f "$ARUKELLT_SELFHOST_WASM" ]]; then
    echo "$ARUKELLT_SELFHOST_WASM"; return 0
  fi
  for cand in \
    "$REPO_ROOT/.build/selfhost/arukellt-s2.wasm" \
    "$REPO_ROOT/.bootstrap-build/arukellt-s2.wasm"; do
    if [[ -f "$cand" ]]; then echo "$cand"; return 0; fi
  done
  return 1
}

# Explicit opt-in: legacy Rust path.
if is_truthy "${ARUKELLT_USE_RUST:-}"; then
  if rust_bin="$(resolve_rust_bin)"; then
    exec "$rust_bin" "$@"
  fi
  echo "arukellt-selfhost: ARUKELLT_USE_RUST set but no Rust binary found" >&2
  exit 127
fi

# Default: selfhost wasm via wasmtime.
if command -v wasmtime >/dev/null 2>&1; then
  if wasm="$(resolve_selfhost_wasm)"; then
    exec wasmtime run --dir="$REPO_ROOT" "$wasm" -- "$@"
  fi
fi

# Transitional fallback: selfhost prereqs missing → use Rust with a warning.
if rust_bin="$(resolve_rust_bin)"; then
  echo "arukellt-selfhost: warning — selfhost wasm or wasmtime unavailable; " \
       "falling back to legacy Rust binary at $rust_bin (set ARUKELLT_USE_RUST=1 to silence)" >&2
  exec "$rust_bin" "$@"
fi

echo "arukellt-selfhost: error — neither selfhost wasm (.build/selfhost/arukellt-s2.wasm) " \
     "nor a Rust binary (target/{release,debug}/arukellt) is available; build one first" >&2
exit 127
