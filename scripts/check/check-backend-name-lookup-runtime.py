#!/usr/bin/env python3
"""Compile focused fixtures, enforce the ratchet, and isolate name lookups."""

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

    totals = {"typed": 0, "name": 0, "fallback": 0, "conflict": 0}
    matches = list(SUMMARY.finditer(result.stdout + "\n" + result.stderr))
    if not matches:
        raise ValueError(
            f"{relative}: gc-layout-audit summary missing\n{result.stdout}\n{result.stderr}"
        )
    for match in matches:
        totals["typed"] += int(match.group(1))
        totals["name"] += int(match.group(2))
        totals["fallback"] += int(match.group(3))
        totals["conflict"] += int(match.group(4))
    if totals["fallback"] != 0 or totals["conflict"] != 0:
        raise ValueError(
            f"{relative}: fallback={totals['fallback']} conflict={totals['conflict']}"
        )
    return {
        "fixture": relative,
        "summary_lines": len(matches),
        "counts": totals,
    }


def main() -> int:
    baseline = json.loads(BASELINE_PATH.read_text(encoding="utf-8"))
    baseline_fixture = str(baseline["fixture"])
    max_name = int(baseline["max_name_lookup_count"])

    ordered: list[str] = []
    for fixture in (baseline_fixture, *PROBE_FIXTURES):
        if fixture not in ordered:
            ordered.append(fixture)
    results = [compile_fixture(fixture) for fixture in ordered]
    by_fixture = {result["fixture"]: result for result in results}
    primary = by_fixture[baseline_fixture]
    primary_counts = primary["counts"]
    if not isinstance(primary_counts, dict):
        raise TypeError("baseline counts must be an object")
    if int(primary_counts["name"]) > max_name:
        raise ValueError(
            f"name lookup regression {primary_counts['name']} > {max_name}"
        )

    nonzero = [
        {
            "fixture": result["fixture"],
            "name_lookup_count": result["counts"]["name"],
        }
        for result in results
        if isinstance(result["counts"], dict) and int(result["counts"]["name"]) > 0
    ]
    document = {
        "schema": "arukellt-backend-name-lookup-audit",
        "schema_version": 2,
        "baseline_fixture": baseline_fixture,
        "fixtures": results,
        "nonzero_name_lookup_fixtures": nonzero,
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
        f"baseline_name={primary_counts['name']}/{max_name} "
        f"nonzero_fixtures={len(nonzero)}"
    )
    for item in nonzero:
        print(
            "backend-name-lookup-runtime: identity-probe: "
            f"fixture={item['fixture']} name={item['name_lookup_count']}"
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
        print(f"backend-name-lookup-runtime: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
