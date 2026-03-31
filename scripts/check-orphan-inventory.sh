#!/usr/bin/env bash
# Orphan / stale file inventory
# Scans docs, tests, benchmarks, and artifacts for potentially orphaned files.
# Exit 0 always (advisory report, not a gate).

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

STALE_DAYS="${STALE_DAYS:-180}"
SIZE_THRESHOLD="${SIZE_THRESHOLD:-1048576}"  # 1 MB

echo "=== Orphan / Stale File Inventory ==="
echo ""

errors=0

# 1. Large files (>1MB) that may be artifacts
echo "--- Large files (>${SIZE_THRESHOLD} bytes) ---"
found_large=0
while IFS= read -r file; do
    size=$(stat -c%s "$file" 2>/dev/null || echo 0)
    if [ "$size" -gt "$SIZE_THRESHOLD" ]; then
        echo "  $(numfmt --to=iec "$size")  $file"
        found_large=$((found_large + 1))
    fi
done < <(git ls-files --others --cached | grep -v '^target/' | grep -v '^\.git/' | grep -v 'node_modules' | grep -v 'Cargo.lock' | grep -v '\.vscode-test/')
if [ "$found_large" -eq 0 ]; then
    echo "  (none)"
fi
echo ""

# 2. Test fixtures not in manifest
echo "--- Test fixtures on disk but NOT in manifest ---"
manifest="tests/fixtures/manifest.txt"
found_orphan_fixtures=0
if [ -f "$manifest" ]; then
    while IFS= read -r ark_file; do
        basename_rel="${ark_file#tests/fixtures/}"
        if ! grep -q "$basename_rel" "$manifest" 2>/dev/null; then
            echo "  $ark_file"
            found_orphan_fixtures=$((found_orphan_fixtures + 1))
        fi
    done < <(find tests/fixtures -name '*.ark' -type f | sort)
fi
if [ "$found_orphan_fixtures" -eq 0 ]; then
    echo "  (none)"
fi
echo ""

# 3. Expected files without matching .ark files
echo "--- .expected files without matching .ark ---"
found_orphan_expected=0
while IFS= read -r expected; do
    ark="${expected%.expected}.ark"
    if [ ! -f "$ark" ]; then
        echo "  $expected"
        found_orphan_expected=$((found_orphan_expected + 1))
    fi
done < <(find tests/fixtures -name '*.expected' -type f | sort)
if [ "$found_orphan_expected" -eq 0 ]; then
    echo "  (none)"
fi
echo ""

# 4. Docs referencing files that don't exist
echo "--- Broken file references in docs ---"
found_broken_refs=0
tmpfile=$(mktemp)
find docs -name '*.md' -type f -print0 | xargs -0 grep -ohP '\]\(\K[^)]+' 2>/dev/null | sort -u | while read -r ref; do
    if echo "$ref" | grep -qE '^(https?://|#|mailto:)'; then
        continue
    fi
    ref="${ref%%#*}"
    [ -z "$ref" ] && continue
    # Try from repo root
    if [ ! -e "$ref" ]; then
        echo "  $ref (not found)" >> "$tmpfile"
    fi
done
if [ -s "$tmpfile" ]; then
    cat "$tmpfile"
    found_broken_refs=$(wc -l < "$tmpfile")
else
    echo "  (none)"
fi
rm -f "$tmpfile"
if [ "$found_broken_refs" -eq 0 ]; then
    echo "  (none)"
fi
echo ""

# 5. Benchmark assets not referenced in results or scripts
echo "--- Benchmark files not referenced in scripts ---"
found_orphan_bench=0
while IFS= read -r bench_file; do
    basename=$(basename "$bench_file")
    if ! grep -rq "$basename" scripts/ benchmarks/README.md benchmarks/*.json 2>/dev/null; then
        # Check if it's a .ark source file (expected)
        case "$bench_file" in
            *.ark) ;; # benchmark source files are fine
            *.expected) ;; # expected output files are fine
            *) echo "  $bench_file"; found_orphan_bench=$((found_orphan_bench + 1)) ;;
        esac
    fi
done < <(find benchmarks -type f -not -name '*.md' -not -name '.gitkeep' 2>/dev/null | sort)
if [ "$found_orphan_bench" -eq 0 ]; then
    echo "  (none)"
fi
echo ""

# Summary
total=$((found_large + found_orphan_fixtures + found_orphan_expected + found_broken_refs + found_orphan_bench))
echo "=== Summary ==="
echo "  Large files:           $found_large"
echo "  Orphan fixtures:       $found_orphan_fixtures"
echo "  Orphan .expected:      $found_orphan_expected"
echo "  Broken doc refs:       $found_broken_refs"
echo "  Orphan bench assets:   $found_orphan_bench"
echo "  Total candidates:      $total"
echo ""
if [ "$total" -gt 0 ]; then
    echo "Review candidates above. This is advisory — no files were modified."
else
    echo "No orphan or stale candidates found."
fi
