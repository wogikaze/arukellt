"""Docs domain check runners."""

from __future__ import annotations

import subprocess
from pathlib import Path


def _exec(cmd: list[str], cwd: Path, dry_run: bool) -> tuple[int, str]:
    if dry_run:
        print(f"DRY-RUN: {cmd}")
        return (0, "")
    result = subprocess.run(
        cmd, cwd=str(cwd), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
    )
    return (result.returncode, result.stdout)


def run_consistency(root: Path, dry_run: bool) -> tuple[int, str]:
    cmd = ["python3", "scripts/check/check-docs-consistency.py"]
    return _exec(cmd, root, dry_run)


def run_freshness(root: Path, dry_run: bool) -> tuple[int, str]:
    cmd = ["python3", "scripts/check/check-docs-freshness.py"]
    return _exec(cmd, root, dry_run)


def run_examples(root: Path, dry_run: bool) -> tuple[int, str]:
    cmd = ["python3", "scripts/check/check-doc-examples.py", "docs/"]
    return _exec(cmd, root, dry_run)


def run_regenerate(
    root: Path, dry_run: bool, check_only: bool = False
) -> tuple[int, str]:
    cmd = ["python3", "scripts/gen/generate-docs.py"]
    if check_only:
        cmd.append("--check")
    return _exec(cmd, root, dry_run)
