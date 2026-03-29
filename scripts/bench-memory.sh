#!/usr/bin/env bash
# Memory / GC telemetry benchmark for Arukellt.
#
# Compiles each benchmarks/*.ark file, then runs the resulting .wasm with
# wasmtime.  For every benchmark it reports:
#
#   wasm_binary_size  – bytes of the compiled .wasm
#   compilation_rss_kb – peak RSS of the compiler process (via /usr/bin/time)
#   peak_rss_kb        – peak RSS of the wasmtime runtime process
#
# GC pause metrics (gc_pause_total_ms, gc_pause_max_ms, gc_pause_count) are
# recorded as "unavailable" — the Wasm runtime does not yet expose GC
# instrumentation hooks.  When runtime support lands, this script should be
# updated to capture those values.
#
# Output: JSON written to stdout (or to the file given by --output).
#
# Usage:
#   scripts/bench-memory.sh [OPTIONS]
#
# Options:
#   --arukellt PATH   Compiler binary         (default: target/release/arukellt)
#   --target   TGT    Compilation target       (default: wasm32-wasi-p1)
#   --output   FILE   Write JSON to FILE       (default: stdout)
#   --help            Show this help
#
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------
ARUKELLT="${ARUKELLT:-$ROOT/target/release/arukellt}"
TARGET="wasm32-wasi-p1"
OUTPUT=""  # empty → stdout

# ---------------------------------------------------------------------------
# CLI parsing
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --arukellt) ARUKELLT="$2"; shift 2 ;;
        --target)   TARGET="$2";   shift 2 ;;
        --output)   OUTPUT="$2";   shift 2 ;;
        --help|-h)
            sed -n '2,/^$/{ s/^# //; s/^#//; p }' "$0"
            exit 0
            ;;
        *) echo >&2 "Unknown option: $1"; exit 1 ;;
    esac
done

# ---------------------------------------------------------------------------
# Tool detection
# ---------------------------------------------------------------------------
if [[ ! -x "$ARUKELLT" ]]; then
    echo >&2 "error: compiler not found at $ARUKELLT"
    echo >&2 "       Build with: cargo build --release -p arukellt"
    exit 1
fi

WASMTIME="$(command -v wasmtime 2>/dev/null || true)"
GNU_TIME=""
if [[ -x /usr/bin/time ]]; then
    GNU_TIME="/usr/bin/time"
fi

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
iso_now() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }

# measure_rss <rss-output-file> <command...>
# Runs the command wrapped in GNU time, writes peak RSS (KiB) into the file.
# Returns the command exit code.
measure_rss() {
    local rss_file="$1"; shift
    if [[ -n "$GNU_TIME" ]]; then
        "$GNU_TIME" -f "%M" -o "$rss_file" -- "$@"
    else
        "$@"
        echo "null" > "$rss_file"
    fi
}

# read_rss <rss-output-file>
# Prints the RSS value (integer KiB) or "null".
# GNU time may prepend error text when the child exits non-zero, so we
# extract the last line that looks like a bare integer.
read_rss() {
    local val
    val="$(grep -E '^[0-9]+$' "$1" 2>/dev/null | tail -1)"
    if [[ -z "$val" ]]; then
        echo "null"
    else
        echo "$val"
    fi
}

