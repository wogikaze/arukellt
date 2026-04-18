#!/usr/bin/env bash
# Performance gate: detect compile-time, execution-time, and binary-size regressions.
#
# Usage: scripts/check/perf-gate.sh [--update]
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

if ! python3 "$ROOT/scripts/util/benchmark_runner.py" \
    --mode "$MODE" \
    --baseline "$ROOT/tests/baselines/perf/baselines.json" \
    --output-json "$ROOT/tests/baselines/perf/current.json" \
    --output-md "$ROOT/docs/process/benchmark-results.md" \
    "${EXTRA_ARGS[@]}"; then
    cat >&2 <<'EOF'
perf gate: non-zero exit (missing baseline, benchmark error, or regression vs tests/baselines/perf/baselines.json).

Thresholds: compile +20%, run +10%, wasm binary size +15% vs baseline.

If the change is intentional, refresh baselines and commit the JSON:
  bash scripts/update-baselines.sh

Local re-run:
  bash scripts/check/perf-gate.sh
  bash scripts/run/verify-harness.sh --perf-gate
EOF
    exit 1
fi
