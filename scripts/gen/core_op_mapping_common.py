"""Shared legacy dispatch -> CoreOpId mapping helpers for #798 generators."""
from __future__ import annotations

import re
from collections import defaultdict
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

CALLEE_LITERAL = re.compile(
    r'eq\(clone\(callee\),\s*(?:String_from\("([^"]+)"\)|"([^"]+)")\)'
)
INTRINSIC_PREFIX = "__intrinsic_"

MIR_OP_KEYS = {"memory_copy", "memory_fill", "trap", "unreachable"}
RUNTIME_PREFIXES = ("env::", "fs::", "stdio::", "process::", "http::", "sockets::", "host::")
RUNTIME_EXACT = {
    "panic", "println", "print", "eprintln", "clock_now", "clock_now_ms", "random_i32",
    "arg_count", "args", "has_flag", "arg_at", "exists", "read_to_string", "write_string",
    "write_bytes", "stdin_read_to_string", "env_var", "env_get_var", "env_var_or_default",
    "http_get", "http_request", "http_serve", "sockets_connect", "sockets_read",
    "sockets_write", "sockets_listen", "sockets_accept",
}


def normalize_key(key: str) -> str:
    if key.startswith(INTRINSIC_PREFIX):
        return key[len(INTRINSIC_PREFIX) :]
    return key


def owner_from_path(path: Path) -> str:
    name = path.stem
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


def intended_core_op_id(norm: str, owner: str) -> str:
    if norm.startswith("__simd_"):
        body = norm.removeprefix("__simd_")
        return f"simd.{body.replace('_', '.')}"
    if norm in MIR_OP_KEYS:
        return f"primitive.{norm}"
    if norm in RUNTIME_EXACT or any(norm.startswith(p) for p in RUNTIME_PREFIXES):
        return f"runtime.{norm.replace('::', '.')}"
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


def collect_legacy_bindings(wasm_dir: Path) -> dict[str, str]:
    groups: dict[str, list[str]] = defaultdict(list)
    owners: dict[str, str] = {}
    for path in sorted(wasm_dir.glob("call_*.ark")):
        owner = owner_from_path(path)
        for m in CALLEE_LITERAL.finditer(path.read_text(encoding="utf-8")):
            key = m.group(1) or m.group(2)
            norm = normalize_key(key)
            groups[norm].append(key)
            owners[norm] = owner
    return {intended_core_op_id(norm, owners[norm]): norm for norm in groups}
