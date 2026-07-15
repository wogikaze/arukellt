"""Shared legacy dispatch -> CoreOpId mapping helpers for #798 generators.

Inventory unit is a *handler branch*: all aliases OR'd in the same
`if eq(clone(callee), ...) || ...` condition share one CoreOpId.
"""
from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

CALLEE_LITERAL = re.compile(
    r'eq\(clone\(callee\),\s*(?:String_from\("([^"]+)"\)|"([^"]+)")\)'
)
# One if-condition line (or continued) that compares callee to one or more literals.
IF_BRANCH = re.compile(
    r"if\s+((?:eq\(clone\(callee\),\s*(?:String_from\(\"[^\"]+\"\)|\"[^\"]+\")\)\s*(?:\|\|\s*)?)+)",
    re.MULTILINE,
)
INTRINSIC_PREFIX = "__intrinsic_"

MIR_OP_KEYS = {"memory_copy", "memory_fill", "trap", "unreachable"}
RUNTIME_EXACT = {
    "panic",
    "println",
    "print",
    "eprintln",
    "clock_now",
    "clock_now_ms",
    "random_i32",
    "arg_count",
    "args",
    "has_flag",
    "arg_at",
    "exists",
    "read_to_string",
    "write_string",
    "write_bytes",
    "stdin_read_to_string",
    "env_var",
    "env_get_var",
    "env_var_or_default",
    "http_get",
    "http_request",
    "http_serve",
    "sockets_connect",
    "sockets_read",
    "sockets_write",
    "sockets_listen",
    "sockets_accept",
    "process_exit",
    "process_abort",
}

# Hand-authored / preferred CoreOpIds for known semantic ops.
PREFERRED_CORE_OP_IDS = {
    "starts_with": "string.starts_with",
    "text::starts_with": "string.starts_with",
    "ends_with": "string.ends_with",
    "text::ends_with": "string.ends_with",
    "panic": "panic",
    "wasm.v128.load": "wasm.v128.load",
}


@dataclass(frozen=True)
class HandlerBranch:
    """One legacy if-branch and its semantic CoreOp mapping."""

    aliases: tuple[str, ...]
    owner: str
    source_file: str
    core_op_id: str
    # Callee literal that existing helpers recognize (not stripped).
    handler_key: str
    layer: str
    lowering_kind: str


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


def _pick_handler_key(aliases: list[str]) -> str:
    """Prefer a literal the emitter helpers already match."""
    # Prefer path-style public names helpers check first.
    for key in aliases:
        if "::" in key and not key.startswith(INTRINSIC_PREFIX):
            return key
    for key in aliases:
        if not key.startswith(INTRINSIC_PREFIX) and not key.startswith("__simd_"):
            return key
    return aliases[0]


def _semantic_base(alias: str) -> str:
    """Strip module prefixes / intrinsic prefix for naming."""
    key = normalize_key(alias)
    # Drop host:: / std::host:: style prefixes for identity
    for prefix in (
        "std::host::",
        "host::",
        "stdio::",
        "env::",
        "fs::",
        "process::",
        "http::",
        "sockets::",
        "text::",
        "string::",
        "math::",
        "core::",
        "convert::",
        "seq::",
    ):
        if key.startswith(prefix):
            key = key[len(prefix) :]
    return key


