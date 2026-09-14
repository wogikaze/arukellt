#!/usr/bin/env python3
"""Close gate for #819: runtime lowering stays in Ark and uses official P2 imports."""
from __future__ import annotations

import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 compatibility
    import tomli as tomllib  # type: ignore[no-redef]

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
        op_id = op.get("id")
        lowering = op.get("lowering", {})
        runtime = lowering.get("runtime", {})
        kind = runtime.get("kind")
        if lowering.get("kind") != "runtime_call":
            return fail(f"{op_id} is not runtime_call")
        if kind not in {"internal", "wit", "native"}:
            return fail(f"{op_id} has no explicit runtime payload kind")
        if kind == "internal":
            if not str(runtime.get("symbol", "")).strip():
                return fail(f"{op_id} has no internal runtime symbol")
            if not str(runtime.get("abi_version", "")).strip():
                return fail(f"{op_id} has no internal runtime ABI version")
        elif kind == "native":
            if not str(runtime.get("backend", "")).strip() or not str(runtime.get("symbol", "")).strip():
                return fail(f"{op_id} has incomplete native runtime payload")
            if not str(runtime.get("abi_version", "")).strip():
                return fail(f"{op_id} has no native runtime ABI version")
        else:
            for field in ("package", "interface", "function", "version"):
                if not str(runtime.get(field, "")).strip():
                    return fail(f"{op_id} has incomplete WIT runtime payload: {field}")

    wasm = ROOT / "src/compiler/wasm"
    if list(wasm.glob("call_host*.ark")):
        return fail("legacy call_host modules remain")
    for family in ("http", "sockets", "stream", "udp"):
        if list(wasm.glob(f"intrinsic_{family}*.ark")):
            return fail(f"legacy intrinsic_{family} emitter ownership remains")
    imports = (wasm / "sections_imports.ark").read_text(encoding="utf-8")
    if "cm32p2|wasi:" not in imports:
        return fail("P2 lowering does not use official WASI core import names")
    for marker in ("arukellt:runtime", "arukellt_host", "p2_component_wrap"):
        if marker in imports:
            return fail(f"repository-specific compatibility marker remains in imports: {marker}")

    print(f"gate-819-runtime-abi-core-op-lowering: PASS ({len(runtime_ops)} Ark runtime CoreOps; official P2 imports)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
