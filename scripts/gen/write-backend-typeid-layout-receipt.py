#!/usr/bin/env python3
"""Write a hash-bound receipt for the explicit backend GC layout boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BOUNDARY_FILES = (
    "src/compiler/mir/gc_layout_plan.ark",
    "src/compiler/mir/gc_layout_plan_validator.ark",
    "src/compiler/mir/module_gc_layout_plan.ark",
    "src/compiler/mir/lower/gc_layout_plan_build.ark",
    "src/compiler/mir/lower/entry.ark",
    "src/compiler/wasm/gc_layout_table_build.ark",
    "src/compiler/wasm/gc_layout_table.ark",
    "src/compiler/wasm/ctx_gc_layout_lookup.ark",
    "src/compiler/wasm/sections_types.ark",
    "scripts/check/check-backend-name-lookup-audit.py",
    "release/proof-policy.json",
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / ".build" / "proof" / "backend-typeid-layout.json",
    )
    args = parser.parse_args()

    files: list[dict[str, str]] = []
    for relative in BOUNDARY_FILES:
        path = ROOT / relative
        if not path.is_file():
            raise ValueError(f"boundary file missing: {relative}")
        files.append({"path": relative, "sha256": sha256(path)})

    document = {
        "schema": "arukellt-backend-typeid-layout-boundary",
        "schema_version": 1,
        "status": "enforced",
        "producer": "typed-mir-gc-layout-plan-v1",
        "consumer": "wasm-gc-layout-table-plan-consumer-v1",
        "backend_type_name_lookup": "removed",
        "backend_layout_offset_inference": "removed",
        "legacy_name_lookup_count": 0,
        "legacy_fallback_lookup_count": 0,
        "files": files,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "backend-typeid-layout-receipt: PASS: "
        f"files={len(files)} output={args.output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"backend-typeid-layout-receipt: FAIL: {exc}")
        raise SystemExit(1)
