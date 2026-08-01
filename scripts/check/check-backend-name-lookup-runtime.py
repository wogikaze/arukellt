#!/usr/bin/env python3
"""Compile a GC-using fixture, enforce the ratchet, and record lookup counts."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASELINE_PATH = ROOT / "tests" / "proof" / "backend-name-lookup-baseline.json"
OUTPUT = ROOT / ".build" / "audit" / "backend-name-lookup.json"
SUMMARY = re.compile(
    r"gc-layout-audit: summary typed=(\d+) name=(\d+) fallback=(\d+) conflict=(\d+)"
)


def main() -> int:
    baseline = json.loads(BASELINE_PATH.read_text(encoding="utf-8"))
    fixture = ROOT / baseline["fixture"]
    max_name = int(baseline["max_name_lookup_count"])

    command = [
        str(ROOT / "scripts" / "run" / "arukellt-selfhost.sh"),
        "compile",
        str(fixture.relative_to(ROOT)),
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
    if not matches:
        print("backend-name-lookup-runtime: FAIL: gc-layout-audit summary missing", file=sys.stderr)
        print(result.stdout, file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        return 1
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
    if totals["name"] > max_name:
        print(
            "backend-name-lookup-runtime: FAIL: "
            f"name lookup regression {totals['name']} > {max_name}",
            file=sys.stderr,
        )
        return 1

    document = {
        "schema": "arukellt-backend-name-lookup-audit",
        "schema_version": 1,
        "fixture": str(fixture.relative_to(ROOT)),
        "summary_lines": len(matches),
        "counts": totals,
        "ratchet": {"max_name_lookup_count": max_name},
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
        f"typed={totals['typed']} name={totals['name']}/{max_name} "
        f"fallback={totals['fallback']} conflict={totals['conflict']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
        print(f"backend-name-lookup-runtime: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
