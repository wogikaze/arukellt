#!/usr/bin/env bash
# Arukellt benchmark runner — standalone shell version.
# Builds the compiler (release), compiles + runs each benchmark .ark file,
# times both phases, and emits JSON matching benchmarks/schema.json.
#
# Usage:
#   bash scripts/run/run-benchmarks.sh                          # quick (1 iteration, default)
#   bash scripts/run/run-benchmarks.sh --quick                  # same as above
#   bash scripts/run/run-benchmarks.sh --full                   # 10 iterations per benchmark
#   bash scripts/run/run-benchmarks.sh --compare                # run both Rust & selfhost, show diff
#   bash scripts/run/run-benchmarks.sh --compare-lang c,rust    # time reference implementations
#   bash scripts/run/run-benchmarks.sh --compare-lang c,rust,go # time C, Rust, and Go refs
#   ARUKELLT_BIN=/path/to/arukellt bash scripts/run/run-benchmarks.sh  # custom compiler
#
# --compare-lang:
#   Looks for benchmarks/<name>.<ext> (.c, .rs, .go) for each benchmark.
#   Compiles those reference programs (cc, rustc, go build) and times them
#   with hyperfine (3 runs) if available, otherwise with the built-in timer.
#   Results are printed in a comparison table at the end.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
COMPILER="${ARUKELLT_BIN:-$ROOT/target/release/arukellt}"
SELFHOST_WASM="$ROOT/src/compiler/arukellt-s1.wasm"
RESULTS_DIR="$ROOT/benchmarks/results"
TARGET="wasm32-wasi-p1"
SCHEMA_VERSION="arukellt-bench-v1"

# --- mode parsing -----------------------------------------------------------
MODE="quick"
COMPILE_ITERS=1
RUNTIME_ITERS=1
WARMUPS=0
COMPARE=false
USE_SELFHOST=false
COMPARE_LANGS=""

# Use positional index loop so we can consume the next arg for space-separated
# options like --compare-lang c,rust
set -- "$@"
while [[ $# -gt 0 ]]; do
  arg="$1"
  case "$arg" in
    --quick) MODE="quick";  COMPILE_ITERS=1;  RUNTIME_ITERS=1;  WARMUPS=0 ;;
    --full)  MODE="full";   COMPILE_ITERS=10; RUNTIME_ITERS=10; WARMUPS=1 ;;
    --target=*) TARGET="${arg#--target=}" ;;
    --target) shift; TARGET="$1" ;;
    --compare) COMPARE=true ;;
    --compare-lang=*) COMPARE_LANGS="${arg#--compare-lang=}" ;;
    --compare-lang)
      # Support both --compare-lang=c,rust and --compare-lang c,rust
      if [[ $# -gt 1 && "${2}" != --* ]]; then
        shift; COMPARE_LANGS="$1"
      else
        COMPARE_LANGS="c,rust,go"  # bare flag defaults to all three
      fi
      ;;
    --selfhost) USE_SELFHOST=true ;;
    *) echo "Unknown flag: $arg" >&2; exit 1 ;;
  esac
  shift
done

MODE_DESC="quick"
[[ "$MODE" == "full" ]] && MODE_DESC="full local benchmark (10 iterations)"
[[ "$MODE" == "quick" ]] && MODE_DESC="single-sample local smoke benchmark"

# --- benchmark cases ---------------------------------------------------------
BENCH_NAMES=(fib binary_tree vec_ops string_concat vec_push_pop json_parse)
declare -A BENCH_DESC=(
  [fib]="Iterative Fibonacci(35)"
  [binary_tree]="Recursive node counting (depth 20)"
  [vec_ops]="Vec push/sum/contains (1k elements)"
  [string_concat]="String concat in loop (100 iterations)"
  [vec_push_pop]="Vec 100K push then 100K pop"
  [json_parse]="JSON token scan (~10KB string)"
)
declare -A BENCH_TAGS=(
  [fib]='["cpu-bound","loop","scalar"]'
  [binary_tree]='["recursion-heavy","allocation-light","call-heavy"]'
  [vec_ops]='["allocation-heavy","container","iteration"]'
  [string_concat]='["string-heavy","allocation-heavy","gc-pressure"]'
  [vec_push_pop]='["allocation-heavy","container","throughput"]'
  [json_parse]='["string-heavy","parse","allocation-heavy"]'
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
  # Read whitespace-separated numbers from stdin, print integer median.
  python3 -c "
import sys, statistics
vals = [float(x) for x in sys.stdin.read().split()]
print(int(statistics.median(vals)))
" 2>/dev/null || {
    # Fallback: sort + pick middle
    local nums=("$@")
    local sorted
    sorted=($(printf '%s\n' "${nums[@]}" | sort -n))
    echo "${sorted[$(( ${#sorted[@]} / 2 ))]}"
  }
}

