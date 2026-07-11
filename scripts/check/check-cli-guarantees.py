#!/usr/bin/env python3
"""Executable smoke contracts for CLI commands advertised as guaranteed."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CLI = ROOT / "scripts" / "run" / "arukellt-selfhost.sh"


def run(command: str) -> int:
    if command == "check":
        result = subprocess.run(
            [str(CLI), "check", "tests/fixtures/diagnostics/type_mismatch.ark"],
            cwd=ROOT,
            text=True,
            capture_output=True,
        )
        output = result.stdout + result.stderr
        if result.returncode == 0 or "E0200" not in output:
            print("cli check guarantee failed: expected non-zero E0200 diagnostic", file=sys.stderr)
            return 1
        return 0
    if command == "help":
        result = subprocess.run([str(CLI), "help"], cwd=ROOT, text=True, capture_output=True)
        if result.returncode != 0 or "Usage: arukellt <COMMAND>" not in result.stdout:
            print("cli help guarantee failed: canonical usage text missing", file=sys.stderr)
            return 1
        return 0
    print(f"unknown CLI guarantee: {command}", file=sys.stderr)
    return 2


def main() -> int:
    commands = ["check", "help"] if len(sys.argv) == 1 or sys.argv[1] == "all" else [sys.argv[1]]
    return 1 if any(run(command) for command in commands) else 0


if __name__ == "__main__":
    raise SystemExit(main())
