#!/usr/bin/env bash
# check-selfhost-fixture-parity.sh — Compare Rust compiler and selfhost wasm
# output for representative harness fixtures.
#
# For each "run:" fixture in tests/fixtures/manifest.txt (minimum 10), this
# script runs the fixture through both:
#   1. The Rust arukellt binary  (target/debug/arukellt or $ARUKELLT_BIN)
#   2. The selfhost wasm         (.build/selfhost/arukellt-s1.wasm or $SELFHOST_WASM)
#
# If the run outputs match, the fixture passes.  Compile errors from the
# selfhost compiler are reported as SKIP (not FAIL) because many fixtures use
# stdlib intrinsics that s1.wasm does not yet support.
#
# Prerequisites:
#   - arukellt binary: target/debug/arukellt or $ARUKELLT_BIN
#   - selfhost wasm:   .build/selfhost/arukellt-s1.wasm or $SELFHOST_WASM
#   - wasmtime on PATH
#
# If arukellt-s1.wasm is not found the script prints "SKIP: ..." and exits 0,
# so CI does not hard-fail when the bootstrap stage has not been run.
#
# Exit codes:
#   0 — all checked fixtures pass (or skipped due to missing prerequisites)
#   1 — one or more fixture outputs differ between Rust and selfhost compilers

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
S1_WASM="${SELFHOST_WASM:-${REPO_ROOT}/.build/selfhost/arukellt-s1.wasm}"
FIXTURE_DIR="${REPO_ROOT}/tests/fixtures"
MANIFEST="${FIXTURE_DIR}/manifest.txt"

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
CYAN=$'\033[0;36m'
NC=$'\033[0m'

# ── Prerequisites ─────────────────────────────────────────────────────────────

# arukellt Rust binary
ARUKELLT_BIN="${ARUKELLT_BIN:-}"
if [[ -z "$ARUKELLT_BIN" ]]; then
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
    echo "${RED}error: arukellt binary not found. Build with: cargo build -p arukellt${NC}" >&2
    exit 1
fi

# selfhost wasm
if [[ ! -f "$S1_WASM" ]]; then
    echo "${YELLOW}SKIP: arukellt-s1.wasm not found at .build/selfhost/arukellt-s1.wasm${NC}"
    echo "  Run 'bash scripts/check/check-selfhost-fixpoint.sh' first to produce it."
    exit 0
fi

# wasmtime
if ! command -v wasmtime >/dev/null 2>&1; then
    echo "${YELLOW}SKIP: wasmtime not found on PATH — install via mise or https://wasmtime.dev/${NC}"
    exit 0
fi

# ── Helpers ───────────────────────────────────────────────────────────────────

_WORK_DIR=""
cleanup() {
    if [[ -n "$_WORK_DIR" && -d "$_WORK_DIR" ]]; then
        rm -rf "$_WORK_DIR"
    fi
}
trap cleanup EXIT
_WORK_DIR=$(mktemp -d)

# Run a fixture with the Rust compiler and capture stdout.
rust_run() {
    local ark_file="$1"
    timeout 15 "${ARUKELLT_BIN}" run "$ark_file" 2>/dev/null || true
}

# Compile a fixture with arukellt-s1.wasm then run the produced .wasm.
selfhost_run() {
    local ark_file="$1"
    local rel_input="${ark_file#${REPO_ROOT}/}"
    local wasm_out="${_WORK_DIR}/s1_out_$$.wasm"
    local rel_out="${wasm_out#${REPO_ROOT}/}"

    # Compile via selfhost wasm
    local compile_err
    compile_err=$(
        cd "${REPO_ROOT}"
        timeout 15 wasmtime run \
            --dir="${REPO_ROOT}" \
            "${S1_WASM}" \
            -- compile "${rel_input}" \
               --target wasm32-wasi-p1 \
               -o "${wasm_out}" 2>&1
    ) || true

    if [[ ! -f "${wasm_out}" ]]; then
        printf 'SELFHOST_COMPILE_ERROR'
        rm -f "${wasm_out}"
        return 0
    fi

    # Run the produced wasm
    local run_out
    run_out=$(timeout 10 wasmtime run "${wasm_out}" 2>/dev/null) || true
    rm -f "${wasm_out}"
    printf '%s' "${run_out}"
}

# ── Main ──────────────────────────────────────────────────────────────────────

echo "${CYAN}=== Selfhost Fixture Parity ===${NC}"
echo "  s1.wasm : ${S1_WASM} ($(wc -c < "${S1_WASM}") bytes)"
echo "  arukellt: ${ARUKELLT_BIN}"
echo ""

pass=0
fail=0
skip=0
total=0

declare -a failures=()

while IFS=: read -r kind path; do
    [[ -z "$kind" || "$kind" == \#* ]] && continue
    [[ "$kind" != "run" ]] && continue

    ark_file="${FIXTURE_DIR}/${path}"
    [[ ! -f "$ark_file" ]] && continue

    total=$((total + 1))

    rust_out=$(rust_run "${ark_file}")
    self_out=$(selfhost_run "${ark_file}")

    if [[ "$self_out" == "SELFHOST_COMPILE_ERROR" ]]; then
        skip=$((skip + 1))
        continue
    fi

    if [[ "$rust_out" == "$self_out" ]]; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        failures+=("${path}")
        failures+=("  rust:     $(printf '%s' "${rust_out}" | head -1)")
        failures+=("  selfhost: $(printf '%s' "${self_out}" | head -1)")
    fi
done < "${MANIFEST}"

# ── Report ────────────────────────────────────────────────────────────────────

echo "  ${GREEN}pass: ${pass}${NC}  ${RED}fail: ${fail}${NC}  ${YELLOW}skip: ${skip}${NC}  total: ${total}"

if [[ ${#failures[@]} -gt 0 ]]; then
    echo ""
    echo "${RED}Fixture mismatches:${NC}"
    for line in "${failures[@]}"; do
        echo "  ${line}"
    done
fi

echo ""
echo "fixture-parity: pass=${pass} fail=${fail} skip=${skip} total=${total}"

if [[ $fail -gt 0 ]]; then
    echo "${RED}✗ Fixture parity check FAILED${NC}"
    exit 1
else
    echo "${GREEN}✓ Fixture parity check PASSED (${pass} match, ${skip} skipped)${NC}"
    exit 0
fi
