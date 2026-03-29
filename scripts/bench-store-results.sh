#!/usr/bin/env bash
# bench-store-results.sh — store benchmark JSON results with date+commit naming.
#
# Usage:
#   bash scripts/run-benchmarks.sh --full | bash scripts/bench-store-results.sh
#   bash scripts/bench-store-results.sh results.json
#   bash scripts/bench-store-results.sh < results.json
#
# Stores into benchmarks/results/<YYYYMMDD>-<short-commit>.json and
# updates benchmarks/results/latest.json as a symlink to the newest file.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RESULTS_DIR="$ROOT/benchmarks/results"
mkdir -p "$RESULTS_DIR"

# --- resolve input -----------------------------------------------------------
INPUT_FILE=""
if [[ $# -ge 1 && -f "$1" ]]; then
  INPUT_FILE="$1"
elif [[ ! -t 0 ]]; then
  # Reading from stdin — buffer to a temp file inside the project
  INPUT_FILE="$RESULTS_DIR/.bench-input-$$.json"
  cat > "$INPUT_FILE"
  trap 'rm -f "$INPUT_FILE"' EXIT
else
  echo "Usage: bench-store-results.sh <results.json>" >&2
  echo "       bench-store-results.sh < results.json" >&2
  echo "       some-command | bench-store-results.sh" >&2
  exit 1
fi

# --- validate it looks like JSON ---------------------------------------------
if ! python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$INPUT_FILE" 2>/dev/null; then
  echo "ERROR: input is not valid JSON" >&2
  exit 1
fi

# --- build target filename ---------------------------------------------------
DATE_STAMP=$(date -u +"%Y%m%d")
COMMIT_SHORT=$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
DEST_NAME="${DATE_STAMP}-${COMMIT_SHORT}.json"
DEST_PATH="$RESULTS_DIR/$DEST_NAME"

# Avoid clobbering: append a counter if file already exists
if [[ -f "$DEST_PATH" ]]; then
  SEQ=2
  while [[ -f "${RESULTS_DIR}/${DATE_STAMP}-${COMMIT_SHORT}-${SEQ}.json" ]]; do
    SEQ=$(( SEQ + 1 ))
  done
  DEST_NAME="${DATE_STAMP}-${COMMIT_SHORT}-${SEQ}.json"
  DEST_PATH="$RESULTS_DIR/$DEST_NAME"
fi

# --- inject commit metadata if missing --------------------------------------
python3 -c "
import json, sys

with open(sys.argv[1]) as f:
    data = json.load(f)

# Add commit info if not present
if 'commit' not in data:
    data['commit'] = sys.argv[2]
if 'commit_short' not in data:
    data['commit_short'] = sys.argv[3]

with open(sys.argv[4], 'w') as f:
    json.dump(data, f, indent=2)
    f.write('\n')
" "$INPUT_FILE" "$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo "unknown")" \
  "$COMMIT_SHORT" "$DEST_PATH"

# --- update latest.json symlink ----------------------------------------------
LATEST="$RESULTS_DIR/latest.json"
rm -f "$LATEST"
ln -s "$DEST_NAME" "$LATEST"

echo "Stored: benchmarks/results/$DEST_NAME"
echo "Latest: benchmarks/results/latest.json -> $DEST_NAME"
