#!/usr/bin/env python3
"""Aggregate close gate for #667/#670/#671/#672/#673/#674/#706."""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GATES = (
    "gate-667-library-specialized-routing.py",
    "gate-670-wit-import-resolver-hardening.py",
    "gate-671-wit-import-type-matrix.py",
    "gate-672-wit-type-binding-codegen.py",
    "gate-673-component-export-aggregate-expansion.py",
    "gate-674-component-composition-deps.py",
    "gate-706-std-wit-full-compliance.py",
)


def main() -> int:
    for name in GATES:
        path = ROOT / "scripts/check" / name
        run = subprocess.run([sys.executable, str(path)], cwd=ROOT)
        if run.returncode != 0:
            print(f"component-wit-productization: FAIL at {name}", file=sys.stderr)
            return run.returncode
    print("component-wit-productization: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
