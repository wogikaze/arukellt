#!/usr/bin/env bash
# Benchmark runner for Arukellt benchmark suite.
# Usage: bash benchmarks/run_benchmarks.sh [--quick]
#   --quick  skip hyperfine, use single 'time' run only
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ARUKELLT="${ARUKELLT:-$REPO_ROOT/target/release/arukellt}"
BENCH_DIR="$REPO_ROOT/benchmarks"
TARGET="wasm32-wasi-p1"
QUICK=false

for arg in "$@"; do
    case "$arg" in
        --quick) QUICK=true ;;
    esac
done

# ── prerequisites ──────────────────────────────────────────────
if [ ! -x "$ARUKELLT" ]; then
    echo "arukellt binary not found at $ARUKELLT"
    echo "Build with: cargo build --release -p arukellt"
    exit 1
fi

HAS_WASMTIME=false
if command -v wasmtime &>/dev/null; then
    HAS_WASMTIME=true
fi

HAS_HYPERFINE=false
if command -v hyperfine &>/dev/null && [ "$QUICK" = false ]; then
    HAS_HYPERFINE=true
fi

BENCHMARKS=(fib binary_tree vec_ops string_concat)

echo "═══════════════════════════════════════════"
echo " Arukellt Benchmark Suite"
echo "═══════════════════════════════════════════"
echo "Compiler : $ARUKELLT"
echo "Target   : $TARGET"
echo "Wasmtime : $HAS_WASMTIME"
echo "Hyperfine: $HAS_HYPERFINE"
echo ""

# ── compile ────────────────────────────────────────────────────
echo "── Compiling ──────────────────────────────"
PASS=0
FAIL=0

for bench in "${BENCHMARKS[@]}"; do
    src="$BENCH_DIR/$bench.ark"
    wasm="$BENCH_DIR/$bench.wasm"

    if [ ! -f "$src" ]; then
        echo "SKIP: $bench ($src not found)"
        continue
    fi

    if "$ARUKELLT" compile --target "$TARGET" -o "$wasm" "$src" 2>/dev/null; then
        size=$(stat -c %s "$wasm" 2>/dev/null || stat -f %z "$wasm")
        printf "  %-20s OK  (%d bytes)\n" "$bench" "$size"
        PASS=$((PASS + 1))
    else
        printf "  %-20s FAIL\n" "$bench"
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "Compiled: $PASS passed, $FAIL failed"

if [ "$FAIL" -gt 0 ] && [ "$PASS" -eq 0 ]; then
    echo "No benchmarks compiled. Aborting."
    exit 1
fi

# ── correctness check ─────────────────────────────────────────
if [ "$HAS_WASMTIME" = true ]; then
    echo ""
    echo "── Correctness Check ──────────────────────"
    for bench in "${BENCHMARKS[@]}"; do
        wasm="$BENCH_DIR/$bench.wasm"
        expected="$BENCH_DIR/$bench.expected"

        if [ ! -f "$wasm" ]; then continue; fi

        actual=$(wasmtime "$wasm" 2>/dev/null) || actual="[ERROR]"

        if [ -f "$expected" ]; then
            exp=$(cat "$expected")
            if [ "$actual" = "$exp" ]; then
                printf "  %-20s PASS\n" "$bench"
            else
                printf "  %-20s FAIL (output mismatch)\n" "$bench"
            fi
        else
            printf "  %-20s OK (no .expected file)\n" "$bench"
        fi
    done
fi

# ── binary sizes ───────────────────────────────────────────────
echo ""
echo "── Binary Sizes ───────────────────────────"
printf "  %-20s %10s\n" "Benchmark" "Size (bytes)"
printf "  %-20s %10s\n" "─────────" "────────────"
for bench in "${BENCHMARKS[@]}"; do
    wasm="$BENCH_DIR/$bench.wasm"
    if [ -f "$wasm" ]; then
        size=$(stat -c %s "$wasm" 2>/dev/null || stat -f %z "$wasm")
        printf "  %-20s %10d\n" "$bench" "$size"
    fi
done

# ── timing ─────────────────────────────────────────────────────
if [ "$HAS_WASMTIME" = true ]; then
    echo ""
    echo "── Timing ─────────────────────────────────"

    if [ "$HAS_HYPERFINE" = true ]; then
        cmds=()
        names=()
        for bench in "${BENCHMARKS[@]}"; do
            wasm="$BENCH_DIR/$bench.wasm"
            if [ -f "$wasm" ]; then
                cmds+=("wasmtime $wasm")
                names+=("$bench")
            fi
        done

        if [ ${#cmds[@]} -gt 0 ]; then
            hyperfine --warmup 3 --min-runs 5 "${cmds[@]}"
        fi
    else
        for bench in "${BENCHMARKS[@]}"; do
            wasm="$BENCH_DIR/$bench.wasm"
            if [ -f "$wasm" ]; then
                echo "  $bench:"
                { time wasmtime "$wasm" >/dev/null 2>&1 ; } 2>&1 | sed 's/^/    /'
                echo ""
            fi
        done
    fi
else
    echo ""
    echo "── Timing (compile-only, no wasmtime) ─────"
    for bench in "${BENCHMARKS[@]}"; do
        src="$BENCH_DIR/$bench.ark"
        wasm="$BENCH_DIR/$bench.wasm"
        if [ -f "$src" ]; then
            echo "  $bench (compile):"
            rm -f "$wasm"
            { time "$ARUKELLT" compile --target "$TARGET" -o "$wasm" "$src" 2>/dev/null ; } 2>&1 | sed 's/^/    /'
            echo ""
        fi
    done
fi

echo ""
echo "Done."
