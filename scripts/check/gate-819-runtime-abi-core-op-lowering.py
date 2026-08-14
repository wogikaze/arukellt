#!/usr/bin/env python3
"""Close gate for #819: runtime-classified CoreOps use explicit runtime payloads."""
from __future__ import annotations

import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def fail(message: str) -> int:
    print(f"gate-819: FAIL: {message}", file=sys.stderr)
    return 1


def main() -> int:
    data = tomllib.loads((ROOT / "data/core-ops.toml").read_text(encoding="utf-8"))
    runtime_ops = [op for op in data.get("operations", []) if op.get("classification", {}).get("layer") == "runtime"]
    if not runtime_ops:
        return fail("no runtime-classified CoreOps")
    for op in runtime_ops:
        lowering = op.get("lowering", {})
        runtime = lowering.get("runtime", {})
        if lowering.get("kind") != "runtime_call":
            return fail(f"{op.get('id')} is not runtime_call")
        if runtime.get("kind") not in {"internal", "wit", "native"}:
            return fail(f"{op.get('id')} has no explicit runtime payload kind")
        if not str(runtime.get("abi_version", "")).strip():
            return fail(f"{op.get('id')} has no runtime ABI version")

    required = {
        "runtime.fs_read_dir",
        "runtime.fs_metadata",
        "runtime.fs_remove_file",
        "runtime.fs_create_dir_all",
        "runtime.env_vars",
        "runtime.env_current_dir",
    }
    ids = {op.get("id") for op in runtime_ops}
    if not required <= ids:
        return fail(f"missing production fs/env CoreOps: {sorted(required - ids)}")

    wasm = ROOT / "src/compiler/wasm"
    if list(wasm.glob("call_host*.ark")):
        return fail("legacy call_host modules remain")
    for family in ("http", "sockets", "fs", "env", "process"):
        if list(wasm.glob(f"intrinsic_{family}*.ark")):
            return fail(f"legacy intrinsic_{family} emitter ownership remains")
    dispatch = (wasm / "core_op_dispatch.ark").read_text(encoding="utf-8")
    if "call_runtime::try_emit_host_call" not in dispatch:
        return fail("CoreOp dispatch is not routed through call_runtime")
    imports = (wasm / "sections_imports.ark").read_text(encoding="utf-8")
    if "arukellt:runtime/host@0.1.0" not in imports:
        return fail("versioned runtime core import module missing")
    if "runtime_fs_read_dir" not in imports or "runtime_env_current_dir" not in imports:
        return fail("expanded runtime import surface missing")

    print(f"gate-819-runtime-abi-core-op-lowering: PASS ({len(runtime_ops)} runtime CoreOps)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
