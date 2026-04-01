#!/usr/bin/env bash
# Arukellt benchmark runner — standalone shell version.
# Builds the compiler (release), compiles + runs each benchmark .ark file,
# times both phases, and emits JSON matching benchmarks/schema.json.
#
# Usage:
#   bash scripts/run-benchmarks.sh            # quick (1 iteration, default)
#   bash scripts/run-benchmarks.sh --quick     # same as above
#   bash scripts/run-benchmarks.sh --full      # 10 iterations per benchmark
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPILER="${ARUKELLT_BIN:-$ROOT/target/release/arukellt}"
RESULTS_DIR="$ROOT/benchmarks/results"
TARGET="wasm32-wasi-p1"
SCHEMA_VERSION="arukellt-bench-v1"

# --- mode parsing -----------------------------------------------------------
MODE="quick"
COMPILE_ITERS=1
RUNTIME_ITERS=1
WARMUPS=0

for arg in "$@"; do
  case "$arg" in
    --quick) MODE="quick";  COMPILE_ITERS=1;  RUNTIME_ITERS=1;  WARMUPS=0 ;;
    --full)  MODE="full";   COMPILE_ITERS=10; RUNTIME_ITERS=10; WARMUPS=1 ;;
    --target=*) TARGET="${arg#--target=}" ;;
    *) echo "Unknown flag: $arg" >&2; exit 1 ;;
  esac
done

MODE_DESC="quick"
[[ "$MODE" == "full" ]] && MODE_DESC="full local benchmark (10 iterations)"
[[ "$MODE" == "quick" ]] && MODE_DESC="single-sample local smoke benchmark"

# --- benchmark cases ---------------------------------------------------------
BENCH_NAMES=(fib binary_tree vec_ops string_concat)
declare -A BENCH_DESC=(
  [fib]="Iterative Fibonacci(35)"
  [binary_tree]="Recursive node counting (depth 20)"
  [vec_ops]="Vec push/sum/contains (1k elements)"
  [string_concat]="String concat in loop (100 iterations)"
)
declare -A BENCH_TAGS=(
  [fib]='["cpu-bound","loop","scalar"]'
  [binary_tree]='["recursion-heavy","allocation-light","call-heavy"]'
  [vec_ops]='["allocation-heavy","container","iteration"]'
  [string_concat]='["string-heavy","allocation-heavy","gc-pressure"]'
)

# --- helpers -----------------------------------------------------------------
now_iso() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }

# Portable millisecond timer (uses date +%s%N where available, falls back to s).
ms_now() {
  if date +%s%N >/dev/null 2>&1; then
    echo $(( $(date +%s%N) / 1000000 ))
  else
    echo $(( $(date +%s) * 1000 ))
  fi
}

median() {
  # Read whitespace-separated numbers from stdin, print median.
  python3 -c "
import sys, statistics
vals = [float(x) for x in sys.stdin.read().split()]
print(statistics.median(vals))
" 2>/dev/null || {
    # Fallback: sort + pick middle
    local nums=("$@")
    local sorted
    sorted=($(printf '%s\n' "${nums[@]}" | sort -n))
    echo "${sorted[$(( ${#sorted[@]} / 2 ))]}"
  }
}

json_escape() { python3 -c "import json,sys; print(json.dumps(sys.stdin.read().rstrip('\n')))" 2>/dev/null || echo '""'; }

# --- ensure release build ----------------------------------------------------
if [[ ! -x "$COMPILER" ]]; then
  echo "::group::Building compiler in release mode"
  cargo build --release -p arukellt
  echo "::endgroup::"
else
  echo "Compiler already built: $COMPILER"
fi

# --- detect tooling ----------------------------------------------------------
WASMTIME="$(command -v wasmtime 2>/dev/null || true)"
TOOLING_WASMTIME="false"
TOOLING_WASMTIME_PATH="null"
if [[ -n "$WASMTIME" ]]; then
  TOOLING_WASMTIME="true"
  TOOLING_WASMTIME_PATH="\"$WASMTIME\""
fi

HYPERFINE="$(command -v hyperfine 2>/dev/null || true)"
TOOLING_HYPERFINE="false"
TOOLING_HYPERFINE_PATH="null"
if [[ -n "$HYPERFINE" ]]; then
  TOOLING_HYPERFINE="true"
  TOOLING_HYPERFINE_PATH="\"$HYPERFINE\""
else
  echo "NOTE: hyperfine not found — using built-in shell timing (skipped: hyperfine)"
fi

if [[ -z "$WASMTIME" ]]; then
  echo "NOTE: wasmtime not found — runtime benchmarks will be skipped"
fi

# --- run benchmarks ----------------------------------------------------------
BENCHMARKS_JSON=""
SEP=""