json_escape() { python3 -c "import json,sys; print(json.dumps(sys.stdin.read().rstrip('\n')))" 2>/dev/null || echo '""'; }

# Threshold for performance warning (percentage)
PERF_WARN_THRESHOLD=200

# --- selfhost compile helper -------------------------------------------------
# compile_with_selfhost SRC WASM_OUT
# Returns: sets SELFHOST_COMPILE_MS and SELFHOST_BINARY_BYTES
selfhost_compile() {
  local src="$1" wasm_out="$2"
  SELFHOST_COMPILE_MS=0
  SELFHOST_BINARY_BYTES=0

  if [[ ! -f "$SELFHOST_WASM" ]]; then
    echo "  [selfhost] SKIP — $SELFHOST_WASM not found"
    return 1
  fi

  local wasmtime_bin
  wasmtime_bin="$(command -v wasmtime 2>/dev/null || true)"
  if [[ -z "$wasmtime_bin" ]]; then
    echo "  [selfhost] SKIP — wasmtime not found"
    return 1
  fi

  # Convert to paths relative to ROOT for wasmtime --dir=.
  local rel_src="${src#$ROOT/}"
  local rel_out="${wasm_out#$ROOT/}"
  local rel_wasm="${SELFHOST_WASM#$ROOT/}"

  rm -f "$wasm_out"
  local selfhost_samples=""
  for (( i=0; i<COMPILE_ITERS; i++ )); do
    rm -f "$wasm_out"
    local t0 t1 elapsed
    t0=$(ms_now)
    (cd "$ROOT" && wasmtime run --dir=. "$rel_wasm" -- compile "$rel_src" --target "$TARGET" -o "$rel_out") >/dev/null 2>&1 || true
    t1=$(ms_now)
    elapsed=$(( t1 - t0 ))
    selfhost_samples="$selfhost_samples $elapsed"
  done
  SELFHOST_COMPILE_MS=$(echo "$selfhost_samples" | median)
  [[ -f "$wasm_out" ]] && SELFHOST_BINARY_BYTES=$(wc -c < "$wasm_out" | tr -d ' ')
  return 0
}

# --- ensure release build ----------------------------------------------------
if [[ "$USE_SELFHOST" == "true" ]]; then
  if [[ ! -f "$SELFHOST_WASM" ]]; then
    echo "ERROR: selfhost wasm not found at $SELFHOST_WASM" >&2
    echo "Build it first: cargo run -p arukellt -- compile src/compiler/main.ark --target wasm32-wasi-p1 -o src/compiler/arukellt-s1.wasm" >&2
    exit 1
  fi
  echo "Using selfhost compiler: $SELFHOST_WASM"
  COMPILER_LABEL="selfhost"
elif [[ ! -x "$COMPILER" ]]; then
  echo "::group::Building compiler in release mode"
  cargo build --release -p arukellt
  echo "::endgroup::"
  COMPILER_LABEL="rust"
