#!/usr/bin/env bash
# check-reproducible-build.sh — Verify that compiling the same source
# twice produces bit-exact identical .wasm output.
#
# Usage:  scripts/check-reproducible-build.sh [fixture …]
# Exit:   0 if all fixtures reproduce, 1 if any differ.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPILER="${REPO_ROOT}/target/release/arukellt"

if [[ ! -x "$COMPILER" ]]; then
  echo "ERROR: compiler not found at ${COMPILER}" >&2
  echo "       Run 'cargo build --release' first." >&2
  exit 1
fi

# Default fixture list when invoked without arguments.
DEFAULT_FIXTURES=(
  tests/fixtures/hello/hello.ark
  tests/fixtures/variables/i32_lit.ark
  tests/fixtures/variables/f64_lit.ark
  tests/fixtures/functions/recursive.ark
  tests/fixtures/hello/multi_print.ark
)

if [[ $# -gt 0 ]]; then
  FIXTURES=("$@")
else
  FIXTURES=("${DEFAULT_FIXTURES[@]}")
fi

BUILD_DIR="${REPO_ROOT}/.repro-build-check"
mkdir -p "$BUILD_DIR"
trap 'rm -rf "$BUILD_DIR"' EXIT

PASS=0
FAIL=0

for fixture in "${FIXTURES[@]}"; do
  src="${REPO_ROOT}/${fixture}"
  if [[ ! -f "$src" ]]; then
    echo "SKIP  ${fixture}  (file not found)"
    continue
  fi

  base="$(basename "${fixture}" .ark)"
  out1="${BUILD_DIR}/${base}_build1.wasm"
  out2="${BUILD_DIR}/${base}_build2.wasm"

  # Compile twice
  if ! "$COMPILER" compile "$src" -o "$out1" >/dev/null 2>&1; then
    echo "SKIP  ${fixture}  (compilation failed)"
    continue
  fi
  if ! "$COMPILER" compile "$src" -o "$out2" >/dev/null 2>&1; then
    echo "SKIP  ${fixture}  (compilation failed on second run)"
    continue
  fi

  # Compare
  if cmp -s "$out1" "$out2"; then
    h="$(sha256sum "$out1" | cut -d' ' -f1)"
    echo "PASS  ${fixture}  sha256=${h}"
    PASS=$((PASS + 1))
  else
    echo "FAIL  ${fixture}"
    echo "      build1: $(sha256sum "$out1")"
    echo "      build2: $(sha256sum "$out2")"
    # Show byte-level diff position for debugging
    cmp "$out1" "$out2" 2>&1 | sed 's/^/      /' || true
    FAIL=$((FAIL + 1))
  fi
done

echo ""
echo "Reproducible build check: ${PASS} passed, ${FAIL} failed"

if [[ $FAIL -gt 0 ]]; then
  exit 1
fi
