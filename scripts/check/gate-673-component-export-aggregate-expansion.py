#!/usr/bin/env python3
"""Close gate for #673: every Tier-2 export shape is supported, rejected, or explicitly deferred."""
from __future__ import annotations
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MATRIX = ROOT / "docs/data/component-export-tier2.toml"
EXPECTED = {
    "option-string", "option-vec-i32", "result-string-string", "result-vec-i32-string",
    "vec-string", "vec-u8", "vec-i64", "vec-option-i32", "tuple-string-string", "tuple-3",
    "general-record-enum-variant", "mixed-scalar-aggregate-multi-export", "multi-export-string-list",
    "recursive-export",
}


def main() -> int:
    data = tomllib.loads(MATRIX.read_text(encoding="utf-8"))
    rows = data.get("shape", [])
    ids = {row.get("id") for row in rows}
    if ids != EXPECTED:
        print(f"gate-673: FAIL: matrix mismatch missing={EXPECTED-ids} extra={ids-EXPECTED}", file=sys.stderr)
        return 1
    for row in rows:
        if row.get("status") not in {"supported", "deferred", "rejected"} or not row.get("reason"):
            print(f"gate-673: FAIL: incomplete row {row!r}", file=sys.stderr)
            return 1
    emit = (ROOT / "src/compiler/component/emit.ark").read_text(encoding="utf-8")
    if emit.index("emit_specialized::emit_specialized_component") > emit.index("comp_emit_wasi_and_core_instance_sections"):
        print("gate-673: FAIL: specialized adapters are still bypassed", file=sys.stderr)
        return 1
    contract = (ROOT / "src/compiler/component/contract_validation.ark").read_text(encoding="utf-8")
    if "E0401" not in contract:
        print("gate-673: FAIL: recursive/unsupported export rejection contract missing", file=sys.stderr)
        return 1
    print("gate-673-component-export-aggregate-expansion: PASS")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