def intended_core_op_id_for_aliases(aliases: list[str], owner: str) -> str:
    for alias in aliases:
        if alias in PREFERRED_CORE_OP_IDS:
            return PREFERRED_CORE_OP_IDS[alias]
        norm = normalize_key(alias)
        if norm in PREFERRED_CORE_OP_IDS:
            return PREFERRED_CORE_OP_IDS[norm]

    # Prefer the shortest semantic base among aliases.
    bases = [_semantic_base(a) for a in aliases]
    bases_sorted = sorted(bases, key=lambda b: (len(b), b))
    primary = bases_sorted[0]
    sample = aliases[0]
    norm_sample = normalize_key(sample)

    if primary.startswith("__simd_") or any(a.startswith("__simd_") for a in aliases):
        body = primary.removeprefix("__simd_")
        if primary.startswith("__simd_"):
            return f"simd.{body.replace('_', '.')}"
        simd_alias = next(a for a in aliases if a.startswith("__simd_") or normalize_key(a).startswith("__simd_"))
        body = normalize_key(simd_alias).removeprefix("__simd_")
        return f"simd.{body.replace('_', '.')}"

    if primary in MIR_OP_KEYS or any(_semantic_base(a) in MIR_OP_KEYS for a in aliases):
        return f"primitive.{primary if primary in MIR_OP_KEYS else next(b for b in bases if b in MIR_OP_KEYS)}"

    if owner == "host" or primary in RUNTIME_EXACT or any(
        a.startswith(("env::", "fs::", "stdio::", "process::", "http::", "sockets::", "host::"))
        for a in aliases
    ):
        # Collapse process::exit / host::process::exit / __intrinsic_process_exit → runtime.process.exit
        if primary in {"exit", "process_exit"} or any("process::exit" in a or a.endswith("process_exit") for a in aliases):
            return "runtime.process.exit"
        if primary in {"abort", "process_abort"} or any("process::abort" in a or a.endswith("process_abort") for a in aliases):
            return "runtime.process.abort"
        if primary in {"println", "print", "eprintln", "panic"}:
            if primary == "panic":
                return "panic"
            return f"runtime.{primary}"
        if primary.startswith("http") or any("http" in a for a in aliases):
            return f"runtime.{primary.replace('::', '.')}"
        if primary.startswith("socket") or any("socket" in a for a in aliases):
            return f"runtime.{primary.replace('::', '.')}"
        return f"runtime.{primary.replace('::', '.')}"

    if owner == "text":
        if primary in {
            "starts_with",
            "ends_with",
            "contains",
            "trim",
            "index_of",
            "replace",
            "repeat",
            "concat",
            "len",
            "char_at",
            "split",
            "join",
            "slice",
            "substring",
            "string_slice",
            "trim_start",
            "trim_end",
            "pad_left",
            "pad_right",
            "to_uppercase",
            "to_lowercase",
            "to_upper",
            "to_lower",
            "reverse",
            "reverse_String",
            "contains_String",
        }:
            # Unify slice / substring / string_slice / text::slice_bytes
            if primary in {"slice", "substring", "string_slice", "slice_bytes"} or any(
                "slice" in a or "substring" in a for a in aliases
            ):
                return "string.slice"
            if primary in {"to_upper", "to_uppercase"}:
                return "string.to_uppercase"
            if primary in {"to_lower", "to_lowercase"}:
                return "string.to_lowercase"
            if primary in {"reverse", "reverse_String"}:
                return "string.reverse"
            if primary in {"contains", "contains_String"}:
                return "string.contains"
            return f"string.{primary.replace('_String', '')}"
        return f"text.{primary.replace('::', '.')}"

    if owner == "vec":
        base = primary.removeprefix("vec_") if primary.startswith("vec_") else primary
        return f"vec.{base}"

    if owner == "scalar":
        if any(a.startswith("math::") for a in aliases) or primary in {
            "sqrt", "min", "max", "abs", "gcd", "clamp", "clamp_i32", "pow_i32",
        }:
            return f"math.{primary.removeprefix('math::')}"
        if any(a.startswith("core::") for a in aliases) or primary.startswith("range_"):
            return f"core.{primary.removeprefix('core::')}"
        return f"scalar.{primary}"

    if owner == "parse":
        return f"parse.{primary.replace('::', '.')}"
    if owner == "seq":
        return f"seq.{primary.replace('::', '.')}"
    return f"{owner}.{primary.replace('::', '.')}"


def classify_layer_and_lowering(op_id: str, handler_key: str) -> tuple[str, str]:
    if op_id.startswith("primitive."):
        return "primitive", "mir_op"
    if op_id.startswith("runtime.") or op_id == "panic":
        return "runtime", "runtime_call"
    if op_id.startswith("simd.") and (
        handler_key.startswith("__simd_") or normalize_key(handler_key).startswith("__simd_")
    ):
        return "target_raw", "target_intrinsic"
    if op_id.startswith("wasm."):
        return "target_raw", "target_intrinsic"
    return "semantic_stdlib", "normal_call"


