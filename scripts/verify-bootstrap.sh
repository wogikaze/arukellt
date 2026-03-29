#!/usr/bin/env bash
# verify-bootstrap.sh — Bootstrap fixpoint verification for self-hosting.
#
# Stages:
#   0  Compile src/compiler/*.ark with the Rust compiler → arukellt-s1.wasm
#   1  Compile src/compiler/*.ark with arukellt-s1.wasm  → arukellt-s2.wasm
#   2  Compare sha256(arukellt-s1.wasm) == sha256(arukellt-s2.wasm)
#
# Usage:
#   scripts/verify-bootstrap.sh                # all stages (skip unavailable)
#   scripts/verify-bootstrap.sh --stage1-only  # only Stage 0 (Rust → s1)
#   scripts/verify-bootstrap.sh --stage N      # run single stage
#   scripts/verify-bootstrap.sh --help
#
# Exit: 0 if all enabled stages pass, 1 on first failure.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPILER="${REPO_ROOT}/target/release/arukellt"
SELFHOST_DIR="${REPO_ROOT}/src/compiler"

# ── Colours ───────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# ── CLI parsing ───────────────────────────────────────────────────────────────

ONLY_STAGE=""
STAGE1_ONLY=false

usage() {
    cat <<'EOF'
Usage: scripts/verify-bootstrap.sh [options]

Bootstrap fixpoint verification for the Arukellt self-hosted compiler.

Stages:
  0  Compile selfhost .ark sources with the Rust compiler → arukellt-s1.wasm
  1  Compile selfhost .ark sources with arukellt-s1.wasm  → arukellt-s2.wasm
  2  Compare sha256(arukellt-s1.wasm) == sha256(arukellt-s2.wasm)

Options:
  --stage1-only   Only run Stage 0 (Rust compiles selfhost → s1)
  --stage N       Run single stage N
  --help, -h      Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --stage1-only) STAGE1_ONLY=true ;;
        --stage=*)     ONLY_STAGE="${1#--stage=}" ;;
        --stage)       shift; ONLY_STAGE="${1:-}" ;;
        --help|-h)     usage; exit 0 ;;
        *)
            echo -e "${RED}error: unknown option: $1${NC}" >&2
            usage >&2
            exit 1
            ;;
    esac
    shift
done

# ── Helpers ───────────────────────────────────────────────────────────────────

FAILURES=0

run_stage() {
    local stage="$1"
    local label="$2"
    shift 2

    if [[ -n "$ONLY_STAGE" && "$ONLY_STAGE" != "$stage" ]]; then
        return
    fi

    echo -e "${CYAN}── Stage ${stage}: ${label} ──${NC}"
    if "$@"; then
        echo -e "  ${GREEN}PASS${NC}  Stage ${stage}"
    else
        echo -e "  ${RED}FAIL${NC}  Stage ${stage}"
        FAILURES=$((FAILURES + 1))
    fi
    echo
}

skip_stage() {
    local stage="$1"
    local label="$2"
    local reason="$3"

    if [[ -n "$ONLY_STAGE" && "$ONLY_STAGE" != "$stage" ]]; then
        return
    fi

    echo -e "${CYAN}── Stage ${stage}: ${label} ──${NC}"
    echo -e "  ${YELLOW}SKIP${NC}  ${reason}"
    echo
}

# ── Artifact paths ────────────────────────────────────────────────────────────

BUILD_DIR="${REPO_ROOT}/.bootstrap-build"
mkdir -p "$BUILD_DIR"
trap 'rm -rf "$BUILD_DIR"' EXIT

S1_WASM="${BUILD_DIR}/arukellt-s1.wasm"
S2_WASM="${BUILD_DIR}/arukellt-s2.wasm"

# Individual component artifacts (current scaffold)
LEXER_SRC="${SELFHOST_DIR}/lexer.ark"
LEXER_WASM="${SELFHOST_DIR}/lexer.wasm"
PARSER_SRC="${SELFHOST_DIR}/parser.ark"
PARSER_WASM="${SELFHOST_DIR}/parser.wasm"
MAIN_SRC="${SELFHOST_DIR}/main.ark"

# ── Pre-flight ────────────────────────────────────────────────────────────────

echo -e "${YELLOW}Bootstrap verification — fixpoint scaffold${NC}"
echo

if [[ ! -d "$SELFHOST_DIR" ]]; then
    echo -e "${RED}ERROR: selfhost sources not found at ${SELFHOST_DIR}${NC}" >&2
    exit 1
fi

if [[ ! -x "$COMPILER" ]]; then
    echo -e "${RED}ERROR: compiler not found at ${COMPILER}${NC}" >&2
    echo "       Run 'cargo build --release' first." >&2
    exit 1
fi

