#!/usr/bin/env python3
"""Compile the T3 fixture suite and persist the #798 CoreOp shadow receipt."""
from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
T3_CHECK = ROOT / "scripts" / "check" / "check-t3-wasm-validate.py"
DEFAULT_RECEIPT = ROOT / "docs" / "data" / "798-core-op-shadow-receipt.json"
SUMMARY = re.compile(
    r"core-op-shadow: summary candidates=(?P<candidates>\d+) "
    r"matched=(?P<matched>\d+) mismatched=(?P<mismatched>\d+) unresolved=(?P<unresolved>\d+)"
)
DEFAULT_PREFIXES = (
    "host/",
    "integration/",
    "stdlib_host/",
    "stdlib_io/",
    "stdlib_string/",
    "stdlib_vec/",
    "stdlib_vec_ops/",
)

SPEC = importlib.util.spec_from_file_location("check_t3_wasm_validate", T3_CHECK)
assert SPEC and SPEC.loader
t3 = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(t3)


def parse_summary(stderr: str) -> dict[str, int] | None:
    matches = list(SUMMARY.finditer(stderr))
    if not matches:
        return None
    return {key: int(value) for key, value in matches[-1].groupdict().items()}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--limit", type=int, default=0, help="Maximum fixture count; zero means all")
    parser.add_argument(
        "--fixture-prefix",
        action="append",
        default=[],
        help="Only compile fixtures whose relative path starts with this prefix",
    )
    parser.add_argument("--all", action="store_true", help="Compile the complete T3 fixture suite")
    parser.add_argument("--write", type=Path, default=DEFAULT_RECEIPT)
    args = parser.parse_args()

    wasmtime = t3.find_wasmtime()
    compiler_wasm = t3.find_selfhost_wasm(t3.REPO_ROOT)
    if not wasmtime or compiler_wasm is None:
        print("FAIL: wasmtime or selfhost compiler wasm is unavailable", file=sys.stderr)
        return 2

    fixtures = list(dict.fromkeys(t3.load_t3_fixtures(t3.MANIFEST)))
    fixtures = [fixture for fixture in fixtures if fixture not in t3.T3_COMPILE_SKIP]
    prefixes = tuple(args.fixture_prefix) if args.fixture_prefix else DEFAULT_PREFIXES
    if not args.all:
        fixtures = [
            fixture
            for fixture in fixtures
            if any(fixture.startswith(prefix) for prefix in prefixes)
        ]
    if args.limit > 0:
        fixtures = fixtures[: args.limit]
    output_dir = ROOT / ".build" / "core-op-shadow"
    output_dir.mkdir(parents=True, exist_ok=True)

    totals = {"candidates": 0, "matched": 0, "mismatched": 0, "unresolved": 0}
    compiled = 0
    compile_failures: list[str] = []
    missing_summaries: list[str] = []
    nonmatching_fixtures: list[dict[str, object]] = []
    for fixture in fixtures:
        source = ROOT / "tests" / "fixtures" / fixture
        if not source.is_file():
            continue
        output = output_dir / fixture.replace("/", "_").replace(".ark", ".wasm")
        ok, stderr = t3.compile_fixture(
            wasmtime,
            compiler_wasm,
            str(Path("tests") / "fixtures" / fixture),
            output,
            ROOT,
        )
        if not ok:
            compile_failures.append(fixture)
            continue
        compiled += 1
        summary = parse_summary(stderr)
        if summary is None:
            missing_summaries.append(fixture)
            continue
        for key in totals:
            totals[key] += summary[key]
        if summary["mismatched"] > 0 or summary["unresolved"] > 0:
            nonmatching_fixtures.append({"fixture": fixture, **summary})

    receipt = {
        "schema_version": 1,
        "issue": 798,
        "suite": (
            "all T3 fixtures excluding declared compile skips"
            if args.all
            else f"targeted T3 prefixes: {', '.join(prefixes)}"
        ),
        "fixtures_selected": len(fixtures),
        "fixtures_compiled": compiled,
        "compile_failures": compile_failures,
        "fixtures_without_candidates": missing_summaries,
        "nonmatching_fixtures": nonmatching_fixtures,
        "totals": totals,
        "match_rate_percent": (
            100.0 * totals["matched"] / totals["candidates"] if totals["candidates"] else 0.0
        ),
    }
    args.write.parent.mkdir(parents=True, exist_ok=True)
    args.write.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")

    is_complete = totals["candidates"] == totals["matched"] + totals["mismatched"] + totals["unresolved"]
    passed = (
        totals["candidates"] > 0
        and is_complete
        and totals["mismatched"] == 0
        and totals["unresolved"] == 0
    )
    print(
        f"core-op shadow: compiled={compiled}/{len(fixtures)}, candidates={totals['candidates']}, "
        f"matched={totals['matched']}, mismatched={totals['mismatched']}, "
        f"unresolved={totals['unresolved']}"
    )
    print(f"receipt: {args.write.relative_to(ROOT) if args.write.is_relative_to(ROOT) else args.write}")
    if not passed:
        print("FAIL: shadow receipt is not 100% resolved and matched", file=sys.stderr)
        return 1
    print("PASS: CoreOp shadow receipt is 100% resolved and matched")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
