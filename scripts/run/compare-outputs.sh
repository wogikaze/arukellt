#!/usr/bin/env bash
# compare-outputs.sh — Compare Rust and selfhost compiler outputs for a given phase.
#
# Usage:
#   scripts/run/compare-outputs.sh <phase> [fixture.ark]
#   scripts/run/compare-outputs.sh --help
#
# Phases (Rust):     parse, resolve, corehir, mir, optimized-mir, backend-plan
# Phases (selfhost): tokens, ast, hir, mir, wasm
#
# The Rust compiler uses ARUKELLT_DUMP_PHASES=<phase> (output on stderr).
# The selfhost compiler uses --dump-phases <phase> (output on stderr).
#
# Exits 0 if output is identical, 1 if different.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

# ── Colours ───────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# ── CLI parsing ───────────────────────────────────────────────────────────────

usage() {
    cat <<'EOF'
Usage: scripts/run/compare-outputs.sh <phase> [fixture.ark]

Compare Rust-hosted and selfhost compiler phase output for a given fixture.

Phases (Rust):     parse, resolve, corehir, mir, optimized-mir, backend-plan
Phases (selfhost): tokens, ast, hir, mir, wasm

If no fixture is given, defaults to tests/fixtures/hello/hello.ark.

Options:
  --rust-bin <path>      Path to Rust compiler (default: target/release/arukellt)
  --selfhost-wasm <path> Path to selfhost .wasm (default: src/compiler/lexer.wasm)
  --context <n>          Unified diff context lines (default: 3)
  --help, -h             Show this help
EOF
}

PHASE=""
FIXTURE=""
RUST_BIN="${REPO_ROOT}/target/release/arukellt"
SELFHOST_WASM="${REPO_ROOT}/src/compiler/lexer.wasm"
DIFF_CONTEXT=3

while [[ $# -gt 0 ]]; do
    case "$1" in
        --help|-h)
            usage
            exit 0
            ;;
        --rust-bin)
            shift
            RUST_BIN="${1:?--rust-bin requires a path}"
            ;;
        --selfhost-wasm)
            shift
            SELFHOST_WASM="${1:?--selfhost-wasm requires a path}"
            ;;
        --context)
            shift
            DIFF_CONTEXT="${1:?--context requires a number}"
            ;;
        -*)
            echo -e "${RED}error: unknown option: $1${NC}" >&2
            usage >&2
            exit 1
            ;;
        *)
            if [[ -z "$PHASE" ]]; then
                PHASE="$1"
            elif [[ -z "$FIXTURE" ]]; then
                FIXTURE="$1"
            else
                echo -e "${RED}error: unexpected argument: $1${NC}" >&2
                usage >&2
                exit 1
            fi
            ;;
    esac
    shift
done

if [[ -z "$PHASE" ]]; then
    echo -e "${RED}error: <phase> is required${NC}" >&2
    usage >&2
    exit 1
fi

if [[ -z "$FIXTURE" ]]; then
    FIXTURE="${REPO_ROOT}/tests/fixtures/hello/hello.ark"
fi

if [[ ! -f "$FIXTURE" ]]; then
    echo -e "${RED}error: fixture not found: ${FIXTURE}${NC}" >&2
    exit 1
fi

# ── Scratch directory ─────────────────────────────────────────────────────────

SCRATCH_DIR="${REPO_ROOT}/.compare-outputs-scratch"
mkdir -p "$SCRATCH_DIR"
trap 'rm -rf "$SCRATCH_DIR"' EXIT

RUST_OUT="${SCRATCH_DIR}/rust-${PHASE}.stderr"
SELF_OUT="${SCRATCH_DIR}/selfhost-${PHASE}.stderr"

# ── Run Rust compiler ─────────────────────────────────────────────────────────

echo -e "${CYAN}Phase: ${PHASE}${NC}"
echo -e "${CYAN}Fixture: ${FIXTURE}${NC}"
echo

if [[ ! -x "$RUST_BIN" ]]; then
    echo -e "${RED}error: Rust compiler not found at ${RUST_BIN}${NC}" >&2
    echo "       Run 'cargo build --release' first." >&2
    exit 1
fi

echo -e "${YELLOW}Running Rust compiler (ARUKELLT_DUMP_PHASES=${PHASE})...${NC}"
ARUKELLT_DUMP_PHASES="$PHASE" "$RUST_BIN" compile "$FIXTURE" -o /dev/null \
    2>"$RUST_OUT" 1>/dev/null || true

# ── Run selfhost compiler ─────────────────────────────────────────────────────

if [[ ! -f "$SELFHOST_WASM" ]]; then
    echo -e "${YELLOW}SKIP: selfhost wasm not found at ${SELFHOST_WASM}${NC}"
    echo -e "${YELLOW}      Only Rust output captured.${NC}"
    echo
    echo -e "${CYAN}Rust output (${PHASE}):${NC}"
    head -50 "$RUST_OUT"
    exit 0
fi

if ! command -v wasmtime &>/dev/null; then
    echo -e "${RED}error: wasmtime not found in PATH${NC}" >&2
    exit 1
fi

echo -e "${YELLOW}Running selfhost compiler (--dump-phases ${PHASE})...${NC}"
wasmtime run "$SELFHOST_WASM" -- compile --dump-phases "$PHASE" "$FIXTURE" \
    2>"$SELF_OUT" 1>/dev/null || true

# ── Compare ───────────────────────────────────────────────────────────────────

echo

if diff -u --label "rust/${PHASE}" --label "selfhost/${PHASE}" \
    -U "$DIFF_CONTEXT" "$RUST_OUT" "$SELF_OUT"; then
    echo -e "${GREEN}PASS${NC}  Phase output identical (${PHASE})"
    exit 0
else
    echo
    echo -e "${RED}FAIL${NC}  Phase output differs (${PHASE})"
    exit 1
fi
