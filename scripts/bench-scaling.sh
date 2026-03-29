#!/usr/bin/env bash
# Input-size sweep and scaling-curve benchmark for Arukellt.
#
# Runs a single benchmark at multiple input sizes, records per-size
# runtime timings, and reports a JSON table suitable for plotting
# scaling curves and detecting algorithmic cliffs.
#
# Usage:
#   scripts/bench-scaling.sh <benchmark> [--quick] [--iterations N] [--json-only]
#
# Benchmarks and their size parameters:
#   fib            Fibonacci sequence length (N in fib.ark line: fib(35))
#   binary_tree    Tree recursion depth      (depth in binary_tree.ark line: let depth: i32 = 20)
#   vec_ops        Vector element count       (upper bound in vec_ops.ark line: while i < 1000)
#   string_concat  Concatenation iterations   (upper bound in string_concat.ark line: while i < 100)
#
# The benchmark .ark files use hardcoded constants. This script compiles
# temporary variants with the constant replaced, times each, and emits
# a JSON scaling table.
#
# Examples:
#   scripts/bench-scaling.sh fib
#   scripts/bench-scaling.sh binary_tree --quick
#   scripts/bench-scaling.sh vec_ops --iterations 3
#
# Requires: target/release/arukellt, wasmtime, python3

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPILER="$ROOT/target/release/arukellt"
BENCH_DIR="$ROOT/benchmarks"
ITERATIONS=5
JSON_ONLY=false
MODE="full"
WORK_DIR="$ROOT/.bench-scaling-tmp"

cleanup() { rm -rf "$WORK_DIR"; }
trap cleanup EXIT
mkdir -p "$WORK_DIR"

# --- CLI parsing -----------------------------------------------------------

BENCHMARK=""
for arg in "$@"; do
    case "$arg" in
        --quick)
            MODE="quick"
            ITERATIONS=3
            ;;
        --iterations=*)
            ITERATIONS="${arg#*=}"
            ;;
        --json-only)
            JSON_ONLY=true
            ;;
        --help|-h)
            sed -n '2,/^$/{ s/^# //; s/^#//; p }' "$0"
            exit 0
            ;;
        -*)
            echo "Unknown option: $arg" >&2
            exit 1
            ;;
        *)
            if [[ -z "$BENCHMARK" ]]; then
                BENCHMARK="$arg"
            else
                echo "error: unexpected argument: $arg" >&2
                exit 1
            fi
            ;;
    esac
done

if [[ -z "$BENCHMARK" ]]; then
    echo "Usage: $0 <benchmark> [--quick] [--iterations=N] [--json-only]" >&2
    echo "Benchmarks: fib, binary_tree, vec_ops, string_concat" >&2
    exit 1
fi

# --- Prerequisite checks ---------------------------------------------------

if [[ ! -x "$COMPILER" ]]; then
    echo "ERROR: compiler not found at $COMPILER" >&2
    echo "       Run 'cargo build --release' first." >&2
    exit 1
fi

if ! command -v wasmtime &>/dev/null; then
    echo "ERROR: wasmtime not found in PATH" >&2
    exit 1
fi

if ! command -v python3 &>/dev/null; then
    echo "ERROR: python3 not found in PATH" >&2
    exit 1
fi

# --- Size configurations per benchmark -------------------------------------
# Each benchmark defines:
#   SRC_FILE   - source .ark file
#   SIZES      - array of input sizes to sweep
#   sed_cmd()  - function that rewrites the source for a given size

case "$BENCHMARK" in
    fib)
        SRC_FILE="$BENCH_DIR/fib.ark"
        if [[ "$MODE" == "quick" ]]; then
            SIZES=(10 20 30)
        else
            SIZES=(10 20 25 30 35)
        fi
        # Replace: fib(35) -> fib(N)
        sed_cmd() { sed "s/fib(35)/fib($1)/" "$SRC_FILE"; }
        SIZE_LABEL="recursion_depth"
        ;;
    binary_tree)
        SRC_FILE="$BENCH_DIR/binary_tree.ark"
        if [[ "$MODE" == "quick" ]]; then
            SIZES=(5 10 15)
        else
            SIZES=(5 10 15 18 20)
        fi
        # Replace: let depth: i32 = 20 -> let depth: i32 = N
        sed_cmd() { sed "s/let depth: i32 = 20/let depth: i32 = $1/" "$SRC_FILE"; }
        SIZE_LABEL="tree_depth"
        ;;
    vec_ops)
        SRC_FILE="$BENCH_DIR/vec_ops.ark"
        if [[ "$MODE" == "quick" ]]; then
            SIZES=(100 1000 10000)
        else
            SIZES=(100 500 1000 5000 10000)
        fi
        # Replace: while i < 1000 -> while i < N  (the push loop)
        sed_cmd() { sed "s/while i < 1000/while i < $1/" "$SRC_FILE"; }
        SIZE_LABEL="vector_size"
        ;;
    string_concat)
        SRC_FILE="$BENCH_DIR/string_concat.ark"
        if [[ "$MODE" == "quick" ]]; then
            SIZES=(50 100 500)
        else
            SIZES=(50 100 200 500 1000)
        fi
        # Replace: while i < 100 -> while i < N
        sed_cmd() { sed "s/while i < 100/while i < $1/" "$SRC_FILE"; }
        SIZE_LABEL="iteration_count"
        ;;
    *)
        echo "error: unknown benchmark '$BENCHMARK'" >&2
        echo "Supported: fib, binary_tree, vec_ops, string_concat" >&2
        exit 1
        ;;
