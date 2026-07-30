#!/usr/bin/env python3
"""Exercise parser/typechecker support for function proof contracts."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WRAPPER = ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
CASES = (
    ("valid.ark", True, None),
    (
        "requires-not-bool.ark",
        False,
        "proof requires contract must have type bool, got i32",
    ),
    (
        "ensures-not-bool.ark",
        False,
        "proof ensures contract must have type bool, got i32",
    ),
)


def run_case(name: str, should_pass: bool, expected: str | None) -> list[str]:
    source = ROOT / "tests" / "proof-contracts" / name
    completed = subprocess.run(
        ["bash", str(WRAPPER), "check", str(source)],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    output = completed.stdout + completed.stderr
    errors: list[str] = []
    if should_pass and completed.returncode != 0:
        errors.append(f"{name}: expected success, got {completed.returncode}\n{output}")
    if not should_pass and completed.returncode == 0:
        errors.append(f"{name}: expected failure, command succeeded")
    if expected is not None and expected not in output:
        errors.append(f"{name}: missing diagnostic {expected!r}\n{output}")
    return errors


def main() -> int:
    errors: list[str] = []
    for name, should_pass, expected in CASES:
        errors.extend(run_case(name, should_pass, expected))
    if errors:
        print("\n\n".join(errors), file=sys.stderr)
        return 1
    print(f"proof-contract frontend: {len(CASES)} cases passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
