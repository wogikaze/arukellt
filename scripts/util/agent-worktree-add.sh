#!/usr/bin/env bash
# Create an isolated git worktree for one agent / acceptance slice.
#
# Usage:
#   bash scripts/util/agent-worktree-add.sh <worktree-name> <new-branch-name> [base-branch]
#
# Example:
#   bash scripts/util/agent-worktree-add.sh wt-593-slice1 feat/593-phase1 master
#
# Default base branch: master
# Worktree parent: parent directory of repo root (sibling folders), override with WT_PARENT.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
WT_NAME="${1:?worktree directory name required (e.g. wt-593-slice1)}"
NEW_BRANCH="${2:?new branch name required (e.g. feat/593-slice1)}"
BASE_BRANCH="${3:-master}"

PARENT="$(dirname "$REPO_ROOT")"
WT_PARENT="${WT_PARENT:-$PARENT}"
WT_PATH="${WT_PARENT}/${WT_NAME}"

if [[ -e "$WT_PATH" ]]; then
  echo "error: path already exists: $WT_PATH" >&2
  exit 1
fi

git -C "$REPO_ROOT" worktree add "$WT_PATH" -b "$NEW_BRANCH" "$BASE_BRANCH"
echo "Added worktree: $WT_PATH (branch $NEW_BRANCH from $BASE_BRANCH)"
echo "Next: cd \"$WT_PATH\""
