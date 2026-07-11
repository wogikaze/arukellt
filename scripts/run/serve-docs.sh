#!/usr/bin/env bash
# scripts/run/serve-docs.sh — Launch the docs/ SPA viewer in a local browser.
#
# Serves a self-contained SPA (tools/doc-viewer/) that renders docs/ markdown
# with zero CDN dependencies. Sidebar + full-text search, all client-side.
#
# Usage:
#   scripts/run/serve-docs.sh              # default port 8765, auto-open browser
#   scripts/run/serve-docs.sh -p 9000      # custom port
#   scripts/run/serve-docs.sh --no-open    # do not open browser
set -euo pipefail

PORT=8765
OPEN_BROWSER=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p|--port) PORT="$2"; shift 2 ;;
    --no-open) OPEN_BROWSER=0; shift ;;
    -h|--help)
      sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "serve-docs: unknown option '$1' (try --help)" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SERVE_PY="$REPO_ROOT/tools/doc-viewer/serve.py"

if [[ ! -f "$SERVE_PY" ]]; then
  echo "serve-docs: error — $SERVE_PY not found" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "serve-docs: error — python3 not found" >&2
  exit 127
fi

ARGS=(-p "$PORT")
if [[ "$OPEN_BROWSER" -ne 1 ]]; then
  ARGS+=(--no-open)
fi

exec python3 "$SERVE_PY" "${ARGS[@]}"
