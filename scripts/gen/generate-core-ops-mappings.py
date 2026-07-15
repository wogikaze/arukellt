#!/usr/bin/env python3
"""Generate CoreOp registry entries from legacy handler-branch inventory.

Inventory unit is a legacy if-branch (OR'd aliases), not each callee string.
Hand-authored scaffold operations (before the migration marker) are preserved.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
CORE_OPS = ROOT / "data" / "core-ops.toml"
WASM_DIR = ROOT / "src" / "compiler" / "wasm"
MIGRATION_MARKER = "# ── #798 migrated legacy handler branches"

sys.path.insert(0, str(Path(__file__).resolve().parent))
from core_op_mapping_common import (  # noqa: E402
    HandlerBranch,
    extract_handler_branches,
)


HAND_AUTHORED_IDS = {
    "string.starts_with",
    "string.ends_with",
    "panic",
    "simd.i32x4.add",
    "simd.i32x4.sub",
    "simd.f32x4.add",
    "wasm.v128.load",
}


def emit_operation(branch: HandlerBranch) -> str:
    op_id = branch.core_op_id
    layer = branch.layer
    lowering_kind = branch.lowering_kind

    # Schema: internal → forbidden; public → optional (or required for hand-authored).
    if op_id.startswith(("primitive.",)) or (
        op_id.startswith("simd.") and branch.handler_key.startswith("__simd_")
    ):
        visibility = "internal"
        binding = "forbidden"
    else:
        visibility = "public"
        binding = "optional"

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
        lines.append('binding = { policy = "forbidden" }')

    alias_list = ", ".join(branch.aliases)
    lines.append(
        f'description = "Legacy handler branch → {branch.handler_key} '
        f'(aliases: {len(branch.aliases)}; {alias_list})"'
    )
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

    if lowering_kind == "mir_op":
        op_name = op_id.removeprefix("primitive.")
        lines.append("[operations.lowering.mir]")
        lines.append(f'opcode = "{op_name}"')
        lines.append(f'operation = "{op_name}"')
    elif lowering_kind == "runtime_call":
        symbol = branch.handler_key.replace("::", "_").removeprefix("__intrinsic_")
        lines.append("[operations.lowering.runtime]")
        lines.append('kind = "internal"')
        lines.append(f'symbol = "{symbol}"')
        lines.append('abi_version = "0.1"')
    elif lowering_kind == "target_intrinsic":
        target_id = branch.handler_key.removeprefix("__simd_").replace("_", ".")
        lines.append("[operations.lowering.target]")
        lines.append('target_family = "wasm"')
        lines.append(f'target_id = "{target_id}"')
        lines.append("required_capabilities = []")
        lines.append('required_target_features = ["simd128"]')

    if lowering_kind == "normal_call":
        lines.append("[operations.fallback]")
        lines.append(
            f'implementation_symbol = "example.invalid.fallback.{op_id.replace(".", "_")}"'
        )
        lines.append("required = true")
    else:
        lines.append("[operations.fallback]")
        lines.append("required = false")
    return "\n".join(lines)


def scaffold_prefix(text: str) -> str:
    if MIGRATION_MARKER in text:
        return text.split(MIGRATION_MARKER)[0].rstrip() + "\n"
    # First migration used "Migrated legacy dispatch" descriptions.
    lines = text.splitlines()
    cut = None
    for i, line in enumerate(lines):
        if 'description = "Migrated legacy dispatch:' in line:
            # back up to [[operations]]
            for j in range(i, -1, -1):
                if lines[j].startswith("[[operations]]"):
                    cut = j
                    break
            break
    if cut is None:
        return text.rstrip() + "\n"
    return "\n".join(lines[:cut]).rstrip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rewrite", action="store_true", help="rewrite migrated section in core-ops.toml")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    branches = extract_handler_branches(WASM_DIR)
    to_emit = [b for b in branches if b.core_op_id not in HAND_AUTHORED_IDS]

    blocks = [emit_operation(b) for b in to_emit]
    fragment = "\n".join(blocks)

    print(
        f"handler branches={len(branches)} emit={len(to_emit)} "
        f"(hand-authored reserved={len(HAND_AUTHORED_IDS)})"
    )

    if args.dry_run:
        print(fragment[:3000])
        print(f"... ({len(to_emit)} operations)")
        return 0

    if not args.rewrite:
        out = ROOT / "data" / "core-ops-generated-fragment.toml"
        out.write_text(fragment + "\n", encoding="utf-8")
        print(f"wrote {out}")
        return 0

    prefix = scaffold_prefix(CORE_OPS.read_text(encoding="utf-8"))
    rebuilt = (
        prefix
        + "\n"
        + MIGRATION_MARKER
        + "\n"
        + "# Generated by scripts/gen/generate-core-ops-mappings.py --rewrite\n"
        + "# Do not hand-edit; regenerate from call_*.ark handler branches.\n"
        + fragment
        + "\n"
    )
    CORE_OPS.write_text(rebuilt, encoding="utf-8")
    print(f"rewrote {CORE_OPS} with {len(to_emit)} migrated operations")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
