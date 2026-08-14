#!/usr/bin/env python3
"""Aggregate close gate for #667/#670/#671/#672/#673/#674/#706.

The aggregate verifies both implementation-specific gates and repository issue closure
receipts so the productization PR cannot leave one of the tracked Markdown issues open.
"""
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
CLOSED_ISSUES = (
    "667-library-component-emit-routing-regression.md",
    "670-wit-import-resolver-hardening.md",
    "671-wit-import-callable-type-matrix.md",
    "672-wit-type-binding-codegen.md",
    "673-component-export-aggregate-expansion.md",
    "674-component-composition-deps-interop.md",
    "706-std-wit-full-compliance.md",
)


def check_close_receipts() -> int:
    for name in CLOSED_ISSUES:
        done = ROOT / "issues/done" / name
        open_path = ROOT / "issues/open" / name
        if not done.is_file():
            print(f"component-wit-productization: FAIL: missing close receipt {done.relative_to(ROOT)}", file=sys.stderr)
            return 1
        text = done.read_text(encoding="utf-8")
        if "Status: done" not in text or "Closed: 2026-08-14" not in text:
            print(f"component-wit-productization: FAIL: incomplete close receipt {done.relative_to(ROOT)}", file=sys.stderr)
            return 1
        if open_path.exists():
            print(f"component-wit-productization: FAIL: issue still present in open set: {open_path.relative_to(ROOT)}", file=sys.stderr)
            return 1
    return 0


def main() -> int:
    if check_close_receipts() != 0:
        return 1
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
