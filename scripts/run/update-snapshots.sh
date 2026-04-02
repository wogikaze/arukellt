#!/usr/bin/env bash
# update-snapshots.sh — Regenerate MIR and diagnostics snapshots.
#
# Usage:
#   bash scripts/update-snapshots.sh          # update all snapshots
#   bash scripts/update-snapshots.sh --mir     # MIR only
#   bash scripts/update-snapshots.sh --diag    # diagnostics only
#
# After running, review the diff and commit the updated snapshots.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MIR_DIR="$ROOT/tests/snapshots/mir"
DIAG_DIR="$ROOT/tests/snapshots/diagnostics"

# ── Parse flags ───────────────────────────────────────────────────────────────
UPDATE_MIR=true
UPDATE_DIAG=true
for arg in "$@"; do
    case "$arg" in
        --mir)  UPDATE_DIAG=false ;;
        --diag) UPDATE_MIR=false ;;
        --help|-h)
            sed -n '2,/^$/s/^# //p' "$0"
            exit 0 ;;
    esac
done

# ── Resolve compiler binary ──────────────────────────────────────────────────
ARUKELLT_BIN="$ROOT/target/debug/arukellt"
if [ ! -x "$ARUKELLT_BIN" ]; then
    ARUKELLT_BIN="$ROOT/target/release/arukellt"
fi
if [ ! -x "$ARUKELLT_BIN" ]; then
    echo "Building arukellt (debug)..."
    cargo build -p arukellt --quiet
    ARUKELLT_BIN="$ROOT/target/debug/arukellt"
fi

# ── Fixture sets ─────────────────────────────────────────────────────────────
# MIR snapshots: representative fixtures covering basic programs, variables,
# and struct layout.  Keep this list small — snapshots are committed.
MIR_FIXTURES=(
    "hello/hello.ark"
    "variables/i32_lit.ark"
    "variables/mut_var.ark"
    "structs/basic_struct.ark"
    "structs/nested_struct.ark"
)

# Diagnostics snapshots: all .ark files under tests/fixtures/diagnostics/ that
# have a corresponding .diag expected-output file.
DIAG_FIXTURE_DIR="$ROOT/tests/fixtures/diagnostics"

# ── Helpers ──────────────────────────────────────────────────────────────────
slug() {
    # hello/hello.ark → hello__hello
    echo "$1" | sed 's|/|__|g; s|\.ark$||'
}

# ── Update MIR snapshots ────────────────────────────────────────────────────
if [ "$UPDATE_MIR" = true ]; then
    mkdir -p "$MIR_DIR"
    echo "Updating MIR snapshots → $MIR_DIR"
    updated=0
    for rel in "${MIR_FIXTURES[@]}"; do
        fixture="$ROOT/tests/fixtures/$rel"
        if [ ! -f "$fixture" ]; then
            echo "  WARN: fixture not found: $rel (skipped)"
            continue
        fi
        out="$MIR_DIR/$(slug "$rel").mir"
        if ARUKELLT_DUMP_PHASES="parse,resolve,mir,optimized-mir" \
           "$ARUKELLT_BIN" compile "$fixture" -o /dev/null 2>"$out" 1>/dev/null; then
            :  # success
        else
            # compilation may fail for diagnostic fixtures; keep stderr output
            :
        fi
        updated=$((updated + 1))
    done
    echo "  $updated MIR snapshot(s) written"
fi

# ── Update diagnostics snapshots ─────────────────────────────────────────────
if [ "$UPDATE_DIAG" = true ]; then
    mkdir -p "$DIAG_DIR"
    echo "Updating diagnostics snapshots → $DIAG_DIR"
    updated=0
    for ark in "$DIAG_FIXTURE_DIR"/*.ark; do
        [ -f "$ark" ] || continue
        base="$(basename "$ark" .ark)"
        diag_expected="$DIAG_FIXTURE_DIR/$base.diag"
        [ -f "$diag_expected" ] || continue

        out="$DIAG_DIR/$base.diag"
        ARUKELLT_DUMP_DIAGNOSTICS=1 \
            "$ARUKELLT_BIN" compile "$ark" -o /dev/null \
            >"$out" 2>&1 || true
        updated=$((updated + 1))
    done
    echo "  $updated diagnostics snapshot(s) written"
fi

echo "Done.  Review changes with: git diff tests/snapshots/"
