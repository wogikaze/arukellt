"""Perf domain check runners."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def _exec(cmd: list[str], cwd: Path, dry_run: bool) -> tuple[int, str]:
    if dry_run:
        print(f"DRY-RUN: {cmd}")
        return (0, "")
    result = subprocess.run(
        cmd, cwd=str(cwd), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
    )
    return (result.returncode, result.stdout)


def run_gate(root: Path, dry_run: bool, update: bool = False) -> tuple[int, str]:
    mode = "update-baseline" if update else "ci"
    cmd = [
        sys.executable,
        "scripts/util/benchmark_runner.py",
        "--mode", mode,
        "--baseline", "tests/baselines/perf/baselines.json",
        "--output-json", "tests/baselines/perf/current.json",
        "--output-md", "docs/process/benchmark-results.md",
    ]
    return _exec(cmd, root, dry_run)


def run_baseline(root: Path, dry_run: bool) -> tuple[int, str]:
    cmd = [sys.executable, "scripts/util/collect-baseline.py"]
    return _exec(cmd, root, dry_run)


def run_benchmarks(root: Path, dry_run: bool, quick: bool = True) -> tuple[int, str]:
    mode = "quick" if quick else "full"
    cmd = [
        sys.executable,
        "scripts/util/benchmark_runner.py",
        "--mode", mode,
        "--output-json", "tests/baselines/perf/current.json",
        "--output-md", "docs/process/benchmark-results.md",
    ]
    return _exec(cmd, root, dry_run)
