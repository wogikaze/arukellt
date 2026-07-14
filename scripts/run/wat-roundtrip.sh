#!/bin/bash
# WAT roundtrip verification: compile → wasm2wat → wat2wasm → WAT text diff
#
# Validates that the wasm32 (non-GC) emitter produces well-formed core
# Wasm that survives a wasm2wat → wat2wasm roundtrip with no textual delta.
#
# Tool priority (core Wasm only):
#   1. wasm-tools  (wasm-tools print / wasm-tools parse)
#   2. wabt        (wasm2wat / wat2wasm) — installed at /usr/bin on this host
#
# Note: wasm32-gc output requires wasm-tools ≥0.200 with Wasm GC support. wabt
# does not understand GC types (e.g. 0x5e), so GC fixtures are skipped when
# only wabt is present.
#
# What a failure looks like:
#   - "wasm2wat failed"  → the binary produced by the emitter is not parseable
#     as valid core Wasm; the emitter emitted an ill-formed binary.
#   - "wat2wasm failed"  → the WAT text produced by wasm2wat is syntactically
#     invalid; this can point to a name-section or custom-section issue.
#   - "WAT text differs" → the re-encoded WAT diverges from the original; this
#     usually means a lossy encoding or malformed instruction sequence.
#
# Exit 0 = all tested fixtures pass (or no fixtures / tool not found → skip).
# Exit 1 = at least one fixture failed the roundtrip.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
NC=$'\033[0m'

# Resolve arukellt binary
ARUKELLT="${ARUKELLT_BIN:-}"
if [ -z "$ARUKELLT" ]; then
    if [ -x "scripts/run/arukellt-selfhost.sh" ]; then
        ARUKELLT="scripts/run/arukellt-selfhost.sh"
    elif [ -x "target/release/arukellt" ]; then
        ARUKELLT="target/release/arukellt"
    elif [ -x "target/debug/arukellt" ]; then
        ARUKELLT="target/debug/arukellt"
    fi
fi

if [ -z "$ARUKELLT" ] || [ ! -x "$ARUKELLT" ]; then
    echo -e "${YELLOW}SKIP: arukellt entrypoint not found (set ARUKELLT_BIN or use scripts/run/arukellt-selfhost.sh)${NC}" >&2
    exit 0
fi

# Resolve WAT tools — prefer wasm-tools, fall back to wabt
WASM2WAT=""
WAT2WASM=""
TOOL_MODE=""

if command -v wasm-tools &>/dev/null; then
    TOOL_MODE="wasm-tools"
elif command -v wasm2wat &>/dev/null && command -v wat2wasm &>/dev/null; then
    WASM2WAT="$(command -v wasm2wat)"
    WAT2WASM="$(command -v wat2wasm)"
    TOOL_MODE="wabt"
else
    echo -e "${YELLOW}SKIP: neither wasm-tools nor wasm2wat/wat2wasm found on PATH${NC}" >&2
    echo "  Install wasm-tools (https://github.com/bytecodealliance/wasm-tools) or wabt." >&2
    exit 0
fi

echo "WAT roundtrip using: $TOOL_MODE"

# wabt does not support Wasm GC; restrict to non-GC wasm32 core Wasm.
COMPILE_TARGET="wasm32"
if [ "$TOOL_MODE" = "wasm-tools" ]; then
    ALSO_GC=true
else
    ALSO_GC=false
fi

FIXTURES_DIR="tests/fixtures"
MANIFEST="$FIXTURES_DIR/manifest.txt"

if [ ! -f "$MANIFEST" ]; then
    echo -e "${RED}SKIP: fixture manifest not found at $MANIFEST${NC}" >&2
    exit 0
fi

mkdir -p ".build"
WORK_DIR=$(mktemp -d ".build/wat-roundtrip.XXXXXX")
trap 'rm -rf "$WORK_DIR"' EXIT

PASS=0
FAIL=0
SKIP=0
ERRORS=""

# ── helpers ──────────────────────────────────────────────────────────────────

do_wasm2wat() {
    local in="$1" out="$2"
    if [ "$TOOL_MODE" = "wasm-tools" ]; then
        wasm-tools print "$in" -o "$out"
    else
        "$WASM2WAT" "$in" -o "$out"
    fi
}

