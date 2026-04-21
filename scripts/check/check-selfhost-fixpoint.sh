#!/usr/bin/env bash
# check-selfhost-fixpoint.sh — Verify that the selfhost bootstrap fixpoint is reached.
#
# The REAL bootstrap fixpoint is:
#   sha256(arukellt-s2.wasm) == sha256(arukellt-s3.wasm)
#
# where:
#   s1.wasm = Rust compiler compiles src/compiler/main.ark  (trusted base)
#   s2.wasm = s1.wasm (selfhost stage-1) compiles src/compiler/main.ark
#   s3.wasm = s2.wasm (selfhost stage-2) compiles src/compiler/main.ark
#
# If sha256(s2)==sha256(s3), the selfhost compiler can reproduce itself bit-for-bit.
# The Rust compiler (s1) applies different optimization passes than the selfhost,
# so sha256(s1)!=sha256(s2) is expected and does NOT indicate a fixpoint failure.
#
# Usage:
#   bash scripts/check/check-selfhost-fixpoint.sh           # build+compare
#   bash scripts/check/check-selfhost-fixpoint.sh --no-build  # skip rebuild, compare cached
#   bash scripts/check/check-selfhost-fixpoint.sh --help
#
# Exit codes:
#   0 — fixpoint reached (sha256(s2) == sha256(s3))
#   1 — fixpoint NOT reached (hash mismatch, build failure, or prerequisites missing)
#   2 — prerequisites missing (arukellt binary or wasmtime not found)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUILD_DIR="${REPO_ROOT}/.build/selfhost"
S1_WASM="${BUILD_DIR}/arukellt-s1.wasm"
S2_WASM="${BUILD_DIR}/arukellt-s2.wasm"
S3_WASM="${BUILD_DIR}/arukellt-s3.wasm"

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
CYAN=$'\033[0;36m'
NC=$'\033[0m'

NO_BUILD=false

usage() {
    cat <<'EOF'
Usage: bash scripts/check/check-selfhost-fixpoint.sh [options]

Check that the selfhost bootstrap fixpoint is reached.
Builds s1.wasm (Rust → Ark), s2.wasm (s1 → Ark), s3.wasm (s2 → Ark),
then verifies sha256(s2) == sha256(s3) (the real bootstrap fixpoint).

The Rust compiler applies different optimization passes, so sha256(s1)!=sha256(s2)
is expected. Only sha256(s2)==sha256(s3) is required for bootstrap attainment.

Options:
  --no-build   Skip rebuilding; compare existing .build/selfhost/s2 and s3
  --help, -h   Show this help

Exit: 0 if sha256(s2) == sha256(s3), 1 otherwise, 2 if prerequisites missing.
EOF
}

for arg in "$@"; do
    case "$arg" in
        --no-build)  NO_BUILD=true ;;
        --help|-h)   usage; exit 0 ;;
        *)           echo "${RED}error: unknown option: $arg${NC}" >&2; usage >&2; exit 2 ;;
    esac
done

# ── Locate arukellt binary ────────────────────────────────────────────────────
ARUKELLT_BIN="${ARUKELLT_BIN:-}"
if [[ -z "$ARUKELLT_BIN" ]]; then
    # Prefer debug over release: the debug build is always freshly compiled;
    # the release build may be stale (older than recent intrinsic additions).
    # Users who want the release binary should set ARUKELLT_BIN explicitly.
    for candidate in \
        "${REPO_ROOT}/target/debug/arukellt" \
        "${REPO_ROOT}/target/release/arukellt"
    do
        if [[ -x "$candidate" ]]; then
            ARUKELLT_BIN="$candidate"
            break
        fi
    done
fi
if [[ -z "$ARUKELLT_BIN" || ! -x "$ARUKELLT_BIN" ]]; then
    echo "${RED}error: arukellt binary not found.${NC}" >&2
    echo "  Build with: cargo build -p arukellt" >&2
    echo "  Or set: ARUKELLT_BIN=/path/to/arukellt" >&2
    exit 2
fi

# ── Locate wasmtime ───────────────────────────────────────────────────────────
if ! command -v wasmtime >/dev/null 2>&1; then
    echo "${RED}error: wasmtime not found on PATH.${NC}" >&2
    echo "  Install from: https://wasmtime.dev/ or via mise/cargo" >&2
    exit 2
fi

mkdir -p "$BUILD_DIR"

# ── Stage 1: Rust compiler → s1.wasm ────────────────────────────────────────
if [[ "$NO_BUILD" = false ]]; then
    echo "${CYAN}[stage-1] Rust compiler → s1.wasm ...${NC}"
    rm -f "$S1_WASM"
    # Run from REPO_ROOT so the stdlib lookup (which searches parent dirs for
    # a 'std' directory) works correctly regardless of where the script is called.
    (
        cd "$REPO_ROOT"
        "$ARUKELLT_BIN" compile src/compiler/main.ark \
            --target wasm32-wasi-p1 \
            -o "$S1_WASM"
    ) 2>&1 || true
    if [[ ! -f "$S1_WASM" ]]; then
        echo "${RED}✗ Stage 1 failed: Rust compiler did not produce s1.wasm${NC}" >&2
        exit 1
    fi
    echo "${GREEN}  s1.wasm: $(wc -c < "$S1_WASM") bytes${NC}"
