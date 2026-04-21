#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DONE_DIR="$ROOT/issues/done"

# Check for false-done issues in issues/done/
# A false-done issue is one that:
# 1. Is in issues/done/
# 2. Has acceptance criteria that are all unchecked (all [ ])
# 3. Or has no acceptance criteria at all

echo "=== False-Done Issue Checker ==="
echo ""

total=0
false_done=0
partial=0

for file in "$DONE_DIR"/*.md; do
    if [ ! -f "$file" ]; then
        continue
    fi
    
    total=$((total + 1))
    issue_id=$(basename "$file" .md | sed 's/-.*$//')
    
    # Extract the Acceptance Criteria section
    in_acceptance=false
    has_acceptance=false
    checked=0
    unchecked=0
    
    while IFS= read -r line; do
        if [[ "$line" =~ ^##[[:space:]]Acceptance[[:space:]]Criteria ]]; then
            in_acceptance=true
            has_acceptance=true
            continue
        fi
        
        if $in_acceptance; then
            if [[ "$line" =~ ^##[[:space:]] ]]; then
                # End of acceptance criteria section
                break
            fi
            
            if [[ "$line" == "- [x]"* ]] || [[ "$line" == "- [X]"* ]]; then
                checked=$((checked + 1))
            elif [[ "$line" == "- [ ]"* ]]; then
                unchecked=$((unchecked + 1))
            fi
        fi
    done < "$file"
    
    if ! $has_acceptance; then
        echo "⚠️  $issue_id: No acceptance criteria found"
        continue
    fi
    
    if [ $checked -eq 0 ] && [ $unchecked -gt 0 ]; then
        echo "❌ $issue_id: FALSE-DONE (all $unchecked criteria unchecked)"
        false_done=$((false_done + 1))
    elif [ $unchecked -gt 0 ]; then
        echo "⚠️  $issue_id: PARTIAL ($checked checked, $unchecked unchecked)"
        partial=$((partial + 1))
    fi
done

echo ""
echo "Summary:"
echo "  Total issues in issues/done/: $total"
echo "  False-done (all unchecked): $false_done"
echo "  Partial (some unchecked): $partial"

if [ $false_done -gt 0 ]; then
    echo ""
    echo "ERROR: Found $false_done false-done issue(s). These should be moved to issues/open/."
    exit 1
fi

if [ $partial -gt 0 ]; then
    echo ""
    echo "WARNING: Found $partial issue(s) with partial acceptance criteria."
    exit 0
fi

echo "✓ All done issues have complete acceptance criteria."
exit 0
