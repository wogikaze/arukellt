#!/usr/bin/env bash
# compare-benchmarks.sh — generate current benchmark results and compare against baseline.
# Usage: bash scripts/compare-benchmarks.sh [--update-baseline]

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="compare"
EXTRA_ARGS=()

for arg in "$@"; do
    case "$arg" in
        --update-baseline)
            MODE="update-baseline"
            ;;
        *)
            EXTRA_ARGS+=("$arg")
            ;;
    esac
done

exec python3 "$ROOT/scripts/benchmark_runner.py" \
    --mode "$MODE" \
    --baseline "$ROOT/tests/baselines/perf/baselines.json" \
    --output-json "$ROOT/tests/baselines/perf/current.json" \
    --output-md "$ROOT/docs/process/benchmark-results.md" \
    "${EXTRA_ARGS[@]}"
