#!/usr/bin/env python3
"""Compare legacy callee-string dispatch keys against CoreOp registry coverage.

Inventory is derived from call_*.ark and core-ops.toml + std/manifest.toml.
It is a migration report, not a new SSOT.
"""
from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
CORE_OPS = ROOT / "data" / "core-ops.toml"
MANIFEST = ROOT / "std" / "manifest.toml"
WASM_DIR = ROOT / "src" / "compiler" / "wasm"

CALLEE_LITERAL = re.compile(
    r'eq\(clone\(callee\),\s*(?:String_from\("([^"]+)"\)|"([^"]+)")\)'
)
INTRINSIC_PREFIX = "__intrinsic_"


@dataclass(frozen=True)
class LegacyKey:
    key: str
    source_file: str
    owner: str


def _owner_from_path(rel: str) -> str:
    name = Path(rel).stem
    if name.startswith("call_host"):
        return "host"
    if name.startswith("call_text"):
        return "text"
    if name.startswith("call_vec"):
        return "vec"
    if name.startswith("call_simd"):
        return "simd"
    if name.startswith("call_scalar"):
        return "scalar"
    if name.startswith("call_parse"):
        return "parse"
    if name.startswith("call_seq"):
        return "seq"
    return name.removeprefix("call_")


def _normalize_key(key: str) -> str:
    if key.startswith(INTRINSIC_PREFIX):
        return key[len(INTRINSIC_PREFIX) :]
    return key


def _intended_core_op_id(key: str, owner: str) -> str:
    norm = _normalize_key(key)
    if norm.startswith("__simd_"):
        body = norm.removeprefix("__simd_")
        return f"simd.{body.replace('_', '.')}"
    runtime_exact = {
        "panic", "println", "print", "eprintln", "clock_now", "clock_now_ms", "random_i32",
        "arg_count", "args", "has_flag", "arg_at", "exists", "read_to_string", "write_string",
        "write_bytes", "stdin_read_to_string", "env_var", "env_get_var", "env_var_or_default",
        "http_get", "http_request", "http_serve", "sockets_connect", "sockets_read",
        "sockets_write", "sockets_listen", "sockets_accept",
    }
    if norm in runtime_exact or norm.startswith(("env::", "fs::", "stdio::", "process::", "http::", "sockets::", "host::")):
        return f"runtime.{norm.replace('::', '.')}"
    if norm in {"memory_copy", "memory_fill", "trap", "unreachable"}:
        return f"primitive.{norm}"
    if owner == "text":
        if norm in {
            "starts_with", "ends_with", "contains", "trim", "index_of", "replace", "repeat",
            "concat", "len", "char_at", "split", "join", "slice", "substring", "trim_start",
            "trim_end", "pad_left", "pad_right", "to_uppercase", "to_lowercase",
            "reverse_String", "contains_String",
        }:
            return f"string.{norm.replace('_String', '')}"
        return f"text.{norm.replace('::', '.')}"
    if owner == "vec":
        base = norm.removeprefix("vec_") if norm.startswith("vec_") else norm
        return f"vec.{base}"
    if owner == "scalar":
        if norm.startswith("math::"):
            return f"math.{norm.removeprefix('math::')}"
        if norm.startswith("core::"):
            return f"core.{norm.removeprefix('core::')}"
        return f"scalar.{norm}"
    if owner == "parse":
        return f"parse.{norm.replace('::', '.')}"
    if owner == "seq":
        return f"seq.{norm.replace('::', '.')}"
    return f"{owner}.{norm.replace('::', '.')}"


def extract_legacy_keys() -> dict[str, LegacyKey]:
    found: dict[str, LegacyKey] = {}
    for path in sorted(WASM_DIR.glob("call_*.ark")):
        rel = str(path.relative_to(ROOT)).replace("\\", "/")
        owner = _owner_from_path(rel)
        for m in CALLEE_LITERAL.finditer(path.read_text(encoding="utf-8")):
            key = m.group(1) or m.group(2)
            norm = _normalize_key(key)
            if norm not in found:
                found[norm] = LegacyKey(key=norm, source_file=rel, owner=owner)
    return found


def load_core_op_ids() -> set[str]:
    data = tomllib.loads(CORE_OPS.read_text(encoding="utf-8"))
    return {op["id"] for op in data.get("operations", []) if isinstance(op, dict) and "id" in op}


def load_manifest_core_op_ids() -> set[str]:
    data = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    ids: set[str] = set()
    for fn in data.get("functions", []):
        if isinstance(fn, dict):
            cid = fn.get("core_op_id")
            if isinstance(cid, str) and cid:
                ids.add(cid)
    return ids


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    legacy = extract_legacy_keys()
    core_ops = load_core_op_ids()
    manifest_ops = load_manifest_core_op_ids()

    unmapped: list[tuple[str, str, str, str]] = []
    mapped = 0
    for norm, entry in sorted(legacy.items()):
        intended = _intended_core_op_id(norm, entry.owner)
        if intended in core_ops:
            status = "mapped"
            mapped += 1
        elif norm in {"println", "print", "eprintln", "panic"} and any(
            op.endswith(norm) for op in core_ops
        ):
            status = "mapped"
            mapped += 1
        else:
            status = "unmapped"
            unmapped.append((norm, entry.owner, entry.source_file, intended))

    report = {
        "legacy_unique_keys": len(legacy),
        "core_op_entries": len(core_ops),
        "manifest_core_op_refs": len(manifest_ops),
        "mapped": mapped,
        "unmapped": len(unmapped),
        "unmapped_samples": unmapped[:20],
    }

    if args.json:
        import json

        print(json.dumps(report, indent=2))
        return 0 if not unmapped else 1

    print(
        f"legacy inventory: {len(legacy)} keys, mapped={mapped}, unmapped={len(unmapped)}, "
        f"core_ops={len(core_ops)}"
    )
    if unmapped:
        print("FAIL: unmapped legacy dispatch keys (first 20):", file=sys.stderr)
        for key, owner, src, intended in unmapped[:20]:
            print(f"  {key} owner={owner} intended={intended} ({src})", file=sys.stderr)
        return 1
    print("PASS: all legacy dispatch keys have CoreOp coverage")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
