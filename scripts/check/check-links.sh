#!/usr/bin/env bash
# Broken link / missing file reference checker
# Validates internal file references in docs and issues.
# Exit 1 if broken links found, 0 otherwise.

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

checked=0
tmpfile=$(mktemp)
trap 'rm -f "$tmpfile"' EXIT

echo "=== Link / Reference Checker ==="
echo ""

check_file() {
    local md_file="$1"
    local dir
    dir=$(dirname "$md_file")

    grep -oP '\]\(\K[^)]+' "$md_file" 2>/dev/null | while read -r ref; do
        # Skip URLs, anchors, mailto
        case "$ref" in
            http://*|https://*|mailto:*|data:*) continue ;;
        esac
        if [ "${ref:0:1}" = "#" ]; then continue; fi

        local path="${ref%%#*}"
        [ -z "$path" ] && continue
        path="${path%%\?*}"
        # Skip pseudo-references (inline code, placeholders)
        case "$path" in
            *\"*|*:\ *|*NNN*|*...*) continue ;;
        esac

        if [ ! -e "$dir/$path" ] && [ ! -e "$path" ]; then
            echo "  BROKEN: $md_file -> $ref" >&2
            echo "$md_file:$ref" >> "$tmpfile"
        fi
    done || true
}

echo "Checking docs/**/*.md ..."
while IFS= read -r f; do
    check_file "$f"
    checked=$((checked + 1))
done < <(find docs -name '*.md' -type f | sort)

echo "Checking issues/**/*.md ..."
while IFS= read -r f; do
    check_file "$f"
    checked=$((checked + 1))
done < <(find issues -name '*.md' -type f | sort)

for extra in README.md AGENTS.md; do
    if [ -f "$extra" ]; then
        echo "Checking $extra ..."
        check_file "$extra"
        checked=$((checked + 1))
    fi
done

echo ""

if [ -s "$tmpfile" ]; then
    errors=$(wc -l < "$tmpfile")
    echo "=== FAILED: $errors broken link(s) in $checked files ==="
    exit 1
else
    echo "=== OK: $checked files checked, no broken links ==="
    exit 0
fi
