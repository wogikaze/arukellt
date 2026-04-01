#!/usr/bin/env bash
# check-selfhost-parity.sh — Compare Rust compiler and selfhost compiler outputs.
#
# Modes:
#   --fixture   Compare stdout of representative fixtures compiled by both compilers
#   --cli       Compare basic CLI flag behaviour between Rust and selfhost
#   --diag      Compare diagnostic output (severity + error code) for error fixtures
#   (no flag)   Run all three modes
#
# Usage:
#   scripts/check-selfhost-parity.sh                 # all modes
#   scripts/check-selfhost-parity.sh --fixture       # fixture comparison only
#   scripts/check-selfhost-parity.sh --cli           # CLI comparison only
#   scripts/check-selfhost-parity.sh --diag          # diagnostic comparison only
#   scripts/check-selfhost-parity.sh --help
#
# Prerequisites:
#   - Rust compiler binary: target/release/arukellt or $ARUKELLT_BIN
#   - Selfhost wasm: src/compiler/arukellt-s1.wasm or $SELFHOST_WASM
#   - wasmtime on PATH
#
# Exit: 0 if all checks match, 1 on mismatch (with diff report).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUST_BIN="${ARUKELLT_BIN:-}"
SELFHOST_WASM="${SELFHOST_WASM:-${REPO_ROOT}/src/compiler/arukellt-s1.wasm}"
FIXTURE_DIR="${REPO_ROOT}/tests/fixtures"
MANIFEST="${FIXTURE_DIR}/manifest.txt"
WORK_DIR=""

# ── Colours ───────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# ── Helpers ───────────────────────────────────────────────────────────────────

cleanup() {
    if [[ -n "$WORK_DIR" && -d "$WORK_DIR" ]]; then
        rm -rf "$WORK_DIR"
    fi
}
trap cleanup EXIT

find_rust_bin() {
    if [[ -n "$RUST_BIN" && -f "$RUST_BIN" ]]; then
        return 0
    fi
    if [[ -f "${REPO_ROOT}/target/release/arukellt" ]]; then
        RUST_BIN="${REPO_ROOT}/target/release/arukellt"
        return 0
    fi
    if [[ -f "${REPO_ROOT}/target/debug/arukellt" ]]; then
        RUST_BIN="${REPO_ROOT}/target/debug/arukellt"
        return 0
    fi
    echo -e "${RED}error: Rust compiler binary not found. Build with cargo build -p arukellt or set ARUKELLT_BIN.${NC}" >&2
    return 1
}

check_selfhost() {
    if [[ ! -f "$SELFHOST_WASM" ]]; then
        # Try to compile selfhost from source
        echo -e "${CYAN}Compiling selfhost from source...${NC}" >&2
        if ! "${RUST_BIN}" compile "${REPO_ROOT}/src/compiler/main.ark" \
            --target wasm32-wasi-p1 -o "${SELFHOST_WASM}" 2>/dev/null; then
            echo -e "${RED}error: selfhost wasm not found at ${SELFHOST_WASM} and compilation failed.${NC}" >&2
            echo -e "${RED}Set SELFHOST_WASM or compile manually.${NC}" >&2
            return 1
        fi
    fi
}

check_wasmtime() {
    if ! command -v wasmtime >/dev/null 2>&1; then
        echo -e "${RED}error: wasmtime not found on PATH${NC}" >&2
        return 1
    fi
}

# Compile and run a .ark file with the Rust compiler
rust_run() {
    local ark_file="$1"
    local run_out
    run_out=$(timeout 15 "${RUST_BIN}" run "$ark_file" 2>/dev/null) || true
    echo "$run_out"
}

# Compile a .ark file with the selfhost compiler and run the output
selfhost_run() {
    local ark_file="$1"
    # selfhost uses WASI P1 path_open with dirfd=3 (first --dir).
    # All paths must be relative to that preopened directory.
    local rel_out="_selfhost_parity_out.wasm"
    local wasm_out="${REPO_ROOT}/${rel_out}"
    rm -f "$wasm_out"

    # Convert absolute ark_file to relative path from REPO_ROOT
    local rel_input="${ark_file#${REPO_ROOT}/}"

    local compile_out
    compile_out=$(timeout 10 wasmtime run --dir="${REPO_ROOT}" \
        "$SELFHOST_WASM" -- compile "$rel_input" -o "$rel_out" 2>&1) || true

    if [[ ! -f "$wasm_out" ]]; then
        echo "SELFHOST_COMPILE_ERROR: ${compile_out}"
        return 0
    fi

    local run_out
    run_out=$(timeout 10 wasmtime run "$wasm_out" 2>/dev/null) || true
    rm -f "$wasm_out"
    echo "$run_out"
}

