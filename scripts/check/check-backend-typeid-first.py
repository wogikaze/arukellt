#!/usr/bin/env python3
"""Require TypeId-only backend GC layout selection with no reconstruction path."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LOOKUP = ROOT / "src" / "compiler" / "wasm" / "ctx_gc_layout_lookup.ark"
BUILD = ROOT / "src" / "compiler" / "wasm" / "gc_layout_table_build.ark"

FORBIDDEN = (
    "MirLocal_type_name",
    "wasm_ref_type_idx_for_type_name",
    "wasm_ref_type_idx_for_named_gc_struct",
    "type_info_to_mir_value",
    "type_table_find_existing",
    "gc_layout_table_ref_offset_for_type_name",
)


def main() -> int:
    lookup = LOOKUP.read_text(encoding="utf-8")
    build = BUILD.read_text(encoding="utf-8")
    start = lookup.index("fn SelfEmitCtx_wasm_ref_type_idx_for_local(")
    end = lookup.index("fn SelfEmitCtx_wasm_ref_type_idx_for_variant_slot(", start)
    function = lookup[start:end]

    typeid_lookup = function.index("let mvt = local_access::MirLocal_value_type(loc)")
    layout_lookup = function.index(
        "SelfEmitCtx_wasm_storage_ref_type_idx_for_mir_value_type(ctx, mvt)",
        typeid_lookup,
    )
    if typeid_lookup >= layout_lookup:
        raise ValueError("backend local layout does not consume MirValueType TypeId")
    if "return mvt_gc" not in function[layout_lookup:]:
        raise ValueError("successful TypeId layout is not returned")

    combined = lookup + "\n" + build
    for token in FORBIDDEN:
        if token in combined:
            raise ValueError(f"backend GC layout reconstruction remains: {token}")
    if "MirModule_gc_layout_plan(mir)" not in build:
        raise ValueError("backend does not consume the explicit MIR GC layout plan")

    print("backend-typeid-first: PASS: no fallback")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"backend-typeid-first: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
