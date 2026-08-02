#!/usr/bin/env python3
"""Check that backend GC layout resolution is TypeId-only."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TABLE = ROOT / "src" / "compiler" / "wasm" / "gc_layout_table.ark"
LOOKUP = ROOT / "src" / "compiler" / "wasm" / "ctx_gc_layout_lookup.ark"
AUDIT = ROOT / "src" / "compiler" / "wasm" / "gc_layout_audit.ark"

FORBIDDEN_LOOKUP_TOKENS = (
    "type_name_ref_index",
    "wasm_ref_type_idx_for_type_name",
    "wasm_ref_type_idx_for_named_gc_struct",
    "gc_layout_type_id_for_name",
    "type_table_find_existing",
    "MirLocal_type_name",
    "vt_to_mir_value",
    "gc_layout_table_observe_name_lookup",
    "gc_layout_table_observe_fallback_lookup",
)


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise ValueError(f"missing {label}: {needle}")


def main() -> int:
    table = TABLE.read_text(encoding="utf-8")
    lookup = LOOKUP.read_text(encoding="utf-8")
    audit = AUDIT.read_text(encoding="utf-8")

    require(table, "typed_lookup_count: i32", "typed lookup counter")
    require(
        lookup,
        "SelfEmitCtx_wasm_ref_type_idx_for_type_id",
        "TypeId lookup boundary",
    )
    require(
        lookup,
        "MirLocal_value_type",
        "typed MIR local lookup",
    )
    for token in FORBIDDEN_LOOKUP_TOKENS:
        if token in lookup:
            raise ValueError(f"backend name/layout recovery remains: {token}")

    require(audit, "let name_count =", "audit summary input")
    require(audit, '" name="', "audit summary output")
    if "name_count > 0 || fallback_count > 0" not in audit:
        raise ValueError("audit must fail when name or fallback lookup is observed")

    print("backend-name-lookup-audit: PASS: TypeId-only")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"backend-name-lookup-audit: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
