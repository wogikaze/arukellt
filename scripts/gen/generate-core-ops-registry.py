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
GEN_DIR = Path(__file__).resolve().parent
CORE_OPS = ROOT / "data" / "core-ops.toml"
OUT = ROOT / "src" / "compiler" / "corehir" / "core_op_registry_generated.ark"

sys.path.insert(0, str(GEN_DIR))
from ark_fnv_index import emit_lookup_fn  # noqa: E402
from ark_table_blob import emit_i32_table, emit_string_table  # noqa: E402

LOWERING_KIND_TO_INT = {
    "normal_call": 1,
    "mir_op": 2,
    "runtime_call": 3,
    "target_intrinsic": 4,
    "legacy_emitter": 5,
}

# Discriminator for [operations.lowering.runtime].kind
RUNTIME_KIND_TO_INT = {
    "": 0,
    "internal": 1,
    "wit": 2,
    "native": 3,
}

LAYER_TO_INT = {
    "primitive": 1,
    "runtime": 2,
    "semantic_stdlib": 3,
    "target_raw": 4,
}

INLINE_POLICY_TO_INT = {
    "never": 0,
    "hint": 1,
    "always": 2,
}


def _handler_symbol(op_id: str) -> str:
    return "core_op_handler_" + re.sub(r"[^a-zA-Z0-9]+", "_", op_id).strip("_")


def render(ops: list[dict]) -> str:
    lines = [
        "// Generated from data/core-ops.toml by scripts/gen/generate-core-ops-registry.py.",
        "// Compact table + index. Do not edit by hand.",
        "",
        "use corehir::table_blob",
        "",
        f"fn core_op_registry_entry_count() -> i32 {{",
        f"    {len(ops)}",
        f"}}",
        "",
    ]

    ids = [op["id"] for op in ops]
    lowering = [
        LOWERING_KIND_TO_INT.get(op.get("lowering", {}).get("kind", "normal_call"), 1)
        for op in ops
    ]
    layers = [
        LAYER_TO_INT.get(op.get("classification", {}).get("layer", "semantic_stdlib"), 3)
        for op in ops
    ]
    inline_policies = [
        INLINE_POLICY_TO_INT.get(op.get("inline", {}).get("policy", "never"), 0)
        for op in ops
    ]
    target_ids: list[str] = []
    runtime_kinds: list[int] = []
    runtime_symbols: list[str] = []
    wit_packages: list[str] = []
    wit_interfaces: list[str] = []
    wit_functions: list[str] = []
    wit_versions: list[str] = []
    mir_ops: list[str] = []
    legacy_handler_ids: list[str] = []
    fallback_symbols: list[str] = []
    for op in ops:
        lowering_kind = op.get("lowering", {}).get("kind", "normal_call")
        if lowering_kind == "target_intrinsic":
            target = op.get("lowering", {}).get("target", {})
            target_ids.append(target.get("target_id", ""))
        else:
            target_ids.append("")
        if lowering_kind == "runtime_call":
            runtime = op.get("lowering", {}).get("runtime", {})
            runtime_kind = runtime.get("kind", "internal")
            runtime_kinds.append(RUNTIME_KIND_TO_INT.get(runtime_kind, 0))
            if runtime_kind == "wit":
                runtime_symbols.append("")
                wit_packages.append(runtime.get("package", ""))
                wit_interfaces.append(runtime.get("interface", ""))
                wit_functions.append(runtime.get("function", ""))
                wit_versions.append(runtime.get("version", ""))
            else:
                runtime_symbols.append(runtime.get("symbol", ""))
                wit_packages.append("")
                wit_interfaces.append("")
                wit_functions.append("")
                wit_versions.append("")
        else:
            runtime_kinds.append(0)
            runtime_symbols.append("")
            wit_packages.append("")
            wit_interfaces.append("")
            wit_functions.append("")
            wit_versions.append("")
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
        fallback_symbols.append(op.get("fallback", {}).get("implementation_symbol", ""))

    lines.extend(emit_string_table("core_op_registry_canonical_id", ids))
    lines.extend(emit_i32_table("core_op_registry_lowering_kind", lowering))
    lines.extend(emit_i32_table("core_op_registry_layer", layers))
    lines.extend(emit_i32_table("core_op_registry_inline_policy", inline_policies, default=0))
    lines.extend(emit_string_table("core_op_registry_target_id", target_ids))
    lines.extend(emit_i32_table("core_op_registry_runtime_kind", runtime_kinds, default=0))
    lines.extend(emit_string_table("core_op_registry_runtime_symbol", runtime_symbols))
    lines.extend(emit_string_table("core_op_registry_wit_package", wit_packages))
    lines.extend(emit_string_table("core_op_registry_wit_interface", wit_interfaces))
    lines.extend(emit_string_table("core_op_registry_wit_function", wit_functions))
    lines.extend(emit_string_table("core_op_registry_wit_version", wit_versions))
    lines.extend(emit_string_table("core_op_registry_mir_operation", mir_ops))
    lines.extend(emit_string_table("core_op_registry_legacy_handler_id", legacy_handler_ids))
    lines.extend(emit_string_table("core_op_registry_fallback_symbol", fallback_symbols))
    lines.extend(
        [
            "fn core_op_registry_has_fallback_symbol(symbol: String) -> bool {",
            "    let count = core_op_registry_entry_count()",
            "    let mut index = 0",
            "    while index < count {",
            "        if eq(clone(symbol), core_op_registry_fallback_symbol_at(index)) {",
            "            return true",
            "        }",
            "        index = index + 1",
            "    }",
            "    false",
            "}",
            "",
        ]
    )

    for index, op_id in enumerate(ids):
        lines.append(f"fn {_handler_symbol(op_id)}() -> i32 {{")
        lines.append(f"    {index}")
        lines.append("}")
        lines.append("")

    lines.extend(
        emit_lookup_fn(
            "core_op_registry_lookup_index",
            "core_op_registry_canonical_id_at",
            "core_op_registry_bucket_at",
            ids,
        )
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
