#!/usr/bin/env bash
# Compare Wasm output sizes between T1 and T3.
set -euo pipefail

ARUKELLT="${ARUKELLT:-target/release/arukellt}"
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

echo "File size comparison (T1 vs T3):"
echo "================================"
printf "%-20s %10s %10s %10s\n" "Fixture" "T1 (bytes)" "T3 (bytes)" "Diff"

for f in benchmarks/*.ark; do
    base=$(basename "$f" .ark)
    t1_out="$TMPDIR/${base}.t1.wasm"
    t3_out="$TMPDIR/${base}.t3.wasm"

    "$ARUKELLT" compile --target wasm32-wasi-p1 -o "$t1_out" "$f" 2>/dev/null || continue
    "$ARUKELLT" compile --target wasm32-wasi-p2 -o "$t3_out" "$f" 2>/dev/null || continue

    t1_size=$(stat -c %s "$t1_out" 2>/dev/null || stat -f %z "$t1_out")
    t3_size=$(stat -c %s "$t3_out" 2>/dev/null || stat -f %z "$t3_out")
    diff=$((t3_size - t1_size))

    printf "%-20s %10d %10d %+10d\n" "$base" "$t1_size" "$t3_size" "$diff"
done
