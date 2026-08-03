#!/usr/bin/env python3
"""Compile focused fixtures and prove backend GC layout lookup is TypeId-only."""

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
NAME_IDENTITY = re.compile(r"gc-layout-audit: name lookup type_name=([^\r\n]+)")
PROBE_FIXTURES = (
    "tests/fixtures/hello_world.ark",
    "tests/fixtures/structs/basic_struct.ark",
    "tests/fixtures/enums/option_some.ark",
    "tests/fixtures/stdlib_string/string_concat.ark",
    "tests/fixtures/stdlib_vec/vec_get.ark",
    "tests/fixtures/stdlib_hashmap/hashmap_basic.ark",
)


def compile_fixture(relative: str) -> dict[str, object]:
    fixture = ROOT / relative
    if not fixture.is_file():
        raise ValueError(f"probe fixture missing: {relative}")
    command = [
        str(ROOT / "scripts" / "run" / "arukellt-selfhost.sh"),
        "compile",
        relative,
        "--opt-level",
        "2",
    ]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    if result.returncode != 0:
        raise ValueError(
            f"{relative}: compile exited {result.returncode}\n{result.stdout}\n{result.stderr}"
        )

    combined = result.stdout + "\n" + result.stderr
    totals = {"typed": 0, "name": 0, "fallback": 0, "conflict": 0}
    matches = list(SUMMARY.finditer(combined))
    if not matches:
        raise ValueError(
            f"{relative}: gc-layout-audit summary missing\n{result.stdout}\n{result.stderr}"
        )
    for match in matches:
        totals["typed"] += int(match.group(1))
        totals["name"] += int(match.group(2))
        totals["fallback"] += int(match.group(3))
        totals["conflict"] += int(match.group(4))

    identities = [match.group(1).strip() for match in NAME_IDENTITY.finditer(combined)]
    if totals["name"] != 0 or identities:
        raise ValueError(
            f"{relative}: backend name lookup observed: count={totals['name']} identities={identities}"
        )
    if totals["fallback"] != 0 or totals["conflict"] != 0:
        raise ValueError(
            f"{relative}: fallback={totals['fallback']} conflict={totals['conflict']}"
        )
    return {
        "fixture": relative,
        "summary_lines": len(matches),
        "counts": totals,
        "typeid_only": True,
    }


def main() -> int:
    baseline = json.loads(BASELINE_PATH.read_text(encoding="utf-8"))
    if int(baseline["max_name_lookup_count"]) != 0:
        raise ValueError("backend name lookup baseline must be exactly zero")
    if baseline.get("name_lookup_must_be_zero") is not True:
        raise ValueError("backend name lookup zero policy is not enabled")

    baseline_fixture = str(baseline["fixture"])
    ordered: list[str] = []
    for fixture in (baseline_fixture, *PROBE_FIXTURES):
        if fixture not in ordered:
            ordered.append(fixture)
    results = [compile_fixture(fixture) for fixture in ordered]

    document = {
        "schema": "arukellt-backend-typeid-only-audit",
        "schema_version": 1,
        "baseline_fixture": baseline_fixture,
        "fixtures": results,
        "policy": {
            "name_lookup_must_be_zero": True,
            "fallback_must_be_zero": True,
            "conflict_must_be_zero": True,
        },
        "status": "passed",
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    typed_total = sum(int(result["counts"]["typed"]) for result in results)
    print(
        "backend-name-lookup-runtime: PASS: "
        f"fixtures={len(results)} typed={typed_total} name=0 fallback=0 conflict=0"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
        print(f"backend-name-lookup-runtime: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
