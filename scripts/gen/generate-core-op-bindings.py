#!/usr/bin/env python3
"""Generate callee/intrinsic -> CoreOpId and CoreOpId -> legacy handler key tables."""
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
WASM_DIR = ROOT / "src" / "compiler" / "wasm"
OUT = ROOT / "src" / "compiler" / "corehir" / "core_op_binding_generated.ark"

sys.path.insert(0, str(GEN_DIR))
from core_op_mapping_common import (  # noqa: E402
    alias_to_core_op_map,
    core_op_to_handler_map,
    extract_handler_branches,
    normalize_key,
)


def _ark_string(s: str) -> str:
    escaped = s.replace("\\", "\\\\").replace('"', '\\"')
    return f'String_from("{escaped}")'


def render(alias_map: dict[str, str], handler_map: dict[str, str], owner_map: dict[str, str]) -> str:
    callees = list(alias_map.keys())
    op_ids = [alias_map[c] for c in callees]
    handlers = list(handler_map.keys())
    handler_keys = [handler_map[h] for h in handlers]
    owner_keys = [owner_map.get(h, "unknown") for h in handlers]

    lines = [
        "// Generated from legacy handler branches + std/manifest.toml.",
        "// Do not edit by hand.",
        "",
        f"fn core_op_binding_count() -> i32 {{",
        f"    {len(callees)}",
        f"}}",
        "",
        f"fn core_op_handler_count() -> i32 {{",
        f"    {len(handlers)}",
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
    emit_string_table("core_op_handler_core_op_id", handlers)
    emit_string_table("core_op_handler_key", handler_keys)
    emit_string_table("core_op_handler_owner", owner_keys)

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
            "",
            "fn core_op_handler_lookup(core_op_id: String) -> i32 {",
            "    let count = core_op_handler_count()",
            "    let mut i = 0",
            "    while i < count {",
            "        if eq(clone(core_op_id), core_op_handler_core_op_id_at(i)) {",
            "            return i",
            "        }",
            "        i = i + 1",
            "    }",
            "    return 0 - 1",
            "}",
            "",
            "fn core_op_handler_key_for_core_op_id(core_op_id: String) -> String {",
            "    let index = core_op_handler_lookup(clone(core_op_id))",
            "    if index < 0 {",
            "        return String_new()",
            "    }",
            "    return core_op_handler_key_at(index)",
            "}",
            "",
            "fn core_op_handler_owner_for_core_op_id(core_op_id: String) -> String {",
            "    let index = core_op_handler_lookup(clone(core_op_id))",
            "    if index < 0 {",
            "        return String_new()",
            "    }",
            "    return core_op_handler_owner_at(index)",
            "}",
            "",
            "fn core_op_legacy_handler_id_for_core_op_id(core_op_id: String) -> String {",
            "    let owner = core_op_handler_owner_for_core_op_id(clone(core_op_id))",
            "    let key = core_op_handler_key_for_core_op_id(clone(core_op_id))",
            "    if len(owner) == 0 || len(key) == 0 {",
            "        return String_new()",
            "    }",
            "    return concat(owner, concat(String_from(\":\"), key))",
            "}",
        ]
    )
    return "\n".join(lines) + "\n"


def collect_bindings() -> tuple[dict[str, str], dict[str, str], dict[str, str]]:
    branches = extract_handler_branches(WASM_DIR)
    alias_map = alias_to_core_op_map(branches)
    handler_map = core_op_to_handler_map(branches)
    owner_map = {b.core_op_id: b.owner for b in branches}

    manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    for fn in manifest.get("functions", []):
        if not isinstance(fn, dict):
            continue
        intrinsic = fn.get("intrinsic")
        core_op = fn.get("core_op_id")
        if isinstance(intrinsic, str) and isinstance(core_op, str) and core_op:
            alias_map[normalize_key(intrinsic)] = core_op
            alias_map[intrinsic] = core_op
            if core_op not in handler_map:
                handler_map[core_op] = normalize_key(intrinsic)
            if core_op not in owner_map:
                owner_map[core_op] = "manifest"

    return dict(sorted(alias_map.items())), dict(sorted(handler_map.items())), dict(sorted(owner_map.items()))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    alias_map, handler_map, owner_map = collect_bindings()
    rendered = render(alias_map, handler_map, owner_map)

    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != rendered:
            print(f"FAIL: stale {OUT}", file=sys.stderr)
            return 1
        print(f"PASS: {OUT.name} fresh ({len(alias_map)} bindings, {len(handler_map)} handlers)")
        return 0

    OUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUT} ({len(alias_map)} bindings, {len(handler_map)} handlers)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
