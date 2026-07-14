#!/usr/bin/env python3
"""CoreOps registry schema and manifest-semantic checker.

Validates data/core-ops.toml structure and cross-references std/manifest.toml.
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
VALID_TYPE_KINDS = {"string", "vec", "simd", "mask", "wasm_v128"}
VALID_LANE_TYPES = {"i8", "i16", "i32", "i64", "f32", "f64"}
VALID_CAPABILITY_NAMES = {"wasm_raw_v128", "wasm_relaxed_simd", "portable_simd_lowering"}
VALID_TARGET_FEATURES = {"simd128"}  # extend as target features are added
VALID_WHEN_VALUES = {
    "backend": {"wasm", "native", "llvm", "interpreter"},
    "target_family": {"wasm", "native", "llvm", "interpreter"},
    "portable_simd_lowering": {"NativeSimd", "ScalarFallback", "Direct", "Default"},
    "wasm_raw_v128": {"true", "false"},
    "wasm_relaxed_simd": {"true", "false"},
}

# Public type names in std/manifest.toml -> canonical type_id in core-ops.toml.
# `v128` is context-dependent; the default below is used for std::wasm.
PUBLIC_TYPE_ALIASES = {
    "String": "string",
    "Vec": "vec",
    "HashMap": "hashmap",
    "Result": "result",
    "Option": "option",
    "v128": "wasm.v128",
}

EXPECTED_VALIDATION_KEYS = {
    "check_unreferenced_required_bindings",
    "check_public_binding_collisions",
    "check_signature_compat",
    "check_effect_lowering_consistency",
    "check_binding_field_consistency",
    "check_forbidden_bindings",
    "check_fallback_resolvable",
    "check_fallback_no_cycle",
    "check_fallback_signature_compat",
    "check_specialization_ambiguity",
}


def load_toml(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


class TypeParseError(Exception):
    pass


class TypeParser:
    """Recursive-descent parser for Arukellt type strings."""

    def __init__(self, text: str, generic_params: frozenset[str], aliases: dict[str, str]):
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
        # Primitive? Lowercase-only identifiers that are in the primitive set.
        if ident.islower() and ident in VALID_PRIMITIVE_NAMES:
            return {"kind": "primitive", "name": ident}

        # Generic type variable
        if ident in self.generic_params:
            return {"kind": "var", "name": ident}

        # Public name alias
        canonical = self.aliases.get(ident)
        if not canonical:
            canonical = ident.lower()
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


def parse_manifest_type(text: str, generic_params: frozenset[str] = frozenset(), aliases: dict[str, str] | None = None) -> dict:
    parser = TypeParser(text, generic_params, aliases or {})
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
    kind = expr.get("kind")
    if kind not in VALID_TYPEEXPR_KINDS:
        errors.append(f"{path}: unknown TypeExpr kind {kind!r}")
        return
    if kind == "ref":
        name = expr.get("name")
        if not isinstance(name, str):
            errors.append(f"{path}: ref TypeExpr needs a string name")
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
        if not isinstance(args, list):
            errors.append(f"{path}: ref args must be a list")
        elif len(args) != expected_arity:
            errors.append(
                f"{path}: ref {name!r} expects {expected_arity} generic args, got {len(args)}"
            )
        else:
            for i, arg in enumerate(args):
                validate_type_expr(arg, generic_params, type_entries, primitive_names, f"{path}.args[{i}]", errors)
    elif kind == "primitive":
        name = expr.get("name")
        if name not in primitive_names:
            errors.append(f"{path}: primitive {name!r} is not in canonical set {sorted(primitive_names)}")
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
                validate_type_expr(elem, generic_params, type_entries, primitive_names, f"{path}.elements[{i}]", errors)
    elif kind == "function":
        params = expr.get("params", [])
        if not isinstance(params, list):
            errors.append(f"{path}: function params must be a list")
        else:
            for i, p in enumerate(params):
                validate_type_expr(p, generic_params, type_entries, primitive_names, f"{path}.params[{i}]", errors)
        result = expr.get("result")
        if result is None:
            errors.append(f"{path}: function TypeExpr needs a result")
        else:
            validate_type_expr(result, generic_params, type_entries, primitive_names, f"{path}.result", errors)


def validate_type_definition(t: dict, errors: list[str]) -> None:
    if not isinstance(t.get("id"), str):
        errors.append("types entry missing id")
        return
    type_id = t["id"]
    kind = t.get("kind")
    if kind not in VALID_TYPE_KINDS:
        errors.append(f"type {type_id}: kind {kind!r} is invalid")
        return
    generics = t.get("generic_params", [])
    if not isinstance(generics, list):
        errors.append(f"type {type_id}: generic_params must be a list")
    else:
        seen = set()
        for g in generics:
            if not isinstance(g, str):
                errors.append(f"type {type_id}: generic_params must be strings")
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
    generic_params = set(sig.get("generic_params", []))
    if not isinstance(generic_params, set):
        errors.append(f"{op['id']}: generic_params must be a list")
    else:
        seen = set()
        for g in sig.get("generic_params", []):
            if g in seen:
                errors.append(f"{op['id']}: duplicate generic_param {g!r}")
            seen.add(g)
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
        validate_type_expr(inp.get("type"), generic_params, type_entries, primitive_names, f"{op['id']}.inputs[{i}].type", errors)

    outputs = sig.get("outputs", [])
    if not isinstance(outputs, list):
        errors.append(f"{op['id']}: signature.outputs must be a list")
    else:
        for i, out in enumerate(outputs):
            validate_type_expr(out.get("type") if isinstance(out, dict) else out, generic_params, type_entries, primitive_names, f"{op['id']}.outputs[{i}]", errors)

    receiver_index = sig.get("receiver_index")
    if receiver_index is not None:
        if not isinstance(receiver_index, int) or receiver_index < 0 or receiver_index >= len(inputs):
            errors.append(f"{op['id']}: receiver_index {receiver_index} is out of range for {len(inputs)} inputs")

    for i, c in enumerate(sig.get("constraints", [])):
        if not isinstance(c, dict):
            errors.append(f"{op['id']}: signature.constraints[{i}] must be a table")
            continue
        kind = c.get("kind")
        if kind not in VALID_CONSTRAINT_KINDS:
            errors.append(f"{op['id']}: signature.constraints[{i}] has unknown kind {kind!r}")
            continue
        if kind == "trait":
            if not isinstance(c.get("trait"), str):
                errors.append(f"{op['id']}: signature.constraints[{i}].trait must be a string")
            params = c.get("params", [])
            if not isinstance(params, list):
                errors.append(f"{op['id']}: signature.constraints[{i}].params must be a list")
            else:
                for j, p in enumerate(params):
                    validate_type_expr(p, generic_params, type_entries, primitive_names, f"{op['id']}.constraints[{i}].params[{j}]", errors)
        elif kind == "type_eq":
            for side in ("lhs", "rhs"):
                if c.get(side) is None:
                    errors.append(f"{op['id']}: signature.constraints[{i}].type_eq needs {side}")
                else:
                    validate_type_expr(c.get(side), generic_params, type_entries, primitive_names, f"{op['id']}.constraints[{i}].{side}", errors)
        elif kind == "capability":
            if not isinstance(c.get("capability"), str):
                errors.append(f"{op['id']}: signature.constraints[{i}].capability must be a string")


def validate_runtime_payload(op_id: str, runtime: dict, errors: list[str]) -> None:
    kind = runtime.get("kind")
    if kind not in VALID_RUNTIME_KIND:
        errors.append(f"{op_id}: runtime_call runtime.kind {kind!r} is invalid")
        return
    if kind == "internal":
        if not isinstance(runtime.get("symbol"), str):
            errors.append(f"{op_id}: runtime internal lowering needs symbol")
        if not isinstance(runtime.get("abi_version"), str):
            errors.append(f"{op_id}: runtime internal lowering needs abi_version")
    elif kind == "wit":
        for key in ("package", "interface", "function", "version"):
            if not isinstance(runtime.get(key), str):
                errors.append(f"{op_id}: runtime wit lowering needs {key}")
    elif kind == "native":
        for key in ("backend", "symbol", "abi_version"):
            if not isinstance(runtime.get(key), str):
                errors.append(f"{op_id}: runtime native lowering needs {key}")


def validate_target_payload(op_id: str, target: dict, errors: list[str]) -> None:
    if not isinstance(target.get("target_family"), str):
        errors.append(f"{op_id}: target_intrinsic needs target_family")
    if not isinstance(target.get("target_id"), str):
        errors.append(f"{op_id}: target_intrinsic needs target_id (backend handler key)")
    caps = target.get("required_capabilities", [])
    if not isinstance(caps, list):
        errors.append(f"{op_id}: target_intrinsic required_capabilities must be a list")
    else:
        for i, c in enumerate(caps):
            if not isinstance(c, str):
                errors.append(f"{op_id}: required_capabilities[{i}] must be a string")
            elif c not in VALID_CAPABILITY_NAMES:
                errors.append(f"{op_id}: required_capabilities[{i}] {c!r} is not a known capability")
    feats = target.get("required_target_features", [])
    if not isinstance(feats, list):
        errors.append(f"{op_id}: target_intrinsic required_target_features must be a list")
    else:
        for i, f in enumerate(feats):
            if not isinstance(f, str):
                errors.append(f"{op_id}: required_target_features[{i}] must be a string")
            elif f not in VALID_TARGET_FEATURES:
                errors.append(f"{op_id}: required_target_features[{i}] {f!r} is not a known target feature")


def validate_lowering(op: dict, errors: list[str]) -> None:
    lowering = op.get("lowering", {})
    if not isinstance(lowering, dict):
        errors.append(f"{op['id']}: lowering must be a table")
        return
    kind = lowering.get("kind")
    if kind not in VALID_LOWERING_KIND:
        errors.append(f"{op['id']}: lowering.kind {kind!r} is invalid")
        return

    # Only the sub-table matching kind is allowed.
    allowed_sub = {"normal_call": set(), "mir_op": {"mir"}, "runtime_call": {"runtime"}, "target_intrinsic": {"target"}}[kind]
    for key in lowering:
        if key == "kind":
            continue
        if key not in allowed_sub:
            errors.append(f"{op['id']}: lowering.kind = {kind} must not contain lowering.{key}")

    if kind == "mir_op":
        mir = lowering.get("mir", {})
        if not isinstance(mir, dict):
            errors.append(f"{op['id']}: mir_op lowering needs lowering.mir table")
        elif not mir.get("opcode") and not mir.get("operation"):
            errors.append(f"{op['id']}: mir_op lowering needs lowering.mir.opcode or operation")
    elif kind == "runtime_call":
        runtime = lowering.get("runtime", {})
        if not isinstance(runtime, dict):
            errors.append(f"{op['id']}: runtime_call lowering needs lowering.runtime table")
        else:
            validate_runtime_payload(op["id"], runtime, errors)
    elif kind == "target_intrinsic":
        target = lowering.get("target", {})
        if not isinstance(target, dict):
            errors.append(f"{op['id']}: target_intrinsic lowering needs lowering.target table")
        else:
            validate_target_payload(op["id"], target, errors)

    # normal_call requires a fallback.
    if kind == "normal_call":
        fallback = op.get("fallback", {})
        if not isinstance(fallback, dict):
            errors.append(f"{op['id']}: normal_call requires a fallback table")
        elif not fallback.get("required"):
            errors.append(f"{op['id']}: normal_call requires fallback.required = true")


def validate_fallback(op: dict, status: str, strict: bool, errors: list[str], warnings: list[str]) -> None:
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
        elif symbol.startswith("std::") or symbol.startswith("prelude::"):
            warnings.append(f"{op['id']}: fallback implementation_symbol looks like a public path: {symbol}")


def validate_when(op_id: str, when: dict, index: int, errors: list[str]) -> None:
    if not isinstance(when, dict):
        errors.append(f"{op_id}.specializations[{index}]: when must be a table")
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


def validate_specialization(op: dict, spec: dict, index: int, errors: list[str]) -> None:
    op_id = op["id"]
    prefix = f"{op_id}.specializations[{index}]"
    if not isinstance(spec.get("priority"), int):
        errors.append(f"{prefix}: priority must be an integer")
    when = spec.get("when", {})
    validate_when(op_id, when, index, errors)
    lowering = spec.get("lowering", {})
    if not isinstance(lowering, dict):
        errors.append(f"{prefix}: lowering must be a table")
    else:
        fake_op = {"id": prefix, "lowering": lowering}
        validate_lowering(fake_op, errors)


def specialization_matches(when: dict, config: dict[str, str]) -> bool:
    for key, value in when.items():
        if config.get(key) != value:
            return False
    return True


def validate_specialization_ambiguity(op: dict, errors: list[str]) -> None:
    specs = op.get("specializations", [])
    if not specs:
        return
    # Build a small domain from known values and values actually used.
    domains = {}
    for key in VALID_WHEN_KEYS:
        domains[key] = set(VALID_WHEN_VALUES.get(key, set()))
    for spec in specs:
        when = spec.get("when", {})
        for key, value in when.items():
            domains.setdefault(key, set()).add(value)
    # For each concrete configuration, determine the highest priority matching specs.
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


def validate_binding_and_lowering(op: dict, errors: list[str], warnings: list[str]) -> None:
    visibility = op.get("visibility")
    classification = op.get("classification", {})
    binding = op.get("binding", {})
    policy = binding.get("policy")

    if visibility == "public" and policy == "forbidden":
        errors.append(f"{op['id']}: public operation cannot have binding.policy = forbidden")
    if visibility == "internal" and policy == "required":
        errors.append(f"{op['id']}: internal operation cannot have binding.policy = required")
    if visibility == "internal" and policy == "optional":
        warnings.append(f"{op['id']}: internal operation with binding.policy = optional should be documented")
    if policy == "optional":
        if not isinstance(binding.get("reason"), str):
            errors.append(f"{op['id']}: binding.optional must include reason")
        if not isinstance(binding.get("tracking_issue"), str):
            warnings.append(f"{op['id']}: binding.optional should include a tracking_issue or RFC reference")

    if op.get("inline", {}).get("policy") not in VALID_INLINE_POLICY:
        errors.append(f"{op['id']}: inline.policy is invalid")

    for field in VALID_SEMANTICS_FIELDS:
        if field not in op.get("semantics", {}):
            errors.append(f"{op['id']}: semantics.{field} is missing")
    semantics = op.get("semantics", {})
    if semantics.get("nan") not in VALID_NAN_VALUES:
        errors.append(f"{op['id']}: semantics.nan {semantics.get('nan')!r} is invalid")
    if semantics.get("equivalence") not in VALID_EQUIVALENCE_VALUES:
        errors.append(f"{op['id']}: semantics.equivalence {semantics.get('equivalence')!r} is invalid")

    effect = op.get("effect", {})
    for field in VALID_EFFECT_FIELDS:
        if field not in effect:
            errors.append(f"{op['id']}: effect.{field} is missing")
    if effect.get("memory") not in VALID_MEMORY_VALUES:
        errors.append(f"{op['id']}: effect.memory {effect.get('memory')!r} is invalid")
    if effect.get("nondeterminism") not in VALID_NONDETERMINISM_VALUES:
        errors.append(f"{op['id']}: effect.nondeterminism {effect.get('nondeterminism')!r} is invalid")

    # lowering variant already checked separately; here we just ensure consistency with effect.
    lowering = op.get("lowering", {})
    if lowering.get("kind") == "normal_call" and effect.get("noreturn"):
        errors.append(f"{op['id']}: normal_call with noreturn = true is inconsistent")
    if lowering.get("kind") == "runtime_call" and effect.get("noreturn"):
        # Panic is a runtime call and noreturn. That is allowed. Nothing to check.
        pass
    if lowering.get("kind") == "target_intrinsic" and effect.get("noreturn"):
        errors.append(f"{op['id']}: target_intrinsic with noreturn = true is inconsistent (use runtime_call)")


def validate_operation(
    op: dict,
    type_entries: dict[str, dict],
    primitive_names: set[str],
    status: str,
    strict: bool,
    errors: list[str],
    warnings: list[str],
) -> None:
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

    validate_signature(op, type_entries, primitive_names, errors)
    validate_lowering(op, errors)
    validate_fallback(op, status, strict, errors, warnings)
    validate_binding_and_lowering(op, errors, warnings)

    for i, spec in enumerate(op.get("specializations", [])):
        validate_specialization(op, spec, i, errors)
    validate_specialization_ambiguity(op, errors)


def validate_manifest_type_compat(
    fn: dict,
    op: dict,
    type_entries: dict[str, dict],
    errors: list[str],
) -> None:
    op_id = op["id"]
    fn_name = f"{fn.get('module') or fn.get('kind')}::{fn.get('name')}"

    # Build aliases for this function. v128 in std::wasm maps to wasm.v128.
    aliases = dict(PUBLIC_TYPE_ALIASES)
    if op_id.startswith("wasm."):
        aliases["v128"] = "wasm.v128"
    elif op_id.startswith("simd."):
        # v128 in std::simd should not appear in a simd core-op signature; map to none.
        aliases.pop("v128", None)

    sig = op.get("signature", {})
    core_inputs = sig.get("inputs", [])
    manifest_params = fn.get("params", [])
    if not isinstance(manifest_params, list):
        errors.append(f"{fn_name}: params must be a list")
        return
    if len(manifest_params) != len(core_inputs):
        errors.append(
            f"{fn_name}: param count mismatch for {op_id}: manifest {len(manifest_params)} vs core-ops {len(core_inputs)}"
        )
    else:
        for i, (param_text, core_inp) in enumerate(zip(manifest_params, core_inputs)):
            if not isinstance(param_text, str):
                errors.append(f"{fn_name}: params[{i}] must be a string")
                continue
            try:
                manifest_type = parse_manifest_type(param_text, frozenset(), aliases)
            except TypeParseError as e:
                errors.append(f"{fn_name}: params[{i}] parse error: {e}")
                continue
            core_type = core_inp.get("type") if isinstance(core_inp, dict) else core_inp
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
            for i, (ret_text, core_out) in enumerate(zip(returns_texts, core_outputs)):
                if not isinstance(ret_text, str):
                    errors.append(f"{fn_name}: returns[{i}] must be a string")
                    continue
                try:
                    manifest_type = parse_manifest_type(ret_text, frozenset(), aliases)
                except TypeParseError as e:
                    errors.append(f"{fn_name}: returns[{i}] parse error: {e}")
                    continue
                core_type = core_out.get("type") if isinstance(core_out, dict) else core_out
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
    if not isinstance(validation, dict):
        errors.append("validation must be a table")
        return
    keys = set(validation.keys())
    missing = EXPECTED_VALIDATION_KEYS - keys
    extra = keys - EXPECTED_VALIDATION_KEYS
    for key in sorted(missing):
        errors.append(f"validation.{key} is missing")
    for key in sorted(extra):
        errors.append(f"validation.{key} is unknown")
    for key in keys & EXPECTED_VALIDATION_KEYS:
        if validation[key] is not True:
            errors.append(f"validation.{key} must be true (invariant)")


def validate_core_ops_and_manifest(core_ops: dict, manifest: dict, status: str, strict: bool, errors: list[str], warnings: list[str]) -> None:
    validate_validation_section(core_ops, errors)

    schema_version = core_ops.get("schema_version")
    if schema_version != EXPECTED_SCHEMA_VERSION:
        errors.append(f"schema_version is {schema_version!r}, expected {EXPECTED_SCHEMA_VERSION}")

    if status not in {"scaffold", "production"}:
        errors.append(f"status {status!r} is invalid")

    types = core_ops.get("types", [])
    if not isinstance(types, list):
        errors.append("types must be a list")
        return

    type_entries: dict[str, dict] = {}
    type_ids = set()
    for t in types:
        if not isinstance(t, dict):
            errors.append("types entry must be a table")
            continue
        type_id = t.get("id")
        if not isinstance(type_id, str):
            errors.append("types entry missing id")
            continue
        if type_id in type_ids:
            errors.append(f"type id {type_id!r} is duplicated")
        type_ids.add(type_id)
        type_entries[type_id] = t
        validate_type_definition(t, errors)

    primitive_names = set(VALID_PRIMITIVE_NAMES)

    operations = core_ops.get("operations", [])
    if not isinstance(operations, list):
        errors.append("operations must be a list")
        return

    op_ids = set()
    for op in operations:
        if not isinstance(op, dict):
            errors.append("operations entry must be a table")
            continue
        op_id = op.get("id")
        if not isinstance(op_id, str):
            errors.append("operations entry missing id")
            continue
        if op_id in op_ids:
            errors.append(f"operation id {op_id!r} is duplicated")
        op_ids.add(op_id)
        validate_operation(op, type_entries, primitive_names, status, strict, errors, warnings)

    # Manifest reference checks.
    manifest_core_op_refs: dict[str, list[str]] = {}
    public_symbol_map: dict[str, str] = {}
    for fn in manifest.get("functions", []):
        if not isinstance(fn, dict):
            continue
        core_op_id = fn.get("core_op_id")
        if core_op_id:
            if core_op_id not in op_ids:
                errors.append(f"std/manifest.toml function {fn.get('name')!r}: core_op_id {core_op_id!r} not found in core-ops")
            manifest_core_op_refs.setdefault(core_op_id, []).append(public_symbol(fn))
        type_id = fn.get("type_id")
        if type_id and type_id not in type_ids:
            errors.append(f"std/manifest.toml function {fn.get('name')!r}: type_id {type_id!r} not found in core-ops")

        symbol = public_symbol(fn)
        if symbol and core_op_id:
            existing = public_symbol_map.get(symbol)
            if existing is not None and existing != core_op_id:
                errors.append(
                    f"std/manifest.toml public symbol {symbol!r} maps to both {existing!r} and {core_op_id!r}"
                )
            else:
                public_symbol_map[symbol] = core_op_id

    for typ in manifest.get("types", []):
        if not isinstance(typ, dict):
            continue
        type_id = typ.get("type_id")
        if type_id and type_id not in type_ids:
            errors.append(f"std/manifest.toml type {typ.get('name')!r}: type_id {type_id!r} not found in core-ops")

    # required binding / optional binding consistency
    for op in operations:
        if not isinstance(op, dict):
            continue
        op_id = op.get("id")
        visibility = op.get("visibility")
        binding = op.get("binding", {})
        policy = binding.get("policy")
        if visibility == "public" and policy == "required":
            refs = manifest_core_op_refs.get(op_id, [])
            if not refs:
                errors.append(f"{op_id}: public required binding has no manifest reference")
        if visibility == "public" and policy == "forbidden":
            errors.append(f"{op_id}: public binding cannot be forbidden")
        if visibility == "internal" and policy == "required":
            errors.append(f"{op_id}: internal binding cannot be required")

    # signature compatibility for core_op_id functions in manifest
    op_by_id = {op["id"]: op for op in operations if isinstance(op, dict) and isinstance(op.get("id"), str)}
    for fn in manifest.get("functions", []):
        if not isinstance(fn, dict):
            continue
        core_op_id = fn.get("core_op_id")
        if not core_op_id:
            continue
        op = op_by_id.get(core_op_id)
        if not op:
            continue
        validate_manifest_type_compat(fn, op, type_entries, errors)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate core-ops.toml schema and manifest references")
    parser.add_argument("--strict", action="store_true", help="fail on example.invalid fallback symbols")
    parser.add_argument("--production-readiness", action="store_true", help="production cutover gate (also implies --strict)")
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
    strict = args.strict or args.production_readiness

    validate_core_ops_and_manifest(core_ops, manifest, status, strict, errors, warnings)

    for w in warnings:
        print(f"警告: {w}")

    if errors:
        for e in errors:
            print(f"エラー: {e}", file=sys.stderr)
        print(f"core-ops エラー {len(errors)} 件", file=sys.stderr)
        return 1

    if args.production_readiness:
        if status != "production":
            print(f"production readiness: status is {status!r}, not production", file=sys.stderr)
            return 1
        print("core-ops production readiness structural check OK")
    else:
        print("core-ops structural + manifest-semantic check OK")
    print("保留中の compiler-aware 検査: fallback symbol 解決、call cycle、Ark signature 互換、effect/lowering 整合、target handler registry、differential tests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
