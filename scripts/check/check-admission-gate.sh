#!/usr/bin/env bash
# scripts/check-admission-gate.sh — Verify every stdlib API has fixture, docs, metadata
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MANIFEST="$REPO_ROOT/std/manifest.toml"
FIXTURE_DIR="$REPO_ROOT/tests/fixtures"
FIXTURE_MANIFEST="$FIXTURE_DIR/manifest.txt"
REFERENCE="$REPO_ROOT/docs/stdlib/reference.md"

errors=0
warnings=0

# Extract function names from manifest
func_names=$(python3 -c "
import toml
with open('$MANIFEST') as f:
    m = toml.load(f)
for fn in m.get('functions', []):
    name = fn['name']
    module = fn.get('module', 'prelude') or 'prelude'
    stability = fn.get('stability', 'stable')
    has_deprecated = 'yes' if fn.get('deprecated_by') else 'no'
    print(f'{name}\t{module}\t{stability}\t{has_deprecated}')
")

total=0
missing_fixture=0
missing_docs=0
missing_stability=0
deprecated_count=0

echo "=== Stdlib API Admission Gate ==="
echo ""

while IFS=$'\t' read -r name module stability deprecated; do
    total=$((total + 1))

    # Skip deprecated functions (they may not need new fixtures)
    if [ "$deprecated" = "yes" ]; then
        deprecated_count=$((deprecated_count + 1))
        continue
    fi

    # Check if function appears in any fixture file
    if ! grep -rq "\b${name}\b" "$FIXTURE_DIR"/*.ark "$FIXTURE_DIR"/**/*.ark 2>/dev/null; then
        if ! grep -rq "\b${name}\b" "$FIXTURE_DIR" --include='*.ark' 2>/dev/null; then
            missing_fixture=$((missing_fixture + 1))
            if [ "$stability" = "stable" ]; then
                echo "WARN: $name ($module) — no fixture coverage"
                warnings=$((warnings + 1))
            fi
        fi
    fi

    # Check if function appears in reference docs
    if [ -f "$REFERENCE" ]; then
        if ! grep -q "\b${name}\b" "$REFERENCE" 2>/dev/null; then
            missing_docs=$((missing_docs + 1))
        fi
    fi

    # Check stability field
    if [ -z "$stability" ] || [ "$stability" = "null" ]; then
        missing_stability=$((missing_stability + 1))
        echo "ERROR: $name ($module) — missing stability field"
        errors=$((errors + 1))
    fi
done <<< "$func_names"

echo ""
echo "=== Coverage Report ==="
echo "Total APIs:        $total"
echo "Deprecated:        $deprecated_count"
echo "Missing fixture:   $missing_fixture"
echo "Missing docs:      $missing_docs"
echo "Missing stability: $missing_stability"
echo "Warnings:          $warnings"
echo "Errors:            $errors"

# Generate per-family report
echo ""
echo "=== Family Coverage ==="
python3 -c "
import toml, os
from collections import Counter, defaultdict

with open('$MANIFEST') as f:
    m = toml.load(f)

funcs = m.get('functions', [])
families = defaultdict(lambda: {'total': 0, 'stable': 0, 'experimental': 0, 'deprecated': 0})

for fn in funcs:
    mod = fn.get('module', 'prelude') or 'prelude'
    # Group into families
    parts = mod.split('::')
    if len(parts) >= 3:
        family = '::'.join(parts[:3])
    elif len(parts) >= 2:
        family = '::'.join(parts[:2])
    else:
        family = mod

    families[family]['total'] += 1
    stab = fn.get('stability', 'stable')
    if fn.get('deprecated_by'):
        families[family]['deprecated'] += 1
    elif stab == 'experimental':
        families[family]['experimental'] += 1
    else:
        families[family]['stable'] += 1

print(f'{\"Family\":<30} {\"Total\":>6} {\"Stable\":>7} {\"Exp\":>5} {\"Depr\":>5}')
print('-' * 55)
for family in sorted(families):
    f = families[family]
    print(f'{family:<30} {f[\"total\"]:>6} {f[\"stable\"]:>7} {f[\"experimental\"]:>5} {f[\"deprecated\"]:>5}')
"

if [ "$errors" -gt 0 ]; then
    echo ""
    echo "FAIL: $errors admission gate errors"
    exit 1
fi

echo ""
echo "OK: Admission gate passed ($warnings warnings)"
exit 0