else
    if [[ ! -f "$S1_WASM" ]]; then
        echo "${RED}error: --no-build specified but ${S1_WASM} does not exist${NC}" >&2
        exit 1
    fi
    echo "${YELLOW}  s1.wasm: $(wc -c < "$S1_WASM") bytes (pre-built, --no-build)${NC}"
fi

# ── Stage 2: s1.wasm compiles itself → s2.wasm ──────────────────────────────
if [[ "$NO_BUILD" = false ]]; then
    echo "${CYAN}[stage-2] s1.wasm selfhost compile → s2.wasm ...${NC}"
    rm -f "$S2_WASM"
    # wasmtime WASI P1: output path must be relative to the preopened directory.
    # We use --dir="$REPO_ROOT" and write to a path relative to REPO_ROOT.
    S2_REL=".build/selfhost/arukellt-s2.wasm"
    s2_output=""
    s2_output=$(
        cd "$REPO_ROOT"
        wasmtime run \
            --dir="$REPO_ROOT" \
            "$S1_WASM" \
            -- compile src/compiler/main.ark \
               --target wasm32-wasi-p1 \
               -o "$S2_REL" 2>&1
    ) || true

    if [[ ! -f "$S2_WASM" ]]; then
        echo "${RED}✗ Stage 2 failed: s1.wasm did not produce s2.wasm${NC}" >&2
        printf '  output: %s\n' "$s2_output" >&2
        exit 1
    fi
    echo "${GREEN}  s2.wasm: $(wc -c < "$S2_WASM") bytes${NC}"
    printf '  stage-2 output: %s\n' "$s2_output"
else
    if [[ ! -f "$S2_WASM" ]]; then
        echo "${RED}error: --no-build specified but ${S2_WASM} does not exist${NC}" >&2
        exit 1
    fi
    echo "${YELLOW}  s2.wasm: $(wc -c < "$S2_WASM") bytes (pre-built, --no-build)${NC}"
fi

# ── Stage 3: s2.wasm compiles itself → s3.wasm ──────────────────────────────
if [[ "$NO_BUILD" = false ]]; then
    echo "${CYAN}[stage-3] s2.wasm selfhost compile → s3.wasm ...${NC}"
    rm -f "$S3_WASM"
    S3_REL=".build/selfhost/arukellt-s3.wasm"
    s3_output=""
    s3_output=$(
        cd "$REPO_ROOT"
        wasmtime run \
            --dir="$REPO_ROOT" \
            "$S2_WASM" \
            -- compile src/compiler/main.ark \
               --target wasm32-wasi-p1 \
               -o "$S3_REL" 2>&1
    ) || true

    if [[ ! -f "$S3_WASM" ]]; then
        echo "${RED}✗ Stage 3 failed: s2.wasm did not produce s3.wasm${NC}" >&2
        printf '  output: %s\n' "$s3_output" >&2
        exit 1
    fi
    echo "${GREEN}  s3.wasm: $(wc -c < "$S3_WASM") bytes${NC}"
    printf '  stage-3 output: %s\n' "$s3_output"
else
    if [[ ! -f "$S3_WASM" ]]; then
        echo "${RED}error: --no-build specified but ${S3_WASM} does not exist${NC}" >&2
        exit 1
    fi
    echo "${YELLOW}  s3.wasm: $(wc -c < "$S3_WASM") bytes (pre-built, --no-build)${NC}"
fi

# ── Compare sha256 hashes ────────────────────────────────────────────────────
echo ""
echo "${CYAN}[fixpoint] Comparing sha256 hashes ...${NC}"
S1_HASH=$(sha256sum "$S1_WASM" | awk '{print $1}')
S2_HASH=$(sha256sum "$S2_WASM" | awk '{print $1}')
S3_HASH=$(sha256sum "$S3_WASM" | awk '{print $1}')

echo "  s1.wasm (Rust):    ${S1_HASH}  ($(wc -c < "$S1_WASM") bytes)"
echo "  s2.wasm (selfhost-1): ${S2_HASH}  ($(wc -c < "$S2_WASM") bytes)"
echo "  s3.wasm (selfhost-2): ${S3_HASH}  ($(wc -c < "$S3_WASM") bytes)"

# Note: sha256(s1) != sha256(s2) is expected — the Rust compiler applies MIR
# optimization passes (inline, dead-code elim) that the selfhost does not.
if [[ "$S1_HASH" = "$S2_HASH" ]]; then
    echo ""
    echo "${GREEN}✓ Strong fixpoint: sha256(s1) == sha256(s2) (Rust and selfhost agree)${NC}"
elif [[ "$S2_HASH" = "$S3_HASH" ]]; then
    echo ""
    echo "${GREEN}✓ Selfhost fixpoint reached: sha256(s2) == sha256(s3)${NC}"
    echo "  (sha256(s1) != sha256(s2): Rust applies extra MIR optimizations — expected)"
    exit 0
else
    echo ""
    echo "${RED}✗ Selfhost fixpoint NOT reached: sha256(s2) ≠ sha256(s3)${NC}"
    echo ""
    echo "  The selfhost compiler produces different output on successive runs."
    echo "  This indicates non-deterministic code generation or a lowering bug."
    echo ""
    echo "  Size delta: s2=$(wc -c < "$S2_WASM") bytes / s3=$(wc -c < "$S3_WASM") bytes"
    exit 1
fi