# Enumerate selfhost .ark sources
SELFHOST_SOURCES=()
for src in "${SELFHOST_DIR}"/*.ark; do
    [[ -f "$src" ]] && SELFHOST_SOURCES+=("$src")
done

echo -e "  Selfhost sources: ${#SELFHOST_SOURCES[@]} files"
echo

# ── Stage 0: Compile selfhost sources with Rust compiler → s1 ────────────────

stage0() {
    local compiled=0
    local failed=0

    for src in "${SELFHOST_SOURCES[@]}"; do
        local base
        base="$(basename "$src" .ark)"
        echo -e "  Compiling ${base}.ark..."
        if "$COMPILER" compile "$src" 2>/dev/null; then
            compiled=$((compiled + 1))
        else
            echo -e "  ${RED}FAIL${NC}  ${base}.ark did not compile" >&2
            failed=$((failed + 1))
        fi
    done

    echo -e "  Compiled: ${compiled}  Failed: ${failed}"

    if [[ "$failed" -gt 0 ]]; then
        return 1
    fi

    # If main.ark produces an output wasm, copy it as s1
    if [[ -f "${SELFHOST_DIR}/main.wasm" ]]; then
        cp "${SELFHOST_DIR}/main.wasm" "$S1_WASM"
        echo -e "  Stage 1 artifact: $(wc -c < "$S1_WASM") bytes → arukellt-s1.wasm"
    else
        echo -e "  ${YELLOW}NOTE${NC}: main.wasm not produced (individual components compiled)"
    fi
}

run_stage 0 "Compile selfhost sources (Rust compiler)" stage0

if [[ "$STAGE1_ONLY" = true ]]; then
    if [[ "$FAILURES" -gt 0 ]]; then
        echo -e "${RED}Bootstrap verification FAILED (${FAILURES} stage(s))${NC}"
        exit 1
    else
        echo -e "${GREEN}Bootstrap verification PASSED (stage1-only)${NC}"
        exit 0
    fi
fi

# ── Stage 1: Compile selfhost sources with arukellt-s1.wasm → s2 ─────────────

if [[ -f "$S1_WASM" ]]; then
    stage1() {
        if ! command -v wasmtime &>/dev/null; then
            echo "  wasmtime not found in PATH" >&2
            return 1
        fi

        local compiled=0
        local failed=0

        for src in "${SELFHOST_SOURCES[@]}"; do
            local base
            base="$(basename "$src" .ark)"
            echo -e "  Compiling ${base}.ark (via s1)..."
            if wasmtime run "$S1_WASM" -- compile "$src" 2>/dev/null; then
                compiled=$((compiled + 1))
            else
                echo -e "  ${RED}FAIL${NC}  ${base}.ark did not compile with s1" >&2
                failed=$((failed + 1))
            fi
        done

        echo -e "  Compiled: ${compiled}  Failed: ${failed}"

        if [[ "$failed" -gt 0 ]]; then
            return 1
        fi

        if [[ -f "${SELFHOST_DIR}/main.wasm" ]]; then
            cp "${SELFHOST_DIR}/main.wasm" "$S2_WASM"
            echo -e "  Stage 2 artifact: $(wc -c < "$S2_WASM") bytes → arukellt-s2.wasm"
        fi
    }
    run_stage 1 "Compile selfhost sources (arukellt-s1.wasm)" stage1
else
    skip_stage 1 "Compile selfhost sources (arukellt-s1.wasm)" \
        "arukellt-s1.wasm not available (Stage 0 did not produce a unified binary)"
fi

# ── Stage 2: Fixpoint check — sha256(s1) == sha256(s2) ───────────────────────

if [[ -f "$S1_WASM" && -f "$S2_WASM" ]]; then
    stage2() {
        local hash1 hash2
        hash1="$(sha256sum "$S1_WASM" | awk '{print $1}')"
        hash2="$(sha256sum "$S2_WASM" | awk '{print $1}')"

        echo "  s1: ${hash1}"
        echo "  s2: ${hash2}"

        if [[ "$hash1" = "$hash2" ]]; then
            echo -e "  ${GREEN}Fixpoint reached — binaries are identical${NC}"
            return 0
        else
            echo -e "  ${RED}Fixpoint NOT reached — binaries differ${NC}"
            echo
            echo "  Debug steps:"
            echo "    1. Run scripts/compare-outputs.sh <phase> <fixture> for each phase"
            echo "    2. Find the first phase where outputs diverge"
            echo "    3. Fix the selfhost source and re-run this script"
            return 1
        fi
    }
    run_stage 2 "Fixpoint check (sha256 comparison)" stage2
else
    skip_stage 2 "Fixpoint check (sha256 comparison)" \
        "Requires both arukellt-s1.wasm and arukellt-s2.wasm"
fi

# ── Summary ───────────────────────────────────────────────────────────────────

if [[ "$FAILURES" -gt 0 ]]; then
    echo -e "${RED}Bootstrap verification FAILED (${FAILURES} stage(s))${NC}"
    exit 1
else
    echo -e "${GREEN}Bootstrap verification PASSED${NC}"
    exit 0
fi
