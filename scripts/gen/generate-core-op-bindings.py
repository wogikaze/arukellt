#!/usr/bin/env python3
"""Generate callee/intrinsic -> CoreOpId binding table for SignatureRegistry."""
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
from core_op_mapping_common import INTRINSIC_PREFIX, intended_core_op_id, normalize_key, owner_from_path  # noqa: E402
from core_op_mapping_common import CALLEE_LITERAL  # noqa: E402


def _ark_string(s: str) -> str:
    escaped = s.replace("\\", "\\\\").replace('"', '\\"')
    return f'String_from("{escaped}")'


def collect_bindings() -> dict[str, str]:
    bindings: dict[str, str] = {}

    manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    for fn in manifest.get("functions", []):
        if not isinstance(fn, dict):
            continue
        intrinsic = fn.get("intrinsic")
        core_op = fn.get("core_op_id")
        if isinstance(intrinsic, str) and isinstance(core_op, str) and core_op:
            callee = normalize_key(intrinsic)
            bindings[callee] = core_op

    for path in sorted(WASM_DIR.glob("call_*.ark")):
        owner = owner_from_path(path)
        for m in CALLEE_LITERAL.finditer(path.read_text(encoding="utf-8")):
            key = m.group(1) or m.group(2)
            norm = normalize_key(key)
            op_id = intended_core_op_id(norm, owner)
            bindings.setdefault(norm, op_id)

    return dict(sorted(bindings.items()))


def render(bindings: dict[str, str]) -> str:
    callees = list(bindings.keys())
    op_ids = [bindings[c] for c in callees]
    lines = [
        "// Generated from std/manifest.toml + legacy dispatch inventory.",
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    bindings = collect_bindings()
    rendered = render(bindings)

    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != rendered:
            print(f"FAIL: stale {OUT}", file=sys.stderr)
            return 1
        print(f"PASS: {OUT.name} fresh ({len(bindings)} bindings)")
        return 0

    OUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUT} ({len(bindings)} bindings)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