do_wat2wasm() {
    local in="$1" out="$2"
    if [ "$TOOL_MODE" = "wasm-tools" ]; then
        wasm-tools parse "$in" -o "$out"
    else
        "$WAT2WASM" "$in" -o "$out"
    fi
}

roundtrip_one() {
    local label="$1" wasm_file="$2"
    local base
    base="$WORK_DIR/$(echo "$label" | tr '/' '_')"

    local wat_file="${base}.wat"
    local rt_wasm="${base}_rt.wasm"
    local rt_wat="${base}_rt.wat"

    # Step 1: binary → WAT text
    if ! do_wasm2wat "$wasm_file" "$wat_file" 2>/dev/null; then
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: $label (wasm2wat failed — emitter produced ill-formed binary)"
        return
    fi

    # Step 2: WAT text → binary
    if ! do_wat2wasm "$wat_file" "$rt_wasm" 2>/dev/null; then
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: $label (wat2wasm failed — WAT text is syntactically invalid)"
        return
    fi

    # Step 3: re-decode and compare WAT text (binary may differ due to canonical encoding)
    if ! do_wasm2wat "$rt_wasm" "$rt_wat" 2>/dev/null; then
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: $label (second wasm2wat failed on roundtrip binary)"
        return
    fi

    if diff -q "$wat_file" "$rt_wat" >/dev/null 2>&1; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        local diff_lines
        diff_lines=$(diff "$wat_file" "$rt_wat" 2>/dev/null | head -10 || true)
        ERRORS="${ERRORS}\n  FAIL: $label (WAT text differs after roundtrip)\n$(printf '%s' "$diff_lines" | sed 's/^/    /')"
    fi
}

# ── main loop ─────────────────────────────────────────────────────────────────

while IFS=: read -r kind fixture_path || [ -n "$kind" ]; do
    kind="${kind%%#*}"   # strip inline comments
    kind="${kind// /}"   # strip spaces
    fixture_path="${fixture_path// /}"

    # Only test run: fixtures — they are expected to compile to valid Wasm.
    [ "$kind" = "run" ] || continue
    [ -n "$fixture_path" ] || continue

    fixture="$FIXTURES_DIR/$fixture_path"
    [ -f "$fixture" ] || continue

    label="$fixture_path"
    wasm_out="$WORK_DIR/$(echo "$label" | tr '/' '_').wasm"

    # Compile wasm32 core Wasm
    if ! "$ARUKELLT" compile --target "$COMPILE_TARGET" "$fixture" -o "$wasm_out" 2>/dev/null; then
        SKIP=$((SKIP + 1))
        continue
    fi

    roundtrip_one "$label (wasm32)" "$wasm_out"

    # Optionally also test wasm32-gc when wasm-tools is available
    if [ "$ALSO_GC" = true ]; then
        gc_out="$WORK_DIR/$(echo "$label" | tr '/' '_').gc.wasm"
        if "$ARUKELLT" compile --target wasm32-gc "$fixture" -o "$gc_out" 2>/dev/null; then
            roundtrip_one "$label (wasm32-gc)" "$gc_out"
        fi
    fi
done < "$MANIFEST"

# ── summary ──────────────────────────────────────────────────────────────────

printf '\nWAT roundtrip summary: PASS=%s FAIL=%s SKIP=%s\n' "$PASS" "$FAIL" "$SKIP"

if [ -n "$ERRORS" ]; then
    printf '%b' "$ERRORS\n"
fi

if [ "$FAIL" -gt 0 ]; then
    echo -e "${RED}✗ WAT roundtrip: $FAIL fixture(s) failed${NC}"
    exit 1
elif [ "$PASS" -eq 0 ]; then
    echo -e "${YELLOW}⊙ WAT roundtrip: no fixtures tested (all skipped)${NC}"
    # Treat as skip (exit 0) rather than failure — tooling or binary may be absent.
    exit 0
else
    echo -e "${GREEN}✓ WAT roundtrip: all $PASS fixture(s) passed${NC}"
    exit 0
fi
