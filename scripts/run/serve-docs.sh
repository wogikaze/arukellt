#!/usr/bin/env bash
# scripts/run/serve-docs.sh — Launch the Arukellt Board in a local browser.
#
# The board is a read-only kanban SPA for issues, ADRs, and docs. It replaces
# the retired tools/doc-viewer.
#
# Usage:
#   scripts/run/serve-docs.sh              # default port 8765, auto-open browser
#   scripts/run/serve-docs.sh -p 9000      # custom port
#   scripts/run/serve-docs.sh --no-open    # do not open browser
#   scripts/run/serve-docs.sh --dev        # vite dev server (hot reload)
set -euo pipefail

PORT=8765
OPEN_BROWSER=1
MODE=prod

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p|--port) PORT="$2"; shift 2 ;;
    --no-open) OPEN_BROWSER=0; shift ;;
    --dev) MODE=dev; shift ;;
    -h|--help)
      sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "serve-docs: unknown option '$1' (try --help)" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BOARD_DIR="$REPO_ROOT/tools/board"

if [[ ! -d "$BOARD_DIR" ]]; then
  echo "serve-docs: error — $BOARD_DIR not found" >&2
  exit 1
fi

if ! command -v node >/dev/null 2>&1; then
  echo "serve-docs: error — node is required" >&2
  exit 127
fi

if [[ ! -d "$BOARD_DIR/node_modules" ]]; then
  echo "serve-docs: installing board dependencies..." >&2
  (cd "$BOARD_DIR" && npm ci)
fi

cd "$BOARD_DIR"

if [[ "$MODE" == "dev" ]]; then
  ARGS=(--port "$PORT" --host 127.0.0.1)
  if [[ "$OPEN_BROWSER" -eq 1 ]]; then
    ARGS+=(--open)
  fi
  exec npx vite "${ARGS[@]}"
else
  if [[ ! -f "$BOARD_DIR/dist/client/index.html" ]]; then
    npm run build
  fi
  SERVER_ARGS=(-p "$PORT")
  if [[ "$OPEN_BROWSER" -eq 1 ]]; then
    SERVER_ARGS+=(--open)
  fi
  exec node dist/server/main.js "${SERVER_ARGS[@]}"
fi
