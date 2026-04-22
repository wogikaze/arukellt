#!/usr/bin/env bash
# check-panic-audit.sh — Detect unwrap/panic/todo/unimplemented in non-test production code.
#
# Skips:  #[test] lines, // comments, expect() calls, lock().unwrap(),
#         and any code inside #[cfg(test)] module blocks.
set -uo pipefail

DIRS=(crates/arukellt/src/ crates/ark-lsp/src/ crates/ark-manifest/src/)
PATTERN='\.unwrap()\|panic!\|todo!()\|unimplemented!()'
EXCLUDE='lock().unwrap\|#\[test\]\|//\|expect('

hits=""
for dir in "${DIRS[@]}"; do
    [ -d "$dir" ] || continue
    while IFS= read -r -d '' file; do
        # Find the line where #[cfg(test)] starts (if any)
        test_start=$(grep -n '#\[cfg(test)\]' "$file" 2>/dev/null | head -1 | cut -d: -f1 || true)
        if [ -n "$test_start" ]; then
            # Only check lines before the test module
            file_hits=$(head -n "$((test_start - 1))" "$file" | grep -n "$PATTERN" 2>/dev/null | grep -v "$EXCLUDE" || true)
        else
            file_hits=$(grep -n "$PATTERN" "$file" 2>/dev/null | grep -v "$EXCLUDE" || true)
        fi
        if [ -n "$file_hits" ]; then
            while IFS= read -r line; do
                hits="${hits}${file}:${line}"$'\n'
            done <<< "$file_hits"
        fi
    done < <(find "$dir" -name '*.rs' -type f -print0)
done

if [ -n "$hits" ]; then
    echo "Potential panic in user-facing crate:"
    printf '%s' "$hits"
    exit 1
fi
exit 0
