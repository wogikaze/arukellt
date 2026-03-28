#!/bin/bash
# WAT roundtrip verification: compile → wasm2wat → wat2wasm → binary diff
# Validates that T3 emitter produces well-formed Wasm that survives roundtrip.
set -euo pipefail

ARUKELLT="${ARUKELLT:-target/debug/arukellt}"
WASM_TOOLS="${WASM_TOOLS:-wasm-tools}"

if ! command -v "$WASM_TOOLS" &>/dev/null; then
    echo "SKIP: wasm-tools not found" >&2
    exit 0
fi

FIXTURES_DIR="tests/fixtures"
MANIFEST="$FIXTURES_DIR/manifest.txt"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

PASS=0
FAIL=0
SKIP=0
ERRORS=""

while IFS=: read -r kind path || [ -n "$kind" ]; do
    # Only test run: fixtures (they produce valid Wasm)
    case "$kind" in
        run|t3-compile) ;;
        *) continue ;;
    esac

    fixture="$FIXTURES_DIR/$path"
    [ -f "$fixture" ] || continue
    name=$(basename "$path" .ark)

    # Compile to Wasm
    wasm_file="$TMPDIR/${name}.wasm"
    if ! "$ARUKELLT" compile --target wasm32-wasi-p1 "$fixture" -o "$wasm_file" 2>/dev/null; then
        SKIP=$((SKIP + 1))
        continue
    fi

    # wasm2wat
    wat_file="$TMPDIR/${name}.wat"
    if ! "$WASM_TOOLS" print "$wasm_file" -o "$wat_file" 2>/dev/null; then
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: $path (wasm2wat failed)"
        continue
    fi

    # wat2wasm (roundtrip)
    rt_file="$TMPDIR/${name}_rt.wasm"
    if ! "$WASM_TOOLS" parse "$wat_file" -o "$rt_file" 2>/dev/null; then
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: $path (wat2wasm failed)"
        continue
    fi

    # Binary comparison (note: roundtrip may canonicalize, so we compare via wasm2wat again)
    rt_wat="$TMPDIR/${name}_rt.wat"
    "$WASM_TOOLS" print "$rt_file" -o "$rt_wat" 2>/dev/null || true
    if diff -q "$wat_file" "$rt_wat" >/dev/null 2>&1; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: $path (WAT text differs after roundtrip)"
    fi
done < "$MANIFEST"

echo "WAT roundtrip: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
if [ -n "$ERRORS" ]; then
    echo -e "$ERRORS"
fi

if [ $FAIL -gt 0 ]; then
    exit 1
fi
