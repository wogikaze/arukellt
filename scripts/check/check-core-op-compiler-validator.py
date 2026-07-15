#!/usr/bin/env python3
"""Compiler-aware CoreOp registry validation (Python structural layer).

Checks binding/registry alignment, alias consistency, and generated artifact freshness.
Compiler-runtime fallback resolution remains a follow-up in-ark check mode.
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
CORE_OPS = ROOT / "data" / "core-ops.toml"
MANIFEST = ROOT / "std" / "manifest.toml"
GEN_REGISTRY = ROOT / "scripts" / "gen" / "generate-core-ops-registry.py"
GEN_BINDINGS = ROOT / "scripts" / "gen" / "generate-core-op-bindings.py"


def load_core_op_ids() -> set[str]:
    data = tomllib.loads(CORE_OPS.read_text(encoding="utf-8"))
    return {op["id"] for op in data.get("operations", []) if isinstance(op, dict) and "id" in op}


def check_manifest_refs(core_ops: set[str]) -> list[str]:
    errors: list[str] = []
    manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    for fn in manifest.get("functions", []):
        if not isinstance(fn, dict):
            continue
        cid = fn.get("core_op_id")
        if isinstance(cid, str) and cid and cid not in core_ops:
            errors.append(f"manifest core_op_id missing from registry: {cid}")
    return errors


def check_alias_consistency() -> list[str]:
    manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    by_op: dict[str, set[str]] = {}
    for fn in manifest.get("functions", []):
        if not isinstance(fn, dict):
            continue
        cid = fn.get("core_op_id")
        intrinsic = fn.get("intrinsic")
        if isinstance(cid, str) and cid and isinstance(intrinsic, str):
            by_op.setdefault(cid, set()).add(intrinsic)
    errors: list[str] = []
    for cid, intrinsics in sorted(by_op.items()):
        if len(intrinsics) > 1:
            errors.append(f"manifest alias drift for {cid}: {sorted(intrinsics)}")
    return errors


def run_generator(script: Path) -> int:
    proc = subprocess.run([sys.executable, str(script), "--check"], cwd=ROOT)
    return proc.returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    args = parser.parse_args()
    errors: list[str] = []

    # Schema/manifest gate must pass before claiming compiler-aware readiness.
    core_ops_proc = subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "check" / "check-core-ops.py")],
        cwd=ROOT,
    )
    if core_ops_proc.returncode != 0:
        errors.append("check-core-ops.py failed")

    core_ops = load_core_op_ids()
    errors.extend(check_manifest_refs(core_ops))
    errors.extend(check_alias_consistency())

    if run_generator(GEN_REGISTRY) != 0:
        errors.append("stale core_op_registry_generated.ark")
    if run_generator(GEN_BINDINGS) != 0:
        errors.append("stale core_op_binding_generated.ark")

    proc = subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "check" / "check-legacy-dispatch-inventory.py")],
        cwd=ROOT,
    )
    if proc.returncode != 0:
        errors.append("legacy dispatch inventory has unmapped keys")

    if errors:
        print("FAIL: compiler-aware core-op validation", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1
    print("PASS: compiler-aware core-op validation")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
