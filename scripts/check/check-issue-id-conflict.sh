#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ISSUES_DIR="$ROOT/issues"

# Check if an issue ID already exists across all issue directories
# Usage: scripts/check/check-issue-id-conflict.sh <issue_id>
# Exit code: 0 if ID is available, 1 if ID conflicts

if [ $# -ne 1 ]; then
    echo "Usage: $0 <issue_id>" >&2
    echo "Check if an issue ID already exists across issues/open/, issues/done/, issues/blocked/" >&2
    exit 1
fi

ISSUE_ID="$1"

# Check for conflicts in all issue directories
for dir in open done blocked; do
    dir_path="$ISSUES_DIR/$dir"
    if [ -d "$dir_path" ]; then
        # Check for exact ID match (e.g., 532.md)
        if [ -f "$dir_path/${ISSUE_ID}.md" ]; then
            echo "ERROR: Issue ID $ISSUE_ID already exists in issues/$dir/" >&2
            exit 1
        fi
        
        # Check for ID prefix match (e.g., 532-something.md)
        if ls "$dir_path/${ISSUE_ID}-"*.md 1>/dev/null 2>&1; then
            echo "ERROR: Issue ID $ISSUE_ID conflicts with existing files in issues/$dir/" >&2
            ls "$dir_path/${ISSUE_ID}-"*.md >&2
            exit 1
        fi
    fi
done

echo "Issue ID $ISSUE_ID is available"
exit 0
