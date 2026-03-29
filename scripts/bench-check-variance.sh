#!/usr/bin/env bash
# bench-check-variance.sh — Run a benchmark fixture N times and report
# statistical variance.  Exits 0 (PASS) when the coefficient of variation
# is below the threshold, 1 (FAIL) otherwise.
#
# Usage:
#   scripts/bench-check-variance.sh <fixture.ark> [cv_threshold_pct]
#
# Environment variables:
#   BENCH_ITERATIONS  Number of iterations (default: 10)
#   ARUKELLT          Path to the compiler binary (default: target/release/arukellt)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# ── Arguments ────────────────────────────────────────────────────────────────
FIXTURE="${1:?Usage: bench-check-variance.sh <fixture.ark> [cv_threshold_pct]}"
CV_THRESHOLD="${2:-10}"
ITERATIONS="${BENCH_ITERATIONS:-10}"
COMPILER="${ARUKELLT:-$ROOT/target/release/arukellt}"

if [[ ! -f "$FIXTURE" ]]; then
    echo "ERROR: fixture not found: $FIXTURE" >&2
    exit 2
fi

if [[ ! -x "$COMPILER" ]]; then
    echo "ERROR: compiler not found or not executable: $COMPILER" >&2
    echo "       Build with:  cargo build --release" >&2
    exit 2
fi

# ── Helpers ──────────────────────────────────────────────────────────────────
# We collect wall-clock compile times (ms) as our measurement.
# Using bash built-in TIMEFORMAT to capture real time.

collect_sample() {
    local start end elapsed_ms
    start=$(date +%s%N)
    "$COMPILER" "$FIXTURE" > /dev/null 2>&1
    end=$(date +%s%N)
    elapsed_ms=$(( (end - start) / 1000000 ))
    echo "$elapsed_ms"
}

# ── Collect samples ─────────────────────────────────────────────────────────
echo "=== Benchmark Variance Check ==="
echo "Fixture:    $FIXTURE"
echo "Compiler:   $COMPILER"
echo "Iterations: $ITERATIONS"
echo "Threshold:  CV < ${CV_THRESHOLD}%"
echo ""
echo "Collecting samples..."

SAMPLES=()
for i in $(seq 1 "$ITERATIONS"); do
    ms=$(collect_sample)
    SAMPLES+=("$ms")
    printf "  [%2d/%d] %s ms\n" "$i" "$ITERATIONS" "$ms"
done

# ── Compute statistics (pure bash + awk) ─────────────────────────────────────
STATS=$(printf '%s\n' "${SAMPLES[@]}" | awk '
BEGIN { n = 0; sum = 0 }
{
    v[n] = $1 + 0
    sum += v[n]
    n++
}
END {
    if (n == 0) { print "ERROR: no samples"; exit 1 }

    # Sort for min/max/median
    for (i = 0; i < n; i++)
        for (j = i + 1; j < n; j++)
            if (v[i] > v[j]) { t = v[i]; v[i] = v[j]; v[j] = t }

    min_v    = v[0]
    max_v    = v[n - 1]
    mean     = sum / n

    if (n % 2 == 1)
        median = v[int(n / 2)]
    else
        median = (v[n / 2 - 1] + v[n / 2]) / 2

    # Standard deviation (population)
    ss = 0
    for (i = 0; i < n; i++)
        ss += (v[i] - mean) ^ 2
    stddev = sqrt(ss / n)

    cv = (mean > 0) ? (stddev / mean) * 100 : 0

    printf "min=%s max=%s median=%s mean=%.2f stddev=%.2f cv=%.2f n=%d\n", \
           min_v, max_v, median, mean, stddev, cv, n
}
')

# Parse stats into variables
MIN=$(echo "$STATS" | sed 's/.*min=\([^ ]*\).*/\1/')
MAX=$(echo "$STATS" | sed 's/.*max=\([^ ]*\).*/\1/')
MEDIAN=$(echo "$STATS" | sed 's/.*median=\([^ ]*\).*/\1/')
MEAN=$(echo "$STATS" | sed 's/.*mean=\([^ ]*\).*/\1/')
STDDEV=$(echo "$STATS" | sed 's/.*stddev=\([^ ]*\).*/\1/')
CV=$(echo "$STATS" | sed 's/.*cv=\([^ ]*\).*/\1/')
N=$(echo "$STATS" | sed 's/.*n=\([^ ]*\).*/\1/')

echo ""
echo "── Results ($N samples) ──────────────────"
printf "  Min:     %s ms\n" "$MIN"
printf "  Max:     %s ms\n" "$MAX"
printf "  Median:  %s ms\n" "$MEDIAN"
printf "  Mean:    %s ms\n" "$MEAN"
printf "  Stddev:  %s ms\n" "$STDDEV"
printf "  CV:      %s%%\n" "$CV"
echo ""

# ── Verdict ──────────────────────────────────────────────────────────────────
PASS=$(awk "BEGIN { print ($CV < $CV_THRESHOLD) ? 1 : 0 }")

if [[ "$PASS" -eq 1 ]]; then
    echo "✅ PASS — CV ${CV}% is below threshold ${CV_THRESHOLD}%"
    exit 0
else
    echo "❌ FAIL — CV ${CV}% exceeds threshold ${CV_THRESHOLD}%"
    exit 1
fi