else
  echo "Compiler already built: $COMPILER"
  COMPILER_LABEL="rust"
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

  # ---- compile timing (primary compiler) ------------------------------------
  if [[ "$USE_SELFHOST" == "true" ]]; then
    # Use selfhost compiler via wasmtime (relative paths for --dir=.)
    _rel_src="${SRC#$ROOT/}"
    _rel_out="${WASM_OUT#$ROOT/}"
    _rel_wasm="${SELFHOST_WASM#$ROOT/}"
    COMPILE_CMD="cd $ROOT && wasmtime run --dir=. $_rel_wasm -- compile $_rel_src --target $TARGET -o $_rel_out"
  else
    COMPILE_CMD="$COMPILER compile $SRC -o $WASM_OUT --target $TARGET"
  fi
  COMPILE_SAMPLES=""
  for (( i=0; i<COMPILE_ITERS; i++ )); do
    rm -f "$WASM_OUT"
    t0=$(ms_now)
    eval $COMPILE_CMD >/dev/null 2>&1 || true
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

  # ---- compare mode: run selfhost compiler on same benchmark ----------------
  if [[ "$COMPARE" == "true" && "$USE_SELFHOST" != "true" ]]; then
    SELFHOST_OUT="$RESULTS_DIR/${bench}_selfhost.wasm"
    if selfhost_compile "$SRC" "$SELFHOST_OUT"; then
      echo "  selfhost compile: ${SELFHOST_COMPILE_MS}ms  binary: ${SELFHOST_BINARY_BYTES}B"

      # Compute ratios and warn on large deltas
      if [[ "$COMPILE_MEDIAN" -gt 0 && "$SELFHOST_COMPILE_MS" -gt 0 ]]; then
        RATIO=$(python3 -c "print(f'{$SELFHOST_COMPILE_MS / $COMPILE_MEDIAN:.1f}')" 2>/dev/null || echo "?")
        echo "  compile ratio (selfhost/rust): ${RATIO}x"
        OVER_THRESHOLD=$(python3 -c "print('yes' if ($SELFHOST_COMPILE_MS / $COMPILE_MEDIAN * 100) > $PERF_WARN_THRESHOLD else 'no')" 2>/dev/null || echo "no")
        if [[ "$OVER_THRESHOLD" == "yes" ]]; then
          echo "  ⚠ WARNING: selfhost compile time is ${RATIO}x slower (threshold: ${PERF_WARN_THRESHOLD}%)"
        fi
      fi

      if [[ "$BINARY_BYTES" -gt 0 && "$SELFHOST_BINARY_BYTES" -gt 0 ]]; then
        SIZE_RATIO=$(python3 -c "print(f'{$SELFHOST_BINARY_BYTES / $BINARY_BYTES:.2f}')" 2>/dev/null || echo "?")
        echo "  size ratio (selfhost/rust): ${SIZE_RATIO}x"
      fi

      # Collect comparison data for final summary
      COMPARE_DATA="${COMPARE_DATA:-}${bench}:${COMPILE_MEDIAN}:${SELFHOST_COMPILE_MS}:${BINARY_BYTES}:${SELFHOST_BINARY_BYTES}\n"
      rm -f "$SELFHOST_OUT"
    fi
  fi
done

