#!/usr/bin/env python3
"""Generate compiler CoreOpRegistry tables from data/core-ops.toml."""
from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
CORE_OPS = ROOT / "data" / "core-ops.toml"
OUT = ROOT / "src" / "compiler" / "corehir" / "core_op_registry_generated.ark"

LOWERING_KIND_TO_INT = {
    "normal_call": 1,
    "mir_op": 2,
    "runtime_call": 3,
    "target_intrinsic": 4,
    "legacy_emitter": 5,
}

LAYER_TO_INT = {
    "primitive": 1,
    "runtime": 2,
    "semantic_stdlib": 3,
    "target_raw": 4,
}


def _ark_string(s: str) -> str:
    escaped = s.replace("\\", "\\\\").replace('"', '\\"')
    return f'String_from("{escaped}")'


def _handler_symbol(op_id: str) -> str:
    return "core_op_handler_" + re.sub(r"[^a-zA-Z0-9]+", "_", op_id).strip("_")


def render(ops: list[dict]) -> str:
    lines = [
        "// Generated from data/core-ops.toml by scripts/gen/generate-core-ops-registry.py.",
        "// Do not edit by hand.",
        "",
        f"fn core_op_registry_entry_count() -> i32 {{",
        f"    {len(ops)}",
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

    def emit_i32_table(name: str, values: list[int]) -> None:
        lines.append(f"fn {name}_at(index: i32) -> i32 {{")
        for i, value in enumerate(values):
            lines.append(f"    if index == {i} {{ return {value} }}")
        lines.append("    return 0 - 1")
        lines.append("}")
        lines.append("")

    ids = [op["id"] for op in ops]
    lowering = [
        LOWERING_KIND_TO_INT.get(op.get("lowering", {}).get("kind", "normal_call"), 1)
        for op in ops
    ]
    layers = [
        LAYER_TO_INT.get(op.get("classification", {}).get("layer", "semantic_stdlib"), 3)
        for op in ops
    ]
    target_ids: list[str] = []
    runtime_symbols: list[str] = []
    mir_ops: list[str] = []
    legacy_handler_ids: list[str] = []
    for op in ops:
        lowering_kind = op.get("lowering", {}).get("kind", "normal_call")
        if lowering_kind == "target_intrinsic":
            target = op.get("lowering", {}).get("target", {})
            target_ids.append(target.get("target_id", ""))
        else:
            target_ids.append("")
        if lowering_kind == "runtime_call":
            runtime = op.get("lowering", {}).get("runtime", {})
            runtime_symbols.append(runtime.get("symbol", ""))
        else:
            runtime_symbols.append("")
        if lowering_kind == "mir_op":
            mir = op.get("lowering", {}).get("mir", {})
            mir_ops.append(mir.get("operation", mir.get("opcode", "")))
        else:
            mir_ops.append("")
        if lowering_kind == "legacy_emitter":
            legacy = op.get("lowering", {}).get("legacy", {})
            legacy_handler_ids.append(legacy.get("handler_id", ""))
        else:
            legacy_handler_ids.append("")

    emit_string_table("core_op_registry_canonical_id", ids)
    emit_i32_table("core_op_registry_lowering_kind", lowering)
    emit_i32_table("core_op_registry_layer", layers)
    emit_string_table("core_op_registry_target_id", target_ids)
    emit_string_table("core_op_registry_runtime_symbol", runtime_symbols)
    emit_string_table("core_op_registry_mir_operation", mir_ops)
    emit_string_table("core_op_registry_legacy_handler_id", legacy_handler_ids)

    for index, op_id in enumerate(ids):
        lines.append(f"fn {_handler_symbol(op_id)}() -> i32 {{")
        lines.append(f"    {index}")
        lines.append("}")
        lines.append("")

    lines.extend(
        [
            "fn core_op_registry_lookup_index(canonical_id: String) -> i32 {",
            "    let count = core_op_registry_entry_count()",
            "    let mut i = 0",
            "    while i < count {",
            "        if eq(clone(canonical_id), core_op_registry_canonical_id_at(i)) {",
            "            return i",
            "        }",
            "        i = i + 1",
            "    }",
            "    return 0 - 1",
            "}",
        ]
    )
    return "\n".join(lines) + "\n"


def content_hash(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if generated file is stale")
    args = parser.parse_args()

    data = tomllib.loads(CORE_OPS.read_text(encoding="utf-8"))
    ops = sorted(data.get("operations", []), key=lambda op: op["id"])
    rendered = render(ops)

    if args.check:
        if not OUT.exists():
            print(f"FAIL: missing generated file {OUT}", file=sys.stderr)
            return 1
        if OUT.read_text(encoding="utf-8") != rendered:
            print(f"FAIL: stale {OUT}; run python3 scripts/gen/generate-core-ops-registry.py", file=sys.stderr)
            return 1
        print(f"PASS: {OUT.name} is fresh ({len(ops)} entries)")
        return 0

    OUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUT} ({len(ops)} entries, hash={content_hash(rendered)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
