#!/usr/bin/env bash
# Repository structure hygiene checks.
# Checks for improper file placement in repository structure.

set -euo pipefail

ROOT="${1:-$(git rev-parse --show-toplevel)}"
ERRORS=0

echo "=== Repository Structure Checks ==="
echo ""

# ── Check 1: Root directory script files ───────────────────────────────────────
echo "Checking root directory for script files (.ark, .py, .sh)..."

ROOT_SCRIPT_ERRORS=0
for ark_file in "$ROOT"/*.ark; do
    if [[ -f "$ark_file" ]]; then
        echo "  ERROR: .ark file in root: $(basename "$ark_file")"
        echo "    Place in: tests/fixtures/, std/, src/, or benchmarks/"
        ((ROOT_SCRIPT_ERRORS++)) || true
    fi
done

for py_file in "$ROOT"/*.py; do
    if [[ -f "$py_file" ]]; then
        echo "  ERROR: .py file in root: $(basename "$py_file")"
        echo "    Place in: scripts/"
        ((ROOT_SCRIPT_ERRORS++)) || true
    fi
done

for sh_file in "$ROOT"/*.sh; do
    if [[ -f "$sh_file" ]]; then
        echo "  ERROR: .sh file in root: $(basename "$sh_file")"
        echo "    Place in: scripts/"
        ((ROOT_SCRIPT_ERRORS++)) || true
    fi
done

if [[ $ROOT_SCRIPT_ERRORS -eq 0 ]]; then
    echo "  ✓ No script files in repository root"
else
    echo "  Found $ROOT_SCRIPT_ERRORS script file(s) in root"
    ((ERRORS += ROOT_SCRIPT_ERRORS)) || true
fi
echo ""

# ── Check 2: Scripts root directory structure ───────────────────────────────
echo "Checking scripts/ root directory structure..."

SCRIPTS_DIR="$ROOT/scripts"
if [[ -d "$SCRIPTS_DIR" ]]; then
    ALLOWED_FILES=("manager.py" "README.md" ".generated-files")
    SCRIPTS_ROOT_ERRORS=0

    for file in "$SCRIPTS_DIR"/*; do
        if [[ -f "$file" ]]; then
            filename=$(basename "$file")
            if [[ ! " ${ALLOWED_FILES[@]} " =~ " ${filename} " ]]; then
                echo "  ERROR: Unexpected file in scripts/ root: $filename"
                echo "    Place utility scripts in scripts/util/"
                echo "    Place benchmark scripts in scripts/perf/"
                ((SCRIPTS_ROOT_ERRORS++)) || true
            fi
        fi
    done

    if [[ $SCRIPTS_ROOT_ERRORS -eq 0 ]]; then
        echo "  ✓ Scripts root directory structure OK"
    else
        echo "  Found $SCRIPTS_ROOT_ERRORS unexpected file(s) in scripts/ root"
        echo "  Allowed files: ${ALLOWED_FILES[*]}"
        ((ERRORS += SCRIPTS_ROOT_ERRORS)) || true
    fi
else
    echo "  WARNING: scripts/ directory not found"
fi
echo ""

# ── Summary ───────────────────────────────────────────────────────────────────
echo "=== Summary ==="
if [[ $ERRORS -eq 0 ]]; then
    echo "✓ All repository structure checks passed"
    exit 0
else
    echo "✗ Found $ERRORS issue(s) in repository structure"
    exit 1
fi