# --- --compare-lang: time reference implementations --------------------------
# Helper: time a single executable using hyperfine (3 runs) or the shell timer.
# Sets LANG_MEDIAN_MS for the caller.
time_ref_binary() {
  local exe="$1"
  LANG_MEDIAN_MS=0
  if [[ ! -x "$exe" ]]; then return 1; fi

  if [[ -n "$HYPERFINE" ]]; then
    local hf_out
    hf_out=$(hyperfine --runs 3 --export-json /dev/stdout "$exe" 2>/dev/null || true)
    LANG_MEDIAN_MS=$(python3 -c "
import json,sys
data = json.loads('$hf_out') if '$hf_out' else {}
r = data.get('results',[{}])[0]
print(int(r.get('median',0)*1000))
" 2>/dev/null || echo 0)
  else
    local samples=""
    for (( _i=0; _i<3; _i++ )); do
      local _t0 _t1
      _t0=$(ms_now)
      "$exe" >/dev/null 2>&1 || true
      _t1=$(ms_now)
      samples="$samples $(( _t1 - _t0 ))"
    done
    LANG_MEDIAN_MS=$(echo "$samples" | median)
  fi
  return 0
}

if [[ -n "$COMPARE_LANGS" ]]; then
  echo ""
  echo "=== Language Comparison (--compare-lang $COMPARE_LANGS) ==="

  # Map language token → file extension and compiler command
  declare -A LANG_EXT=([c]="c" [rust]="rs" [go]="go")
  declare -A LANG_CC=([c]="cc" [rust]="rustc" [go]="go")
  declare -A LANG_FLAGS=([c]="-O2 -o" [rust]="-O -o" [go]="build -o")

  IFS=',' read -ra LANG_LIST <<< "$COMPARE_LANGS"

  # Warn if required toolchain is missing (but continue — affected benchmarks
  # will be shown as "(no cc)" / "(no rustc)" / "(no go)" in the table).
  for lang in "${LANG_LIST[@]}"; do
    case "$lang" in
      c)
        if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then
          echo "NOTE: cc/gcc not found — C reference benchmarks will be skipped."
          echo "      Install gcc or clang to enable C comparison (e.g. apt install gcc)."
        fi
        ;;
      rust)
        if ! command -v rustc >/dev/null 2>&1; then
          echo "NOTE: rustc not found — Rust reference benchmarks will be skipped."
        fi
        ;;
      go)
        if ! command -v go >/dev/null 2>&1; then
          echo "NOTE: go not found — Go reference benchmarks will be skipped."
        fi
        ;;
    esac
  done

  # Print header
  printf "%-16s %12s" "benchmark" "ark(ms)"
  for lang in "${LANG_LIST[@]}"; do
    printf " %10s" "$lang(ms)"
  done
  printf " %10s\n" "ratio(best)"

  printf "%-16s %12s" "--------" "-------"
  for lang in "${LANG_LIST[@]}"; do
    printf " %10s" "-------"
  done
  printf " %10s\n" "---------"

  LANG_TMP="$RESULTS_DIR/lang_refs"
  mkdir -p "$LANG_TMP"

  for bench in "${BENCH_NAMES[@]}"; do
    BENCH_WASM_MEDIAN="${RUNTIME_MEDIAN:-0}"

    # re-run to get runtime median for this bench (already printed above but not retained)
    BENCH_WASM="$RESULTS_DIR/${bench}.wasm"
    ARK_RUNTIME_MS=0
    if [[ -n "$WASMTIME" && -f "$BENCH_WASM" ]]; then
      local_samples=""
      for (( _i=0; _i<3; _i++ )); do
        _t0=$(ms_now)
        wasmtime run "$BENCH_WASM" >/dev/null 2>&1 || true
        _t1=$(ms_now)
        local_samples="$local_samples $(( _t1 - _t0 ))"
      done
      ARK_RUNTIME_MS=$(echo "$local_samples" | median)
    fi

    printf "%-16s %12s" "$bench" "$ARK_RUNTIME_MS"

    BEST_LANG_MS=$ARK_RUNTIME_MS

    for lang in "${LANG_LIST[@]}"; do
      ext="${LANG_EXT[$lang]:-}"
      src="$ROOT/benchmarks/${bench}.${ext}"
      ref_bin="$LANG_TMP/${bench}_${lang}"

      if [[ -z "$ext" || ! -f "$src" ]]; then
        printf " %10s" "(no ref)"
        continue
      fi

      # Compile the reference binary (once)
      if [[ ! -x "$ref_bin" ]]; then
        case "$lang" in
          c)    cc -O2 -o "$ref_bin" "$src" 2>/dev/null || { printf " %10s" "(cc err)"; continue; } ;;
          rust) rustc -O -o "$ref_bin" "$src" 2>/dev/null || { printf " %10s" "(rustc err)"; continue; } ;;
          go)   (cd "$LANG_TMP" && go build -o "$ref_bin" "$src") 2>/dev/null || { printf " %10s" "(go err)"; continue; } ;;
        esac
      fi

      time_ref_binary "$ref_bin"
      printf " %10s" "$LANG_MEDIAN_MS"
      if [[ "$LANG_MEDIAN_MS" -gt 0 ]]; then
        if [[ "$BEST_LANG_MS" -eq 0 || "$LANG_MEDIAN_MS" -lt "$BEST_LANG_MS" ]]; then
          BEST_LANG_MS=$LANG_MEDIAN_MS
        fi
      fi
    done

    if [[ "$BEST_LANG_MS" -gt 0 && "$ARK_RUNTIME_MS" -gt 0 ]]; then
      ratio=$(python3 -c "print(f'{$ARK_RUNTIME_MS/$BEST_LANG_MS:.2f}x')" 2>/dev/null || echo "?")
    else
      ratio="N/A"
    fi
    printf " %10s\n" "$ratio"
  done

  # Clean up compiled reference binaries
  rm -rf "$LANG_TMP"
  echo ""
fi

# --- comparison summary table ------------------------------------------------
if [[ "$COMPARE" == "true" && -n "${COMPARE_DATA:-}" ]]; then
  echo ""
  echo "=== Rust vs Selfhost Comparison ==="
  printf "%-15s %10s %10s %8s %10s %10s %8s\n" "benchmark" "rust(ms)" "self(ms)" "ratio" "rust(B)" "self(B)" "ratio"
  printf "%-15s %10s %10s %8s %10s %10s %8s\n" "--------" "--------" "--------" "-----" "-------" "-------" "-----"
  echo -e "$COMPARE_DATA" | while IFS=: read -r name rust_ms self_ms rust_b self_b; do
    [[ -z "$name" ]] && continue
    compile_ratio=$(python3 -c "print(f'{$self_ms/$rust_ms:.1f}x' if $rust_ms > 0 else 'N/A')" 2>/dev/null || echo "?")
    size_ratio=$(python3 -c "print(f'{$self_b/$rust_b:.2f}x' if $rust_b > 0 else 'N/A')" 2>/dev/null || echo "?")
    printf "%-15s %10s %10s %8s %10s %10s %8s\n" "$name" "$rust_ms" "$self_ms" "$compile_ratio" "$rust_b" "$self_b" "$size_ratio"
  done
  echo ""
fi

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
    "path": "$([[ "$USE_SELFHOST" == "true" ]] && echo "$SELFHOST_WASM" || echo "target/release/arukellt")",
    "kind": "$COMPILER_LABEL"
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
