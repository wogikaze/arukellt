#!/usr/bin/env python3
"""ADR-040 PR-4-wide-audit: aggregate reg-vt-audit / mono-vt-audit across T3 fixtures.

Compiles each t3-compile/t3-run fixture, captures stderr audit lines, and
prints a category-bucketed summary.  Emit behavior is unchanged; this is
observation-only tooling for Lane A.

Usage:
  python3 scripts/check/reg-vt-audit-t3.py
  python3 scripts/check/reg-vt-audit-t3.py --limit 40
  python3 scripts/check/reg-vt-audit-t3.py --write .build/reg-vt-audit-summary.json
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

import importlib.util

REPO_ROOT = Path(__file__).resolve().parents[2]

_t3_spec = importlib.util.spec_from_file_location(
    "check_t3_wasm_validate",
    REPO_ROOT / "scripts" / "check" / "check-t3-wasm-validate.py",
)
assert _t3_spec and _t3_spec.loader
_t3 = importlib.util.module_from_spec(_t3_spec)
_t3_spec.loader.exec_module(_t3)

MANIFEST = _t3.MANIFEST
T3_ROOT = _t3.REPO_ROOT
T3_COMPILE_SKIP = _t3.T3_COMPILE_SKIP
compile_fixture = _t3.compile_fixture
find_selfhost_wasm = _t3.find_selfhost_wasm
find_wasmtime = _t3.find_wasmtime
load_t3_fixtures = _t3.load_t3_fixtures

REG_MISMATCH = re.compile(
    r"reg-vt-audit: mismatch callee=(?P<callee>\S+) old=(?P<old>\d+) new=(?P<new>\d+)"
)
REG_SUMMARY = re.compile(
    r"reg-vt-audit: summary candidates=(?P<cand>\d+) matched=(?P<match>\d+) mismatched=(?P<miss>\d+)"
)
MONO_MISMATCH = re.compile(
    r"mono-vt-audit: mismatch callee=(?P<callee>\S+) table=(?P<table>\d+) legacy=(?P<legacy>\d+)"
)

VT_NAMES = {
    0: "VOID",
    1: "I32",
    2: "I64",
    3: "F64",
    4: "REF",
    5: "GC_REF",
    6: "F32",
    7: "V128",
    8: "FUNCREF",
}


def fixture_category(fixture: str) -> str:
    if fixture.startswith("generics/"):
        return "generics_v1"
    if fixture.startswith("trait/") or "trait" in fixture:
        return "stdlib_trait"
    if fixture.startswith("stdlib_io/") or fixture.startswith("stdio/"):
        return "stdlib_io"
    if any(fixture.startswith(p) for p in ("vec/", "string/", "hashmap/", "hashset/", "filter/")):
        return "vec_string_map_filter"
    if any(fixture.startswith(p) for p in ("toml/", "json/", "csv/")):
        return "toml_json_csv"
    if fixture.startswith("closure/"):
        return "closure"
    if fixture.startswith("struct/") or fixture.startswith("match/"):
        return "struct_match"
    if fixture.startswith("host/") or fixture.startswith("stdlib_host/"):
        return "host_intrinsic"
    if fixture.startswith("integration/"):
        return "integration"
    return "other"


def callee_category(callee: str) -> str:
    if callee.startswith("__intrinsic_") or callee.startswith("__simd_"):
        return "host_intrinsic"
    if "__" in callee and any(
        key in callee
        for key in ("__i32", "__i64", "__f64", "__String", "__bool", "__char")
    ):
        return "mono_generic"
    if starts_traitish(callee):
        return "stdlib_trait"
    return "normal"


def starts_traitish(callee: str) -> bool:
    return "__" in callee and not callee.startswith("__")


def vt_label(v: str) -> str:
    return VT_NAMES.get(int(v), v)


def parse_stderr(stderr: str) -> dict:
    reg_mismatches = [m.groupdict() for m in REG_MISMATCH.finditer(stderr)]
    mono_mismatches = [m.groupdict() for m in MONO_MISMATCH.finditer(stderr)]
    summaries = [m.groupdict() for m in REG_SUMMARY.finditer(stderr)]
    summary = summaries[-1] if summaries else None
    return {
        "reg_mismatches": reg_mismatches,
        "mono_mismatches": mono_mismatches,
        "reg_summary": summary,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Aggregate reg-vt-audit across T3 fixtures")
    parser.add_argument("--limit", type=int, default=0, help="Max fixtures to scan (0=all)")
    parser.add_argument("--write", type=Path, default=None, help="Write JSON summary path")
    parser.add_argument("--fixture-prefix", action="append", default=[], help="Only fixtures with prefix")
    args = parser.parse_args()

    wasmtime = find_wasmtime()
    if not wasmtime:
        print("error: wasmtime not found", file=sys.stderr)
        return 2
    compiler_wasm = find_selfhost_wasm(T3_ROOT)
    if compiler_wasm is None:
        print("error: selfhost wasm not found", file=sys.stderr)
        return 2

    fixtures = load_t3_fixtures(MANIFEST)
    seen: set[str] = set()
    unique: list[str] = []
    for f in fixtures:
        if f in seen:
            continue
        seen.add(f)
        if f in T3_COMPILE_SKIP:
            continue
        if args.fixture_prefix and not any(f.startswith(p) for p in args.fixture_prefix):
            continue
        unique.append(f)
    if args.limit > 0:
        unique = unique[: args.limit]

    tmpdir = T3_ROOT / ".build" / "reg-vt-audit-tmp"
    tmpdir.mkdir(parents=True, exist_ok=True)

    totals = Counter(candidates=0, matched=0, mismatched=0)
    fixture_hits: list[dict] = []
    reg_pair_counts: Counter[str] = Counter()
    mono_pair_counts: Counter[str] = Counter()
    by_fixture_cat: dict[str, Counter] = defaultdict(Counter)
    by_callee_cat: dict[str, Counter] = defaultdict(Counter)

    compiled = 0
    for fixture in unique:
        src = str(Path("tests") / "fixtures" / fixture)
        src_abs = T3_ROOT / src
        if not src_abs.is_file():
            continue
        out = tmpdir / fixture.replace("/", "_").replace(".ark", ".wasm")
        ok, stderr = compile_fixture(wasmtime, compiler_wasm, src, out, T3_ROOT)
        if not ok:
            continue
        compiled += 1
        parsed = parse_stderr(stderr)
        cat = fixture_category(fixture)
        if parsed["reg_summary"]:
            s = parsed["reg_summary"]
            totals["candidates"] += int(s["cand"])
            totals["matched"] += int(s["match"])
            totals["mismatched"] += int(s["miss"])
            by_fixture_cat[cat]["candidates"] += int(s["cand"])
            by_fixture_cat[cat]["matched"] += int(s["match"])
            by_fixture_cat[cat]["mismatched"] += int(s["miss"])
        for m in parsed["reg_mismatches"]:
            pair = f"old={vt_label(m['old'])} new={vt_label(m['new'])}"
            reg_pair_counts[pair] += 1
            cc = callee_category(m["callee"])
            by_callee_cat[cc]["reg_mismatch"] += 1
            by_fixture_cat[cat]["reg_lines"] += 1
        for m in parsed["mono_mismatches"]:
            pair = f"table={vt_label(m['table'])} legacy={vt_label(m['legacy'])}"
            mono_pair_counts[pair] += 1
            cc = callee_category(m["callee"])
            by_callee_cat[cc]["mono_mismatch"] += 1
            by_fixture_cat[cat]["mono_lines"] += 1
        if parsed["reg_mismatches"] or parsed["mono_mismatches"] or parsed["reg_summary"]:
            fixture_hits.append({"fixture": fixture, "category": cat, **parsed})

    report = {
        "fixtures_scanned": len(unique),
        "fixtures_compiled": compiled,
        "reg_totals": dict(totals),
        "reg_mismatch_pairs": reg_pair_counts.most_common(20),
        "mono_mismatch_pairs": mono_pair_counts.most_common(20),
        "by_fixture_category": {k: dict(v) for k, v in sorted(by_fixture_cat.items())},
        "by_callee_category": {k: dict(v) for k, v in sorted(by_callee_cat.items())},
        "fixture_hits": fixture_hits,
    }

    print(f"reg-vt-audit T3 scan: {compiled}/{len(unique)} compiled")
    if totals["candidates"]:
        rate = 100.0 * totals["matched"] / totals["candidates"]
        print(
            f"  reg totals: candidates={totals['candidates']} matched={totals['matched']} "
            f"mismatched={totals['mismatched']} match_rate={rate:.1f}%"
        )
    if reg_pair_counts:
        print("  top reg mismatch pairs:")
        for pair, count in reg_pair_counts.most_common(8):
            print(f"    {pair}: {count}")
    if mono_pair_counts:
        print("  top mono mismatch pairs:")
        for pair, count in mono_pair_counts.most_common(8):
            print(f"    {pair}: {count}")
    if by_fixture_cat:
        print("  by fixture category:")
        for cat, counts in sorted(by_fixture_cat.items()):
            print(f"    {cat}: {dict(counts)}")

    if args.write:
        args.write.parent.mkdir(parents=True, exist_ok=True)
        args.write.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"  wrote {args.write}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
