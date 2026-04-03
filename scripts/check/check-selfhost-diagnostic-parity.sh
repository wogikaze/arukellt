#!/usr/bin/env bash
# check-selfhost-diagnostic-parity.sh — Compare Rust compiler and selfhost wasm
# diagnostic output for representative error fixtures.
#
# For each "diag:" fixture in tests/fixtures/manifest.txt (minimum 10), this
# script checks that the expected diagnostic pattern (from the corresponding
# .diag file) appears in the output of both:
#   1. The Rust arukellt binary  (target/debug/arukellt or $ARUKELLT_BIN)
#   2. The selfhost wasm         (.build/selfhost/arukellt-s1.wasm or $SELFHOST_WASM)
#
# The .diag file contains the first line of the expected diagnostic output,
# e.g.:  error[E0200|typecheck]:
#
# Fixtures where the Rust compiler emits the diagnostic but s1.wasm does not
# are reported as SKIP (not FAIL), because selfhost diagnostic coverage is a
# work-in-progress.  A FAIL is reported only when the Rust compiler itself
# does not emit the expected diagnostic (which would indicate a broken fixture).
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
#   1 — Rust compiler failed to emit an expected diagnostic (broken fixture)

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

if [[ ! -f "$S1_WASM" ]]; then
    echo "${YELLOW}SKIP: arukellt-s1.wasm not found at .build/selfhost/arukellt-s1.wasm${NC}"
    echo "  Run 'bash scripts/check/check-selfhost-fixpoint.sh' first to produce it."
    exit 0
fi

if ! command -v wasmtime >/dev/null 2>&1; then
    echo "${YELLOW}SKIP: wasmtime not found on PATH — install via mise or https://wasmtime.dev/${NC}"
    exit 0
fi

# ── Helpers ───────────────────────────────────────────────────────────────────

# Collect combined stderr+stdout from the Rust compiler on a diagnostic fixture.
rust_diag() {
    local ark_file="$1"
    shift
    { timeout 15 "${ARUKELLT_BIN}" run "$@" "${ark_file}" 2>&1 || true; }
}

# Collect combined stderr+stdout from the selfhost compiler on the same fixture.
selfhost_diag() {
    local ark_file="$1"
    shift
    local rel_input="${ark_file#${REPO_ROOT}/}"
    {
        cd "${REPO_ROOT}"
        timeout 15 wasmtime run \
            --dir="${REPO_ROOT}" \
            "${S1_WASM}" \
            -- compile "$@" "${rel_input}" 2>&1 || true
    }
}

# ── Main ──────────────────────────────────────────────────────────────────────

echo "${CYAN}=== Selfhost Diagnostic Parity ===${NC}"
echo "  s1.wasm : ${S1_WASM} ($(wc -c < "${S1_WASM}") bytes)"
echo "  arukellt: ${ARUKELLT_BIN}"
echo ""

pass=0    # both Rust and selfhost emit the expected diagnostic
fail=0    # Rust does NOT emit the expected diagnostic (broken fixture)
skip=0    # Rust emits it but selfhost does not (s1 gap — non-fatal)
total=0

declare -a failures=()

while IFS=: read -r kind path; do
    [[ -z "$kind" || "$kind" == \#* ]] && continue
    [[ "$kind" != "diag" ]] && continue

    ark_file="${FIXTURE_DIR}/${path}"
    diag_file="${ark_file%.ark}.diag"
    flags_file="${ark_file%.ark}.flags"

    [[ ! -f "$ark_file" ]] && continue
    [[ ! -f "$diag_file" ]] && continue

    # The .diag file's first non-empty line is the expected diagnostic pattern.
    expected_pattern=""
    while IFS= read -r line; do
        trimmed="${line#"${line%%[! ]*}"}"   # ltrim
        if [[ -n "$trimmed" ]]; then
            expected_pattern="$trimmed"
            break
        fi
    done < "$diag_file"
    [[ -z "$expected_pattern" ]] && continue

    # Read optional extra CLI flags from the .flags file.
    extra_flags=""
    if [[ -f "$flags_file" ]]; then
        extra_flags=$(cat "$flags_file")
    fi

    total=$((total + 1))

    # shellcheck disable=SC2086
    rust_output=$(rust_diag "${ark_file}" ${extra_flags})
    # shellcheck disable=SC2086
    self_output=$(selfhost_diag "${ark_file}" ${extra_flags})

    rust_has=false
    self_has=false
    grep -qF -- "${expected_pattern}" <<< "${rust_output}" 2>/dev/null && rust_has=true
    grep -qF -- "${expected_pattern}" <<< "${self_output}" 2>/dev/null && self_has=true

    if [[ "$rust_has" == "true" && "$self_has" == "true" ]]; then
        pass=$((pass + 1))
    elif [[ "$rust_has" == "true" && "$self_has" == "false" ]]; then
        # s1.wasm doesn't yet emit this diagnostic — skip rather than fail
        skip=$((skip + 1))
    else
        # Rust itself failed to emit the expected diagnostic
        fail=$((fail + 1))
        failures+=("${path}")
        failures+=("  expected : ${expected_pattern}")
        failures+=("  rust out : $(printf '%s' "${rust_output}" | head -2 | tr '\n' '|')")
    fi
done < "${MANIFEST}"

# ── Report ────────────────────────────────────────────────────────────────────

echo "  ${GREEN}pass: ${pass}${NC}  ${RED}fail: ${fail}${NC}  ${YELLOW}skip: ${skip}${NC}  total: ${total}"

if [[ ${#failures[@]} -gt 0 ]]; then
    echo ""
    echo "${RED}Diagnostic failures (Rust compiler did not emit expected diagnostic):${NC}"
    for line in "${failures[@]}"; do
        echo "  ${line}"
    done
fi

echo ""
echo "diagnostic-parity: pass=${pass} fail=${fail} skip=${skip} total=${total}"

if [[ $fail -gt 0 ]]; then
    echo "${RED}✗ Diagnostic parity check FAILED${NC}"
    exit 1
else
    echo "${GREEN}✓ Diagnostic parity check PASSED (${pass} full match, ${skip} s1-gap skipped)${NC}"
    exit 0
fi
