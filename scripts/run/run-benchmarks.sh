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
SCALING=false

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
    --scaling) SCALING=true ;;
    --help|-h)
      cat <<'USAGE'
Usage: bash scripts/run/run-benchmarks.sh [OPTIONS]

Subcommands (via mise):
  mise bench                   Full benchmark: release build + all metrics (10 iters)
  mise bench:quick             Single-sample smoke benchmark
  mise bench:compare           Benchmark + show diff against stored baseline
  mise bench:selfhost          Benchmark using selfhost compiler only
  mise bench:update-baseline   Replace baseline with current measurements
  mise bench:ci                Compare + fail on threshold regression (CI gate)

Options:
  --quick                      Single-sample run (1 compile iter, 1 runtime iter, 0 warmups)
  --full                       10-iteration run  (10 compile iters, 10 runtime iters, 1 warmup)
  --compare                    Run Rust & selfhost compilers, print comparison table
  --selfhost                   Use selfhost (wasm) compiler instead of Rust compiler
  --scaling                    Run input-size sweep and emit scaling curve report (3 pts quick, 5 pts full)
  --compare-lang [c,rust,go]   Also time reference implementations in C/Rust/Go
  --target <TARGET>            Wasm target triple (default: wasm32-wasi-p1)
  -h, --help                   Show this help text

Optional tools (skipped if absent):
  wasmtime   — required for runtime benchmarks
  hyperfine  — improves timing accuracy; falls back to built-in shell timer
  /usr/bin/time — enables RSS memory measurement

Results are written to benchmarks/results/ as JSON (schema: arukellt-bench-v1).
USAGE
      exit 0
      ;;
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

# Compute a percentile from whitespace-separated numbers on stdin.
# Usage: echo "1 2 3 4 5" | percentile 95
percentile() {
  local pct="$1"
  python3 -c "
import sys, statistics
vals = sorted(float(x) for x in sys.stdin.read().split() if x)
if not vals:
    print('null'); sys.exit(0)
n = len(vals)
if n == 1:
    print(round(vals[0], 3)); sys.exit(0)
idx = (${pct} / 100) * (n - 1)
lo, hi = int(idx), min(int(idx) + 1, n - 1)
result = vals[lo] + (vals[hi] - vals[lo]) * (idx - lo)
print(round(result, 3))
" 2>/dev/null || echo "null"
}

# Compute sample standard deviation from whitespace-separated numbers on stdin.
stddev_calc() {
  python3 -c "
import sys, statistics
vals = [float(x) for x in sys.stdin.read().split() if x]
if len(vals) < 2:
    print('null'); sys.exit(0)
print(round(statistics.stdev(vals), 3))
" 2>/dev/null || echo "null"
}

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

TIME_BIN=""
TOOLING_TIME="false"
TOOLING_TIME_PATH="null"
if [[ -x "/usr/bin/time" ]]; then
  TIME_BIN="/usr/bin/time"
  TOOLING_TIME="true"
  TOOLING_TIME_PATH="\"/usr/bin/time\""
else
  echo "NOTE: /usr/bin/time not found — RSS memory measurement will be skipped"
fi

if [[ -z "$WASMTIME" ]]; then
  echo "NOTE: wasmtime not found — runtime benchmarks will be skipped"
fi

