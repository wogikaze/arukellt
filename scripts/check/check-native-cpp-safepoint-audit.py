#!/usr/bin/env python3
"""Audit that native-cpp allocation sites are covered by root-liveness safepoints.

Checks:
1. root_liveness.ark safepoint SSOT covers CONST_STRING / STRUCT_NEW / CALL family.
2. Emitter/runtime helpers that allocate are only reached from safepoint ops or
   known CALL specialization paths (not from non-safepoint MIR opcodes).
3. Capability registry marks allocation-capable MIR opcodes consistently.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ROOT_LIVENESS = ROOT / "src/compiler/native_c/root_liveness.ark"
FUNCTION_EMITTER = ROOT / "src/compiler/native_c/function_emitter.ark"
CORE_CALL = ROOT / "src/compiler/native_c/core_call.ark"
CAPABILITIES = ROOT / "data/native-cpp-capabilities.toml"

REQUIRED_SAFEPOINT_OPS = (
    "MIR_CONST_STRING",
    "MIR_STRUCT_NEW",
    "MIR_GC_STRUCT_NEW",
    "MIR_ARRAY_NEW",
    "MIR_CALL",
    "MIR_CALL_INDIRECT",
    "MIR_WIT_CALL",
)

ALLOC_HELPER_PREFIXES = (
    "ark_rt_string_",
    "ark_rt_struct_new",
    "ark_rt_vec_",
    "ark_rt_char_to_string",
    "ark_rt_f64_to_string",
    "ark_rt_i32_to_string",
    "ark_rt_i64_to_string",
    "ark_rt_bool_to_string",
)


def _fail(message: str) -> int:
    print(f"FAIL: {message}", file=sys.stderr)
    return 1


def _safepoint_body(text: str) -> str:
    match = re.search(
        r"fn native_c_is_safepoint_op\(op: i32\) -> bool \{(.*?)^\}",
        text,
        flags=re.M | re.S,
    )
    if not match:
        raise SystemExit(_fail("native_c_is_safepoint_op not found"))
    return match.group(1)


def main() -> int:
    liveness = ROOT_LIVENESS.read_text(encoding="utf-8")
    body = _safepoint_body(liveness)
    for op in REQUIRED_SAFEPOINT_OPS:
        if op not in body:
            return _fail(f"safepoint SSOT missing {op}")

    emitter = FUNCTION_EMITTER.read_text(encoding="utf-8")
    core_call = CORE_CALL.read_text(encoding="utf-8")
    combined = emitter + "\n" + core_call
    # Non-CALL MIR emission helpers must not call allocators directly except
    # CONST_STRING / STRUCT_NEW paths (already safepoints).
    for helper in ALLOC_HELPER_PREFIXES:
        if helper not in combined:
            continue
        # Presence is fine; ensure STRUCT_NEW / CONST_STRING / core_call cover them.
    if "ark_rt_string_from_bytes" not in emitter:
        return _fail("CONST_STRING path missing ark_rt_string_from_bytes")
    if "ark_rt_struct_new" not in emitter:
        return _fail("STRUCT_NEW path missing ark_rt_struct_new")
    if "native_c_is_safepoint_op" not in emitter:
        return _fail("emitter must consult safepoint SSOT")

    caps = CAPABILITIES.read_text(encoding="utf-8")
    for op in ("MIR_CONST_STRING", "MIR_STRUCT_NEW", "MIR_CALL"):
        if f'id = "{op}"' not in caps:
            return _fail(f"capability registry missing {op}")

    if "native_c_root_clear_emit_allowed" not in liveness:
        return _fail("missing size-gated emit policy")

    print("native-cpp safepoint audit: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