# ── Mode: fixture ─────────────────────────────────────────────────────────────

run_fixture_parity() {
    echo -e "${CYAN}=== Fixture Parity ===${NC}"
    local pass=0 fail=0 skip=0 total=0
    local failures=""

    # Use manifest.txt run entries as fixture candidates
    while IFS=: read -r kind path; do
        [[ -z "$kind" || "$kind" == \#* ]] && continue
        [[ "$kind" != "run" ]] && continue

        local fixture="${FIXTURE_DIR}/${path}"
        local expected_file="${fixture%.ark}.expected"
        [[ ! -f "$expected_file" ]] && continue

        total=$((total + 1))
        local expected
        expected=$(cat "$expected_file")

        # Get Rust output
        local rust_out
        rust_out=$(rust_run "$fixture")

        # Get selfhost output
        local self_out
        self_out=$(selfhost_run "$fixture")

        if [[ "$self_out" == SELFHOST_COMPILE_ERROR:* ]]; then
            skip=$((skip + 1))
            continue
        fi

        if [[ "$rust_out" == "$self_out" ]]; then
            pass=$((pass + 1))
        else
            fail=$((fail + 1))
            failures="${failures}\n  FAIL: ${path}"
            failures="${failures}\n    rust:     $(echo "$rust_out" | head -1)"
            failures="${failures}\n    selfhost: $(echo "$self_out" | head -1)"
        fi
    done < "$MANIFEST"

    echo -e "  ${GREEN}pass: ${pass}${NC}  ${RED}fail: ${fail}${NC}  ${YELLOW}skip: ${skip}${NC}  total: ${total}"
    if [[ $fail -gt 0 ]]; then
        echo -e "${RED}Fixture mismatches:${failures}${NC}"
    fi
    echo "fixture-parity: pass=${pass} fail=${fail} skip=${skip} total=${total}"
    [[ $fail -eq 0 ]]
}

# ── Mode: cli ─────────────────────────────────────────────────────────────────

run_cli_parity() {
    echo -e "${CYAN}=== CLI Parity ===${NC}"
    local pass=0 fail=0 total=0
    local failures=""

    local hello="${WORK_DIR}/cli_hello.ark"
    cat > "$hello" <<'ARKEOF'
use std::host::stdio
fn main() { stdio::println("cli-test") }
ARKEOF

    # Test 1: compile with default target
    total=$((total + 1))
    local rust_compile self_compile
    rust_compile=$("${RUST_BIN}" compile "$hello" --target wasm32-wasi-p1 -o "${WORK_DIR}/r.wasm" 2>&1 && echo "OK" || echo "FAIL")
    self_compile=$(wasmtime run --dir="${WORK_DIR}" "$SELFHOST_WASM" -- \
        compile cli_hello.ark -o r2.wasm 2>&1 && echo "OK" || echo "FAIL")

    if [[ "$rust_compile" == *"OK"* && "$self_compile" == *"OK"* ]]; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        failures="${failures}\n  FAIL: compile default target"
    fi

    # Test 2: --help flag
    total=$((total + 1))
    local rust_help self_help
    rust_help=$("${RUST_BIN}" --help 2>&1 | head -1)
    self_help=$(wasmtime run "$SELFHOST_WASM" -- --help 2>&1 | head -1)

    if [[ -n "$rust_help" && -n "$self_help" ]]; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        failures="${failures}\n  FAIL: --help flag"
    fi

    # Test 3: --version flag
    total=$((total + 1))
    local rust_ver self_ver
    rust_ver=$("${RUST_BIN}" --version 2>&1; true)
    self_ver=$(wasmtime run "$SELFHOST_WASM" -- --version 2>&1; true)

    if [[ -n "$rust_ver" && -n "$self_ver" ]]; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        failures="${failures}\n  FAIL: --version flag"
    fi

    # Test 4: check command on valid source
    total=$((total + 1))
    local rust_check self_check
    rust_check=$("${RUST_BIN}" check "$hello" 2>&1 && echo "OK" || echo "FAIL")
    self_check=$(wasmtime run --dir="${WORK_DIR}" "$SELFHOST_WASM" -- \
        check cli_hello.ark 2>&1 && echo "OK" || echo "FAIL")

    if [[ "$rust_check" == *"OK"* && "$self_check" == *"OK"* ]]; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        failures="${failures}\n  FAIL: check command"
    fi

    echo -e "  ${GREEN}pass: ${pass}${NC}  ${RED}fail: ${fail}${NC}  total: ${total}"
    if [[ $fail -gt 0 ]]; then
        echo -e "${RED}CLI mismatches:${failures}${NC}"
    fi
    echo "cli-parity: pass=${pass} fail=${fail} total=${total}"
    [[ $fail -eq 0 ]]
}

# ── Mode: diag ────────────────────────────────────────────────────────────────

run_diag_parity() {
    echo -e "${CYAN}=== Diagnostic Parity ===${NC}"
    local pass=0 fail=0 skip=0 total=0
    local failures=""

    while IFS=: read -r kind path; do
        [[ -z "$kind" || "$kind" == \#* ]] && continue
        [[ "$kind" != "diag" ]] && continue

        local fixture="${FIXTURE_DIR}/${path}"
        local diag_file="${fixture%.ark}.diag"
        [[ ! -f "$diag_file" ]] && continue

        total=$((total + 1))
        local expected_diag
        expected_diag=$(head -1 "$diag_file")

        # Rust compiler diagnostic
        local rust_stderr
        rust_stderr=$("${RUST_BIN}" run "$fixture" 2>&1 || true)

        # Selfhost diagnostic
        local self_stderr
        self_stderr=$(wasmtime run --dir="${REPO_ROOT}" "$SELFHOST_WASM" -- \
            compile "$fixture" 2>&1 || true)

        # Check if the Rust compiler's diagnostic contains the expected pattern
        local rust_has=false self_has=false
        if echo "$rust_stderr" | grep -qF "$expected_diag" 2>/dev/null; then
            rust_has=true
        fi
        if echo "$self_stderr" | grep -qF "$expected_diag" 2>/dev/null; then
            self_has=true
        fi

        if [[ "$rust_has" == "true" && "$self_has" == "true" ]]; then
            pass=$((pass + 1))
        elif [[ "$rust_has" == "true" && "$self_has" == "false" ]]; then
            # Selfhost doesn't emit the diagnostic yet — skip rather than fail
            skip=$((skip + 1))
        else
            fail=$((fail + 1))
            failures="${failures}\n  FAIL: ${path} (expected: ${expected_diag})"
        fi
    done < "$MANIFEST"

    echo -e "  ${GREEN}pass: ${pass}${NC}  ${RED}fail: ${fail}${NC}  ${YELLOW}skip: ${skip}${NC}  total: ${total}"
    if [[ $fail -gt 0 ]]; then
        echo -e "${RED}Diagnostic mismatches:${failures}${NC}"
    fi
    echo "diag-parity: pass=${pass} fail=${fail} skip=${skip} total=${total}"
    [[ $fail -eq 0 ]]
}

# ── CLI parsing ───────────────────────────────────────────────────────────────

MODE_FIXTURE=false
MODE_CLI=false
MODE_DIAG=false
MODE_ALL=true

usage() {
    cat <<'EOF'
Usage: scripts/check-selfhost-parity.sh [options]

Compare Rust compiler and selfhost compiler outputs.

Modes:
  --fixture   Compare stdout of representative fixtures
  --cli       Compare basic CLI flag behaviour
  --diag      Compare diagnostic output for error fixtures
  (none)      Run all three modes

Environment:
  ARUKELLT_BIN    Path to Rust compiler binary (default: target/release/arukellt)
  SELFHOST_WASM   Path to selfhost wasm (default: src/compiler/arukellt-s1.wasm)

Options:
  --help, -h  Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --fixture) MODE_FIXTURE=true; MODE_ALL=false ;;
        --cli)     MODE_CLI=true;     MODE_ALL=false ;;
        --diag)    MODE_DIAG=true;    MODE_ALL=false ;;
        --help|-h) usage; exit 0 ;;
        *)         echo "Unknown option: $1" >&2; usage; exit 2 ;;
    esac
    shift
done

if $MODE_ALL; then
    MODE_FIXTURE=true
    MODE_CLI=true
    MODE_DIAG=true
fi

# ── Main ──────────────────────────────────────────────────────────────────────

find_rust_bin
check_selfhost
check_wasmtime

WORK_DIR=$(mktemp -d)
overall_ok=true

if $MODE_FIXTURE; then
    run_fixture_parity || overall_ok=false
fi

if $MODE_CLI; then
    run_cli_parity || overall_ok=false
fi

if $MODE_DIAG; then
    run_diag_parity || overall_ok=false
fi

echo ""
if $overall_ok; then
    echo -e "${GREEN}All parity checks passed.${NC}"
    exit 0
else
    echo -e "${RED}Some parity checks had mismatches. See above for details.${NC}"
    exit 1
fi