# --- measure startup overhead ------------------------------------------------
# Compile and run benchmarks/startup.ark (a no-op fixture) to capture the
# wasmtime instantiation + process startup cost.  The median of
# RUNTIME_ITERS samples (floored at 2) is stored in GLOBAL_STARTUP_MS
# and subtracted from each benchmark's median to produce guest_ms.
GLOBAL_STARTUP_MS="null"
_STARTUP_ITERS=$(( RUNTIME_ITERS > 2 ? RUNTIME_ITERS : 2 ))
_STARTUP_SRC="$ROOT/benchmarks/startup.ark"
_STARTUP_WASM="$RESULTS_DIR/startup_overhead.wasm"
if [[ -n "$WASMTIME" && -f "$_STARTUP_SRC" ]]; then
  echo "--- measuring startup overhead (no-op fixture, ${_STARTUP_ITERS} samples) ---"
  # compile the no-op fixture once
  if [[ "$USE_SELFHOST" == "true" ]]; then
    _rel_src="benchmarks/startup.ark"
    _rel_out="${_STARTUP_WASM#$ROOT/}"
    _rel_wasm="${SELFHOST_WASM#$ROOT/}"
    (cd "$ROOT" && wasmtime run --dir=. "$_rel_wasm" -- compile "$_rel_src" --target "$TARGET" -o "$_rel_out") >/dev/null 2>&1 || true
  else
    "$COMPILER" compile "$_STARTUP_SRC" -o "$_STARTUP_WASM" --target "$TARGET" >/dev/null 2>&1 || true
  fi

  if [[ -f "$_STARTUP_WASM" ]]; then
    _startup_samples=""
    _startup_run_cmd="wasmtime run $_STARTUP_WASM"
    [[ "$TARGET" == "wasm32-wasi-p2" ]] && _startup_run_cmd="wasmtime run --wasm gc $_STARTUP_WASM"
    for (( _si=0; _si<_STARTUP_ITERS; _si++ )); do
      _st0=$(ms_now)
      $WASMTIME run "$_STARTUP_WASM" >/dev/null 2>&1 || true
      _st1=$(ms_now)
      _startup_samples="$_startup_samples $(( _st1 - _st0 ))"
    done
    GLOBAL_STARTUP_MS=$(echo "$_startup_samples" | median)
    echo "  startup overhead: ${GLOBAL_STARTUP_MS}ms (median of ${_STARTUP_ITERS} samples)"
  else
    echo "  startup.ark compile skipped — no wasmtime or compiler not ready"
  fi
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
  COMPILE_RSS_SAMPLES=""
  for (( i=0; i<COMPILE_ITERS; i++ )); do
    rm -f "$WASM_OUT"
    if [[ -n "$TIME_BIN" ]]; then
      _rss_file=$(mktemp /tmp/ark-bench-rss-XXXXXX.txt)
      t0=$(ms_now)
      "$TIME_BIN" -f "%M" -o "$_rss_file" bash -c "$COMPILE_CMD" >/dev/null 2>&1 || true
      t1=$(ms_now)
      _rss_val=$(cat "$_rss_file" 2>/dev/null | tr -d '[:space:]')
      rm -f "$_rss_file"
      [[ "$_rss_val" =~ ^[0-9]+$ ]] && COMPILE_RSS_SAMPLES="$COMPILE_RSS_SAMPLES $_rss_val"
    else
      t0=$(ms_now)
      eval $COMPILE_CMD >/dev/null 2>&1 || true
      t1=$(ms_now)
    fi
    elapsed=$(( t1 - t0 ))
    COMPILE_SAMPLES="$COMPILE_SAMPLES $elapsed"
  done
  COMPILE_MEDIAN=$(echo "$COMPILE_SAMPLES" | median)
  COMPILE_RSS_MEDIAN="null"
  if [[ -n "$COMPILE_RSS_SAMPLES" ]]; then
    COMPILE_RSS_MEDIAN=$(echo "$COMPILE_RSS_SAMPLES" | median)
  fi
  BINARY_BYTES=0
  [[ -f "$WASM_OUT" ]] && BINARY_BYTES=$(wc -c < "$WASM_OUT" | tr -d ' ')

  COMPILE_SAMPLES_JSON=$(echo "$COMPILE_SAMPLES" | python3 -c "
import sys
vals = sys.stdin.read().split()
print('[' + ','.join(vals) + ']')
" 2>/dev/null || echo "[]")

  # ---- per-phase compile latency breakdown ----------------------------------
  # Uses `arukellt compile --json` to get machine-readable phase timings
  # (lex, parse, resolve, typecheck, lower, opt, emit, total).
  # Skipped for selfhost runs where the --json flag is unavailable.
  PHASE_MS_JSON="null"
  if [[ "$USE_SELFHOST" != "true" && -x "$COMPILER" ]]; then
    _phase_raw=$("$COMPILER" compile "$SRC" --target "$TARGET" --json 2>/dev/null || true)
    if [[ -n "$_phase_raw" ]]; then
      PHASE_MS_JSON=$(python3 -c "
import json, sys
raw = sys.argv[1]
try:
    data = json.loads(raw)
except Exception:
    print('null')
    sys.exit(0)
timing = data.get('timing')
if not timing:
    print('null')
    sys.exit(0)
phase = {}
mapping = {
    'lex_ms':       'lex',
    'parse_ms':     'parse',
    'resolve_ms':   'resolve',
    'typecheck_ms': 'typecheck',
    'lower_ms':     'lower',
    'opt_ms':       'opt',
    'emit_ms':      'emit',
    'total_ms':     'total',
}
for src_key, dst_key in mapping.items():
    if src_key in timing and timing[src_key] is not None:
        phase[dst_key] = timing[src_key]
print(json.dumps(phase) if phase else 'null')
" "$_phase_raw" 2>/dev/null || echo 'null')
    fi
  fi

  # ---- runtime timing -------------------------------------------------------
  RUNTIME_STATUS="skipped"
  RUNTIME_SAMPLES_JSON="[]"
  RUNTIME_MEDIAN="0"
  RUNTIME_RSS_MEDIAN="null"
  RUNTIME_ITERS_DONE=0
  RUNTIME_P50="null"
  RUNTIME_P95="null"
  RUNTIME_P99="null"
  RUNTIME_STDDEV="null"
  RUNTIME_STARTUP_MS="$GLOBAL_STARTUP_MS"
  RUNTIME_GUEST_MS="null"
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
    RUNTIME_RSS_SAMPLES=""
    CAPTURED_STDOUT=""
    for (( i=0; i<RUNTIME_ITERS; i++ )); do
      if [[ -n "$TIME_BIN" ]]; then
        _rss_file=$(mktemp /tmp/ark-bench-rss-XXXXXX.txt)
        t0=$(ms_now)
        CAPTURED_STDOUT=$("$TIME_BIN" -f "%M" -o "$_rss_file" $RUN_CMD 2>/dev/null) || true
        t1=$(ms_now)
        _rss_val=$(cat "$_rss_file" 2>/dev/null | tr -d '[:space:]')
        rm -f "$_rss_file"
        [[ "$_rss_val" =~ ^[0-9]+$ ]] && RUNTIME_RSS_SAMPLES="$RUNTIME_RSS_SAMPLES $_rss_val"
      else
        t0=$(ms_now)
        CAPTURED_STDOUT=$($RUN_CMD 2>/dev/null) || true
        t1=$(ms_now)
      fi
      elapsed=$(( t1 - t0 ))
      RUNTIME_SAMPLES="$RUNTIME_SAMPLES $elapsed"
    done
    RUNTIME_ITERS_DONE=$RUNTIME_ITERS
    RUNTIME_MEDIAN=$(echo "$RUNTIME_SAMPLES" | median)
    RUNTIME_RSS_MEDIAN="null"
    if [[ -n "$RUNTIME_RSS_SAMPLES" ]]; then
      RUNTIME_RSS_MEDIAN=$(echo "$RUNTIME_RSS_SAMPLES" | median)
    fi
    RUNTIME_SAMPLES_JSON=$(echo "$RUNTIME_SAMPLES" | python3 -c "
import sys
vals = sys.stdin.read().split()
print('[' + ','.join(vals) + ']')
" 2>/dev/null || echo "[]")

    # ---- percentile + stddev computation ------------------------------------
    RUNTIME_P50=$(echo "$RUNTIME_SAMPLES" | percentile 50)
    RUNTIME_P95=$(echo "$RUNTIME_SAMPLES" | percentile 95)
    RUNTIME_P99=$(echo "$RUNTIME_SAMPLES" | percentile 99)
    RUNTIME_STDDEV=$(echo "$RUNTIME_SAMPLES" | stddev_calc)

    # startup_ms comes from the global no-op fixture measurement
    RUNTIME_STARTUP_MS="$GLOBAL_STARTUP_MS"

    # guest_ms = median_ms - startup_ms, floored at 0
    RUNTIME_GUEST_MS="null"
    if [[ "$RUNTIME_STARTUP_MS" != "null" && "$RUNTIME_STARTUP_MS" =~ ^[0-9]+$ ]]; then
      _guest=$(( RUNTIME_MEDIAN - RUNTIME_STARTUP_MS ))
      [[ $_guest -lt 0 ]] && _guest=0
      RUNTIME_GUEST_MS=$_guest
    fi

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
    "max_rss_kb": $COMPILE_RSS_MEDIAN,
    "binary_bytes": $BINARY_BYTES,
    "command": "$COMPILE_CMD",
    "phase_ms": $PHASE_MS_JSON
  },
  "runtime": {
    "status": "$RUNTIME_STATUS",
    "iterations": $RUNTIME_ITERS_DONE,
    "warmups": $WARMUPS,
    "samples_ms": $RUNTIME_SAMPLES_JSON,
    "median_ms": $RUNTIME_MEDIAN,
    "p50_ms": $RUNTIME_P50,
    "p95_ms": $RUNTIME_P95,
    "p99_ms": $RUNTIME_P99,
    "stddev_ms": $RUNTIME_STDDEV,
    "startup_ms": $RUNTIME_STARTUP_MS,
    "guest_ms": $RUNTIME_GUEST_MS,
    "max_rss_kb": $RUNTIME_RSS_MEDIAN,
    "stdout": $STDOUT_ESCAPED,
    "correctness": "$CORRECTNESS",
    "command": "$RUN_CMD"
  }
}
JSONEOF

  BENCHMARKS_JSON="${BENCHMARKS_JSON}${SEP}${ENTRY}"
  SEP=","

  echo "  compile: ${COMPILE_MEDIAN}ms  binary: ${BINARY_BYTES}B  compile_rss: ${COMPILE_RSS_MEDIAN}KB  run: ${RUNTIME_MEDIAN}ms  p95: ${RUNTIME_P95}ms  p99: ${RUNTIME_P99}ms  guest: ${RUNTIME_GUEST_MS}ms  run_rss: ${RUNTIME_RSS_MEDIAN}KB  correctness: ${CORRECTNESS}"

  # Print phase breakdown if available
  if [[ "$PHASE_MS_JSON" != "null" ]]; then
    python3 -c "
import json, sys
try:
    p = json.loads(sys.argv[1])
    if isinstance(p, dict):
        parts = [f'{k}:{v:.1f}ms' for k, v in p.items() if k != 'total']
        total = p.get('total')
        print('  phases: ' + '  '.join(parts) + (f'  total:{total:.1f}ms' if total is not None else ''))
except Exception:
    pass
" "$PHASE_MS_JSON" 2>/dev/null || true
  fi

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

# --- --scaling: input-size sweep and scaling curve report --------------------
# Runs a set of parameterized benchmarks across multiple input sizes.
# quick = 3 size points; full = 5 size points.
# Emits a text table and a JSON file to benchmarks/results/.
#
# Template substitution strategy: for each "sweepable" benchmark we define
# the template token (a literal substring in the .ark source) and the list of
# size values that will be substituted into a tmp source file.
if [[ "$SCALING" == "true" ]]; then
  echo ""
  echo "=== Scaling Curve Sweep (mode: $MODE) ==="

  # Number of size points: 3 for quick, 5 for full
  if [[ "$MODE" == "full" ]]; then
    SCALING_POINTS=5
  else
    SCALING_POINTS=3
  fi

  # Define sweepable benchmarks:
  # Each entry = "name|token_in_source|label1:val1,label2:val2,..."
  # The token is the literal string in the .ark that encodes the size parameter.
  # It is replaced with each val in turn to produce a tmp .ark file.
  declare -a SCALING_BENCH_DEFS=(
    "fib|fib(35)|n=10:fib(10),n=20:fib(20),n=30:fib(30),n=35:fib(35),n=40:fib(40)"
    "binary_tree|depth: i32 = 20|depth=8:depth: i32 = 8,depth=12:depth: i32 = 12,depth=16:depth: i32 = 16,depth=20:depth: i32 = 20,depth=22:depth: i32 = 22"
    "string_concat|i < 100|n=10:i < 10,n=50:i < 50,n=100:i < 100,n=250:i < 250,n=500:i < 500"
    "vec_ops|i < 1000|n=100:i < 100,n=500:i < 500,n=1000:i < 1000,n=3000:i < 3000,n=8000:i < 8000"
  )

  SCALING_TMP_DIR="$RESULTS_DIR/scaling_tmp"
  mkdir -p "$SCALING_TMP_DIR"

  # Accumulate JSON entries for all benchmarks
  SCALING_BENCH_JSON_ARR=""
  SCALING_BENCH_SEP=""

  CLIFF_THRESHOLD=3  # ratio > 3x between adjacent sizes is a cliff

  for bench_def in "${SCALING_BENCH_DEFS[@]}"; do
    IFS='|' read -r bench_name token_literal sizes_spec <<< "$bench_def"

    SRC="$ROOT/benchmarks/${bench_name}.ark"
    if [[ ! -f "$SRC" ]]; then
      echo "  [scaling] SKIP $bench_name — source not found"
      continue
    fi

    # Parse sizes_spec into parallel label/value arrays
    IFS=',' read -ra PAIR_LIST <<< "$sizes_spec"
    LABELS=()
    TOKENS=()
    for pair in "${PAIR_LIST[@]}"; do
      IFS=':' read -r lbl tok <<< "$pair"
      LABELS+=("$lbl")
      TOKENS+=("$tok")
    done
    TOTAL_POINTS=${#LABELS[@]}

    # Select which points to use based on SCALING_POINTS
    # Always include first, last, and evenly-spaced intermediates
    SELECTED_INDICES=()
    if [[ $SCALING_POINTS -ge $TOTAL_POINTS ]]; then
      for (( idx=0; idx<TOTAL_POINTS; idx++ )); do
        SELECTED_INDICES+=($idx)
      done
    else
      # Pick SCALING_POINTS evenly spaced from TOTAL_POINTS
      for (( k=0; k<SCALING_POINTS; k++ )); do
        idx=$(python3 -c "print(round($k * ($TOTAL_POINTS - 1) / ($SCALING_POINTS - 1)))" 2>/dev/null || echo $k)
        SELECTED_INDICES+=($idx)
      done
    fi

    echo ""
    echo "--- scaling: $bench_name (${#SELECTED_INDICES[@]} points) ---"
    printf "  %-12s %12s %12s %12s\n" "size" "compile_ms" "runtime_ms" "binary_B"

    PREV_COMPILE=0
    PREV_RUNTIME=0
    POINT_JSON_ARR=""
    POINT_SEP=""

    for idx in "${SELECTED_INDICES[@]}"; do
      lbl="${LABELS[$idx]}"
      tok="${TOKENS[$idx]}"

      # Generate temp source by substituting token_literal → tok
      TMP_SRC="$SCALING_TMP_DIR/${bench_name}_${lbl//=/_}.ark"
      sed "s|${token_literal}|${tok}|g" "$SRC" > "$TMP_SRC"

      TMP_WASM="$SCALING_TMP_DIR/${bench_name}_${lbl//=/_}.wasm"

      # Compile
      if [[ "$USE_SELFHOST" == "true" ]]; then
        _rel_src="${TMP_SRC#$ROOT/}"
        _rel_out="${TMP_WASM#$ROOT/}"
        _rel_wasm="${SELFHOST_WASM#$ROOT/}"
        t0=$(ms_now)
        (cd "$ROOT" && wasmtime run --dir=. "$_rel_wasm" -- compile "$_rel_src" --target "$TARGET" -o "$_rel_out") >/dev/null 2>&1 || true
        t1=$(ms_now)
      else
        t0=$(ms_now)
        "$COMPILER" compile "$TMP_SRC" -o "$TMP_WASM" --target "$TARGET" >/dev/null 2>&1 || true
        t1=$(ms_now)
      fi
      SCALE_COMPILE_MS=$(( t1 - t0 ))
      SCALE_BINARY_BYTES=0
      [[ -f "$TMP_WASM" ]] && SCALE_BINARY_BYTES=$(wc -c < "$TMP_WASM" | tr -d ' ')

      # Runtime
      SCALE_RUNTIME_MS=0
      if [[ -n "$WASMTIME" && -f "$TMP_WASM" ]]; then
        t0=$(ms_now)
        $WASMTIME run "$TMP_WASM" >/dev/null 2>&1 || true
        t1=$(ms_now)
        SCALE_RUNTIME_MS=$(( t1 - t0 ))
      fi

      printf "  %-12s %12s %12s %12s" "$lbl" "$SCALE_COMPILE_MS" "$SCALE_RUNTIME_MS" "$SCALE_BINARY_BYTES"

      # Cliff detection: compare with previous point
      if [[ $PREV_COMPILE -gt 0 && $SCALE_COMPILE_MS -gt 0 ]]; then
        COMPILE_RATIO=$(python3 -c "print(f'{$SCALE_COMPILE_MS / $PREV_COMPILE:.2f}')" 2>/dev/null || echo "?")
        if python3 -c "import sys; sys.exit(0 if $SCALE_COMPILE_MS / $PREV_COMPILE > $CLIFF_THRESHOLD else 1)" 2>/dev/null; then
          printf "  ⚠ compile-cliff x%s" "$COMPILE_RATIO"
        fi
      fi
      if [[ $PREV_RUNTIME -gt 0 && $SCALE_RUNTIME_MS -gt 0 ]]; then
        RUNTIME_RATIO=$(python3 -c "print(f'{$SCALE_RUNTIME_MS / $PREV_RUNTIME:.2f}')" 2>/dev/null || echo "?")
        if python3 -c "import sys; sys.exit(0 if $SCALE_RUNTIME_MS / $PREV_RUNTIME > $CLIFF_THRESHOLD else 1)" 2>/dev/null; then
          printf "  ⚠ runtime-cliff x%s" "$RUNTIME_RATIO"
        fi
      fi
      printf "\n"

      PREV_COMPILE=$SCALE_COMPILE_MS
      PREV_RUNTIME=$SCALE_RUNTIME_MS

      # Accumulate JSON for this point
      POINT_JSON_ARR="${POINT_JSON_ARR}${POINT_SEP}{\"size_label\":\"${lbl}\",\"compile_ms\":${SCALE_COMPILE_MS},\"runtime_ms\":${SCALE_RUNTIME_MS},\"binary_bytes\":${SCALE_BINARY_BYTES}}"
      POINT_SEP=","
    done

    # Compute compile slope (ratio last/first)
    COMPILE_SLOPE="null"
    RUNTIME_SLOPE="null"
    if [[ ${#SELECTED_INDICES[@]} -ge 2 ]]; then
      FIRST_IDX="${SELECTED_INDICES[0]}"
      LAST_IDX="${SELECTED_INDICES[-1]}"
      _FIRST_LBL="${LABELS[$FIRST_IDX]}"
      _LAST_LBL="${LABELS[$LAST_IDX]}"
      # re-read compile values from the accumulated JSON
      COMPILE_SLOPE=$(python3 -c "
import json, sys
pts = json.loads('[' + sys.argv[1] + ']')
first_c = next((p['compile_ms'] for p in pts if p['compile_ms'] > 0), 0)
last_c  = next((p['compile_ms'] for p in reversed(pts) if p['compile_ms'] > 0), 0)
if first_c > 0 and last_c > 0:
    print(round(last_c / first_c, 2))
else:
    print('null')
" "$POINT_JSON_ARR" 2>/dev/null || echo "null")
      RUNTIME_SLOPE=$(python3 -c "
import json, sys
pts = json.loads('[' + sys.argv[1] + ']')
first_r = next((p['runtime_ms'] for p in pts if p['runtime_ms'] > 0), 0)
last_r  = next((p['runtime_ms'] for p in reversed(pts) if p['runtime_ms'] > 0), 0)
if first_r > 0 and last_r > 0:
    print(round(last_r / first_r, 2))
else:
    print('null')
" "$POINT_JSON_ARR" 2>/dev/null || echo "null")
      echo "  slope (last/first): compile=${COMPILE_SLOPE}x  runtime=${RUNTIME_SLOPE}x"
    fi

    SCALING_BENCH_JSON_ARR="${SCALING_BENCH_JSON_ARR}${SCALING_BENCH_SEP}{\"benchmark\":\"${bench_name}\",\"size_points\":[${POINT_JSON_ARR}],\"compile_slope\":${COMPILE_SLOPE},\"runtime_slope\":${RUNTIME_SLOPE}}"
    SCALING_BENCH_SEP=","
  done

  # Write scaling JSON report
  SCALING_RESULT_FILE="$RESULTS_DIR/scaling-${MODE}-$(date -u +%Y%m%dT%H%M%SZ).json"
  cat > "$SCALING_RESULT_FILE" <<SCALINGJSON
{
  "schema_version": "arukellt-scaling-v1",
  "generated_at": "$(now_iso)",
  "mode": "$MODE",
  "scaling_points": $SCALING_POINTS,
  "cliff_threshold": $CLIFF_THRESHOLD,
  "benchmarks": [
    $SCALING_BENCH_JSON_ARR
  ]
}
SCALINGJSON

  # Clean up tmp sources
  rm -rf "$SCALING_TMP_DIR"

  echo ""
  echo "Scaling results written to: $SCALING_RESULT_FILE"
fi

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
    "hyperfine": { "name": "hyperfine", "available": $TOOLING_HYPERFINE, "path": $TOOLING_HYPERFINE_PATH },
    "time": { "name": "/usr/bin/time", "available": $TOOLING_TIME, "path": $TOOLING_TIME_PATH }
  },
  "benchmarks": [
    $BENCHMARKS_JSON
  ]
}
TOPJSON

echo ""
echo "Results written to: $RESULT_FILE"
echo "Mode: $MODE ($MODE_DESC)"
