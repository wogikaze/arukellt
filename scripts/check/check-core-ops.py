#!/usr/bin/env python3
"""CoreOps registry schema sanity check.

Validates data/core-ops.toml structure and its cross-references to
std/manifest.toml. Does not parse manifest type strings yet (see
TypeExpr grammar in docs/compiler/core-ops-registry.md).

Exit codes:
  0  no errors (warnings may be printed for scaffold placeholders)
  1  schema errors detected
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

try:
    import tomllib  # type: ignore
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
CORE_OPS = ROOT / "data" / "core-ops.toml"
MANIFEST = ROOT / "std" / "manifest.toml"

EXPECTED_SCHEMA_VERSION = 4

VALID_VISIBILITY = {"public", "internal"}
VALID_CLASSIFICATION = {"primitive", "runtime", "semantic_stdlib", "target_raw"}
VALID_BINDING_POLICY = {"required", "optional", "forbidden"}
VALID_INLINE_POLICY = {"never", "hint", "always"}
VALID_LOWERING_KIND = {"normal_call", "mir_op", "runtime_call", "target_intrinsic"}
VALID_RUNTIME_KIND = {"internal", "wit", "native"}
VALID_WHEN_KEYS = {"backend", "target_family", "portable_simd_lowering", "wasm_raw_v128", "wasm_relaxed_simd"}
VALID_PRIMITIVE_NAMES = {
    "i32", "i64", "f32", "f64", "bool", "char",
    "u8", "i8", "u16", "i16", "u32", "u64",
    "unit", "never",
}
VALID_TYPEEXPR_KINDS = {"ref", "primitive", "var", "tuple", "function"}
VALID_CONSTRAINT_KINDS = {"trait", "type_eq", "capability"}
VALID_SEMANTICS_FIELDS = {"const_evaluable", "overflow", "nan", "trap", "equivalence"}
VALID_EFFECT_FIELDS = {"memory", "allocates", "may_trap", "noreturn", "external_io", "nondeterminism", "atomic", "volatile"}
VALID_NAN_VALUES = {"none", "canonical", "preserve_nan_class_payload_unspecified"}
VALID_EQUIVALENCE_VALUES = {"exact_bool", "exact_bitwise", "float_value_nan_payload_ignored", "noreturn", "set_order_agnostic"}


def load_toml(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def validate_type_expr(expr, generic_params: set[str], type_ids: set[str], path: str, errors: list[str]) -> None:
    if not isinstance(expr, dict):
        errors.append(f"{path}: TypeExpr must be a table")
        return
    kind = expr.get("kind")
    if kind not in VALID_TYPEEXPR_KINDS:
        errors.append(f"{path}: unknown TypeExpr kind {kind!r}")
        return
    if kind == "ref":
        name = expr.get("name")
        if not isinstance(name, str):
            errors.append(f"{path}: ref TypeExpr needs a string name")
            return
        if name not in type_ids and name not in VALID_PRIMITIVE_NAMES:
            errors.append(f"{path}: ref TypeExpr {name!r} is not a known type id or primitive")
        for i, arg in enumerate(expr.get("args", [])):
            validate_type_expr(arg, generic_params, type_ids, f"{path}.args[{i}]", errors)
    elif kind == "primitive":
        name = expr.get("name")
        if name not in VALID_PRIMITIVE_NAMES:
            errors.append(f"{path}: primitive {name!r} is not in canonical set")
    elif kind == "var":
        name = expr.get("name")
        if name not in generic_params:
            errors.append(f"{path}: var {name!r} is not in generic_params {generic_params}")
    elif kind == "tuple":
        elements = expr.get("elements", [])
        if not isinstance(elements, list):
            errors.append(f"{path}: tuple elements must be a list")
        else:
            for i, elem in enumerate(elements):
                validate_type_expr(elem, generic_params, type_ids, f"{path}.elements[{i}]", errors)
    elif kind == "function":
        params = expr.get("params", [])
        if not isinstance(params, list):
            errors.append(f"{path}: function params must be a list")
        else:
            for i, p in enumerate(params):
                validate_type_expr(p, generic_params, type_ids, f"{path}.params[{i}]", errors)
        result = expr.get("result")
        if result is None:
            errors.append(f"{path}: function TypeExpr needs a result")
        else:
            validate_type_expr(result, generic_params, type_ids, f"{path}.result", errors)


def validate_signature(op: dict, type_ids: set[str], errors: list[str]) -> None:
    sig = op.get("signature", {})
    generic_params = set(sig.get("generic_params", []))
    inputs = sig.get("inputs", [])
    if not isinstance(inputs, list):
        errors.append(f"{op['id']}: signature.inputs must be a list")
        return
    for i, inp in enumerate(inputs):
        if not isinstance(inp, dict):
            errors.append(f"{op['id']}: signature.inputs[{i}] must be a table")
            continue
        if not isinstance(inp.get("name"), str):
            errors.append(f"{op['id']}: signature.inputs[{i}] needs a name")
        validate_type_expr(inp.get("type"), generic_params, type_ids, f"{op['id']}.inputs[{i}].type", errors)

    outputs = sig.get("outputs", [])
    if not isinstance(outputs, list):
        errors.append(f"{op['id']}: signature.outputs must be a list")
    else:
        for i, out in enumerate(outputs):
            validate_type_expr(out.get("type") if isinstance(out, dict) else out, generic_params, type_ids, f"{op['id']}.outputs[{i}]", errors)

    receiver_index = sig.get("receiver_index")
    if receiver_index is not None:
        if not isinstance(receiver_index, int) or receiver_index < 0 or receiver_index >= len(inputs):
            errors.append(f"{op['id']}: receiver_index {receiver_index} is out of range for {len(inputs)} inputs")

    for i, c in enumerate(sig.get("constraints", [])):
        if not isinstance(c, dict):
            errors.append(f"{op['id']}: signature.constraints[{i}] must be a table")
            continue
        if c.get("kind") not in VALID_CONSTRAINT_KINDS:
            errors.append(f"{op['id']}: signature.constraints[{i}] has unknown kind {c.get('kind')!r}")


def validate_lowering(op: dict, errors: list[str]) -> None:
    lowering = op.get("lowering", {})
    kind = lowering.get("kind")
    if kind not in VALID_LOWERING_KIND:
        errors.append(f"{op['id']}: lowering.kind {kind!r} is invalid")
        return

    if kind == "mir_op":
        mir = lowering.get("mir", {})
        if not mir.get("opcode") and not mir.get("operation"):
            errors.append(f"{op['id']}: mir_op lowering needs lowering.mir.opcode or operation")

    elif kind == "runtime_call":
        runtime = lowering.get("runtime", {})
        runtime_kind = runtime.get("kind")
        if runtime_kind not in VALID_RUNTIME_KIND:
            errors.append(f"{op['id']}: runtime_call runtime.kind {runtime_kind!r} is invalid")
        else:
            if runtime_kind == "internal":
                if not isinstance(runtime.get("symbol"), str):
                    errors.append(f"{op['id']}: runtime internal lowering needs symbol")
                if not isinstance(runtime.get("abi_version"), str):
                    errors.append(f"{op['id']}: runtime internal lowering needs abi_version")
            elif runtime_kind == "wit":
                for key in ("package", "interface", "function", "version"):
                    if not isinstance(runtime.get(key), str):
                        errors.append(f"{op['id']}: runtime wit lowering needs {key}")
            elif runtime_kind == "native":
                for key in ("backend", "symbol", "abi_version"):
                    if not isinstance(runtime.get(key), str):
                        errors.append(f"{op['id']}: runtime native lowering needs {key}")

    elif kind == "target_intrinsic":
        target = lowering.get("target", {})
        if not isinstance(target.get("target_family"), str):
            errors.append(f"{op['id']}: target_intrinsic needs target_family")
        if not isinstance(target.get("target_id"), str):
            errors.append(f"{op['id']}: target_intrinsic needs target_id (backend handler key)")
        if not isinstance(target.get("required_capabilities", []), list):
            errors.append(f"{op['id']}: target_intrinsic required_capabilities must be a list")
        if not isinstance(target.get("required_target_features", []), list):
            errors.append(f"{op['id']}: target_intrinsic required_target_features must be a list")


def validate_specialization(op: dict, spec: dict, index: int, errors: list[str]) -> None:
    prefix = f"{op['id']}.specializations[{index}]"
    if not isinstance(spec.get("priority"), int):
        errors.append(f"{prefix}: priority must be an integer")
    when = spec.get("when", {})
    if not isinstance(when, dict):
        errors.append(f"{prefix}: when must be a table")
    else:
        for key in when:
            if key not in VALID_WHEN_KEYS:
                errors.append(f"{prefix}: when key {key!r} is not allowed")
    lowering = spec.get("lowering", {})
    if not isinstance(lowering, dict):
        errors.append(f"{prefix}: lowering must be a table")
    else:
        # Temporarily wrap lowering into a fake op dict for reuse.
        fake_op = {"id": prefix, "lowering": lowering}
        validate_lowering(fake_op, errors)


def validate_fallback(op: dict, status: str, strict: bool, errors: list[str], warnings: list[str]) -> None:
    fallback = op.get("fallback", {})
    required = fallback.get("required", False)
    symbol = fallback.get("implementation_symbol")
    if required and not isinstance(symbol, str):
        errors.append(f"{op['id']}: fallback.required is true but implementation_symbol is missing")
    elif isinstance(symbol, str):
        if symbol.startswith("example.invalid."):
            msg = f"{op['id']}: fallback implementation_symbol is an example placeholder ({symbol})"
            if strict or status != "scaffold":
                errors.append(msg)
            else:
                warnings.append(msg)


def validate_operation(op: dict, type_ids: set[str], status: str, strict: bool, errors: list[str], warnings: list[str]) -> None:
    if not isinstance(op.get("id"), str):
        errors.append("operation entry missing id")
        return
    op_id = op["id"]

    visibility = op.get("visibility")
    if visibility not in VALID_VISIBILITY:
        errors.append(f"{op_id}: visibility {visibility!r} is invalid")

    classification = op.get("classification", {})
    layer = classification.get("layer")
    if layer not in VALID_CLASSIFICATION:
        errors.append(f"{op_id}: classification.layer {layer!r} is invalid")

    binding = op.get("binding", {})
    policy = binding.get("policy")
    if policy not in VALID_BINDING_POLICY:
        errors.append(f"{op_id}: binding.policy {policy!r} is invalid")
    else:
        if visibility == "public" and policy == "forbidden":
            errors.append(f"{op_id}: public operation cannot have binding.policy = forbidden")
        if visibility == "internal" and policy == "required":
            errors.append(f"{op_id}: internal operation cannot have binding.policy = required")
        if visibility == "internal" and policy == "optional":
            warnings.append(f"{op_id}: internal operation with binding.policy = optional should be documented")
        if policy == "optional":
            if not isinstance(binding.get("reason"), str):
                errors.append(f"{op_id}: binding.optional must include reason")
            if not isinstance(binding.get("tracking_issue"), str):
                warnings.append(f"{op_id}: binding.optional should include a tracking_issue or RFC reference")

    if op.get("inline", {}).get("policy") not in VALID_INLINE_POLICY:
        errors.append(f"{op_id}: inline.policy is invalid")

    for field in VALID_SEMANTICS_FIELDS:
        if field not in op.get("semantics", {}):
            errors.append(f"{op_id}: semantics.{field} is missing")
    semantics = op.get("semantics", {})
    if semantics.get("nan") not in VALID_NAN_VALUES:
        errors.append(f"{op_id}: semantics.nan {semantics.get('nan')!r} is invalid")
    if semantics.get("equivalence") not in VALID_EQUIVALENCE_VALUES:
        errors.append(f"{op_id}: semantics.equivalence {semantics.get('equivalence')!r} is invalid")

    for field in VALID_EFFECT_FIELDS:
        if field not in op.get("effect", {}):
            errors.append(f"{op_id}: effect.{field} is missing")

    validate_signature(op, type_ids, errors)
    validate_lowering(op, errors)
    validate_fallback(op, status, strict, errors, warnings)

    for i, spec in enumerate(op.get("specializations", [])):
        validate_specialization(op, spec, i, errors)


def validate_manifest_refs(core_ops: dict, manifest: dict, errors: list[str]) -> None:
    op_ids = {op["id"] for op in core_ops.get("operations", [])}
    type_ids = {t["id"] for t in core_ops.get("types", [])}

    for fn in manifest.get("functions", []):
        core_op_id = fn.get("core_op_id")
        if core_op_id and core_op_id not in op_ids:
            errors.append(f"std/manifest.toml function {fn.get('name')!r}: core_op_id {core_op_id!r} not found in core-ops")

    for typ in manifest.get("types", []):
        type_id = typ.get("type_id")
        if type_id and type_id not in type_ids:
            errors.append(f"std/manifest.toml type {typ.get('name')!r}: type_id {type_id!r} not found in core-ops")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate core-ops.toml schema")
    parser.add_argument("--strict", action="store_true", help="fail on example.invalid fallback symbols even if status is scaffold")
    args = parser.parse_args()

    errors: list[str] = []
    warnings: list[str] = []

    try:
        core_ops = load_toml(CORE_OPS)
    except Exception as e:
        print(f"core-ops.toml を読み込めません: {e}", file=sys.stderr)
        return 1

    try:
        manifest = load_toml(MANIFEST)
    except Exception as e:
        print(f"std/manifest.toml を読み込めません: {e}", file=sys.stderr)
        return 1

    schema_version = core_ops.get("schema_version")
    if schema_version != EXPECTED_SCHEMA_VERSION:
        errors.append(f"schema_version is {schema_version!r}, expected {EXPECTED_SCHEMA_VERSION}")

    status = core_ops.get("status", "scaffold")
    if status not in {"scaffold", "production"}:
        errors.append(f"status {status!r} is invalid")

    types = core_ops.get("types", [])
    type_ids = set()
    for t in types:
        if not isinstance(t.get("id"), str):
            errors.append("types entry missing id")
        else:
            type_ids.add(t["id"])

    for op in core_ops.get("operations", []):
        validate_operation(op, type_ids, status, args.strict, errors, warnings)

    validate_manifest_refs(core_ops, manifest, errors)

    for w in warnings:
        print(f"警告: {w}")

    if errors:
        for e in errors:
            print(f"エラー: {e}", file=sys.stderr)
        print(f"core-ops エラー {len(errors)} 件", file=sys.stderr)
        return 1

    print("core-ops OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
