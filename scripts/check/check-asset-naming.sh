#!/usr/bin/env bash
# Asset naming linter: enforces snake_case for test fixtures and benchmarks.
# Runs as part of pre-commit / CI to prevent naming drift.
set -euo pipefail

ROOT="${1:-$(git rev-parse --show-toplevel)}"
ERRORS=0

check_snake_case() {
    local path="$1"
    local base
    base=$(basename "$path")
    # Remove known extensions for the check
    local name="${base%%.*}"
    # snake_case: lowercase letters, digits, underscores only
    if [[ ! "$name" =~ ^[a-z][a-z0-9_]*$ ]]; then
        echo "  VIOLATION: $path"
        echo "    Expected snake_case, got: $name"
        echo "    Suggestion: $(echo "$name" | sed -E 's/-/_/g; s/([a-z])([A-Z])/\1_\2/g' | tr '[:upper:]' '[:lower:]')"
        ((ERRORS++)) || true
    fi
}

echo "Checking asset naming conventions (snake_case)..."

# Check benchmark .ark files
while IFS= read -r -d '' file; do
    check_snake_case "$file"
done < <(find "$ROOT/benchmarks" -maxdepth 1 -name '*.ark' -print0 2>/dev/null)

# Check benchmark support files (.wasm, .expected, .input.txt)
while IFS= read -r -d '' file; do
    base=$(basename "$file")
    name="${base%%.*}"
    if [[ "$base" == *.bench.wasm ]]; then
        name="${base%.bench.wasm}"
    fi
    if [[ ! "$name" =~ ^[a-z][a-z0-9_]*$ ]]; then
        echo "  VIOLATION: $file"
        echo "    Expected snake_case, got: $name"
        ((ERRORS++)) || true
    fi
done < <(find "$ROOT/benchmarks" -maxdepth 1 \( -name '*.wasm' -o -name '*.expected' -o -name '*.input.txt' \) -print0 2>/dev/null)

# Check fixture directories
while IFS= read -r -d '' dir; do
    check_snake_case "$dir"
done < <(find "$ROOT/tests/fixtures" -mindepth 1 -maxdepth 1 -type d -print0 2>/dev/null)

# Check fixture .ark files
while IFS= read -r -d '' file; do
    check_snake_case "$file"
done < <(find "$ROOT/tests/fixtures" -name '*.ark' -print0 2>/dev/null)

# Check top-level fixture files (non-directory)
while IFS= read -r -d '' file; do
    base=$(basename "$file")
    # Skip manifest.txt — it's a known infra file
    [[ "$base" == "manifest.txt" ]] && continue
    check_snake_case "$file"
done < <(find "$ROOT/tests/fixtures" -maxdepth 1 -type f -name '*.ark' -print0 2>/dev/null)

if [[ $ERRORS -gt 0 ]]; then
    echo ""
    echo "Found $ERRORS naming violation(s)."
    echo "Convention: all test fixtures and benchmark assets use snake_case."
    echo "  - Lowercase letters, digits, underscores only"
    echo "  - Must start with a lowercase letter"
    echo "  - No hyphens, no camelCase, no PascalCase"
    exit 1
else
    echo "✓ All asset names follow snake_case convention."
fi
