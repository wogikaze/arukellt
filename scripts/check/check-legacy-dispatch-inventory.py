#!/usr/bin/env python3
"""Compare legacy handler-branch inventory against CoreOp registry coverage.

Inventory unit is a handler branch (OR'd aliases), not each callee string.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
CORE_OPS = ROOT / "data" / "core-ops.toml"
WASM_DIR = ROOT / "src" / "compiler" / "wasm"
GEN_DIR = ROOT / "scripts" / "gen"

sys.path.insert(0, str(GEN_DIR))
from core_op_mapping_common import extract_handler_branches  # noqa: E402


def load_core_op_ids() -> set[str]:
    data = tomllib.loads(CORE_OPS.read_text(encoding="utf-8"))
    return {op["id"] for op in data.get("operations", []) if isinstance(op, dict) and "id" in op}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    branches = extract_handler_branches(WASM_DIR)
    core_ops = load_core_op_ids()

    mapped = 0
    unmapped = []
    for branch in branches:
        if branch.core_op_id in core_ops:
            mapped += 1
        else:
            unmapped.append(
                {
                    "core_op_id": branch.core_op_id,
                    "handler_key": branch.handler_key,
                    "aliases": list(branch.aliases),
                    "owner": branch.owner,
                    "source": branch.source_file,
                }
            )

    alias_count = sum(len(b.aliases) for b in branches)
    report = {
        "handler_branches": len(branches),
        "legacy_alias_literals": alias_count,
        "core_op_entries": len(core_ops),
        "mapped_branches": mapped,
        "unmapped_branches": len(unmapped),
        "unmapped_samples": unmapped[:20],
    }

    if args.json:
        print(json.dumps(report, indent=2))
        return 0 if not unmapped else 1

    print(
        f"legacy inventory: {len(branches)} handler branches "
        f"({alias_count} aliases), mapped={mapped}, unmapped={len(unmapped)}, "
        f"core_ops={len(core_ops)}"
    )
    if unmapped:
        print("FAIL: unmapped handler branches (first 20):", file=sys.stderr)
        for item in unmapped[:20]:
            print(
                f"  {item['core_op_id']} handler={item['handler_key']} "
                f"aliases={item['aliases']} ({item['source']})",
                file=sys.stderr,
            )
        return 1
    print("PASS: all legacy handler branches have CoreOp coverage")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
