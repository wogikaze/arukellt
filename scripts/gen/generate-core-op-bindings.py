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
from core_op_mapping_common import normalize_key  # noqa: E402


def _ark_string(s: str) -> str:
    escaped = s.replace("\\", "\\\\").replace('"', '\\"')
    return f'String_from("{escaped}")'


def render(alias_map: dict[str, str]) -> str:
    callees = list(alias_map.keys())
    op_ids = [alias_map[c] for c in callees]

    lines = [
        "// Generated from data/core-ops.toml legacy_bindings + std/manifest.toml.",
        "// Do not edit by hand.",
        "",
        f"fn core_op_binding_count() -> i32 {{",
        f"    {len(callees)}",
        f"}}",
        "",
    ]

    def emit_string_table(name: str, values: list[str]) -> None:
        lines.append(f"fn {name}_at(index: i32) -> String {{")
        for i, value in enumerate(values):
            lines.append(f"    if index == {i} {{ return {_ark_string(value)} }}")
        lines.append("    return String_new()")
        lines.append("}")
        lines.append("")

    emit_string_table("core_op_binding_callee", callees)
    emit_string_table("core_op_binding_core_op_id", op_ids)

    lines.extend(
        [
            "fn core_op_binding_lookup_callee(callee: String) -> i32 {",
            "    let count = core_op_binding_count()",
            "    let mut i = 0",
            "    while i < count {",
            "        if eq(clone(callee), core_op_binding_callee_at(i)) {",
            "            return i",
            "        }",
            "        i = i + 1",
            "    }",
            "    return 0 - 1",
            "}",
            "",
            "fn core_op_binding_core_op_id_for_callee(callee: String) -> String {",
            "    let index = core_op_binding_lookup_callee(clone(callee))",
            "    if index < 0 {",
            "        return String_new()",
            "    }",
            "    return core_op_binding_core_op_id_at(index)",
            "}",
        ]
    )
    return "\n".join(lines) + "\n"


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
