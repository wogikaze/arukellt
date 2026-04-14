#!/usr/bin/env bash
# Performance gate: detect compile-time, execution-time, and binary-size regressions.
#
# Usage: scripts/perf-gate.sh [--update]
#   --update   Write current measurements as new baselines (no comparison failure).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MODE="ci"
EXTRA_ARGS=()

for arg in "$@"; do
    case "$arg" in
        --update)
            MODE="update-baseline"
            ;;
        *)
            EXTRA_ARGS+=("$arg")
            ;;
    esac
done

exec python3 "$ROOT/scripts/util/benchmark_runner.py" \
    --mode "$MODE" \
    --baseline "$ROOT/tests/baselines/perf/baselines.json" \
    --output-json "$ROOT/tests/baselines/perf/current.json" \
    --output-md "$ROOT/docs/process/benchmark-results.md" \
    "${EXTRA_ARGS[@]}"
