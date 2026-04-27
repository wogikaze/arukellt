#!/usr/bin/env bash
# Update performance baselines by measuring current compile time, runtime, and binary size.
#
# Usage:
#   bash scripts/update-baselines.sh [--dry-run]
#
# Options:
#   --dry-run   Print what would be written but do not write files.
#
# Thresholds (used by scripts/check/perf-gate.sh in CI):
#   compile_ms:    +20% → failure
#   run_ms:        +10% → failure
#   binary_bytes:  +15% → failure
#
# Output:
#   tests/baselines/perf/baselines.json   (updated in-place)
#   docs/process/benchmark-results.md     (updated in-place)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DRY_RUN=false

for arg in "$@"; do
    case "$arg" in
        --dry-run)
            DRY_RUN=true
            ;;
        --help|-h)
            sed -n '2,/^$/p' "$0" | grep '^#' | sed 's/^# \?//'
            exit 0
            ;;
        *)
            echo "error: unknown option: $arg" >&2
            exit 1
            ;;
    esac
done

BASELINE="$ROOT/tests/baselines/perf/baselines.json"
OUTPUT_JSON="$ROOT/tests/baselines/perf/current.json"
OUTPUT_MD="$ROOT/docs/process/benchmark-results.md"

if [ "$DRY_RUN" = true ]; then
    echo "[dry-run] Would run benchmark_runner.py --mode update-baseline"
    echo "[dry-run] Would write: $BASELINE"
    echo "[dry-run] Would write: $OUTPUT_JSON"
    echo "[dry-run] Would write: $OUTPUT_MD"
    exit 0
fi

echo "Updating performance baselines..."
echo "  baseline: $BASELINE"
echo "  output:   $OUTPUT_JSON"
echo ""

python3 "$ROOT/scripts/util/benchmark_runner.py" \
    --mode update-baseline \
    --baseline "$BASELINE" \
    --output-json "$OUTPUT_JSON" \
    --output-md "$OUTPUT_MD"

echo ""
echo "Baselines updated. Commit tests/baselines/perf/baselines.json to lock in new baselines."
echo "Run 'bash scripts/run/verify-harness.sh --perf-gate' to validate."
