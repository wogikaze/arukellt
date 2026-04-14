#!/usr/bin/env bash
# scripts/check/check-playground-size.sh — Playground Wasm and JS bundle size gates
#
# Usage:
#   check-playground-size.sh [--wasm <file>] [--wasm-limit <bytes>]
#                            [--bundle-dir <dir>] [--bundle-limit <bytes>]
#
# Environment overrides (lower precedence than CLI flags):
#   PLAYGROUND_WASM_LIMIT   bytes; default 307200 (300 KB)
#   PLAYGROUND_BUNDLE_LIMIT bytes; default 524288 (512 KB)
#
# Exits 0 when all checked sizes are within budget.
# Exits 1 when any checked size exceeds budget.
# Prints a skip notice (exit 0) when a checked path does not exist.
#
# Thresholds documented in docs/playground/deployment-strategy.md §4.3.
# To adjust a threshold: update the environment variable in the CI workflow
# (.github/workflows/playground-ci.yml) and document the rationale in a PR.

set -euo pipefail

# ─── Defaults ─────────────────────────────────────────────────────────────────
WASM_FILE=""
WASM_LIMIT="${PLAYGROUND_WASM_LIMIT:-307200}"     # 300 KB
BUNDLE_DIR=""
BUNDLE_LIMIT="${PLAYGROUND_BUNDLE_LIMIT:-524288}" # 512 KB

FAIL=0

# ─── Argument parsing ─────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --wasm)        WASM_FILE="$2";  shift 2 ;;
        --wasm-limit)  WASM_LIMIT="$2"; shift 2 ;;
        --bundle-dir)  BUNDLE_DIR="$2"; shift 2 ;;
        --bundle-limit) BUNDLE_LIMIT="$2"; shift 2 ;;
        --help|-h)
            sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            echo "Run with --help for usage." >&2
            exit 1
            ;;
    esac
done

# ─── Helpers ──────────────────────────────────────────────────────────────────
to_kb() { echo "$(( ($1 + 1023) / 1024 )) KB"; }

# ─── Check Wasm binary ────────────────────────────────────────────────────────
if [[ -n "$WASM_FILE" ]]; then
    if [[ ! -f "$WASM_FILE" ]]; then
        echo "[playground-size] SKIP: wasm file not found: $WASM_FILE"
        echo "[playground-size]   Run 'npm run build:wasm' in playground/ to build the wasm binary."
    else
        SIZE=$(stat -c%s "$WASM_FILE" 2>/dev/null || stat -f%z "$WASM_FILE")
        echo "[playground-size] Wasm binary: $(to_kb "$SIZE") / budget $(to_kb "$WASM_LIMIT")"
        if [[ "$SIZE" -le "$WASM_LIMIT" ]]; then
            echo "[playground-size] PASS: wasm binary within budget"
        else
            echo "[playground-size] FAIL: wasm binary $(to_kb "$SIZE") exceeds budget $(to_kb "$WASM_LIMIT")" >&2
            echo "[playground-size]   To raise the budget: update PLAYGROUND_WASM_LIMIT in" >&2
            echo "[playground-size]   .github/workflows/playground-ci.yml and document the reason." >&2
            FAIL=1
        fi
    fi
fi

# ─── Check JS bundle directory ────────────────────────────────────────────────
if [[ -n "$BUNDLE_DIR" ]]; then
    if [[ ! -d "$BUNDLE_DIR" ]]; then
        echo "[playground-size] SKIP: bundle dir not found: $BUNDLE_DIR"
        echo "[playground-size]   Run 'npm run build:app' in playground/ to build the JS bundle."
    else
        TOTAL=0
        # Sum all .js files at the top level of the bundle dir (excludes tests/ subdir)
        while IFS= read -r f; do
            S=$(stat -c%s "$f" 2>/dev/null || stat -f%z "$f")
            TOTAL=$((TOTAL + S))
        done < <(find "$BUNDLE_DIR" -maxdepth 1 -name "*.js")
        echo "[playground-size] JS bundle total: $(to_kb "$TOTAL") / budget $(to_kb "$BUNDLE_LIMIT")"
        if [[ "$TOTAL" -le "$BUNDLE_LIMIT" ]]; then
            echo "[playground-size] PASS: JS bundle within budget"
        else
            echo "[playground-size] FAIL: JS bundle $(to_kb "$TOTAL") exceeds budget $(to_kb "$BUNDLE_LIMIT")" >&2
            echo "[playground-size]   To raise the budget: update PLAYGROUND_BUNDLE_LIMIT in" >&2
            echo "[playground-size]   .github/workflows/playground-ci.yml and document the reason." >&2
            FAIL=1
        fi
    fi
fi

exit "$FAIL"