for bench in "${BENCH_NAMES[@]}"; do
  SRC="benchmarks/${bench}.ark"
  EXPECTED="benchmarks/${bench}.expected"
  WASM_OUT="$RESULTS_DIR/${bench}.wasm"

  echo "--- $bench ---"

  # ---- compile timing -------------------------------------------------------
  COMPILE_CMD="$COMPILER compile $SRC -o $WASM_OUT --target $TARGET"
  COMPILE_SAMPLES=""
  for (( i=0; i<COMPILE_ITERS; i++ )); do
    rm -f "$WASM_OUT"
    t0=$(ms_now)
    $COMPILE_CMD >/dev/null 2>&1
    t1=$(ms_now)
    elapsed=$(( t1 - t0 ))
    COMPILE_SAMPLES="$COMPILE_SAMPLES $elapsed"
  done
  COMPILE_MEDIAN=$(echo "$COMPILE_SAMPLES" | median)
  BINARY_BYTES=0
  [[ -f "$WASM_OUT" ]] && BINARY_BYTES=$(wc -c < "$WASM_OUT" | tr -d ' ')

  COMPILE_SAMPLES_JSON=$(echo "$COMPILE_SAMPLES" | python3 -c "
import sys
vals = sys.stdin.read().split()
print('[' + ','.join(vals) + ']')
" 2>/dev/null || echo "[]")

  # ---- runtime timing -------------------------------------------------------
  RUNTIME_STATUS="skipped"
  RUNTIME_SAMPLES_JSON="[]"
  RUNTIME_MEDIAN="0"
  RUNTIME_ITERS_DONE=0
  STDOUT_ESCAPED='""'
  CORRECTNESS="fail"
  RUN_CMD=""

  if [[ -n "$WASMTIME" && -f "$WASM_OUT" ]]; then
    if [[ "$TARGET" == "wasm32-wasi-p2" ]]; then
      RUN_CMD="wasmtime run --wasm gc $WASM_OUT"
    else
      RUN_CMD="wasmtime run $WASM_OUT"
    fi
    RUNTIME_STATUS="ok"

    # warmups
    for (( w=0; w<WARMUPS; w++ )); do
      $RUN_CMD >/dev/null 2>&1 || true
    done

    RUNTIME_SAMPLES=""
    CAPTURED_STDOUT=""
    for (( i=0; i<RUNTIME_ITERS; i++ )); do
      t0=$(ms_now)
      CAPTURED_STDOUT=$($RUN_CMD 2>/dev/null) || true
      t1=$(ms_now)
      elapsed=$(( t1 - t0 ))
      RUNTIME_SAMPLES="$RUNTIME_SAMPLES $elapsed"
    done
    RUNTIME_ITERS_DONE=$RUNTIME_ITERS
    RUNTIME_MEDIAN=$(echo "$RUNTIME_SAMPLES" | median)
    RUNTIME_SAMPLES_JSON=$(echo "$RUNTIME_SAMPLES" | python3 -c "
import sys
vals = sys.stdin.read().split()
print('[' + ','.join(vals) + ']')
" 2>/dev/null || echo "[]")

    STDOUT_ESCAPED=$(echo "$CAPTURED_STDOUT" | json_escape)

    # correctness check
    if [[ -f "$EXPECTED" ]]; then
      EXPECTED_CONTENT=$(cat "$EXPECTED")
      if [[ "$CAPTURED_STDOUT" == "$EXPECTED_CONTENT" ]]; then
        CORRECTNESS="pass"
      else
        CORRECTNESS="fail"
      fi
    fi
  fi

  # ---- assemble per-benchmark JSON ------------------------------------------
  read -r -d '' ENTRY <<JSONEOF || true
{
  "name": "$bench",
  "source": "$SRC",
  "expected": "$EXPECTED",
  "description": "${BENCH_DESC[$bench]}",
  "tags": ${BENCH_TAGS[$bench]},
  "metrics": ["compile","runtime","size","memory"],
  "compile": {
    "status": "ok",
    "iterations": $COMPILE_ITERS,
    "samples_ms": $COMPILE_SAMPLES_JSON,
    "median_ms": $COMPILE_MEDIAN,
    "max_rss_kb": null,
    "binary_bytes": $BINARY_BYTES,
    "command": "$COMPILE_CMD"
  },
  "runtime": {
    "status": "$RUNTIME_STATUS",
    "iterations": $RUNTIME_ITERS_DONE,
    "warmups": $WARMUPS,
    "samples_ms": $RUNTIME_SAMPLES_JSON,
    "median_ms": $RUNTIME_MEDIAN,
    "max_rss_kb": null,
    "stdout": $STDOUT_ESCAPED,
    "correctness": "$CORRECTNESS",
    "command": "$RUN_CMD"
  }
}
JSONEOF

  BENCHMARKS_JSON="${BENCHMARKS_JSON}${SEP}${ENTRY}"
  SEP=","

  echo "  compile: ${COMPILE_MEDIAN}ms  binary: ${BINARY_BYTES}B  run: ${RUNTIME_MEDIAN}ms  correctness: ${CORRECTNESS}"
done

# --- assemble top-level JSON -------------------------------------------------
GENERATED_AT=$(now_iso)
PLATFORM=$(uname -s)
MACHINE=$(uname -m)
KERNEL=$(uname -r)

TARGET_SHORT="${TARGET##*-}"
RESULT_FILE="$RESULTS_DIR/bench-${MODE}-${TARGET_SHORT}-$(date -u +%Y%m%dT%H%M%SZ).json"

cat > "$RESULT_FILE" <<TOPJSON
{
  "schema_version": "$SCHEMA_VERSION",
  "generated_at": "$GENERATED_AT",
  "mode": "$MODE",
  "mode_description": "$MODE_DESC",
  "target": "$TARGET",
  "thresholds": {
    "compile_ms": 20,
    "run_ms": 10,
    "binary_bytes": 15
  },
  "compiler": {
    "path": "target/release/arukellt"
  },
  "environment": {
    "platform": "$PLATFORM",
    "machine": "$MACHINE",
    "kernel": "$KERNEL"
  },
  "tooling": {
    "wasmtime": { "name": "wasmtime", "available": $TOOLING_WASMTIME, "path": $TOOLING_WASMTIME_PATH },
    "hyperfine": { "name": "hyperfine", "available": $TOOLING_HYPERFINE, "path": $TOOLING_HYPERFINE_PATH }
  },
  "benchmarks": [
    $BENCHMARKS_JSON
  ]
}
TOPJSON

echo ""
echo "Results written to: $RESULT_FILE"
echo "Mode: $MODE ($MODE_DESC)"
