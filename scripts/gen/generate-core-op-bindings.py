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


def collect_family_patterns(core_ops: dict | None = None) -> list[dict[str, object]]:
    if core_ops is None:
        core_ops = tomllib.loads(CORE_OPS.read_text(encoding="utf-8"))
    operations = {
        op.get("id"): op
        for op in core_ops.get("operations", [])
        if isinstance(op, dict) and isinstance(op.get("id"), str)
    }
    patterns: list[dict[str, object]] = []
    seen_prefixes: dict[str, str] = {}
    for entry in core_ops.get("legacy_binding_patterns", []):
        if not isinstance(entry, dict):
            raise ValueError("legacy_binding_patterns entries must be tables")
        kind = entry.get("kind")
        pattern = entry.get("pattern")
        core_op_id = entry.get("core_op_id")
        precedence = entry.get("precedence", "after_exact")
        require_nonempty_suffix = bool(entry.get("require_nonempty_suffix", True))
        if kind != "prefix":
            raise ValueError(f"unsupported legacy_binding_patterns.kind: {kind!r}")
        if not isinstance(pattern, str) or not pattern:
            raise ValueError("legacy_binding_patterns.pattern must be a non-empty string")
        if not isinstance(core_op_id, str) or not core_op_id:
            raise ValueError(f"legacy_binding_patterns for {pattern!r}: core_op_id required")
        if precedence != "after_exact":
            raise ValueError(
                f"legacy_binding_patterns for {pattern!r}: unsupported precedence {precedence!r}"
            )
        if core_op_id not in operations:
            raise ValueError(
                f"legacy_binding_patterns for {pattern!r}: unknown core_op_id {core_op_id!r}"
            )
        previous = seen_prefixes.get(pattern)
        if previous is not None and previous != core_op_id:
            raise ValueError(
                f"conflicting legacy_binding_patterns for {pattern!r}: {previous} vs {core_op_id}"
            )
        seen_prefixes[pattern] = core_op_id
        patterns.append(
            {
                "kind": kind,
                "pattern": pattern,
                "core_op_id": core_op_id,
                "precedence": precedence,
                "require_nonempty_suffix": require_nonempty_suffix,
            }
        )
    # Longest prefix first for deterministic match order.
    patterns.sort(key=lambda item: (-len(str(item["pattern"])), str(item["pattern"])))
    return patterns


def render(alias_map: dict[str, str], patterns: list[dict[str, object]]) -> str:
    callees = list(alias_map.keys())
    op_ids = [alias_map[c] for c in callees]

    lines = [
        "// Generated from data/core-ops.toml legacy_bindings +",
        "// legacy_binding_patterns + std/manifest.toml.",
        "// Do not edit by hand.",
        "",
        "fn core_op_binding_count() -> i32 {",
        f"    {len(callees)}",
        "}",
        "",
        "fn core_op_binding_pattern_count() -> i32 {",
        f"    {len(patterns)}",
        "}",
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
    emit_string_table("core_op_binding_pattern", [str(p["pattern"]) for p in patterns])
    emit_string_table("core_op_binding_pattern_core_op_id", [str(p["core_op_id"]) for p in patterns])

    lines.append("fn core_op_binding_pattern_requires_nonempty_suffix_at(index: i32) -> bool {")
    for i, pattern in enumerate(patterns):
        flag = "true" if pattern["require_nonempty_suffix"] else "false"
        lines.append(f"    if index == {i} {{ return {flag} }}")
    lines.append("    return true")
    lines.append("}")
    lines.append("")

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
            # Exact-only API: keep historical meaning for Wasm effective lowering.
            "fn core_op_binding_core_op_id_for_callee_exact(callee: String) -> String {",
            "    let index = core_op_binding_lookup_callee(clone(callee))",
            "    if index < 0 {",
            "        return String_new()",
            "    }",
            "    return core_op_binding_core_op_id_at(index)",
            "}",
            "",
            "fn core_op_binding_core_op_id_for_callee_family(callee: String) -> String {",
            "    let count = core_op_binding_pattern_count()",
            "    let mut i = 0",
            "    while i < count {",
            "        let pattern = core_op_binding_pattern_at(i)",
            "        if starts_with(clone(callee), clone(pattern)) {",
            "            let suffix_len = len(callee) - len(pattern)",
            "            if core_op_binding_pattern_requires_nonempty_suffix_at(i) && suffix_len <= 0 {",
            "                i = i + 1",
            "                continue",
            "            }",
            "            return core_op_binding_pattern_core_op_id_at(i)",
            "        }",
            "        i = i + 1",
            "    }",
            "    return String_new()",
            "}",
            "",
            # Compatibility wrapper: exact only (Wasm / historical callers).
            "fn core_op_binding_core_op_id_for_callee(callee: String) -> String {",
            "    return core_op_binding_core_op_id_for_callee_exact(clone(callee))",
            "}",
            "",
            # MIR call_func_id path: exact then family.
            "fn core_op_binding_core_op_id_for_callee_exact_then_family(callee: String) -> String {",
            "    let exact = core_op_binding_core_op_id_for_callee_exact(clone(callee))",
            "    if len(exact) > 0 {",
            "        return exact",
            "    }",
            "    return core_op_binding_core_op_id_for_callee_family(clone(callee))",
            "}",
        ]
    )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    core_ops = tomllib.loads(CORE_OPS.read_text(encoding="utf-8"))
    alias_map = collect_bindings()
    patterns = collect_family_patterns(core_ops)
    rendered = render(alias_map, patterns)

    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != rendered:
            print(f"FAIL: stale {OUT}", file=sys.stderr)
            return 1
        print(
            f"PASS: {OUT.name} fresh ({len(alias_map)} bindings, {len(patterns)} patterns)"
        )
        return 0

    OUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUT} ({len(alias_map)} bindings, {len(patterns)} patterns)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
