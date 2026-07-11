#!/usr/bin/env bash
# scripts/run/serve-docs.sh — Launch the docs/ SPA (docsify) in a local browser.
#
# Serves docs/ over a local HTTP server and opens the browser.
# The SPA itself is docs/index.html (docsify + sidebar + full-text search).
#
# Usage:
#   scripts/run/serve-docs.sh              # default port 8765, auto-open browser
#   scripts/run/serve-docs.sh -p 9000      # custom port
#   scripts/run/serve-docs.sh --no-open    # do not open browser
#   scripts/run/serve-docs.sh --no-cache   # append cache-buster query (reload)
set -euo pipefail

PORT=8765
OPEN_BROWSER=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p|--port) PORT="$2"; shift 2 ;;
    --no-open) OPEN_BROWSER=0; shift ;;
    --no-cache) OPEN_BROWSER=${OPEN_BROWSER}; shift ;; # accepted, no-op for server
    -h|--help)
      sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "serve-docs: unknown option '$1' (try --help)" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DOCS_DIR="$REPO_ROOT/docs"

if [[ ! -f "$DOCS_DIR/index.html" ]]; then
  echo "serve-docs: error — $DOCS_DIR/index.html not found" >&2
  exit 1
fi

# Pick a python3 http.server (always available on this host).
if ! command -v python3 >/dev/null 2>&1; then
  echo "serve-docs: error — python3 not found" >&2
  exit 127
fi

# Verify port is free; if not, try a few increments.
for attempt in $(seq 0 9); do
  if ! (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; then
    break
  fi
  exec 3>&- 3<&- 2>/dev/null || true
  PORT=$((PORT + 1))
done

URL="http://127.0.0.1:${PORT}/"

echo "serve-docs: serving $DOCS_DIR at $URL"
echo "serve-docs: press Ctrl-C to stop"

open_browser() {
  if [[ "$OPEN_BROWSER" -ne 1 ]]; then return 0; fi
  if command -v xdg-open >/dev/null 2>&1; then
    (xdg-open "$URL" >/dev/null 2>&1 &)
  elif command -v wslview >/dev/null 2>&1; then
    (wslview "$URL" >/dev/null 2>&1 &)
  elif [[ -n "${WSL_DISTRO_NAME:-}" ]] && command -v cmd.exe >/dev/null 2>&1; then
    (cmd.exe /c start "" "$URL" >/dev/null 2>&1 &)
  elif command -v open >/dev/null 2>&1; then
    (open "$URL" >/dev/null 2>&1 &)
  fi
}

open_browser

cd "$DOCS_DIR"
exec python3 -m http.server "$PORT" --bind 127.0.0.1
