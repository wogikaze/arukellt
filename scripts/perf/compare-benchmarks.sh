#!/usr/bin/env bash
# Benchmark comparison helper for Arukellt benchmark suite.
#
# This wrapper runs the canonical benchmark runner in compare mode and writes
# the markdown report to docs/history/benchmarks/benchmark-results.md.
#
# Usage:
#   bash scripts/compare-benchmarks.sh
#   bash scripts/compare-benchmarks.sh --quick
#   bash scripts/compare-benchmarks.sh --compare-lang grain
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MODE="compare"
EXTRA_ARGS=()

COMPARE_LANG=""

usage() {
  cat <<'EOF'
Usage: compare-benchmarks.sh [OPTIONS]

Run the Arukellt benchmark runner in compare mode and update
docs/history/benchmarks/benchmark-results.md.

Options:
  --quick                 Low iteration counts for a fast sanity run
  --full                  Full iteration counts (default)
  --compare-lang LANG     Optional cross-language hook (comma-separated)
                          Supported: grain — compile benchmarks/fib.grain when
                          the grain CLI is installed; skips gracefully otherwise
  --help                  Show this help and exit

Examples:
  bash scripts/compare-benchmarks.sh
  bash scripts/compare-benchmarks.sh --compare-lang grain
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h)
      usage
      exit 0
      ;;
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
  echo "Grain hook: optional Wasm GC cross-language comparison"
  if ! command -v grain >/dev/null 2>&1; then
    echo "grain CLI not installed — skipping Grain compile/compare (see benchmarks/fib.grain)"
  else
    echo "Compiling Grain benchmark..."
    grain compile "$ROOT/benchmarks/fib.grain" -o "$ROOT/benchmarks/fib_grain.wasm" || true
    if command -v hyperfine >/dev/null; then
      echo "Running cross-language benchmark with hyperfine..."
      hyperfine --warmup 1 "wasmtime $ROOT/benchmarks/legacy/fib.wasm" "wasmtime $ROOT/benchmarks/fib_grain.wasm"
    else
      echo "Hyperfine not installed. Skipping cross-language hyperfine comparison."
    fi
  fi
fi


exec python3 "$ROOT/scripts/util/benchmark_runner.py" \
  --mode "$MODE" \
  --output-md "$ROOT/docs/history/benchmarks/benchmark-results.md" \
  "${EXTRA_ARGS[@]}"
