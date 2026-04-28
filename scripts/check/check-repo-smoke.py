#!/usr/bin/env python3
"""Quick repository smoke check.

Verifies that essential files, directories, and tooling exist.
Fast — runs in <1 second. Used by autonomous orchestrator as Layer 3 check.

Usage:
    check-repo-smoke.py
"""

import shutil
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

REQUIRED_DIRS = [
    "crates",
    "docs",
    "docs/adr",
    "docs/language",
    "docs/stdlib",
    "docs/platform",
    "docs/process",
    "issues/open",
    "issues/done",
    "scripts/check",
    "scripts/gen",
    "scripts/lib",
    "scripts/run",
    "src/compiler",
    "std",
    "tests/fixtures",
]

REQUIRED_FILES = [
    "AGENTS.md",
    "Cargo.toml",
    "docs/current-state.md",
    "issues/open/index.md",
    "issues/open/dependency-graph.md",
    "scripts/manager.py",
]

OPTIONAL_FILES = [
    "playground/src/index.ts" if (REPO_ROOT / "playground").exists() else None,
]


def check() -> int:
    errors = 0

    for d in REQUIRED_DIRS:
        if not (REPO_ROOT / d).is_dir():
            print(f"MISSING_DIR: {d}")
            errors += 1

    for f in REQUIRED_FILES:
        if not (REPO_ROOT / f).is_file():
            print(f"MISSING_FILE: {f}")
            errors += 1

    # Check tool availability
    tools = ["python3", "git", "cargo", "rustc"]
    if shutil.which("wasmtime"):
        pass  # optional
    for tool in tools:
        if not shutil.which(tool):
            print(f"MISSING_TOOL: {tool}")
            errors += 1

    # Check git worktree support
    import subprocess
    result = subprocess.run(
        ["git", "worktree", "list"],
        capture_output=True, text=True, cwd=str(REPO_ROOT)
    )
    if result.returncode != 0:
        print(f"GIT_WORKTREE_FAILED: {result.stderr.strip()}")
        errors += 1

    if errors == 0:
        print("REPO_SMOKE: PASS")
        return 0
    else:
        print(f"REPO_SMOKE: {errors} issue(s)")
        return 1


if __name__ == "__main__":
    sys.exit(check())
