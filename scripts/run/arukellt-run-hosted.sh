#!/usr/bin/env bash
# scripts/run/arukellt-run-hosted.sh — Run user wasm with WASI + arukellt_host linker.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HOST_RUN_RELEASE="$REPO_ROOT/tools/host-linker/target/release/arukellt-host-run"
HOST_RUN_DEBUG="$REPO_ROOT/tools/host-linker/target/debug/arukellt-host-run"

ensure_host_run() {
  if [[ -x "$HOST_RUN_RELEASE" ]]; then
    echo "$HOST_RUN_RELEASE"
    return 0
  fi
  if [[ -x "$HOST_RUN_DEBUG" ]]; then
    echo "$HOST_RUN_DEBUG"
    return 0
  fi
  if ! command -v cargo >/dev/null 2>&1; then
    echo "arukellt-run-hosted: error — cargo not found; build tools/host-linker first" >&2
    exit 127
  fi
  cargo build --release --manifest-path "$REPO_ROOT/tools/host-linker/Cargo.toml" >&2
  if [[ -x "$HOST_RUN_RELEASE" ]]; then
    echo "$HOST_RUN_RELEASE"
    return 0
  fi
  cargo build --manifest-path "$REPO_ROOT/tools/host-linker/Cargo.toml" >&2
  if [[ -x "$HOST_RUN_DEBUG" ]]; then
    echo "$HOST_RUN_DEBUG"
    return 0
  fi
  echo "arukellt-run-hosted: error — failed to build arukellt-host-run" >&2
  exit 127
}

HOST_RUN="$(ensure_host_run)"
exec "$HOST_RUN" "$@"
