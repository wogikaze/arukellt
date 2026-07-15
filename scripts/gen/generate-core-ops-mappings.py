#!/usr/bin/env python3
"""Generate CoreOp registry entries from legacy callee-string dispatch inventory.

Reads src/compiler/wasm/call_*.ark dispatch keys and emits TOML operations for
data/core-ops.toml. Does not replace hand-authored entries; merges by id.
"""
from __future__ import annotations

import argparse
import sys
from collections import defaultdict
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
CORE_OPS = ROOT / "data" / "core-ops.toml"
WASM_DIR = ROOT / "src" / "compiler" / "wasm"
from core_op_mapping_common import (  # noqa: E402
    CALLEE_LITERAL,
    INTRINSIC_PREFIX,
    MIR_OP_KEYS,
    RUNTIME_EXACT,
    RUNTIME_PREFIXES,
    intended_core_op_id,
    normalize_key as _normalize,
    owner_from_path as _owner,
)


def classify_layer_and_lowering(op_id: str, norm: str) -> tuple[str, str, dict | None]:
    if op_id.startswith("primitive."):
        return "primitive", "mir_op", {"opcode": norm, "operation": norm}
    if op_id.startswith("runtime."):
        symbol = op_id.removeprefix("runtime.")
        return (
            "runtime",
            "runtime_call",
            {"kind": "internal", "symbol": symbol.replace(".", "_"), "abi_version": "0.1"},
        )
    if op_id.startswith("simd.") and norm.startswith("__simd_"):
        target_id = norm.removeprefix("__simd_").replace("_", ".")
        return (
            "target_raw",
            "target_intrinsic",
            {
                "target_family": "wasm",
                "target_id": target_id,
                "required_capabilities": [],
                "required_target_features": ["simd128"],
            },
        )
    if op_id.startswith("wasm."):
        return "target_raw", "target_intrinsic", None
    return "semantic_stdlib", "normal_call", None


def emit_operation(op_id: str, norm: str, aliases: list[str]) -> str:
    layer, lowering_kind, payload = classify_layer_and_lowering(op_id, norm)
    visibility = "internal" if op_id.startswith(("primitive.", "simd.")) and not op_id.startswith(
        "simd.i32x4"
    ) else "public"
    if op_id.startswith("simd.") and "__simd_" in norm:
        visibility = "internal"
    binding = "optional" if visibility == "internal" else "forbidden"
    if op_id in {"string.starts_with", "string.ends_with", "panic", "wasm.v128.load"}:
        binding = "required"

    lines = [
        "",
        "[[operations]]",
        f'id = "{op_id}"',
        f'visibility = "{visibility}"',
        f'classification = {{ layer = "{layer}" }}',
    ]
    if binding == "optional":
        lines.append(
            'binding = { policy = "optional", reason = "legacy dispatch migration", tracking_issue = "798" }'
        )
    else:
        lines.append(f'binding = {{ policy = "{binding}" }}')
    lines.append(f'description = "Migrated legacy dispatch: {norm} (aliases: {len(aliases)})"')
    lines.append("[operations.signature]")
    lines.append('inputs = [{ name = "arg0", type = { kind = "primitive", name = "i32" } }]')
    lines.append('outputs = [{ type = { kind = "primitive", name = "i32" } }]')
    lines.append("generic_params = []")
    lines.append("constraints = []")
    lines.append("[operations.semantics]")
    lines.append("const_evaluable = false")
    lines.append('overflow = "none"')
    lines.append('nan = "none"')
    lines.append('trap = "none"')
    lines.append('equivalence = "exact_bitwise"')
    lines.append("[operations.effect]")
    lines.append('memory = "none"')
    lines.append("allocates = false")
    lines.append("may_trap = false")
    lines.append("noreturn = false")
    lines.append("external_io = false")
    lines.append('nondeterminism = "deterministic"')
    lines.append("atomic = false")
    lines.append("volatile = false")
    lines.append("[operations.inline]")
    lines.append('policy = "never"')
    lines.append("[operations.lowering]")
    lines.append(f'kind = "{lowering_kind}"')
    if lowering_kind == "mir_op" and payload:
        lines.append("[operations.lowering.mir]")
        lines.append(f'opcode = "{payload["opcode"]}"')
        lines.append(f'operation = "{payload["operation"]}"')
    if lowering_kind == "runtime_call" and payload:
        lines.append("[operations.lowering.runtime]")
        lines.append(f'kind = "{payload["kind"]}"')
        lines.append(f'symbol = "{payload["symbol"]}"')
        lines.append(f'abi_version = "{payload["abi_version"]}"')
    if lowering_kind == "target_intrinsic" and payload:
        lines.append("[operations.lowering.target]")
        for k, v in payload.items():
            if isinstance(v, list):
                inner = ", ".join(f'"{x}"' for x in v)
                lines.append(f"{k} = [{inner}]")
            else:
                lines.append(f'{k} = "{v}"')
    if lowering_kind == "normal_call":
        lines.append("[operations.fallback]")
        lines.append(f'implementation_symbol = "example.invalid.fallback.{op_id.replace(".", "_")}"')
        lines.append("required = true")
    else:
        lines.append("[operations.fallback]")
        lines.append("required = false")
    return "\n".join(lines)


def collect_legacy_groups() -> dict[str, tuple[str, list[str]]]:
    groups: dict[str, list[str]] = defaultdict(list)
    owners: dict[str, str] = {}
    for path in sorted(WASM_DIR.glob("call_*.ark")):
        owner = _owner(path)
        for m in CALLEE_LITERAL.finditer(path.read_text(encoding="utf-8")):
            key = m.group(1) or m.group(2)
            norm = _normalize(key)
            groups[norm].append(key)
            owners[norm] = owner
    result: dict[str, tuple[str, list[str]]] = {}
    for norm, aliases in groups.items():
        op_id = intended_core_op_id(norm, owners[norm])
        result[op_id] = (norm, sorted(set(aliases)))
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--merge", action="store_true", help="merge into data/core-ops.toml")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    groups = collect_legacy_groups()
    existing = tomllib.loads(CORE_OPS.read_text(encoding="utf-8"))
    existing_ids = {op["id"] for op in existing.get("operations", [])}

    new_blocks: list[str] = []
    for op_id in sorted(groups):
        if op_id in existing_ids:
            continue
        norm, aliases = groups[op_id]
        new_blocks.append(emit_operation(op_id, norm, aliases))

    if not new_blocks:
        print("no new operations to generate")
        return 0

    fragment = "\n".join(new_blocks)
    if args.dry_run:
        print(fragment[:4000])
        print(f"... ({len(new_blocks)} new operations)")
        return 0

    if args.merge:
        text = CORE_OPS.read_text(encoding="utf-8").rstrip() + "\n" + fragment + "\n"
        CORE_OPS.write_text(text, encoding="utf-8")
        print(f"merged {len(new_blocks)} operations into {CORE_OPS}")
        return 0

    out = ROOT / "data" / "core-ops-generated-fragment.toml"
    out.write_text(fragment + "\n", encoding="utf-8")
    print(f"wrote {len(new_blocks)} operations to {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