def extract_handler_branches(wasm_dir: Path) -> list[HandlerBranch]:
    branches: list[HandlerBranch] = []
    seen_ops: dict[str, HandlerBranch] = {}

    for path in sorted(wasm_dir.glob("call_*.ark")):
        # Skip pure routers / facades that only forward without leaf callee compares.
        if path.name in {
            "call_dispatch.ark",
            "call_dispatch_table.ark",
            "call_host.ark",
            "call_simd.ark",
            "call_hash.ark",
            "call_fallback.ark",
            "call_fallback_resolved.ark",
            "call_wit.ark",
            "call_indirect.ark",
        }:
            continue
        owner = owner_from_path(path)
        text = path.read_text(encoding="utf-8")
        rel = str(path.as_posix()).split("src/compiler/wasm/")[-1]
        # Also catch single-line and multi-OR on one if
        for m in IF_BRANCH.finditer(text):
            cond = m.group(1)
            aliases = []
            for lit in CALLEE_LITERAL.finditer(cond):
                aliases.append(lit.group(1) or lit.group(2))
            if not aliases:
                continue
            # Deduplicate while preserving order
            uniq: list[str] = []
            for a in aliases:
                if a not in uniq:
                    uniq.append(a)
            core_op_id = intended_core_op_id_for_aliases(uniq, owner)
            handler_key = _pick_handler_key(uniq)
            layer, lowering = classify_layer_and_lowering(core_op_id, handler_key)
            branch = HandlerBranch(
                aliases=tuple(uniq),
                owner=owner,
                source_file=rel,
                core_op_id=core_op_id,
                handler_key=handler_key,
                layer=layer,
                lowering_kind=lowering,
            )
            if core_op_id in seen_ops:
                # Merge aliases into existing branch
                prev = seen_ops[core_op_id]
                merged_aliases = tuple(dict.fromkeys(list(prev.aliases) + list(uniq)))
                # Prefer existing preferred handler_key if still present
                handler = prev.handler_key if prev.handler_key in merged_aliases else _pick_handler_key(list(merged_aliases))
                merged = HandlerBranch(
                    aliases=merged_aliases,
                    owner=prev.owner,
                    source_file=prev.source_file,
                    core_op_id=core_op_id,
                    handler_key=handler,
                    layer=prev.layer,
                    lowering_kind=prev.lowering_kind,
                )
                seen_ops[core_op_id] = merged
            else:
                seen_ops[core_op_id] = branch

        # Also collect any lone literals not captured by IF_BRANCH (edge cases)
        for lit in CALLEE_LITERAL.finditer(text):
            key = lit.group(1) or lit.group(2)
            # Skip if already covered by some branch
            if any(key in b.aliases for b in seen_ops.values()):
                continue
            core_op_id = intended_core_op_id_for_aliases([key], owner)
            if core_op_id in seen_ops:
                prev = seen_ops[core_op_id]
                if key not in prev.aliases:
                    merged_aliases = prev.aliases + (key,)
                    seen_ops[core_op_id] = HandlerBranch(
                        aliases=merged_aliases,
                        owner=prev.owner,
                        source_file=prev.source_file,
                        core_op_id=core_op_id,
                        handler_key=prev.handler_key,
                        layer=prev.layer,
                        lowering_kind=prev.lowering_kind,
                    )
                continue
            handler_key = _pick_handler_key([key])
            layer, lowering = classify_layer_and_lowering(core_op_id, handler_key)
            seen_ops[core_op_id] = HandlerBranch(
                aliases=(key,),
                owner=owner,
                source_file=rel,
                core_op_id=core_op_id,
                handler_key=handler_key,
                layer=layer,
                lowering_kind=lowering,
            )

    return sorted(seen_ops.values(), key=lambda b: b.core_op_id)


def alias_to_core_op_map(branches: list[HandlerBranch]) -> dict[str, str]:
    """Map every legacy callee literal (and normalized form) to CoreOpId."""
    out: dict[str, str] = {}
    for branch in branches:
        for alias in branch.aliases:
            out[alias] = branch.core_op_id
            out[normalize_key(alias)] = branch.core_op_id
    return out


def core_op_to_handler_map(branches: list[HandlerBranch]) -> dict[str, str]:
    return {b.core_op_id: b.handler_key for b in branches}
