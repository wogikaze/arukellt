#!/usr/bin/env python3
"""Check agent worktree state.

Verifies that a worktree directory exists, is valid, contains a git branch,
and has no conflicts. Used by the autonomous orchestrator for child supervision.

Usage:
    check-agent-state.py <worktree-path>
    check-agent-state.py --list-all
"""

import argparse
import subprocess
import sys
from pathlib import Path


def check_worktree(path: Path) -> int:
    if not path.exists():
        print(f"WORKTREE_MISSING: {path}")
        return 1
    if not (path / ".git").exists():
        print(f"NOT_A_GIT_REPO: {path}")
        return 2

    result = subprocess.run(
        ["git", "status", "--short"],
        cwd=str(path), capture_output=True, text=True
    )
    dirty = bool(result.stdout.strip())
    result_merge = subprocess.run(
        ["git", "rev-parse", "--verify", "HEAD"],
        cwd=str(path), capture_output=True, text=True
    )
    head = result_merge.stdout.strip() if result_merge.returncode == 0 else "none"

    result_branch = subprocess.run(
        ["git", "rev-parse", "--abbrev-ref", "HEAD"],
        cwd=str(path), capture_output=True, text=True
    )
    branch = result_branch.stdout.strip() if result_branch.returncode == 0 else "none"

    result_ahead = subprocess.run(
        ["git", "log", "--oneline", "@{u}..HEAD", "--max-count=5"],
        cwd=str(path), capture_output=True, text=True
    )
    ahead_count = len([l for l in result_ahead.stdout.splitlines() if l.strip()])

    status = "DIRTY" if dirty else "CLEAN"
    print(f"WORKTREE_STATUS: {status}")
    print(f"WORKTREE_PATH: {path}")
    print(f"BRANCH: {branch}")
    print(f"HEAD: {head}")
    print(f"AHEAD: {ahead_count}")
    if dirty:
        print(f"DIRTY_FILES: {result.stdout.count(chr(10))}")
        for line in result.stdout.splitlines()[:20]:
            print(f"  {line}")
    return 0 if not dirty else 1


def list_all_worktrees() -> int:
    result = subprocess.run(
        ["git", "worktree", "list"],
        capture_output=True, text=True
    )
    print(result.stdout)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Check agent worktree state")
    parser.add_argument("worktree_path", nargs="?", help="Path to worktree directory")
    parser.add_argument("--list-all", action="store_true", help="List all worktrees")
    args = parser.parse_args()

    if args.list_all:
        return list_all_worktrees()

    if not args.worktree_path:
        parser.print_help()
        return 1

    return check_worktree(Path(args.worktree_path))


if __name__ == "__main__":
    sys.exit(main())
