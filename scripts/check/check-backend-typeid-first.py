#!/usr/bin/env python3
"""Require explicit MirLocal TypeId lookup before backend name reconstruction."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "src" / "compiler" / "wasm" / "ctx_gc_layout_lookup.ark"


def main() -> int:
    text = SOURCE.read_text(encoding="utf-8")
    start = text.index("fn SelfEmitCtx_wasm_ref_type_idx_for_local(")
    end = text.index("fn strip_module_double_colon(", start)
    function = text[start:end]

    typeid_lookup = function.index("let mvt = local_access::MirLocal_value_type(loc)")
    layout_lookup = function.index(
        "SelfEmitCtx_wasm_storage_ref_type_idx_for_mir_value_type(ctx, mvt)",
        typeid_lookup,
    )
    name_lookup = function.index(
        "SelfEmitCtx_wasm_ref_type_idx_for_type_name(ctx, clone(local_type_name))"
    )
    legacy_lookup = function.index("type_info_to_mir_value::vt_to_mir_value")

    if not (typeid_lookup < layout_lookup < name_lookup < legacy_lookup):
        raise ValueError("backend local layout lookup is not TypeId-first")
    if "if mvt_gc >= 0" not in function[typeid_lookup:name_lookup]:
        raise ValueError("TypeId lookup result is not consumed before name fallback")
    if "return mvt_gc" not in function[typeid_lookup:name_lookup]:
        raise ValueError("successful TypeId lookup does not return directly")

    print("backend-typeid-first: PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"backend-typeid-first: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
