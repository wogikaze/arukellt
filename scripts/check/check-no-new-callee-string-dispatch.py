#!/usr/bin/env python3
"""Ratchet gate: forbid new callee-string dispatch patterns in call_*.ark.

Counts `eq(clone(callee),` occurrences under src/compiler/wasm/call_*.ark and
compares against docs/data/callee-string-dispatch-baseline.toml.
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
WASM_CALL_GLOB = "call_*.ark"
BASELINE_PATH = ROOT / "docs" / "data" / "callee-string-dispatch-baseline.toml"
PATTERN = re.compile(r"eq\(clone\(callee\)")


def _rel(path: Path) -> str:
    return str(path.relative_to(ROOT)).replace("\\", "/")


def scan_dispatch_patterns() -> tuple[int, list[tuple[str, int]]]:
    wasm_dir = ROOT / "src" / "compiler" / "wasm"
    total = 0
    hits: list[tuple[str, int]] = []
    for path in sorted(wasm_dir.glob(WASM_CALL_GLOB)):
        text = path.read_text(encoding="utf-8")
        count = len(PATTERN.findall(text))
        if count:
            hits.append((_rel(path), count))
            total += count
    return total, hits


def load_baseline() -> int:
    data = tomllib.loads(BASELINE_PATH.read_text(encoding="utf-8"))
    counts = data.get("counts", {})
    value = counts.get("eq_clone_callee_patterns")
    if not isinstance(value, int):
        raise SystemExit(f"FAIL: {BASELINE_PATH}: counts.eq_clone_callee_patterns missing or not int")
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write-baseline", action="store_true")
    parser.add_argument("--issue", type=int, default=0)
    args = parser.parse_args()

    total, hits = scan_dispatch_patterns()
    if args.write_baseline:
        if args.issue <= 0:
            print("FAIL: --write-baseline requires --issue", file=sys.stderr)
            return 1
        text = BASELINE_PATH.read_text(encoding="utf-8")
        updated = re.sub(
            r"eq_clone_callee_patterns = \d+",
            f"eq_clone_callee_patterns = {total}",
            text,
            count=1,
        )
        updated = re.sub(
            r"last_update_issue = \d+",
            f"last_update_issue = {args.issue}",
            updated,
            count=1,
        )
        BASELINE_PATH.write_text(updated, encoding="utf-8")
        print(f"wrote baseline: {_rel(BASELINE_PATH)} eq_clone_callee_patterns={total}")
        return 0

    baseline = load_baseline()
    if total > baseline:
        print(
            f"FAIL: callee-string dispatch patterns increased: {total} > baseline {baseline}",
            file=sys.stderr,
        )
        for rel, count in hits:
            print(f"  {rel}: {count}", file=sys.stderr)
        return 1
    print(f"PASS: callee-string dispatch ratchet ({total} <= {baseline})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
