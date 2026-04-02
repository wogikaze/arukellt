#!/usr/bin/env bash
# verify-bootstrap.sh — Bootstrap fixpoint verification for self-hosting.
#
# Stages:
#   0  Compile src/compiler/*.ark with the Rust compiler → arukellt-s1.wasm
#   1  Compile src/compiler/*.ark with arukellt-s1.wasm  → arukellt-s2.wasm
#   2  Compare sha256(arukellt-s1.wasm) == sha256(arukellt-s2.wasm)
#
# Usage:
#   scripts/run/verify-bootstrap.sh                # all stages (skip unavailable)
#   scripts/run/verify-bootstrap.sh --stage1-only  # only Stage 0 (Rust → s1)
#   scripts/run/verify-bootstrap.sh --stage N      # run single stage
#   scripts/run/verify-bootstrap.sh --help
#
# Exit: 0 if all enabled stages pass, 1 on first failure.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
if [[ -n "${ARUKELLT_BIN:-}" ]]; then
    COMPILER="$ARUKELLT_BIN"
elif [[ -f "${REPO_ROOT}/target/debug/arukellt" ]]; then
    COMPILER="${REPO_ROOT}/target/debug/arukellt"
elif [[ -f "${REPO_ROOT}/target/release/arukellt" ]]; then
    COMPILER="${REPO_ROOT}/target/release/arukellt"
else
    echo "error: no arukellt binary found. Run cargo build -p arukellt." >&2
    exit 1
fi
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
CHECK_MODE=false
FIXTURE_PARITY=false

usage() {
    cat <<'EOF'
Usage: scripts/run/verify-bootstrap.sh [options]

Bootstrap fixpoint verification for the Arukellt self-hosted compiler.

Stages:
  0  Compile selfhost .ark sources with the Rust compiler → arukellt-s1.wasm
  1  Compile selfhost .ark sources with arukellt-s1.wasm  → arukellt-s2.wasm
  2  Compare sha256(arukellt-s1.wasm) == sha256(arukellt-s2.wasm)

Options:
  --stage1-only      Only run Stage 0 (Rust compiles selfhost → s1)
  --stage N          Run single stage N
  --fixture-parity   Run fixture parity check after Stage 0
  --check            Machine-readable exit: 0 = fixpoint reached, 1 = not reached
  --help, -h         Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --stage1-only)     STAGE1_ONLY=true ;;
        --fixture-parity)  FIXTURE_PARITY=true ;;
        --check)           CHECK_MODE=true ;;
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

echo -e "${YELLOW}Bootstrap verification${NC}"
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
    echo -e "  Compiling main.ark → arukellt-s1.wasm (unified binary)..."
    if "$COMPILER" compile "${MAIN_SRC}" --target wasm32-wasi-p1 -o "$S1_WASM" 2>/dev/null; then
        local size
        size=$(wc -c < "$S1_WASM")
        echo -e "  ${GREEN}OK${NC}  arukellt-s1.wasm (${size} bytes)"
        return 0
    else
        echo -e "  ${RED}FAIL${NC}  main.ark did not compile" >&2
        return 1
    fi
}

run_stage 0 "Compile selfhost sources (Rust compiler)" stage0

# ── Fixture parity (optional, after Stage 0) ──────────────────────────────────

if [[ "$FIXTURE_PARITY" = true && -f "$S1_WASM" ]]; then
    PARITY_SCRIPT="${REPO_ROOT}/scripts/check/check-selfhost-parity.sh"
    if [[ -x "$PARITY_SCRIPT" ]]; then
        echo -e "${CYAN}── Fixture Parity ──${NC}"
        SELFHOST_WASM="$S1_WASM" REPO_ROOT="$REPO_ROOT" bash "$PARITY_SCRIPT" --fixture
    else
        echo -e "${YELLOW}SKIP${NC}  check-selfhost-parity.sh not found"
    fi
    echo
fi

if [[ "$STAGE1_ONLY" = true || "$FIXTURE_PARITY" = true ]]; then
    if [[ "$FAILURES" -gt 0 ]]; then
        echo -e "${RED}Bootstrap verification FAILED (${FAILURES} stage(s))${NC}"
        exit 1
    else
        echo -e "${GREEN}Bootstrap verification PASSED${NC}"
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

        local rel_src="${MAIN_SRC#$REPO_ROOT/}"
        local rel_out="${S2_WASM#$REPO_ROOT/}"
        echo -e "  Compiling main.ark → arukellt-s2.wasm (via s1)..."
        if timeout 120 wasmtime run --dir="${REPO_ROOT}" \
            "$S1_WASM" -- compile "$rel_src" --target wasm32-wasi-p1 \
            -o "$rel_out" 2>/dev/null; then
            local size
            size=$(wc -c < "$S2_WASM")
            echo -e "  ${GREEN}OK${NC}  arukellt-s2.wasm (${size} bytes)"
            return 0
        else
            echo -e "  ${RED}FAIL${NC}  main.ark did not compile with s1" >&2
            echo -e "  ${YELLOW}NOTE${NC}  Self-compilation requires features the selfhost may not yet support."
            return 1
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
            echo "    1. Run scripts/run/compare-outputs.sh <phase> <fixture> for each phase"
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

if [[ "$CHECK_MODE" = true ]]; then
    # Machine-readable: just report achieved/not-achieved per criteria
    echo "selfhost-check:"
    echo "  stage0-compile: $([ -f "$S1_WASM" ] && echo reached || echo not-reached)"
    echo "  stage1-compile: $([ -f "$S2_WASM" ] && echo reached || echo not-reached)"
    if [[ -f "$S1_WASM" && -f "$S2_WASM" ]]; then
        h1="$(sha256sum "$S1_WASM" | awk '{print $1}')"
        h2="$(sha256sum "$S2_WASM" | awk '{print $1}')"
        echo "  fixpoint: $([ "$h1" = "$h2" ] && echo reached || echo not-reached)"
    else
        echo "  fixpoint: not-reached"
    fi
    # Run parity checks if check-selfhost-parity.sh exists
    if [[ -x "${REPO_ROOT}/scripts/check/check-selfhost-parity.sh" && -f "$S1_WASM" ]]; then
        parity_out=$(SELFHOST_WASM="$S1_WASM" "${REPO_ROOT}/scripts/check/check-selfhost-parity.sh" 2>&1 || true)
        echo "  $(echo "$parity_out" | grep 'fixture-parity:' | head -1 || echo 'fixture-parity: not-verified')"
        echo "  $(echo "$parity_out" | grep 'cli-parity:' | head -1 || echo 'cli-parity: not-verified')"
        echo "  $(echo "$parity_out" | grep 'diag-parity:' | head -1 || echo 'diag-parity: not-verified')"
    else
        echo "  fixture-parity: not-verified"
        echo "  cli-parity: not-verified"
        echo "  diagnostic-parity: not-verified"
    fi
    echo "  determinism: not-verified"
    exit "$FAILURES"
fi

if [[ "$FAILURES" -gt 0 ]]; then
    echo -e "${RED}Bootstrap verification FAILED (${FAILURES} stage(s))${NC}"
    exit 1
else
    echo -e "${GREEN}Bootstrap verification PASSED${NC}"
    exit 0
fi
