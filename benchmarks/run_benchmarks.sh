#!/usr/bin/env bash
# Benchmark runner for Arukellt benchmark suite.
# Usage: bash benchmarks/run_benchmarks.sh [--quick]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="full"
ARGS=()

for arg in "$@"; do
    case "$arg" in
        --quick)
            MODE="quick"
            ;;
        *)
            ARGS+=("$arg")
            ;;
    esac
done

exec python3 "$ROOT/scripts/util/benchmark_runner.py" --mode "$MODE" "${ARGS[@]}"
