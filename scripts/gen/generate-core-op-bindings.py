#!/usr/bin/env python3
"""Generate the migration-only callee/intrinsic -> CoreOpId table."""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
GEN_DIR = Path(__file__).resolve().parent
MANIFEST = ROOT / "std" / "manifest.toml"
CORE_OPS = ROOT / "data" / "core-ops.toml"
OUT = ROOT / "src" / "compiler" / "corehir" / "core_op_binding_generated.ark"

sys.path.insert(0, str(GEN_DIR))
from ark_fnv_index import emit_lookup_fn  # noqa: E402
from ark_table_blob import emit_string_table  # noqa: E402
from core_op_mapping_common import normalize_key  # noqa: E402


def collect_bindings() -> dict[str, str]:
    core_ops = tomllib.loads(CORE_OPS.read_text(encoding="utf-8"))
    alias_map: dict[str, str] = {}
    for binding in core_ops.get("legacy_bindings", []):
        if not isinstance(binding, dict):
            continue
        alias = binding.get("alias")
        core_op_id = binding.get("core_op_id")
        if isinstance(alias, str) and isinstance(core_op_id, str):
            previous = alias_map.get(alias)
            if previous is not None and previous != core_op_id:
                raise ValueError(f"conflicting legacy binding for {alias}: {previous} vs {core_op_id}")
            alias_map[alias] = core_op_id

    # Fallback implementation symbols are compiler-visible callees too.  They
    # must carry the same CoreOp identity as their public operation so a
    # fallback call reaches the semantic handler instead of an empty body.
    for operation in core_ops.get("operations", []):
        if not isinstance(operation, dict):
            continue
        fallback = operation.get("fallback", {})
        if not isinstance(fallback, dict):
            continue
        symbol = fallback.get("implementation_symbol")
        core_op_id = operation.get("id")
        if not isinstance(symbol, str) or not symbol:
            continue
        if not isinstance(core_op_id, str) or not core_op_id:
            continue
        previous = alias_map.get(symbol)
        if previous is not None and previous != core_op_id:
            raise ValueError(f"conflicting fallback binding for {symbol}: {previous} vs {core_op_id}")
        alias_map[symbol] = core_op_id

    manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    for fn in manifest.get("functions", []):
        if not isinstance(fn, dict):
            continue
        intrinsic = fn.get("intrinsic")
        core_op = fn.get("core_op_id")
        if isinstance(intrinsic, str) and isinstance(core_op, str) and core_op:
            alias_map[normalize_key(intrinsic)] = core_op
            alias_map[intrinsic] = core_op

    return dict(sorted(alias_map.items()))


def render(alias_map: dict[str, str]) -> str:
    callees = list(alias_map.keys())
    op_ids = [alias_map[c] for c in callees]

    lines = [
        "// Generated from data/core-ops.toml legacy_bindings + std/manifest.toml.",
        "// Compact table + index. Do not edit by hand.",
        "",
        "use std::prelude::{byte_at, substring}",
        "use std::text",
        "use corehir::table_blob",
        "",
        "fn core_op_binding_count() -> i32 {",
        f"    {len(callees)}",
        "}",
        "",
    ]

    lines.extend(emit_string_table("core_op_binding_callee", callees))
    lines.extend(emit_string_table("core_op_binding_core_op_id", op_ids))

    lines.extend(
        emit_lookup_fn(
            "core_op_binding_lookup_callee",
            "core_op_binding_callee_at",
            "core_op_binding_bucket_at",
            callees,
        )
    )
    lines.extend(
        [
            "",
            "fn core_op_binding_core_op_id_for_callee(callee: String) -> String {",
            "    let index = core_op_binding_lookup_callee(callee.clone())",
            "    if index < 0 {",
            "        return String_new()",
            "    }",
            "    return core_op_binding_core_op_id_at(index)",
            "}",
        ]
    )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    alias_map = collect_bindings()
    rendered = render(alias_map)

    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != rendered:
            print(f"FAIL: stale {OUT}", file=sys.stderr)
            return 1
        print(f"PASS: {OUT.name} fresh ({len(alias_map)} bindings)")
        return 0

    OUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUT} ({len(alias_map)} bindings)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
