#!/usr/bin/env bash
# Compile-latency benchmark with per-phase breakdown.
#
# Measures cold (no prior output) and warm (re-compile with output present)
# compilation times across fixtures of varying complexity.
#
# Usage:
#   scripts/bench-compile-latency.sh [--quick] [--iterations N] [--json] [--md]
#
# Options:
#   --quick        Run only the representative fixture (fib.ark)
#   --iterations N Number of iterations per fixture (default: 5)
#   --json         Print JSON results to stdout (default)
#   --md           Print Markdown table to stdout
#   --output FILE  Write JSON results to FILE
#
# Requires: target/release/arukellt (build with `cargo build --release`)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPILER="$ROOT/target/release/arukellt"
SCRATCH_DIR="$ROOT/.bench-scratch"

# Defaults
ITERATIONS=5
QUICK=false
FORMAT="json"
OUTPUT_FILE=""

# --- CLI parsing -----------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --quick)      QUICK=true; shift ;;
        --iterations) ITERATIONS="$2"; shift 2 ;;
        --json)       FORMAT="json"; shift ;;
        --md)         FORMAT="md"; shift ;;
        --output)     OUTPUT_FILE="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,/^$/{ s/^# //; s/^#//; p }' "$0"
            exit 0
            ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# --- Fixture list -----------------------------------------------------------
# Small / medium / large fixtures that compile successfully.
ALL_FIXTURES=(
    "tests/fixtures/hello/hello.ark"
    "benchmarks/fib.ark"
    "benchmarks/binary_tree.ark"
    "benchmarks/vec_ops.ark"
)
QUICK_FIXTURES=(
    "benchmarks/fib.ark"
)

if $QUICK; then
    FIXTURES=("${QUICK_FIXTURES[@]}")
else
    FIXTURES=("${ALL_FIXTURES[@]}")
fi

# --- Sanity checks ----------------------------------------------------------
if [[ ! -x "$COMPILER" ]]; then
    echo "error: compiler not found at $COMPILER" >&2
    echo "       run: cargo build --release" >&2
    exit 1
fi

for f in "${FIXTURES[@]}"; do
    if [[ ! -f "$ROOT/$f" ]]; then
        echo "error: fixture not found: $f" >&2
        exit 1
    fi
done

# --- Helpers ----------------------------------------------------------------

# Parse phase timing lines from compiler --time output.
# Input: raw stderr/stdout from `arukellt compile --time`
# Output: JSON object with phase timings in ms
parse_phases() {
    awk '
    /^\[arukellt\]/ {
        # e.g. [arukellt] lex:           0.0ms
        phase = $2
        sub(/:$/, "", phase)
        val = $3
        sub(/ms$/, "", val)
        if (phase == "total") {
            total = val + 0
        } else {
            phases[phase] = val + 0
            order[++n] = phase
        }
    }
    END {
        printf "{"
        printf "\"total_ms\":%.2f", total
        for (i = 1; i <= n; i++) {
            printf ",\"%s_ms\":%.2f", order[i], phases[order[i]]
        }
        printf "}"
    }
    '
}

# Run a single compile and return JSON phase breakdown.
# Args: $1=source $2=output_wasm
run_once() {
    local src="$1" out="$2"
    "$COMPILER" compile --time "$src" -o "$out" 2>&1 | parse_phases
}

# Compute median of an array of floats (one per line).
median() {
    sort -g | awk '{a[NR]=$1} END {
        if (NR%2==1) print a[(NR+1)/2]
        else printf "%.2f\n", (a[NR/2]+a[NR/2+1])/2
    }'
}

# Extract total_ms from a phase-JSON string.
extract_total() {
    echo "$1" | sed 's/.*"total_ms":\([0-9.]*\).*/\1/'
}

# Merge multiple phase-JSON results by taking the median of each field.
# Input: newline-separated JSON strings
# Output: single JSON with median values
median_phases() {
    local lines=()
    while IFS= read -r line; do
        lines+=("$line")
    done

    # Collect all keys from the first entry.
    local keys
    keys=$(echo "${lines[0]}" | sed 's/[{}]//g' | tr ',' '\n' | sed 's/:.*//' | sed 's/"//g')

    local first=true
    printf "{"
    for key in $keys; do
        local values=""
        for json in "${lines[@]}"; do
            local val
            val=$(echo "$json" | sed 's/.*"'"$key"'":\([0-9.]*\).*/\1/')
            values+="$val"$'\n'
        done
        local med
        med=$(echo "$values" | grep -v '^$' | median)
        if $first; then
            first=false
        else
            printf ","
        fi
        printf '"%s":%.2f' "$key" "$med"
    done
    printf "}"
}

# --- Main benchmark loop ----------------------------------------------------

mkdir -p "$SCRATCH_DIR"
trap 'rm -rf "$SCRATCH_DIR"' EXIT

RESULTS=()

