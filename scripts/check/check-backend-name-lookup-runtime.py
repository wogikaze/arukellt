#!/usr/bin/env python3
"""Compile a GC-using fixture and record backend layout lookup counts."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests" / "fixtures" / "stdlib_vec" / "vec_get.ark"
OUTPUT = ROOT / ".build" / "audit" / "backend-name-lookup.json"
SUMMARY = re.compile(
    r"gc-layout-audit: summary typed=(\d+) name=(\d+) fallback=(\d+) conflict=(\d+)"
)


def main() -> int:
    command = [
        str(ROOT / "scripts" / "run" / "arukellt-selfhost.sh"),
        "compile",
        str(FIXTURE.relative_to(ROOT)),
        "--opt-level",
        "2",
    ]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"backend-name-lookup-runtime: FAIL: compile exited {result.returncode}", file=sys.stderr)
        print(result.stdout, file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        return 1

    totals = {"typed": 0, "name": 0, "fallback": 0, "conflict": 0}
    matches = list(SUMMARY.finditer(result.stdout + "\n" + result.stderr))
    for match in matches:
        totals["typed"] += int(match.group(1))
        totals["name"] += int(match.group(2))
        totals["fallback"] += int(match.group(3))
        totals["conflict"] += int(match.group(4))

    if totals["fallback"] != 0 or totals["conflict"] != 0:
        print(
            "backend-name-lookup-runtime: FAIL: "
            f"fallback={totals['fallback']} conflict={totals['conflict']}",
            file=sys.stderr,
        )
        return 1

    document = {
        "schema": "arukellt-backend-name-lookup-audit",
        "schema_version": 1,
        "fixture": str(FIXTURE.relative_to(ROOT)),
        "summary_lines": len(matches),
        "counts": totals,
        "policy": {
            "fallback_must_be_zero": True,
            "conflict_must_be_zero": True,
            "name_lookup_is_observational": True,
        },
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        "backend-name-lookup-runtime: PASS: "
        f"typed={totals['typed']} name={totals['name']} "
        f"fallback={totals['fallback']} conflict={totals['conflict']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