esac

if [[ ! -f "$SRC_FILE" ]]; then
    echo "error: source file not found: $SRC_FILE" >&2
    exit 1
fi

# --- Helpers ----------------------------------------------------------------

log() {
    if [[ "$JSON_ONLY" != "true" ]]; then
        echo "$@" >&2
    fi
}

measure_ms() {
    local cmd="$1"
    local n="$2"
    local times=()
    for ((i = 0; i < n; i++)); do
        local start end elapsed
        start=$(python3 -c 'import time; print(time.perf_counter_ns())')
        eval "$cmd" >/dev/null 2>&1
        end=$(python3 -c 'import time; print(time.perf_counter_ns())')
        elapsed=$(python3 -c "print(round(($end - $start) / 1_000_000, 3))")
        times+=("$elapsed")
    done
    echo "${times[*]}"
}

compute_stats() {
    local values="$1"
    python3 -c "
import statistics, json, sys
vals = sorted(float(x) for x in sys.argv[1].split())
n = len(vals)
print(json.dumps({
    'min_ms': round(vals[0], 3),
    'median_ms': round(statistics.median(vals), 3),
    'max_ms': round(vals[-1], 3),
    'samples': n,
    'stdev_ms': round(statistics.stdev(vals), 3) if n > 1 else 0.0,
}))
" "$values"
}

# --- Main sweep -------------------------------------------------------------

log "=== Arukellt Scaling Benchmark ==="
log "Benchmark:  $BENCHMARK"
log "Sizes:      ${SIZES[*]}"
log "Iterations: $ITERATIONS"
log ""

RESULTS=()
PREV_MEDIAN=""

for size in "${SIZES[@]}"; do
    log "--- size=$size ---"

    # Generate variant source
    variant_src="$WORK_DIR/${BENCHMARK}_n${size}.ark"
    sed_cmd "$size" > "$variant_src"

    # Compile
    variant_wasm="$WORK_DIR/${BENCHMARK}_n${size}.wasm"
    if ! "$COMPILER" compile "$variant_src" -o "$variant_wasm" --opt-level 2 >/dev/null 2>&1; then
        log "  SKIP: compile failed for size=$size"
        RESULTS+=("{\"size\":$size,\"status\":\"compile_error\"}")
        continue
    fi

    # Time runtime execution
    exec_times=$(measure_ms "wasmtime run --dir=. '$variant_wasm'" "$ITERATIONS")
    stats_json=$(compute_stats "$exec_times")

    # Extract median for ratio computation
    current_median=$(python3 -c "import json,sys; print(json.loads(sys.argv[1])['median_ms'])" "$stats_json")

    # Compute growth ratio from previous size point
    ratio="null"
    if [[ -n "$PREV_MEDIAN" ]]; then
        ratio=$(python3 -c "
prev, curr = float('$PREV_MEDIAN'), float('$current_median')
print(round(curr / prev, 3) if prev > 0 else 'null')
")
    fi

    # Cliff warning: ratio > 10x signals a potential algorithmic cliff
    cliff_warning="false"
    if [[ "$ratio" != "null" ]]; then
        cliff_warning=$(python3 -c "print('true' if float('$ratio') > 10.0 else 'false')")
    fi

    entry=$(python3 -c "
import json, sys
stats = json.loads(sys.argv[1])
entry = {
    'size': int(sys.argv[2]),
    'status': 'ok',
    'timing': stats,
    'ratio_vs_prev': float(sys.argv[3]) if sys.argv[3] != 'null' else None,
    'cliff_warning': sys.argv[4] == 'true',
}
print(json.dumps(entry))
" "$stats_json" "$size" "$ratio" "$cliff_warning")

    RESULTS+=("$entry")
    PREV_MEDIAN="$current_median"

    log "  median=${current_median}ms  ratio=${ratio}  cliff=${cliff_warning}"
done

# --- Assemble final JSON output ---------------------------------------------

python3 -c "
import json, sys, platform
from datetime import datetime, timezone

results = [json.loads(r) for r in sys.argv[3:]]

out = {
    'schema': 'arukellt-scaling-bench-v1',
    'timestamp': datetime.now(timezone.utc).isoformat(),
    'platform': platform.platform(),
    'benchmark': sys.argv[1],
    'size_label': sys.argv[2],
    'iterations': int('$ITERATIONS'),
    'mode': '$MODE',
    'data_points': results,
}
print(json.dumps(out, indent=2))
" "$BENCHMARK" "$SIZE_LABEL" "${RESULTS[@]}"

log ""
log "Done."
