#!/usr/bin/env python3
"""CoreOps registry schema and manifest-semantic checker.

Validates `data/core-ops.toml` structure and cross-references `std/manifest.toml`.
This is the structural + manifest-semantic layer. Compiler-aware validation
(fallback symbol resolution, call cycle detection, full signature compatibility
against Ark types, effect consistency, target handler registry) is intentionally
not in this file and is listed as pending in the output.
"""
from __future__ import annotations

import argparse
import itertools
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
VALID_MEMORY_VALUES = {"none", "read", "write", "read-write"}
VALID_NONDETERMINISM_VALUES = {"deterministic", "clock", "random", "external"}
VALID_OVERFLOW_VALUES = {"none", "wrap", "saturate", "ieee754"}
VALID_TRAP_VALUES = {"none", "memory", "arithmetic", "unreachable"}
VALID_TYPE_KINDS = {"string", "vec", "simd", "mask", "wasm_v128"}
VALID_LANE_TYPES = {"i8", "i16", "i32", "i64", "f32", "f64"}
VALID_CAPABILITY_NAMES = {"wasm_raw_v128", "wasm_relaxed_simd"}
VALID_TARGET_FEATURES = {"simd128"}  # extend as target features are added
VALID_WHEN_VALUES = {
    "backend": {"wasm", "native", "llvm", "interpreter"},
    "target_family": {"wasm", "native", "llvm"},
    "portable_simd_lowering": {"NativeSimd", "Scalar", "Unsupported"},
    "wasm_raw_v128": {"Enabled", "Disabled"},
    "wasm_relaxed_simd": {"Enabled", "Disabled"},
}
VALID_RUNTIME_FIELDS = {
    "internal": {"symbol", "abi_version"},
    "wit": {"package", "interface", "function", "version"},
    "native": {"backend", "symbol", "abi_version"},
}
VALID_EFFECT_BOOL_FIELDS = {"allocates", "may_trap", "noreturn", "external_io", "atomic", "volatile"}
VALID_SEMANTICS_ENUM_FIELDS = {
    "overflow": VALID_OVERFLOW_VALUES,
    "trap": VALID_TRAP_VALUES,
    "nan": VALID_NAN_VALUES,
    "equivalence": VALID_EQUIVALENCE_VALUES,
}

# Contextual alias for v128 in std::wasm. std::simd uses explicit SIMD types (i32x4 etc.).
V128_ALIAS = "wasm.v128"

EXPECTED_PYTHON_VALIDATION_KEYS = {
    "check_unreferenced_required_bindings",
    "check_public_binding_collisions",
    "check_signature_compat",
    "check_effect_lowering_consistency",
    "check_binding_field_consistency",
    "check_forbidden_bindings",
    "check_fallback_resolvable",
    "check_specialization_ambiguity",
}
EXPECTED_COMPILER_VALIDATION_KEYS = {
    "check_fallback_no_cycle",
    "check_fallback_signature_compat",
}


