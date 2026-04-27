#!/usr/bin/env bash
# Check that no .ark, .py, or .sh files exist in the repository root directory.
# Script files should be organized in proper subdirectories.

set -euo pipefail

ROOT="${1:-$(git rev-parse --show-toplevel)}"
ERRORS=0

# Check for .ark files in root directory (excluding subdirectories)
for ark_file in "$ROOT"/*.ark; do
    if [[ -f "$ark_file" ]]; then
        echo "ERROR: .ark file found in repository root: $(basename "$ark_file")"
        echo "  Ark files should be placed in appropriate subdirectories:"
        echo "  - tests/fixtures/ for test fixtures"
        echo "  - std/ for standard library sources"
        echo "  - src/ for main source files"
        echo "  - benchmarks/ for benchmark sources"
        ((ERRORS++)) || true
    fi
done

# Check for .py files in root directory (excluding subdirectories)
for py_file in "$ROOT"/*.py; do
    if [[ -f "$py_file" ]]; then
        echo "ERROR: .py file found in repository root: $(basename "$py_file")"
        echo "  Python scripts should be placed in scripts/"
        ((ERRORS++)) || true
    fi
done

# Check for .sh files in root directory (excluding subdirectories)
for sh_file in "$ROOT"/*.sh; do
    if [[ -f "$sh_file" ]]; then
        echo "ERROR: .sh file found in repository root: $(basename "$sh_file")"
        echo "  Shell scripts should be placed in scripts/"
        ((ERRORS++)) || true
    fi
done

if [[ $ERRORS -gt 0 ]]; then
    echo ""
    echo "Found $ERRORS script file(s) in repository root. Please move them to appropriate subdirectories."
    exit 1
else
    echo "✓ No script files (.ark, .py, .sh) in repository root"
    exit 0
fi