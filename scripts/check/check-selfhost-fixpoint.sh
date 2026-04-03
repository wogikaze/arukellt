#!/usr/bin/env bash
# check-selfhost-fixpoint.sh — Verify that the selfhost bootstrap fixpoint is reached.
#
# A selfhost fixpoint means:
#   sha256(arukellt-s1.wasm) == sha256(arukellt-s2.wasm)
#
# where:
#   s1.wasm = Rust compiler compiles src/compiler/main.ark
#   s2.wasm = s1.wasm (selfhost stage-1) compiles src/compiler/main.ark
#
# If the hashes match, the selfhost compiler can reproduce itself bit-for-bit.
#
# Root cause of current fixpoint failure (issue #459):
#   The selfhost compiler (s1.wasm) does NOT implement multi-file module loading.
#   When compiling src/compiler/main.ark, it only processes the entry file without
#   following `use module_name` imports to recursively load driver.ark, lexer.ark,
#   parser.ark, resolver.ark, typechecker.ark, mir.ark, and emitter.ark.
#
#   As a result, s2.wasm contains only 24 functions (the CLI constants and argument
#   parser from main.ark directly) vs s1.wasm's 556 functions (the full compiler).
#   All cross-module function calls (e.g., driver::compile_file, lexer::tokenize)
#   are treated as "Unknown function (cross-module or unresolved)" in emitter.ark
#   and replaced with `i32.const 0` stubs.
#
# Blockers for fixpoint (STOP_IF triggered — not fixed in this slice):
#   1. driver.ark needs multi-file module loading:
#        Implement recursive source loading for `use <local_module>` imports,
#        topologically sort and concatenate all module sources before compilation.
#   2. emitter.ark needs qualified call resolution:
#        When lowering a MIR CALL with str_val="lexer::tokenize", strip the module
#        prefix and resolve to the bare function name "tokenize" (matching the
#        Rust MIR lowerer behavior in crates/ark-mir/src/lower/expr.rs:283).
#   3. mir.ark needs consistent qualified name handling to match Rust MIR output.
#
#   These span more than one self-contained feature in src/compiler/*.ark, so
#   fixpoint is not achievable in the issue-459 slice.
#
# Usage:
#   bash scripts/check/check-selfhost-fixpoint.sh           # build+compare
#   bash scripts/check/check-selfhost-fixpoint.sh --no-build  # skip rebuild, compare cached
#   bash scripts/check/check-selfhost-fixpoint.sh --help
#
# Exit codes:
#   0 — fixpoint reached (sha256 of s1 == sha256 of s2)
#   1 — fixpoint NOT reached (hash mismatch, build failure, or prerequisites missing)
#   2 — prerequisites missing (arukellt binary or wasmtime not found)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUILD_DIR="${REPO_ROOT}/.build/selfhost"
S1_WASM="${BUILD_DIR}/arukellt-s1.wasm"
S2_WASM="${BUILD_DIR}/arukellt-s2.wasm"

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
Builds s1.wasm (Rust → Ark), then s2.wasm (s1 → Ark), then compares sha256.

Options:
  --no-build   Skip rebuilding; compare existing .build/selfhost/s1 and s2
  --help, -h   Show this help

Exit: 0 if sha256(s1) == sha256(s2), 1 otherwise, 2 if prerequisites missing.
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

# ── Compare sha256 hashes ────────────────────────────────────────────────────
echo ""
echo "${CYAN}[fixpoint] Comparing sha256 hashes ...${NC}"
S1_HASH=$(sha256sum "$S1_WASM" | awk '{print $1}')
S2_HASH=$(sha256sum "$S2_WASM" | awk '{print $1}')

echo "  s1.wasm: ${S1_HASH}  ($(wc -c < "$S1_WASM") bytes)"
echo "  s2.wasm: ${S2_HASH}  ($(wc -c < "$S2_WASM") bytes)"

if [[ "$S1_HASH" = "$S2_HASH" ]]; then
    echo ""
    echo "${GREEN}✓ Selfhost fixpoint reached: sha256(s1) == sha256(s2)${NC}"
    exit 0
else
    echo ""
    echo "${RED}✗ Selfhost fixpoint NOT reached: sha256(s1) ≠ sha256(s2)${NC}"
    echo ""
    echo "${YELLOW}Root cause (issue #459 — not yet fixed):${NC}"
    echo "  The selfhost compiler (s1.wasm) does not implement multi-file module"
    echo "  loading.  When compiling src/compiler/main.ark it only processes the"
    echo "  entry file, ignoring 'use driver', 'use lexer', 'use parser', etc."
    echo "  All cross-module function calls are silently replaced with i32.const 0"
    echo "  stubs by emitter.ark (line ~8475: 'Unknown function — drop the args')."
    echo ""
    echo "  Blockers (must be fixed before fixpoint is achievable):"
    echo "    1. driver.ark: implement recursive multi-file module loading"
    echo "       (load driver.ark, lexer.ark, parser.ark, resolver.ark,"
    echo "        typechecker.ark, mir.ark, emitter.ark from WASI filesystem)"
    echo "    2. emitter.ark: strip module qualifiers in call resolution"
    echo "       (resolve 'lexer::tokenize' → 'tokenize', matching Rust"
    echo "        MIR lowerer behavior in crates/ark-mir/src/lower/expr.rs:283)"
    echo "    3. mir.ark: consistent qualified name lowering for cross-module calls"
    echo ""
    echo "  Size delta: s1=$(wc -c < "$S1_WASM") bytes / s2=$(wc -c < "$S2_WASM") bytes"
    echo "  s1 functions: 556 (full compiler)  s2 functions: 24 (CLI stubs only)"
    exit 1
fi
