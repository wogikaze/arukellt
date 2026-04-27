#!/usr/bin/env bash
# Check that no .ark files exist in the repository root directory.
# Ark files should be organized in proper subdirectories (tests/fixtures/, std/, src/, etc.)

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

if [[ $ERRORS -gt 0 ]]; then
    echo ""
    echo "Found $ERRORS .ark file(s) in repository root. Please move them to appropriate subdirectories."
    exit 1
else
    echo "✓ No .ark files in repository root"
    exit 0
fi