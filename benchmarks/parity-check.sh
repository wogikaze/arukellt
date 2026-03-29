#!/usr/bin/env bash
# Parity check: verify T1 and T3 produce identical output.
set -euo pipefail

ARUKELLT="${ARUKELLT:-target/release/arukellt}"
PASS=0
FAIL=0

for f in benchmarks/*.ark; do
    base=$(basename "$f" .ark)
    expected="benchmarks/$base.expected"

    if [ ! -f "$expected" ]; then
        echo "SKIP: $base (no .expected)"
        continue
    fi

    if [ "$base" = "bench_parse_tree_distance" ]; then
        echo "SKIP: $base (large file-backed benchmark; excluded from parity check)"
        continue
    fi

    t1_out=$("$ARUKELLT" run --target wasm32-wasi-p1 "$f" 2>/dev/null) || t1_out="[T1 ERROR]"
    t3_out=$("$ARUKELLT" run --target wasm32-wasi-p2 "$f" 2>/dev/null) || t3_out="[T3 ERROR]"
    exp=$(cat "$expected")

    if [ "$t1_out" = "$exp" ] && [ "$t3_out" = "$exp" ]; then
        echo "PASS: $base (T1=T3=expected)"
        PASS=$((PASS + 1))
    else
        echo "FAIL: $base"
        [ "$t1_out" != "$exp" ] && echo "  T1 mismatch"
        [ "$t3_out" != "$exp" ] && echo "  T3 mismatch"
        [ "$t1_out" != "$t3_out" ] && echo "  T1 != T3 (parity violation)"
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "Parity: PASS=$PASS FAIL=$FAIL"
[ "$FAIL" -eq 0 ] || exit 1
