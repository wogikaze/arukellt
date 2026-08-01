#!/usr/bin/env python3
"""Check that successful backend name-based layout lookups are observable."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TABLE = ROOT / "src" / "compiler" / "wasm" / "gc_layout_table.ark"
LOOKUP = ROOT / "src" / "compiler" / "wasm" / "ctx_gc_layout_lookup.ark"
AUDIT = ROOT / "src" / "compiler" / "wasm" / "gc_layout_audit.ark"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise ValueError(f"missing {label}: {needle}")


def main() -> int:
    table = TABLE.read_text(encoding="utf-8")
    lookup = LOOKUP.read_text(encoding="utf-8")
    audit = AUDIT.read_text(encoding="utf-8")

    require(table, "name_lookup_count: i32", "name lookup counter")
    require(table, "fn gc_layout_table_observe_name_lookup", "counter observer")
    require(table, "fn gc_layout_table_name_lookup_count", "counter accessor")
    require(
        lookup,
        "gc_layout_table::gc_layout_table_observe_name_lookup",
        "successful name lookup observation",
    )
    require(audit, "let name_count =", "audit summary input")
    require(audit, '" name="', "audit summary output")
    if "name_count > 0 || fallback_count > 0" in audit:
        raise ValueError("name lookup count must remain observational until the zero baseline is established")

    print("backend-name-lookup-audit: PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"backend-name-lookup-audit: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
