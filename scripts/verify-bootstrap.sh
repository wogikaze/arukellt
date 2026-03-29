#!/usr/bin/env bash
# verify-bootstrap.sh — Fixpoint verification scaffold for self-hosting.
#
# Stages:
#   0  Compile lexer.ark with the Rust-hosted compiler → lexer.wasm
#   1  Run lexer.wasm under wasmtime and verify exit 0
#   2  (placeholder) Compile parser.ark → parser.wasm, run it
#   3  (placeholder) Fixpoint — compile the compiler with itself,
#      compare output to the previous stage's binary
#
# Usage:  scripts/verify-bootstrap.sh [--stage N]
# Exit:   0 if all enabled stages pass, 1 on first failure.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPILER="${REPO_ROOT}/target/release/arukellt"

# ── Colours ───────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# ── CLI parsing ───────────────────────────────────────────────────────────────

ONLY_STAGE=""
for arg in "$@"; do
    case "$arg" in
        --stage=*) ONLY_STAGE="${arg#--stage=}" ;;
        --stage)   shift; ONLY_STAGE="${1:-}" ;;
    esac
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

LEXER_SRC="${REPO_ROOT}/src/compiler/lexer.ark"
LEXER_WASM="${REPO_ROOT}/src/compiler/lexer.wasm"
PARSER_SRC="${REPO_ROOT}/src/compiler/parser.ark"
PARSER_WASM="${REPO_ROOT}/src/compiler/parser.wasm"

# ── Pre-flight ────────────────────────────────────────────────────────────────

echo -e "${YELLOW}Bootstrap verification — fixpoint scaffold${NC}"
echo

if [[ ! -x "$COMPILER" ]]; then
    echo -e "${RED}ERROR: compiler not found at ${COMPILER}${NC}" >&2
    echo "       Run 'cargo build --release' first." >&2
    exit 1
fi

# ── Stage 0: Compile lexer.ark with the Rust compiler ────────────────────────

stage0() {
    if [[ ! -f "$LEXER_SRC" ]]; then
        echo "  lexer.ark not found at ${LEXER_SRC}" >&2
        return 1
    fi
    "$COMPILER" compile "$LEXER_SRC"
    if [[ ! -f "$LEXER_WASM" ]]; then
        echo "  Expected artifact ${LEXER_WASM} not produced" >&2
        return 1
    fi
    echo "  Produced $(wc -c < "$LEXER_WASM") bytes → lexer.wasm"
}

run_stage 0 "Compile lexer.ark (Rust compiler)" stage0

# ── Stage 1: Run lexer.wasm under wasmtime ────────────────────────────────────

stage1() {
    if [[ ! -f "$LEXER_WASM" ]]; then
        echo "  lexer.wasm not found — Stage 0 must succeed first" >&2
        return 1
    fi
    if ! command -v wasmtime &>/dev/null; then
        echo "  wasmtime not found in PATH" >&2
        return 1
    fi
    wasmtime run "$LEXER_WASM"
}

run_stage 1 "Run lexer.wasm (wasmtime)" stage1

# ── Stage 2: Compile & run parser.ark (placeholder) ──────────────────────────

if [[ -f "$PARSER_SRC" ]]; then
    stage2() {
        "$COMPILER" compile "$PARSER_SRC"
        if [[ ! -f "$PARSER_WASM" ]]; then
            echo "  Expected artifact ${PARSER_WASM} not produced" >&2
            return 1
        fi
        echo "  Produced $(wc -c < "$PARSER_WASM") bytes → parser.wasm"
        wasmtime run "$PARSER_WASM"
    }
    run_stage 2 "Compile & run parser.ark" stage2
else
    skip_stage 2 "Compile & run parser.ark" "parser.ark not yet written"
fi

# ── Stage 3: Fixpoint check (placeholder) ────────────────────────────────────
# When the self-hosted compiler can compile itself:
#   1. Compile compiler.ark with the Rust compiler    → compiler-s0.wasm
#   2. Compile compiler.ark with compiler-s0.wasm     → compiler-s1.wasm
#   3. Compile compiler.ark with compiler-s1.wasm     → compiler-s2.wasm
#   4. diff compiler-s1.wasm compiler-s2.wasm         → must be identical
#
# The identical output proves the compiler is a fixpoint: compiling
# itself with itself always yields the same binary.

skip_stage 3 "Fixpoint (compile self with self)" \
    "Self-hosted compiler not yet available"

# ── Summary ───────────────────────────────────────────────────────────────────

if [[ "$FAILURES" -gt 0 ]]; then
    echo -e "${RED}Bootstrap verification FAILED (${FAILURES} stage(s))${NC}"
    exit 1
else
    echo -e "${GREEN}Bootstrap verification PASSED${NC}"
    exit 0
fi
