#!/usr/bin/env bash
# verify-bootstrap.sh — Bootstrap fixpoint verification for self-hosting.
#
# Scaffold contract (issues/open/154-bootstrap-verification-scaffold.md, v5 roadmap):
#   Canonical stages are fixed slots; behavior is implemented here (not no-op stubs).
#
#   Stage 0 — Trusted base: Rust-hosted `arukellt` compiles `src/compiler/main.ark`
#             to a single wasm artifact (wasm32-wasi-p1).
#   Stage 1 — First self-compile: run Stage 0 output under wasmtime with the repo
#             root mounted; compile the same `main.ark` to a second wasm artifact.
#   Stage 2 — Fixpoint gate: byte identity of Stage 0 vs Stage 1 outputs
#             (sha256 equality on the two wasm files).
#
# Artifact naming (under REPO_ROOT, all ephemeral unless noted):
#   BUILD_DIR="${REPO_ROOT}/.bootstrap-build"   # removed on exit (trap)
#   S1_WASM="${BUILD_DIR}/arukellt-s1.wasm"      # Stage 0 output
#   S2_WASM="${BUILD_DIR}/arukellt-s2.wasm"      # Stage 1 output
#   Stage stderr logs: "${BUILD_DIR}/stage0.stderr", "${BUILD_DIR}/stage1.stderr"
#
# Comparison targets:
#   Stage 2 compares sha256(S1_WASM) vs sha256(S2_WASM).  When they differ, the
#   script prints both digests and sizes; it does not emit a binary diff.  Use
#   `scripts/run/compare-outputs.sh` on fixtures to find the first divergent
#   compiler phase (see docs/compiler/bootstrap.md).
#
# Future integration: `scripts/run/verify-harness.sh` may call this script once
# bootstrap is stable; today optional `--fixpoint` uses check-selfhost-fixpoint.sh
# (see docs/process/bootstrap-verification.md).
#
# Usage:
#   scripts/run/verify-bootstrap.sh                # full bootstrap attainment gate
#   scripts/run/verify-bootstrap.sh --stage1-only  # partial Stage 0 smoke only
#   scripts/run/verify-bootstrap.sh --stage N      # partial single-stage run
#   scripts/run/verify-bootstrap.sh --help
#
# Exit: 0 if the requested verification scope passes, 1 otherwise.

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
    --check            Machine-readable full bootstrap gate; incompatible with partial modes
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
STAGE0_STATUS="not-run"
STAGE1_STATUS="not-run"
STAGE2_STATUS="not-run"

mark_stage_status() {
    local stage="$1"
    local status="$2"

    case "$stage" in
        0) STAGE0_STATUS="$status" ;;
        1) STAGE1_STATUS="$status" ;;
        2) STAGE2_STATUS="$status" ;;
    esac
}

is_partial_mode() {
    [[ -n "$ONLY_STAGE" || "$STAGE1_ONLY" = true || "$FIXTURE_PARITY" = true ]]
}

run_stage() {
    local stage="$1"
    local label="$2"
    shift 2

    if [[ -n "$ONLY_STAGE" && "$ONLY_STAGE" != "$stage" ]]; then
        mark_stage_status "$stage" "not-requested"
        return
    fi

    echo -e "${CYAN}── Stage ${stage}: ${label} ──${NC}"
    if "$@"; then
        mark_stage_status "$stage" "reached"
        echo -e "  ${GREEN}PASS${NC}  Stage ${stage}"
    else
        mark_stage_status "$stage" "not-reached"
        echo -e "  ${RED}FAIL${NC}  Stage ${stage}"
        FAILURES=$((FAILURES + 1))
    fi
    echo
}



# ── Artifact paths ────────────────────────────────────────────────────────────

BUILD_DIR="${REPO_ROOT}/.bootstrap-build"
mkdir -p "$BUILD_DIR"
trap 'rm -rf "$BUILD_DIR"' EXIT

S1_WASM="${BUILD_DIR}/arukellt-s1.wasm"
S2_WASM="${BUILD_DIR}/arukellt-s2.wasm"
MAIN_SRC="${SELFHOST_DIR}/main.ark"

# ── Pre-flight ────────────────────────────────────────────────────────────────

echo -e "${YELLOW}Bootstrap verification${NC}"
echo

if [[ "$CHECK_MODE" = true ]] && is_partial_mode; then
    echo -e "${RED}ERROR:${NC} --check requires the full Stage 0 → 1 → 2 bootstrap gate." >&2
    echo "       Remove --stage, --stage1-only, and --fixture-parity when using --check." >&2
    exit 1
fi

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
    local stderr_file="${BUILD_DIR}/stage0.stderr"
    if "$COMPILER" compile "${MAIN_SRC}" --target wasm32-wasi-p1 -o "$S1_WASM" 2>"$stderr_file"; then
        local size
        size=$(wc -c < "$S1_WASM")
        echo -e "  ${GREEN}OK${NC}  arukellt-s1.wasm (${size} bytes)"
        return 0
    else
        echo -e "  ${RED}FAIL${NC}  main.ark did not compile" >&2
        if [[ -s "$stderr_file" ]]; then
            sed 's/^/    /' "$stderr_file" >&2
        fi
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
        echo -e "${RED}Bootstrap partial verification FAILED (${FAILURES} stage(s))${NC}"
        exit 1
    else
        echo -e "${GREEN}Bootstrap partial verification PASSED${NC}"
        echo "  Scope: requested partial verification completed; bootstrap attainment was not evaluated."
        exit 0
    fi
