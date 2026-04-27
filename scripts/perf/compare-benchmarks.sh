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

COMPARE_LANG=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick)
      # Compare mode with low iteration counts for a fast sanity run.
      EXTRA_ARGS+=(--compile-iterations 1 --runtime-iterations 1 --runtime-warmups 0)
      shift
      ;;
    --full)
      shift
      ;;
    --compare-lang)
      COMPARE_LANG="$2"
      shift 2
      ;;
    *)
      EXTRA_ARGS+=("$1")
      shift
      ;;
  esac
done

if [[ "$COMPARE_LANG" == *"grain"* ]]; then
  echo "Compiling Grain benchmark..."
  grain compile "$ROOT/benchmarks/fib.grain" -o "$ROOT/benchmarks/fib_grain.wasm" || true
  if command -v hyperfine >/dev/null; then
    echo "Running cross-language benchmark with hyperfine..."
    hyperfine --warmup 1 "wasmtime $ROOT/benchmarks/legacy/fib.wasm" "wasmtime $ROOT/benchmarks/fib_grain.wasm"
  else
    echo "Hyperfine not installed. Skipping cross-language hyperfine comparison."
  fi
fi


exec python3 "$ROOT/scripts/util/benchmark_runner.py" \
  --mode "$MODE" \
  --output-md "$ROOT/docs/process/benchmark-results.md" \
  "${EXTRA_ARGS[@]}"
