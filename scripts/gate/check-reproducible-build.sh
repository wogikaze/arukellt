#!/usr/bin/env bash
# check-reproducible-build.sh — Assert that compiling the same source twice
# produces byte-for-byte identical .wasm output (bit-exact reproducibility).
#
# Usage:
#   bash scripts/gate/check-reproducible-build.sh [--fixture <path>] [--target <target>] [--verbose]
#
# Options:
#   --fixture <path>   .ark source file to compile (default: tests/fixtures/hello/hello.ark)
#   --target <target>  Compilation target (default: wasm32-wasi-p1)
#   --verbose          Show sha256 hashes even on success
#   --help             Show this help message
#
# Exit codes:
#   0  Both builds are byte-for-byte identical (reproducible)
#   1  Builds differ or compilation failed
#   2  Prerequisites missing (compiler binary not found)

set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
NC=$'\033[0m'

FIXTURE="tests/fixtures/hello/hello.ark"
TARGET="wasm32-wasi-p1"
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --fixture) FIXTURE="$2"; shift 2 ;;
        --target)  TARGET="$2";  shift 2 ;;
        --verbose) VERBOSE=true; shift ;;
        --help|-h)
            sed -n '2,/^$/p' "$0" | grep '^#' | sed 's/^# \?//'
            exit 0
            ;;
        *) echo -e "${RED}error: unknown option: $1${NC}" >&2; exit 1 ;;
    esac
done

# Locate compiler binary
ARUKELLT_BIN="${ARUKELLT_BIN:-}"
if [ -z "$ARUKELLT_BIN" ]; then
    if   [ -x "./target/debug/arukellt"   ]; then ARUKELLT_BIN="./target/debug/arukellt"
    elif [ -x "./target/release/arukellt" ]; then ARUKELLT_BIN="./target/release/arukellt"
    fi
fi

if [ -z "$ARUKELLT_BIN" ] || [ ! -x "$ARUKELLT_BIN" ]; then
    echo -e "${RED}✗ reproducible build: compiler binary not found (build first or set ARUKELLT_BIN)${NC}" >&2
    exit 2
fi

if [ ! -f "$FIXTURE" ]; then
    echo -e "${RED}✗ reproducible build: fixture not found: $FIXTURE${NC}" >&2
    exit 1
fi

TMPDIR_REPRO=$(mktemp -d /tmp/ark-repro-XXXXXX)
trap 'rm -rf "$TMPDIR_REPRO"' EXIT

OUT1="$TMPDIR_REPRO/build1.wasm"
OUT2="$TMPDIR_REPRO/build2.wasm"

echo -e "${YELLOW}[reproducible-build] Compiling '$FIXTURE' twice (target: $TARGET)...${NC}"

# First compilation
if ! "$ARUKELLT_BIN" compile "$FIXTURE" --target "$TARGET" -o "$OUT1" >/dev/null 2>&1; then
    echo -e "${RED}✗ reproducible build: first compilation failed${NC}" >&2
    "$ARUKELLT_BIN" compile "$FIXTURE" --target "$TARGET" -o /dev/null 2>&1 | tail -20 >&2
    exit 1
fi

# Second compilation
if ! "$ARUKELLT_BIN" compile "$FIXTURE" --target "$TARGET" -o "$OUT2" >/dev/null 2>&1; then
    echo -e "${RED}✗ reproducible build: second compilation failed${NC}" >&2
    exit 1
fi

# Compute checksums
SHA1=$(sha256sum "$OUT1" | awk '{print $1}')
SHA2=$(sha256sum "$OUT2" | awk '{print $1}')

if [ "$VERBOSE" = true ]; then
    echo "  build1 sha256: $SHA1"
    echo "  build2 sha256: $SHA2"
fi

if [ "$SHA1" = "$SHA2" ]; then
    SIZE=$(wc -c < "$OUT1")
    echo -e "${GREEN}✓ reproducible build: both outputs identical (sha256=$SHA1, ${SIZE} bytes)${NC}"
    exit 0
fi

# Builds differ — emit diagnostic info to help track down the source
echo -e "${RED}✗ reproducible build: outputs differ!${NC}" >&2
echo "  build1 sha256: $SHA1" >&2
echo "  build2 sha256: $SHA2" >&2
echo "" >&2

# WAT disassembly diff (optional — skip if wasm2wat not available)
if command -v wasm2wat >/dev/null 2>&1; then
    WAT1="$TMPDIR_REPRO/build1.wat"
    WAT2="$TMPDIR_REPRO/build2.wat"
    wasm2wat "$OUT1" -o "$WAT1" 2>/dev/null || true
    wasm2wat "$OUT2" -o "$WAT2" 2>/dev/null || true
    if [ -f "$WAT1" ] && [ -f "$WAT2" ]; then
        echo "  WAT diff (first 40 lines):" >&2
        diff "$WAT1" "$WAT2" 2>/dev/null | head -40 >&2 || true
    fi
else
    echo "  (install wasm2wat / wabt for WAT-level diff)" >&2
fi

# Binary-level diff summary
echo "" >&2
echo "  Binary diff summary:" >&2
cmp -l "$OUT1" "$OUT2" 2>/dev/null | head -20 >&2 || true

exit 1