for fixture in "${FIXTURES[@]}"; do
    src="$ROOT/$fixture"
    name=$(basename "$fixture" .ark)

    echo "benchmarking: $fixture ($ITERATIONS iterations)" >&2

    # --- Cold runs (remove output before each compile) ---
    cold_phases=()
    for (( i = 1; i <= ITERATIONS; i++ )); do
        out="$SCRATCH_DIR/${name}_cold.wasm"
        rm -f "$out"
        result=$(run_once "$src" "$out")
        cold_phases+=("$result")
    done

    # --- Warm runs (output already present, re-compile over it) ---
    # Seed with one compile so output file exists.
    warm_out="$SCRATCH_DIR/${name}_warm.wasm"
    run_once "$src" "$warm_out" > /dev/null
    warm_phases=()
    for (( i = 1; i <= ITERATIONS; i++ )); do
        result=$(run_once "$src" "$warm_out")
        warm_phases+=("$result")
    done

    # Compute medians.
    cold_median=$(printf '%s\n' "${cold_phases[@]}" | median_phases)
    warm_median=$(printf '%s\n' "${warm_phases[@]}" | median_phases)

    cold_ms=$(extract_total "$cold_median")
    warm_ms=$(extract_total "$warm_median")

    # Self-check: phase sum should be close to total.
    phase_sum=$(echo "$cold_median" | sed 's/[{}]//g' | tr ',' '\n' \
        | grep -v total_ms | sed 's/"[^"]*"://g' | awk '{s+=$1} END {printf "%.2f",s}')
    drift=$(awk "BEGIN { d=($cold_ms - $phase_sum); if(d<0) d=-d; printf \"%.2f\",d }")
    if awk "BEGIN { exit !($drift > $cold_ms * 0.20) }" 2>/dev/null; then
        echo "  warning: phase sum ($phase_sum ms) differs from total ($cold_ms ms) by ${drift}ms" >&2
    fi

    entry=$(printf '{"fixture":"%s","cold_ms":%.2f,"warm_ms":%.2f,"cold_phase_breakdown":%s,"warm_phase_breakdown":%s}' \
        "$fixture" "$cold_ms" "$warm_ms" "$cold_median" "$warm_median")
    RESULTS+=("$entry")

    echo "  cold: ${cold_ms}ms  warm: ${warm_ms}ms" >&2
done

# --- Output -----------------------------------------------------------------

json_output() {
    local first=true
    printf '{"iterations":%d,"results":[' "$ITERATIONS"
    for r in "${RESULTS[@]}"; do
        if $first; then first=false; else printf ','; fi
        printf '%s' "$r"
    done
    printf ']}\n'
}

md_output() {
    echo "# Compile Latency Benchmark"
    echo ""
    echo "Iterations per fixture: $ITERATIONS"
    echo ""
    echo "| Fixture | Cold (ms) | Warm (ms) | lex | parse | resolve | typecheck | lower | opt | emit |"
    echo "|---------|-----------|-----------|-----|-------|---------|-----------|-------|-----|------|"
    for r in "${RESULTS[@]}"; do
        local fix cold warm lex parse resolve tc lower opt emit
        fix=$(echo "$r" | sed 's/.*"fixture":"\([^"]*\)".*/\1/')
        cold=$(echo "$r" | sed 's/.*"cold_ms":\([0-9.]*\).*/\1/')
        warm=$(echo "$r" | sed 's/.*"warm_ms":\([0-9.]*\).*/\1/')
        # Extract from cold_phase_breakdown
        lex=$(echo "$r"   | sed 's/.*"cold_phase_breakdown":{[^}]*"lex_ms":\([0-9.]*\).*/\1/')
        parse=$(echo "$r" | sed 's/.*"cold_phase_breakdown":{[^}]*"parse_ms":\([0-9.]*\).*/\1/')
        resolve=$(echo "$r" | sed 's/.*"cold_phase_breakdown":{[^}]*"resolve_ms":\([0-9.]*\).*/\1/')
        tc=$(echo "$r"   | sed 's/.*"cold_phase_breakdown":{[^}]*"typecheck_ms":\([0-9.]*\).*/\1/')
        lower=$(echo "$r" | sed 's/.*"cold_phase_breakdown":{[^}]*"lower_ms":\([0-9.]*\).*/\1/')
        opt=$(echo "$r"   | sed 's/.*"cold_phase_breakdown":{[^}]*"opt_ms":\([0-9.]*\).*/\1/')
        emit=$(echo "$r"  | sed 's/.*"cold_phase_breakdown":{[^}]*"emit_ms":\([0-9.]*\).*/\1/')
        printf "| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n" \
            "$fix" "$cold" "$warm" "$lex" "$parse" "$resolve" "$tc" "$lower" "$opt" "$emit"
    done
}

case "$FORMAT" in
    json)
        if [[ -n "$OUTPUT_FILE" ]]; then
            json_output > "$OUTPUT_FILE"
            echo "results written to $OUTPUT_FILE" >&2
        else
            json_output
        fi
        ;;
    md)
        if [[ -n "$OUTPUT_FILE" ]]; then
            md_output > "$OUTPUT_FILE"
            echo "results written to $OUTPUT_FILE" >&2
        else
            md_output
        fi
        ;;
esac
