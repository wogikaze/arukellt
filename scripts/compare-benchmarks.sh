#!/usr/bin/env bash
# Benchmark comparison helper for Arukellt benchmark suite.
#
# This wrapper runs the canonical benchmark runner in compare mode and writes
# the markdown report to docs/process/benchmark-results.md.
#
# Usage:
#   bash scripts/compare-benchmarks.sh
#   bash scripts/compare-benchmarks.sh --quick
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="compare"
EXTRA_ARGS=()

for arg in "$@"; do
  case "$arg" in
    --quick)
      # Compare mode with low iteration counts for a fast sanity run.
      EXTRA_ARGS+=(--compile-iterations 1 --runtime-iterations 1 --runtime-warmups 0)
      ;;
    --full)
      ;;
    *)
      EXTRA_ARGS+=("$arg")
      ;;
  esac
done

exec python3 "$ROOT/scripts/util/benchmark_runner.py" \
  --mode "$MODE" \
  --output-md "$ROOT/docs/process/benchmark-results.md" \
  "${EXTRA_ARGS[@]}"
