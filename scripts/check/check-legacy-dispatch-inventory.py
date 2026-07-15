#!/usr/bin/env python3
"""Validate the frozen migration alias inventory and absence of string dispatch."""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
CORE_OPS = ROOT / "data" / "core-ops.toml"
WASM_DIR = ROOT / "src" / "compiler" / "wasm"
LEGACY_COMPARISON = re.compile(r"\beq\s*\(\s*clone\s*\(\s*callee\s*\)")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    data = tomllib.loads(CORE_OPS.read_text(encoding="utf-8"))
    core_op_ids = {
        op["id"] for op in data.get("operations", [])
        if isinstance(op, dict) and isinstance(op.get("id"), str)
    }
    aliases: dict[str, str] = {}
    conflicts: list[str] = []
    missing_core_ops: list[dict[str, str]] = []
    for binding in data.get("legacy_bindings", []):
        if not isinstance(binding, dict):
            continue
        alias = binding.get("alias")
        core_op_id = binding.get("core_op_id")
        if not isinstance(alias, str) or not isinstance(core_op_id, str):
            conflicts.append("legacy binding must contain string alias and core_op_id")
            continue
        previous = aliases.get(alias)
        if previous is not None and previous != core_op_id:
            conflicts.append(f"{alias}: {previous} vs {core_op_id}")
        aliases[alias] = core_op_id
        if core_op_id not in core_op_ids:
            missing_core_ops.append({"alias": alias, "core_op_id": core_op_id})

    string_dispatch: list[str] = []
    for source in sorted(WASM_DIR.glob("call_*.ark")):
        for line_number, line in enumerate(source.read_text(encoding="utf-8").splitlines(), 1):
            if LEGACY_COMPARISON.search(line):
                string_dispatch.append(f"{source.relative_to(ROOT)}:{line_number}")

    report = {
        "legacy_aliases": len(aliases),
        "mapped_core_ops": len(set(aliases.values())),
        "core_op_entries": len(core_op_ids),
        "conflicts": conflicts,
        "missing_core_ops": missing_core_ops,
        "callee_string_dispatch": string_dispatch,
    }
    failed = bool(conflicts or missing_core_ops or string_dispatch or not aliases)
    if args.json:
        print(json.dumps(report, indent=2))
        return 1 if failed else 0

    print(
        f"migration aliases={len(aliases)}, mapped_core_ops={len(set(aliases.values()))}, "
        f"callee_string_dispatch={len(string_dispatch)}"
    )
    if failed:
        for problem in conflicts:
            print(f"FAIL: conflicting binding: {problem}")
        for problem in missing_core_ops:
            print(f"FAIL: unknown CoreOpId: {problem['alias']} -> {problem['core_op_id']}")
        for location in string_dispatch:
            print(f"FAIL: callee string dispatch remains at {location}")
        if not aliases:
            print("FAIL: frozen legacy binding inventory is empty")
        return 1
    print("PASS: frozen aliases are valid and helper dispatch uses typed handler IDs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