def load_toml(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


class TypeParseError(Exception):
    pass


class TypeParser:
    """Recursive-descent parser for Arukellt type strings."""

    def __init__(self, text: str, generic_params: set[str], aliases: dict[str, str]):
        self.text = text
        self.pos = 0
        self.generic_params = generic_params
        self.aliases = aliases

    def _peek(self) -> str:
        self._skip_ws()
        if self.pos >= len(self.text):
            return ""
        return self.text[self.pos]

    def _get(self) -> str:
        self._skip_ws()
        if self.pos >= len(self.text):
            return ""
        ch = self.text[self.pos]
        self.pos += 1
        return ch

    def _skip_ws(self) -> None:
        while self.pos < len(self.text) and self.text[self.pos].isspace():
            self.pos += 1

    def _expect(self, expected: str) -> None:
        self._skip_ws()
        if self.pos + len(expected) > len(self.text):
            raise TypeParseError(f"expected {expected!r} at end of type {self.text!r}")
        if self.text[self.pos : self.pos + len(expected)] != expected:
            raise TypeParseError(f"expected {expected!r} at position {self.pos} in {self.text!r}")
        self.pos += len(expected)

    def _parse_ident(self) -> str:
        self._skip_ws()
        start = self.pos
        if self.pos >= len(self.text):
            raise TypeParseError(f"expected identifier at end of type {self.text!r}")
        ch = self.text[self.pos]
        if not (ch.isalpha() or ch == "_"):
            raise TypeParseError(f"expected identifier at {self.text[self.pos:]!r}")
        self.pos += 1
        while self.pos < len(self.text) and (self.text[self.pos].isalnum() or self.text[self.pos] == "_"):
            self.pos += 1
        return self.text[start:self.pos]

    def parse(self) -> dict:
        result = self._parse_type()
        self._skip_ws()
        if self.pos != len(self.text):
            raise TypeParseError(f"trailing characters in type {self.text!r}")
        return result

    def _parse_type(self) -> dict:
        self._skip_ws()
        if self._peek() == "(":
            self._get()
            self._skip_ws()
            if self._peek() == ")":
                self._get()
                return {"kind": "primitive", "name": "unit"}
            elements = []
            while True:
                elements.append(self._parse_type())
                self._skip_ws()
                if self._peek() == ",":
                    self._get()
                    continue
                if self._peek() == ")":
                    self._get()
                    break
                raise TypeParseError(f"expected ',' or ')' in tuple in {self.text!r}")
            return {"kind": "tuple", "elements": elements}

        if self.text[self.pos : self.pos + 2] == "fn":
            self._expect("fn")
            self._expect("(")
            params = []
            self._skip_ws()
            if self._peek() != ")":
                while True:
                    params.append(self._parse_type())
                    self._skip_ws()
                    if self._peek() == ",":
                        self._get()
                        continue
                    if self._peek() == ")":
                        break
                    raise TypeParseError(f"expected ',' or ')' in function params in {self.text!r}")
            self._expect(")")
            self._expect("->")
            result = self._parse_type()
            return {"kind": "function", "params": params, "result": result}

        ident = self._parse_ident()
        # Primitive: lowercase-only identifier in the primitive set.
        if ident.islower() and ident in VALID_PRIMITIVE_NAMES:
            return {"kind": "primitive", "name": ident}

        # Generic type variable
        if ident in self.generic_params:
            return {"kind": "var", "name": ident}

        # Public name alias (must be declared in manifest types or context)
        if ident in self.aliases:
            canonical = self.aliases[ident]
        else:
            raise TypeParseError(f"unknown type identifier {ident!r}")
        args = []
        self._skip_ws()
        if self._peek() == "<":
            self._get()
            while True:
                args.append(self._parse_type())
                self._skip_ws()
                if self._peek() == ",":
                    self._get()
                    continue
                if self._peek() == ">":
                    self._get()
                    break
                raise TypeParseError(f"expected ',' or '>' in generic args in {self.text!r}")
        return {"kind": "ref", "name": canonical, "args": args}


def parse_manifest_type(text: str, generic_params: set[str], aliases: dict[str, str]) -> dict:
    parser = TypeParser(text, generic_params, aliases)
    return parser.parse()


def public_symbol(fn: dict) -> str:
    """Canonical public symbol key for a manifest function."""
    if "module" in fn and isinstance(fn["module"], str):
        return f"{fn['module']}::{fn.get('name', '')}"
    if fn.get("prelude") and fn.get("kind") == "prelude_wrapper":
        return f"prelude::{fn.get('name', '')}"
    return fn.get("name", "")


def type_expr_to_key(expr: dict) -> str:
    """Serialize a TypeExpr for comparison and error messages."""
    if not isinstance(expr, dict):
        return repr(expr)
    kind = expr.get("kind")
    if kind == "ref":
        args = ",".join(type_expr_to_key(a) for a in expr.get("args", []))
        name = expr.get("name", "")
        if args:
            return f"{name}<{args}>"
        return name
    if kind == "primitive":
        return expr.get("name", "")
    if kind == "var":
        return f"var {expr.get('name', '')}"
    if kind == "tuple":
        return "(" + ",".join(type_expr_to_key(e) for e in expr.get("elements", [])) + ")"
    if kind == "function":
        params = ",".join(type_expr_to_key(p) for p in expr.get("params", []))
        result = type_expr_to_key(expr.get("result", {}))
        return f"fn({params})->{result}"
    return repr(expr)


def _type_guard_dict(value, path: str, errors: list[str]) -> bool:
    if not isinstance(value, dict):
        errors.append(f"{path}: expected a table, got {type(value).__name__}")
        return False
    return True


def _type_guard_list(value, path: str, errors: list[str]) -> bool:
    if not isinstance(value, list):
        errors.append(f"{path}: expected a list, got {type(value).__name__}")
        return False
    return True


def _type_guard_str(value, path: str, errors: list[str]) -> bool:
    if not isinstance(value, str):
        errors.append(f"{path}: expected a string, got {type(value).__name__}")
        return False
    return True


def validate_type_expr(
    expr,
    generic_params: set[str],
    type_entries: dict[str, dict],
    primitive_names: set[str],
    path: str,
    errors: list[str],
) -> None:
    if not isinstance(expr, dict):
        errors.append(f"{path}: TypeExpr must be a table")
        return
    if len(set(expr.keys()) - {"kind", "name", "args", "elements", "params", "result"}) > 0:
        unknown = set(expr.keys()) - {"kind", "name", "args", "elements", "params", "result"}
        errors.append(f"{path}: unknown TypeExpr fields {sorted(unknown)}")
    kind = expr.get("kind")
    if kind not in VALID_TYPEEXPR_KINDS:
        errors.append(f"{path}: unknown TypeExpr kind {kind!r}")
        return
    if kind == "ref":
        name = expr.get("name")
        if not _type_guard_str(name, f"{path}.name", errors):
            return
        if name in primitive_names:
            errors.append(f"{path}: ref {name!r} is a primitive name; use kind = primitive")
            return
        if name not in type_entries:
            errors.append(f"{path}: ref TypeExpr {name!r} is not a known type id")
            return
        entry = type_entries[name]
        expected_arity = len(entry.get("generic_params", []))
        args = expr.get("args", [])
        if not _type_guard_list(args, f"{path}.args", errors):
            return
        if len(args) != expected_arity:
            errors.append(f"{path}: ref {name!r} expects {expected_arity} generic args, got {len(args)}")
        else:
            for i, arg in enumerate(args):
                validate_type_expr(arg, generic_params, type_entries, primitive_names, f"{path}.args[{i}]", errors)
    elif kind == "primitive":
        name = expr.get("name")
        if name not in primitive_names:
            errors.append(f"{path}: primitive {name!r} is not in canonical set {sorted(primitive_names)}")
    elif kind == "var":
        name = expr.get("name")
        if not _type_guard_str(name, f"{path}.name", errors):
            return
        if name not in generic_params:
            errors.append(f"{path}: var {name!r} is not in generic_params {sorted(generic_params)}")
    elif kind == "tuple":
        elements = expr.get("elements", [])
        if not _type_guard_list(elements, f"{path}.elements", errors):
            return
        for i, elem in enumerate(elements):
            validate_type_expr(elem, generic_params, type_entries, primitive_names, f"{path}.elements[{i}]", errors)
    elif kind == "function":
        params = expr.get("params", [])
        if not _type_guard_list(params, f"{path}.params", errors):
            return
        for i, p in enumerate(params):
            validate_type_expr(p, generic_params, type_entries, primitive_names, f"{path}.params[{i}]", errors)
        result = expr.get("result")
        if result is None:
            errors.append(f"{path}: function TypeExpr needs a result")
        else:
            validate_type_expr(result, generic_params, type_entries, primitive_names, f"{path}.result", errors)


def validate_type_definition(t: dict, errors: list[str]) -> None:
    if not isinstance(t, dict):
        errors.append("types entry must be a table")
        return
    if not _type_guard_str(t.get("id"), "types[].id", errors):
        return
    type_id = t["id"]
    kind = t.get("kind")
    if kind not in VALID_TYPE_KINDS:
        errors.append(f"type {type_id}: kind {kind!r} is invalid")
        return
    if not _type_guard_list(t.get("generic_params", []), f"type {type_id}.generic_params", errors):
        return
    generics = t.get("generic_params", [])
    seen = set()
    for i, g in enumerate(generics):
        if not isinstance(g, str):
            errors.append(f"type {type_id}: generic_params[{i}] must be a string")
        elif g in seen:
            errors.append(f"type {type_id}: duplicate generic_param {g!r}")
        seen.add(g)
    if kind == "simd":
        lane_type = t.get("lane_type")
        if lane_type not in VALID_LANE_TYPES:
            errors.append(f"type {type_id}: lane_type {lane_type!r} invalid")
        lanes = t.get("lanes")
        if not isinstance(lanes, int) or lanes <= 0:
            errors.append(f"type {type_id}: lanes must be a positive integer")
    elif kind == "mask":
        lanes = t.get("lanes")
        if not isinstance(lanes, int) or lanes <= 0:
            errors.append(f"type {type_id}: lanes must be a positive integer")
    elif kind == "vec":
        if len(generics) != 1:
            errors.append(f"type {type_id}: vec requires exactly one generic_param")


def validate_signature(op: dict, type_entries: dict[str, dict], primitive_names: set[str], errors: list[str]) -> None:
    sig = op.get("signature", {})
    if not _type_guard_dict(sig, f"{op['id']}.signature", errors):
        return

    generics_raw = sig.get("generic_params", [])
    if not _type_guard_list(generics_raw, f"{op['id']}.signature.generic_params", errors):
        return
    generic_params = set()
    seen = set()
    for i, g in enumerate(generics_raw):
        if not isinstance(g, str):
            errors.append(f"{op['id']}: generic_params[{i}] must be a string")
        elif g in seen:
            errors.append(f"{op['id']}: duplicate generic_param {g!r}")
        else:
            generic_params.add(g)
        seen.add(g)

    inputs = sig.get("inputs", [])
    if not _type_guard_list(inputs, f"{op['id']}.signature.inputs", errors):
        return
    for i, inp in enumerate(inputs):
        if not _type_guard_dict(inp, f"{op['id']}.signature.inputs[{i}]", errors):
            continue
        if not _type_guard_str(inp.get("name"), f"{op['id']}.signature.inputs[{i}].name", errors):
            pass
        validate_type_expr(inp.get("type"), generic_params, type_entries, primitive_names, f"{op['id']}.signature.inputs[{i}].type", errors)

    outputs = sig.get("outputs", [])
    if not _type_guard_list(outputs, f"{op['id']}.signature.outputs", errors):
        return
    for i, out in enumerate(outputs):
        if isinstance(out, dict):
            validate_type_expr(out.get("type"), generic_params, type_entries, primitive_names, f"{op['id']}.signature.outputs[{i}]", errors)
        else:
            validate_type_expr(out, generic_params, type_entries, primitive_names, f"{op['id']}.signature.outputs[{i}]", errors)

    receiver_index = sig.get("receiver_index")
    if receiver_index is not None:
        if not isinstance(receiver_index, int) or receiver_index < 0 or receiver_index >= len(inputs):
            errors.append(f"{op['id']}: receiver_index {receiver_index} is out of range for {len(inputs)} inputs")

    constraints = sig.get("constraints", [])
    if not _type_guard_list(constraints, f"{op['id']}.signature.constraints", errors):
        return
    for i, c in enumerate(constraints):
        if not _type_guard_dict(c, f"{op['id']}.signature.constraints[{i}]", errors):
            continue
        if len(set(c.keys()) - {"kind", "trait", "params", "lhs", "rhs", "capability"}) > 0:
            unknown = set(c.keys()) - {"kind", "trait", "params", "lhs", "rhs", "capability"}
            errors.append(f"{op['id']}: signature.constraints[{i}] has unknown fields {sorted(unknown)}")
        kind = c.get("kind")
        if kind not in VALID_CONSTRAINT_KINDS:
            errors.append(f"{op['id']}: signature.constraints[{i}] has unknown kind {kind!r}")
            continue
        if kind == "trait":
            if not _type_guard_str(c.get("trait"), f"{op['id']}.signature.constraints[{i}].trait", errors):
                pass
            params = c.get("params", [])
            if not _type_guard_list(params, f"{op['id']}.signature.constraints[{i}].params", errors):
                pass
            else:
                for j, p in enumerate(params):
                    validate_type_expr(p, generic_params, type_entries, primitive_names, f"{op['id']}.signature.constraints[{i}].params[{j}]", errors)
        elif kind == "type_eq":
            for side in ("lhs", "rhs"):
                if c.get(side) is None:
                    errors.append(f"{op['id']}: signature.constraints[{i}].type_eq needs {side}")
                else:
                    validate_type_expr(c.get(side), generic_params, type_entries, primitive_names, f"{op['id']}.signature.constraints[{i}].{side}", errors)
        elif kind == "capability":
            if not _type_guard_str(c.get("capability"), f"{op['id']}.signature.constraints[{i}].capability", errors):
                pass


def validate_runtime_payload(op_id: str, runtime: dict, errors: list[str]) -> None:
    kind = runtime.get("kind")
    if kind not in VALID_RUNTIME_KIND:
        errors.append(f"{op_id}: runtime_call runtime.kind {kind!r} is invalid")
        return
    allowed = VALID_RUNTIME_FIELDS[kind]
    for key in runtime:
        if key == "kind":
            continue
        if key not in allowed:
            errors.append(f"{op_id}: runtime.kind = {kind} must not contain field {key!r}")
    for key in allowed:
        if not isinstance(runtime.get(key), str):
            errors.append(f"{op_id}: runtime {kind} requires string field {key!r}")


def validate_target_payload(op_id: str, target: dict, errors: list[str]) -> None:
    if not _type_guard_str(target.get("target_family"), f"{op_id}.lowering.target.target_family", errors):
        pass
    if not _type_guard_str(target.get("target_id"), f"{op_id}.lowering.target.target_id", errors):
        pass
    caps = target.get("required_capabilities", [])
    if not _type_guard_list(caps, f"{op_id}.lowering.target.required_capabilities", errors):
        pass
    else:
        for i, c in enumerate(caps):
            if not isinstance(c, str):
                errors.append(f"{op_id}: required_capabilities[{i}] must be a string")
            elif c not in VALID_CAPABILITY_NAMES:
                errors.append(f"{op_id}: required_capabilities[{i}] {c!r} is not a known capability")
    feats = target.get("required_target_features", [])
    if not _type_guard_list(feats, f"{op_id}.lowering.target.required_target_features", errors):
        pass
    else:
        for i, f in enumerate(feats):
            if not isinstance(f, str):
                errors.append(f"{op_id}: required_target_features[{i}] must be a string")
            elif f not in VALID_TARGET_FEATURES:
                errors.append(f"{op_id}: required_target_features[{i}] {f!r} is not a known target feature")


def validate_lowering(op: dict, errors: list[str]) -> None:
    lowering = op.get("lowering", {})
    if not _type_guard_dict(lowering, f"{op['id']}.lowering", errors):
        return
    kind = lowering.get("kind")
    if kind not in VALID_LOWERING_KIND:
        errors.append(f"{op['id']}: lowering.kind {kind!r} is invalid")
        return

    allowed_sub = {"normal_call": set(), "mir_op": {"mir"}, "runtime_call": {"runtime"}, "target_intrinsic": {"target"}}[kind]
    for key in lowering:
        if key == "kind":
            continue
        if key not in allowed_sub:
            errors.append(f"{op['id']}: lowering.kind = {kind} must not contain lowering.{key}")

    if kind == "mir_op":
        mir = lowering.get("mir", {})
        if not _type_guard_dict(mir, f"{op['id']}.lowering.mir", errors):
            pass
        elif not mir.get("opcode") and not mir.get("operation"):
            errors.append(f"{op['id']}: mir_op lowering needs lowering.mir.opcode or operation")
    elif kind == "runtime_call":
        runtime = lowering.get("runtime", {})
        if not _type_guard_dict(runtime, f"{op['id']}.lowering.runtime", errors):
            pass
        else:
            validate_runtime_payload(op["id"], runtime, errors)
    elif kind == "target_intrinsic":
        target = lowering.get("target", {})
        if not _type_guard_dict(target, f"{op['id']}.lowering.target", errors):
            pass
        else:
            validate_target_payload(op["id"], target, errors)

    if kind == "normal_call":
        fallback = op.get("fallback", {})
        if not isinstance(fallback, dict):
            errors.append(f"{op['id']}: normal_call requires a fallback table")
        elif not fallback.get("required"):
            errors.append(f"{op['id']}: normal_call requires fallback.required = true")


def validate_fallback(op: dict, status: str, strict: bool, public_symbols: set[str], errors: list[str], warnings: list[str]) -> None:
    fallback = op.get("fallback", {})
    if not isinstance(fallback, dict):
        return
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
        elif symbol in public_symbols or symbol.startswith("std::") or symbol.startswith("prelude::"):
            errors.append(f"{op['id']}: fallback implementation_symbol must not be a public path: {symbol}")


def validate_when(op_id: str, when: dict, index: int, errors: list[str]) -> None:
    if not _type_guard_dict(when, f"{op_id}.specializations[{index}].when", errors):
        return
    for key, value in when.items():
        if key not in VALID_WHEN_KEYS:
            errors.append(f"{op_id}.specializations[{index}]: when key {key!r} is not allowed")
            continue
        if not isinstance(value, str):
            errors.append(f"{op_id}.specializations[{index}]: when.{key} must be a string")
            continue
        if key in VALID_WHEN_VALUES and value not in VALID_WHEN_VALUES[key]:
            errors.append(f"{op_id}.specializations[{index}]: when.{key} value {value!r} is not in known set {sorted(VALID_WHEN_VALUES[key])}")
    # cross-axis constraint: if both backend and target_family are present, they must be compatible
    if "backend" in when and "target_family" in when:
        backend = when["backend"]
        target_family = when["target_family"]
        if backend == "wasm" and target_family != "wasm":
            errors.append(f"{op_id}.specializations[{index}]: when backend = wasm requires target_family = wasm")
        if backend == "native" and target_family not in {"native", "llvm"}:
            errors.append(f"{op_id}.specializations[{index}]: when backend = native requires target_family = native or llvm")
        if backend == "llvm" and target_family != "llvm":
            errors.append(f"{op_id}.specializations[{index}]: when backend = llvm requires target_family = llvm")
        if backend == "interpreter" and target_family != "interpreter":
            errors.append(f"{op_id}.specializations[{index}]: when backend = interpreter cannot target a different family")


def validate_specialization(op: dict, spec: dict, index: int, errors: list[str]) -> None:
    op_id = op["id"]
    prefix = f"{op_id}.specializations[{index}]"
    if not _type_guard_dict(spec, prefix, errors):
        return
    if not isinstance(spec.get("priority"), int):
        errors.append(f"{prefix}: priority must be an integer")
    when = spec.get("when", {})
    validate_when(op_id, when, index, errors)
    lowering = spec.get("lowering", {})
    if not _type_guard_dict(lowering, f"{prefix}.lowering", errors):
        return
    fake_op = {"id": prefix, "lowering": lowering}
    validate_lowering(fake_op, errors)


def _when_domains(specs: list[dict]) -> dict[str, set[str]]:
    """Build a minimal domain for each when-key that is actually used.

    Unused keys are omitted to avoid false-positive ambiguity reports.
    """
    domains: dict[str, set[str]] = {}
    used_keys = set()
    for spec in specs:
        when = spec.get("when", {})
        if isinstance(when, dict):
            for key in when:
                used_keys.add(key)
    for key in used_keys:
        domains[key] = set(VALID_WHEN_VALUES.get(key, set()))
    return domains


def specialization_matches(when: dict, config: dict[str, str]) -> bool:
    for key, value in when.items():
        if config.get(key) != value:
            return False
    return True


def validate_specialization_ambiguity(op: dict, errors: list[str]) -> None:
    specs = op.get("specializations", [])
    if not specs or not isinstance(specs, list):
        return
    domains = _when_domains(specs)
    if not domains:
        return
    keys = list(domains.keys())
    for values in itertools.product(*[sorted(domains[k]) for k in keys]):
        config = dict(zip(keys, values))
        matches = [(i, spec) for i, spec in enumerate(specs) if specialization_matches(spec.get("when", {}), config)]
        if not matches:
            continue
        max_priority = max(spec.get("priority", 0) for _, spec in matches)
        best = [i for i, spec in matches if spec.get("priority", 0) == max_priority]
        if len(best) > 1:
            errors.append(
                f"{op['id']}: specialization ambiguity at configuration {config}: "
                f"specs {best} all have priority {max_priority}"
            )


def validate_binding_and_lowering(op: dict, manifest_refs: dict[str, list[str]], errors: list[str]) -> None:
    op_id = op["id"]
    visibility = op.get("visibility")
    binding = op.get("binding", {})
    if not _type_guard_dict(binding, f"{op_id}.binding", errors):
        return
    policy = binding.get("policy")

    # Unknown binding fields
    if len(set(binding.keys()) - {"policy", "reason", "tracking_issue"}) > 0:
        unknown = set(binding.keys()) - {"policy", "reason", "tracking_issue"}
        errors.append(f"{op_id}: binding has unknown fields {sorted(unknown)}")

    if visibility == "public" and policy == "forbidden":
        errors.append(f"{op_id}: public operation cannot have binding.policy = forbidden")
    if visibility == "internal" and policy == "required":
        errors.append(f"{op_id}: internal operation cannot have binding.policy = required")
    if visibility == "internal" and policy == "optional":
        errors.append(f"{op_id}: internal operation cannot have binding.policy = optional")
    if policy == "optional":
        if not isinstance(binding.get("reason"), str):
            errors.append(f"{op_id}: binding.optional must include reason")
        if not isinstance(binding.get("tracking_issue"), str):
            errors.append(f"{op_id}: binding.optional must include tracking_issue or RFC reference")

    if op.get("inline", {}).get("policy") not in VALID_INLINE_POLICY:
        errors.append(f"{op_id}: inline.policy is invalid")

    # semantics
    semantics = op.get("semantics", {})
    if not _type_guard_dict(semantics, f"{op_id}.semantics", errors):
        semantics = {}
    else:
        unknown_semantics = set(semantics.keys()) - VALID_SEMANTICS_FIELDS
        if unknown_semantics:
            errors.append(f"{op_id}: semantics has unknown fields {sorted(unknown_semantics)}")
        for field in VALID_SEMANTICS_FIELDS:
            if field not in semantics:
                errors.append(f"{op_id}: semantics.{field} is missing")
        for field, allowed in VALID_SEMANTICS_ENUM_FIELDS.items():
            if semantics.get(field) not in allowed:
                errors.append(f"{op_id}: semantics.{field} {semantics.get(field)!r} is invalid")
        if not isinstance(semantics.get("const_evaluable"), bool):
            errors.append(f"{op_id}: semantics.const_evaluable must be a boolean")

    # effect
    effect = op.get("effect", {})
    if not _type_guard_dict(effect, f"{op_id}.effect", errors):
        effect = {}
    else:
        unknown_effect = set(effect.keys()) - VALID_EFFECT_FIELDS
        if unknown_effect:
            errors.append(f"{op_id}: effect has unknown fields {sorted(unknown_effect)}")
        for field in VALID_EFFECT_FIELDS:
            if field not in effect:
                errors.append(f"{op_id}: effect.{field} is missing")
        for field in VALID_EFFECT_BOOL_FIELDS:
            if not isinstance(effect.get(field), bool):
                errors.append(f"{op_id}: effect.{field} must be a boolean")
        if effect.get("memory") not in VALID_MEMORY_VALUES:
            errors.append(f"{op_id}: effect.memory {effect.get('memory')!r} is invalid")
        if effect.get("nondeterminism") not in VALID_NONDETERMINISM_VALUES:
            errors.append(f"{op_id}: effect.nondeterminism {effect.get('nondeterminism')!r} is invalid")

    # lowering/effect consistency
    lowering = op.get("lowering", {})
    if lowering.get("kind") == "normal_call" and effect.get("noreturn"):
        errors.append(f"{op_id}: normal_call with noreturn = true is inconsistent")
    if lowering.get("kind") == "target_intrinsic" and effect.get("noreturn"):
        errors.append(f"{op_id}: target_intrinsic with noreturn = true is inconsistent (use runtime_call)")


def validate_operation(
    op: dict,
    type_entries: dict[str, dict],
    primitive_names: set[str],
    status: str,
    strict: bool,
    public_symbols: set[str],
    manifest_refs: dict[str, list[str]],
    errors: list[str],
    warnings: list[str],
) -> None:
    if not isinstance(op, dict) or not isinstance(op.get("id"), str):
        errors.append("operation entry missing id")
        return
    op_id = op["id"]

    classification = op.get("classification", {})
    if not _type_guard_dict(classification, f"{op_id}.classification", errors):
        classification = {}
    else:
        if len(set(classification.keys()) - {"layer"}) > 0:
            unknown = set(classification.keys()) - {"layer"}
            errors.append(f"{op_id}: classification has unknown fields {sorted(unknown)}")
        layer = classification.get("layer")
        if layer not in VALID_CLASSIFICATION:
            errors.append(f"{op_id}: classification.layer {layer!r} is invalid")

    visibility = op.get("visibility")
    if visibility not in VALID_VISIBILITY:
        errors.append(f"{op_id}: visibility {visibility!r} is invalid")

    binding = op.get("binding", {})
    policy = binding.get("policy")
    if policy not in VALID_BINDING_POLICY:
        errors.append(f"{op_id}: binding.policy {policy!r} is invalid")

    validate_signature(op, type_entries, primitive_names, errors)
    validate_lowering(op, errors)
    validate_fallback(op, status, strict, public_symbols, errors, warnings)
    validate_binding_and_lowering(op, manifest_refs, errors)

    specs = op.get("specializations", [])
    if not _type_guard_list(specs, f"{op_id}.specializations", errors):
        return
    for i, spec in enumerate(specs):
        validate_specialization(op, spec, i, errors)
    validate_specialization_ambiguity(op, errors)


def validate_manifest_type_compat(
    fn: dict,
    op: dict,
    type_entries: dict[str, dict],
    manifest_type_aliases: dict[str, str],
    errors: list[str],
) -> None:
    op_id = op["id"]
    fn_name = f"{fn.get('module') or fn.get('kind')}::{fn.get('name')}"

    sig = op.get("signature", {})
    if not isinstance(sig, dict):
        return

    aliases = dict(manifest_type_aliases)
    if op_id.startswith("wasm."):
        aliases["v128"] = V128_ALIAS
    elif op_id.startswith("simd."):
        aliases.pop("v128", None)
    core_inputs = sig.get("inputs", [])
    manifest_params = fn.get("params", [])
    if not _type_guard_list(manifest_params, f"{fn_name}.params", errors):
        return
    if len(manifest_params) != len(core_inputs):
        errors.append(
            f"{fn_name}: param count mismatch for {op_id}: manifest {len(manifest_params)} vs core-ops {len(core_inputs)}"
        )
    else:
        for i, param_text in enumerate(manifest_params):
            if not _type_guard_str(param_text, f"{fn_name}.params[{i}]", errors):
                continue
            core_inp = core_inputs[i]
            core_type = core_inp.get("type") if isinstance(core_inp, dict) else core_inp
            try:
                manifest_type = parse_manifest_type(param_text, set(fn.get("generic_params") or []), aliases)
            except TypeParseError as e:
                errors.append(f"{fn_name}: params[{i}] parse error: {e}")
                continue
            if not compare_type_expr(manifest_type, core_type, errors):
                errors.append(
                    f"{fn_name}: params[{i}] type mismatch for {op_id}: "
                    f"manifest {type_expr_to_key(manifest_type)} vs core-ops {type_expr_to_key(core_type)}"
                )

    core_outputs = sig.get("outputs", [])
    manifest_returns = fn.get("returns")
    if manifest_returns is None:
        errors.append(f"{fn_name}: returns missing")
    else:
        if isinstance(manifest_returns, list):
            returns_texts = manifest_returns
        elif isinstance(manifest_returns, str):
            returns_texts = [manifest_returns]
        else:
            errors.append(f"{fn_name}: returns must be a string or list")
            return
        if len(returns_texts) != len(core_outputs):
            errors.append(
                f"{fn_name}: return count mismatch for {op_id}: manifest {len(returns_texts)} vs core-ops {len(core_outputs)}"
            )
        else:
            for i, ret_text in enumerate(returns_texts):
                if not _type_guard_str(ret_text, f"{fn_name}.returns[{i}]", errors):
                    continue
                core_out = core_outputs[i]
                core_type = core_out.get("type") if isinstance(core_out, dict) else core_out
                try:
                    manifest_type = parse_manifest_type(ret_text, set(fn.get("generic_params") or []), aliases)
                except TypeParseError as e:
                    errors.append(f"{fn_name}: returns[{i}] parse error: {e}")
                    continue
                if not compare_type_expr(manifest_type, core_type, errors):
                    errors.append(
                        f"{fn_name}: returns[{i}] type mismatch for {op_id}: "
                        f"manifest {type_expr_to_key(manifest_type)} vs core-ops {type_expr_to_key(core_type)}"
                    )


def compare_type_expr(a: dict, b: dict, errors: list[str]) -> bool:
    if not isinstance(a, dict) or not isinstance(b, dict):
        return False
    if a.get("kind") != b.get("kind"):
        return False
    kind = a.get("kind")
    if kind == "ref":
        if a.get("name") != b.get("name"):
            return False
        a_args = a.get("args", [])
        b_args = b.get("args", [])
        if len(a_args) != len(b_args):
            return False
        return all(compare_type_expr(x, y, errors) for x, y in zip(a_args, b_args))
    if kind == "primitive":
        return a.get("name") == b.get("name")
    if kind == "var":
        return a.get("name") == b.get("name")
    if kind == "tuple":
        a_el = a.get("elements", [])
        b_el = b.get("elements", [])
        if len(a_el) != len(b_el):
            return False
        return all(compare_type_expr(x, y, errors) for x, y in zip(a_el, b_el))
    if kind == "function":
        a_params = a.get("params", [])
        b_params = b.get("params", [])
        if len(a_params) != len(b_params):
            return False
        if not all(compare_type_expr(x, y, errors) for x, y in zip(a_params, b_params)):
            return False
        return compare_type_expr(a.get("result", {}), b.get("result", {}), errors)
    return False


def validate_validation_section(core_ops: dict, errors: list[str]) -> None:
    validation = core_ops.get("validation", {})
    if not _type_guard_dict(validation, "validation", errors):
        return
    python = validation.get("python", {})
    compiler = validation.get("compiler", {})
    if not _type_guard_dict(python, "validation.python", errors):
        pass
    else:
        python_keys = set(python.keys())
        missing = EXPECTED_PYTHON_VALIDATION_KEYS - python_keys
        extra = python_keys - EXPECTED_PYTHON_VALIDATION_KEYS
        for key in sorted(missing):
            errors.append(f"validation.python.{key} is missing")
        for key in sorted(extra):
            errors.append(f"validation.python.{key} is unknown")
        for key in python_keys & EXPECTED_PYTHON_VALIDATION_KEYS:
            if python[key] is not True:
                errors.append(f"validation.python.{key} must be true")
    if not _type_guard_dict(compiler, "validation.compiler", errors):
        pass
    else:
        compiler_keys = set(compiler.keys())
        missing = EXPECTED_COMPILER_VALIDATION_KEYS - compiler_keys
        extra = compiler_keys - EXPECTED_COMPILER_VALIDATION_KEYS
        for key in sorted(missing):
            errors.append(f"validation.compiler.{key} is missing")
        for key in sorted(extra):
            errors.append(f"validation.compiler.{key} is unknown")
        for key in compiler_keys & EXPECTED_COMPILER_VALIDATION_KEYS:
            if compiler[key] is not True:
                errors.append(f"validation.compiler.{key} must be true")


def validate_core_ops_and_manifest(
    core_ops: dict,
    manifest: dict,
    status: str,
    strict: bool,
    errors: list[str],
    warnings: list[str],
) -> None:
    validate_validation_section(core_ops, errors)

    schema_version = core_ops.get("schema_version")
    if schema_version != EXPECTED_SCHEMA_VERSION:
        errors.append(f"schema_version is {schema_version!r}, expected {EXPECTED_SCHEMA_VERSION}")

    if status not in {"scaffold", "production"}:
        errors.append(f"status {status!r} is invalid")

    types = core_ops.get("types", [])
    if not _type_guard_list(types, "types", errors):
        return

    type_entries: dict[str, dict] = {}
    type_ids = set()
    for i, t in enumerate(types):
        validate_type_definition(t, errors)
        if isinstance(t, dict):
            type_id = t.get("id")
            if isinstance(type_id, str):
                if type_id in type_ids:
                    errors.append(f"type id {type_id!r} is duplicated")
                type_ids.add(type_id)
                type_entries[type_id] = t

    primitive_names = set(VALID_PRIMITIVE_NAMES)

    operations = core_ops.get("operations", [])
    if not _type_guard_list(operations, "operations", errors):
        return

    # Build manifest type aliases from manifest [[types]] entries with type_id
    manifest_type_aliases: dict[str, str] = {}
    for t in manifest.get("types", []):
        if isinstance(t, dict):
            name = t.get("name")
            type_id = t.get("type_id")
            if isinstance(name, str) and isinstance(type_id, str):
                if name in manifest_type_aliases and manifest_type_aliases[name] != type_id:
                    errors.append(f"std/manifest.toml type alias {name!r} maps to both {manifest_type_aliases[name]!r} and {type_id!r}")
                manifest_type_aliases[name] = type_id

    # Precompute manifest public symbols and core_op_id references
    public_symbols = set()
    manifest_core_op_refs: dict[str, list[str]] = {}
    public_symbol_map: dict[str, str] = {}
    for fn in manifest.get("functions", []):
        if not isinstance(fn, dict):
            continue
        symbol = public_symbol(fn)
        if symbol:
            public_symbols.add(symbol)
        core_op_id = fn.get("core_op_id")
        if core_op_id:
            if isinstance(core_op_id, str):
                manifest_core_op_refs.setdefault(core_op_id, []).append(symbol)
            if symbol:
                existing = public_symbol_map.get(symbol)
                if existing is not None and existing != core_op_id:
                    errors.append(f"std/manifest.toml public symbol {symbol!r} maps to both {existing!r} and {core_op_id!r}")
                else:
                    public_symbol_map[symbol] = core_op_id

    op_ids = set()
    op_by_id: dict[str, dict] = {}
    for op in operations:
        if isinstance(op, dict):
            op_id = op.get("id")
            if isinstance(op_id, str):
                if op_id in op_ids:
                    errors.append(f"operation id {op_id!r} is duplicated")
                op_ids.add(op_id)
                op_by_id[op_id] = op

    for op in operations:
        validate_operation(op, type_entries, primitive_names, status, strict, public_symbols, manifest_core_op_refs, errors, warnings)

    # type_id reference checks
    for fn in manifest.get("functions", []):
        if not isinstance(fn, dict):
            continue
        type_id = fn.get("type_id")
        if isinstance(type_id, str) and type_id not in type_ids:
            errors.append(f"std/manifest.toml function {fn.get('name')!r}: type_id {type_id!r} not found in core-ops")
        core_op_id = fn.get("core_op_id")
        if isinstance(core_op_id, str) and core_op_id not in op_ids:
            errors.append(f"std/manifest.toml function {fn.get('name')!r}: core_op_id {core_op_id!r} not found in core-ops")

    for typ in manifest.get("types", []):
        if not isinstance(typ, dict):
            continue
        type_id = typ.get("type_id")
        if isinstance(type_id, str) and type_id not in type_ids:
            errors.append(f"std/manifest.toml type {typ.get('name')!r}: type_id {type_id!r} not found in core-ops")

    # required binding / optional binding consistency
    for op in operations:
        if not isinstance(op, dict):
            continue
        op_id = op.get("id")
        visibility = op.get("visibility")
        binding = op.get("binding", {})
        policy = binding.get("policy")
        if visibility == "public" and policy == "required" and not manifest_core_op_refs.get(op_id):
            errors.append(f"{op_id}: public required binding has no manifest reference")
        if visibility == "public" and policy == "forbidden":
            errors.append(f"{op_id}: public binding cannot be forbidden")
        if visibility == "internal" and policy == "required":
            errors.append(f"{op_id}: internal binding cannot be required")
        if visibility == "internal" and (policy == "forbidden" or policy == "optional") and manifest_core_op_refs.get(op_id):
            errors.append(f"{op_id}: internal operation must not be referenced from manifest")

    # signature compatibility for core_op_id functions in manifest
    for fn in manifest.get("functions", []):
        if not isinstance(fn, dict):
            continue
        core_op_id = fn.get("core_op_id")
        if not isinstance(core_op_id, str):
            continue
        op = op_by_id.get(core_op_id)
        if not op:
            continue
        validate_manifest_type_compat(fn, op, type_entries, manifest_type_aliases, errors)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate core-ops.toml schema and manifest references")
    parser.add_argument("--strict", action="store_true", help="fail on example.invalid fallback symbols")
    parser.add_argument("--production-structural-readiness", action="store_true", help="production structural gate (also implies --strict)")
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

    status = core_ops.get("status", "scaffold")
    strict = args.strict or args.production_structural_readiness

    validate_core_ops_and_manifest(core_ops, manifest, status, strict, errors, warnings)

    for w in warnings:
        print(f"警告: {w}")

    if errors:
        for e in errors:
            print(f"エラー: {e}", file=sys.stderr)
        print(f"core-ops エラー {len(errors)} 件", file=sys.stderr)
        return 1

    if args.production_structural_readiness:
        if status != "production":
            print(f"production-structural-readiness: status is {status!r}, not production", file=sys.stderr)
            return 1
        print("core-ops production structural readiness check OK")
    else:
        print("core-ops structural + manifest-semantic check OK")
    print("保留中の compiler-aware 検査: fallback symbol 解決、call cycle、Ark signature 互換、effect/lowering 整合、target handler registry、differential tests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
