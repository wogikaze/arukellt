#!/usr/bin/env bash
# Runtime latency/throughput benchmark runner for Arukellt.
# Compiles .ark benchmarks to .wasm, runs each with wasmtime,
# and reports startup vs execution latency with min/median/max.
#
# Usage: scripts/bench-runtime.sh [--iterations N] [--quick] [--json-only]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPILER="$ROOT/target/release/arukellt"
BENCH_DIR="$ROOT/benchmarks"
ITERATIONS=5
JSON_ONLY=false
WORK_DIR="$ROOT/.bench-runtime-tmp"

cleanup() { rm -rf "$WORK_DIR"; }
trap cleanup EXIT
mkdir -p "$WORK_DIR"

for arg in "$@"; do
    case "$arg" in
        --quick)
            ITERATIONS=3
            ;;
        --iterations=*)
            ITERATIONS="${arg#*=}"
            ;;
        --json-only)
            JSON_ONLY=true
            ;;
        --help|-h)
            echo "Usage: $0 [--iterations=N] [--quick] [--json-only]"
            exit 0
            ;;
    esac
done

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

# Benchmark .ark files: startup first, then workloads.
STARTUP_ARK="$BENCH_DIR/startup.ark"
WORKLOAD_ARKS=()
for f in "$BENCH_DIR"/*.ark; do
    [[ "$(basename "$f")" == "startup.ark" ]] && continue
    WORKLOAD_ARKS+=("$f")
done

# --- helpers ---------------------------------------------------------------

measure_ms() {
    # Run a command N times, collect wall-clock times in milliseconds.
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
    # Given space-separated ms values, write JSON stats to a file.
    local values="$1"
    local outfile="$2"
    python3 -c "
import statistics, json, sys
vals = sorted(float(x) for x in sys.argv[1].split())
n = len(vals)
json.dump({
    'min_ms': round(vals[0], 3),
    'median_ms': round(statistics.median(vals), 3),
    'max_ms': round(vals[-1], 3),
    'samples': n,
    'stdev_ms': round(statistics.stdev(vals), 3) if n > 1 else 0.0,
}, open(sys.argv[2], 'w'))
" "$values" "$outfile"
}

get_field() {
    python3 -c "import json,sys; print(json.load(open(sys.argv[1]))[sys.argv[2]])" "$1" "$2"
}

compile_ark() {
    local ark="$1" wasm="$2"
    "$COMPILER" compile "$ark" -o "$wasm" --opt-level 2 >/dev/null 2>&1
}

log() {
    if [[ "$JSON_ONLY" != "true" ]]; then
        echo "$@" >&2
    fi
}

# --- main ------------------------------------------------------------------

log "=== Arukellt Runtime Benchmark ==="
log "Iterations: $ITERATIONS"
log ""

# 1. Compile all benchmarks
log "Compiling benchmarks..."
WASM_FILES=()
ALL_ARKS=("$STARTUP_ARK" "${WORKLOAD_ARKS[@]}")
for ark in "${ALL_ARKS[@]}"; do
    name="$(basename "$ark" .ark)"
    wasm="$WORK_DIR/${name}.wasm"
    if ! compile_ark "$ark" "$wasm"; then
        log "  FAIL: $name (compile error)"
        continue
    fi
    WASM_FILES+=("$name:$wasm")
    log "  $name -> $(basename "$wasm")"
done

# 2. Measure startup latency (empty main)
log ""
log "Measuring startup latency (empty main)..."
STARTUP_WASM=""
for entry in "${WASM_FILES[@]}"; do
    if [[ "$entry" == startup:* ]]; then
        STARTUP_WASM="${entry#startup:}"
        break
    fi
done

STARTUP_STATS_FILE="$WORK_DIR/startup_stats.json"
STARTUP_MEDIAN="0"
if [[ -n "$STARTUP_WASM" ]]; then
    STARTUP_TIMES=$(measure_ms "wasmtime run --dir=. '$STARTUP_WASM'" "$ITERATIONS")
    compute_stats "$STARTUP_TIMES" "$STARTUP_STATS_FILE"
    STARTUP_MEDIAN=$(get_field "$STARTUP_STATS_FILE" "median_ms")
    log "  startup median: ${STARTUP_MEDIAN}ms"
else
    echo '{}' > "$STARTUP_STATS_FILE"
    log "  WARNING: no startup.ark found, startup=0"
fi

# 3. Measure each workload and write per-benchmark JSON
log ""
log "Running workloads..."
BENCH_INDEX=0

for entry in "${WASM_FILES[@]}"; do
    name="${entry%%:*}"
    wasm="${entry#*:}"
    [[ "$name" == "startup" ]] && continue

    log "  $name ($ITERATIONS iterations)..."
    exec_times=$(measure_ms "wasmtime run --dir=. '$wasm'" "$ITERATIONS")
    stats_file="$WORK_DIR/stats_${BENCH_INDEX}.json"
    compute_stats "$exec_times" "$stats_file"

    exec_median=$(get_field "$stats_file" "median_ms")
    adjusted_ms=$(python3 -c "print(round(max(0, $exec_median - $STARTUP_MEDIAN), 3))")
    throughput=$(python3 -c "m=$exec_median; print(round(1000.0/m, 2) if m > 0 else 0)")

    # Write individual benchmark result
    python3 -c "
import json, sys
stats = json.load(open(sys.argv[1]))
json.dump({
    'benchmark': sys.argv[2],
    'startup_ms': float(sys.argv[3]),
    'execution_ms': float(sys.argv[4]),
    'total_ms': stats,
    'throughput_ops_per_sec': float(sys.argv[5]),
}, open(sys.argv[6], 'w'), indent=2)
" "$stats_file" "$name" "$STARTUP_MEDIAN" "$adjusted_ms" "$throughput" \
  "$WORK_DIR/result_${BENCH_INDEX}.json"

    log "    median=${exec_median}ms  adjusted=${adjusted_ms}ms  throughput=${throughput} ops/s"
    BENCH_INDEX=$((BENCH_INDEX + 1))
done

# 4. Assemble final JSON
python3 -c "
import json, glob, platform, sys
from datetime import datetime, timezone
from pathlib import Path

work = Path(sys.argv[1])
results = []
for f in sorted(work.glob('result_*.json')):
    results.append(json.load(open(f)))

startup_file = work / 'startup_stats.json'
startup = json.load(open(startup_file)) if startup_file.exists() else {}

out = {
    'schema': 'arukellt-runtime-bench-v1',
    'timestamp': datetime.now(timezone.utc).isoformat(),
    'platform': platform.platform(),
    'iterations': int(sys.argv[2]),
    'startup': startup,
    'benchmarks': results,
}
print(json.dumps(out, indent=2))
" "$WORK_DIR" "$ITERATIONS"

log ""
log "Done."