# ---------------------------------------------------------------------------
# Collect benchmark .ark files
# ---------------------------------------------------------------------------
ARK_FILES=()
for f in "$ROOT"/benchmarks/*.ark; do
    [[ -f "$f" ]] && ARK_FILES+=("$f")
done

if [[ ${#ARK_FILES[@]} -eq 0 ]]; then
    echo >&2 "error: no .ark files found in benchmarks/"
    exit 1
fi

# ---------------------------------------------------------------------------
# Work directory (inside project tree – no /tmp)
# ---------------------------------------------------------------------------
WORK_DIR="$ROOT/.bench-memory-work"
mkdir -p "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

# ---------------------------------------------------------------------------
# Run benchmarks
# ---------------------------------------------------------------------------
BENCHMARKS_JSON="[]"

for ark in "${ARK_FILES[@]}"; do
    name="$(basename "$ark" .ark)"
    wasm="$WORK_DIR/${name}.wasm"
    rss_file="$WORK_DIR/${name}.rss"

    # -- Compile --------------------------------------------------------
    compile_ok="true"
    measure_rss "$rss_file" \
        "$ARUKELLT" compile --target "$TARGET" -o "$wasm" "$ark" \
        >/dev/null 2>&1 \
        || compile_ok="false"

    compilation_rss="$(read_rss "$rss_file")"

    if [[ "$compile_ok" == "true" && -f "$wasm" ]]; then
        wasm_binary_size="$(stat --printf='%s' "$wasm" 2>/dev/null \
                            || stat -f '%z' "$wasm" 2>/dev/null)"
    else
        wasm_binary_size="null"
    fi

    # -- Runtime --------------------------------------------------------
    peak_rss="null"
    if [[ -n "$WASMTIME" && "$compile_ok" == "true" && -f "$wasm" ]]; then
        rss_file_rt="$WORK_DIR/${name}.rt.rss"
        measure_rss "$rss_file_rt" \
            "$WASMTIME" "$wasm" \
            >/dev/null 2>&1 \
            || true
        peak_rss="$(read_rss "$rss_file_rt")"
    fi

    # -- GC placeholder ------------------------------------------------
    # GC pause telemetry is not yet available from the Wasm runtime.
    # When runtime instrumentation is added, capture:
    #   gc_pause_total_ms, gc_pause_max_ms, gc_pause_count
    gc_status="unavailable"
    gc_note="Runtime GC instrumentation not yet implemented; see issue #143."

    # -- Assemble per-benchmark JSON -----------------------------------
    entry="$(cat <<ENTRY_EOF
{
  "name": "$name",
  "source": "benchmarks/${name}.ark",
  "wasm_binary_size": $wasm_binary_size,
  "compilation_rss_kb": $compilation_rss,
  "peak_rss_kb": $peak_rss,
  "gc_pause": {
    "status": "$gc_status",
    "note": "$gc_note",
    "total_ms": null,
    "max_ms": null,
    "count": null
  }
}
ENTRY_EOF
)"
    BENCHMARKS_JSON="$(echo "$BENCHMARKS_JSON" | jq --argjson e "$entry" '. + [$e]')"
done

# ---------------------------------------------------------------------------
# Assemble top-level JSON
# ---------------------------------------------------------------------------
RESULT="$(jq -n \
    --arg sv   "arukellt-bench-v1" \
    --arg ts   "$(iso_now)" \
    --arg tgt  "$TARGET" \
    --arg comp "$(realpath --relative-to="$ROOT" "$ARUKELLT" 2>/dev/null || echo "$ARUKELLT")" \
    --argjson wasmtime_avail "$(if [[ -n "$WASMTIME" ]]; then echo true; else echo false; fi)" \
    --argjson time_avail     "$(if [[ -n "$GNU_TIME" ]]; then echo true; else echo false; fi)" \
    --argjson benchmarks     "$BENCHMARKS_JSON" \
'{
  "schema_version": $sv,
  "generated_at": $ts,
  "report": "memory-gc-telemetry",
  "target": $tgt,
  "compiler": { "path": $comp },
  "tooling": {
    "wasmtime": { "name": "wasmtime", "available": $wasmtime_avail },
    "gnu_time": { "name": "/usr/bin/time", "available": $time_avail }
  },
  "gc_support": {
    "status": "unavailable",
    "note": "Runtime GC instrumentation not yet implemented. GC pause fields are placeholders."
  },
  "benchmarks": $benchmarks
}')"

# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------
if [[ -n "$OUTPUT" ]]; then
    mkdir -p "$(dirname "$OUTPUT")"
    echo "$RESULT" > "$OUTPUT"
    echo >&2 "Results written to $OUTPUT"
else
    echo "$RESULT"
fi
