#!/usr/bin/env python3
"""Verify the post-encoder component export boundary.

Aggregate export adapters used to be emitted by a large in-tree component
encoder. Official WASI P2 tooling now owns canonical ABI lowering, so the
compiler keeps the WIT/contract checks and rejects unsupported shapes before it
hands a standard core module to the launcher.
"""

from __future__ import annotations

import sys
try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 on supported development hosts
    import tomli as tomllib  # type: ignore[no-redef]
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
        print(f"gate-673: FAIL: matrix mismatch missing={EXPECTED - ids} extra={ids - EXPECTED}", file=sys.stderr)
        return 1
    for row in rows:
        if row.get("status") not in {"supported", "deferred", "rejected"} or not row.get("reason"):
            print(f"gate-673: FAIL: incomplete row {row!r}", file=sys.stderr)
            return 1

    retired = (
        "src/compiler/component/emit.ark",
        "src/compiler/component/emit_specialized.ark",
        "src/compiler/component/export_plan.ark",
        "src/compiler/wasm/library_component_emit.ark",
    )
    for rel in retired:
        if (ROOT / rel).exists():
            print(f"gate-673: FAIL: retired aggregate encoder remains: {rel}", file=sys.stderr)
            return 1

    contract = (ROOT / "src/compiler/component/contract_validation.ark").read_text(encoding="utf-8")
    if "E0401" not in contract:
        print("gate-673: FAIL: unsupported export rejection contract missing", file=sys.stderr)
        return 1
    wrapper = (ROOT / "scripts/run/arukellt-selfhost.sh").read_text(encoding="utf-8")
    if "component embed" not in wrapper or "component new" not in wrapper:
        print("gate-673: FAIL: official component packaging is not launcher-owned", file=sys.stderr)
        return 1
    if "--adapt" in wrapper:
        print("gate-673: FAIL: Preview 1 adapter compatibility path remains", file=sys.stderr)
        return 1

    print("gate-673-component-export-aggregate-expansion: PASS (contract boundary + official packaging)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