fi

# ── Stage 1: Compile selfhost sources with arukellt-s1.wasm → s2 ─────────────

stage1() {
    if [[ ! -f "$S1_WASM" ]]; then
        echo -e "  ${RED}FAIL${NC}  Cannot run Stage 1: arukellt-s1.wasm not available" >&2
        echo "  Cause: Stage 0 did not produce a unified binary." >&2
        echo "  Hint:  Fix Stage 0 compilation errors first, then re-run." >&2
        return 1
    fi

    if ! command -v wasmtime &>/dev/null; then
        echo -e "  ${RED}FAIL${NC}  wasmtime not found in PATH" >&2
        echo "  Hint:  Install wasmtime: curl https://wasmtime.dev/install.sh -sSf | bash" >&2
        return 1
    fi

    local rel_src="${MAIN_SRC#$REPO_ROOT/}"
    local rel_out="${S2_WASM#$REPO_ROOT/}"
    local stderr_file="${BUILD_DIR}/stage1.stderr"
    echo -e "  Compiling main.ark → arukellt-s2.wasm (via s1)..."
    if timeout 120 wasmtime run --dir="${REPO_ROOT}" \
        "$S1_WASM" -- compile "$rel_src" --target wasm32-wasi-p1 \
        -o "$rel_out" 2>"$stderr_file"; then
        local size
        size=$(wc -c < "$S2_WASM")
        echo -e "  ${GREEN}OK${NC}  arukellt-s2.wasm (${size} bytes)"
        return 0
    else
        local exit_code=$?
        echo -e "  ${RED}FAIL${NC}  main.ark did not compile with s1 (exit ${exit_code})" >&2
        if [[ -s "$stderr_file" ]]; then
            echo "  stderr:" >&2
            sed 's/^/    /' "$stderr_file" >&2
        fi
        echo -e "  ${YELLOW}ATTAINMENT UNMET${NC}  Stage 1 did not produce arukellt-s2.wasm." >&2
        return 1
    fi
}
run_stage 1 "Compile selfhost sources (arukellt-s1.wasm)" stage1

# ── Stage 2: Fixpoint check — sha256(s1) == sha256(s2) ───────────────────────

stage2() {
    local missing=0
    if [[ ! -f "$S1_WASM" ]]; then
        echo -e "  ${RED}Missing: arukellt-s1.wasm (Stage 0 output)${NC}" >&2
        missing=1
    fi
    if [[ ! -f "$S2_WASM" ]]; then
        echo -e "  ${RED}Missing: arukellt-s2.wasm (Stage 1 output)${NC}" >&2
        missing=1
    fi
    if [[ "$missing" -ne 0 ]]; then
        echo -e "  ${RED}FAIL${NC}  Cannot run fixpoint check: prerequisite artifacts missing" >&2
        echo "  Hint:  Fix earlier stage failures first." >&2
        return 1
    fi

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
        local s1_size s2_size
        s1_size=$(wc -c < "$S1_WASM")
        s2_size=$(wc -c < "$S2_WASM")
        echo "  s1 size: ${s1_size} bytes"
        echo "  s2 size: ${s2_size} bytes"
        echo
        echo "  Debug steps:"
        echo "    1. Run scripts/run/compare-outputs.sh <phase> <fixture> for each phase"
        echo "    2. Find the first phase where outputs diverge"
        echo "    3. Fix the selfhost source and re-run this script"
        return 1
    fi
}
run_stage 2 "Fixpoint check (sha256 comparison)" stage2

# ── Summary ───────────────────────────────────────────────────────────────────

if [[ -n "$ONLY_STAGE" ]]; then
    for stage in 0 1 2; do
        if [[ "$stage" != "$ONLY_STAGE" ]]; then
            mark_stage_status "$stage" "not-requested"
        fi
    done
fi

if [[ "$CHECK_MODE" = true ]]; then
    echo "bootstrap-check:"
    echo "  stage0-compile: ${STAGE0_STATUS}"
    echo "  stage1-self-compile: ${STAGE1_STATUS}"
    echo "  stage2-fixpoint: ${STAGE2_STATUS}"
    if [[ "$FAILURES" -eq 0 ]]; then
        echo "  attainment: reached"
        exit 0
    fi

    echo "  attainment: not-reached"
    exit 1
fi

if [[ "$FAILURES" -gt 0 ]]; then
    echo -e "${RED}Bootstrap verification FAILED (${FAILURES} stage(s))${NC}"
    exit 1
else
    if is_partial_mode; then
        echo -e "${GREEN}Bootstrap partial verification PASSED${NC}"
        echo "  Scope: requested subset completed; bootstrap attainment was not evaluated."
        exit 0
    fi

    echo -e "${GREEN}Bootstrap verification PASSED${NC}"
    exit 0
fi
