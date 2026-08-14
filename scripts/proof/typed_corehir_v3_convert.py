"""TypedCoreHIR v3 lowering to VerifiedCore v1 for proof Phases 5-7."""
from __future__ import annotations

import copy
from typing import Any

from proof import typed_corehir_v2_convert as v2
from proof.readonly_memory_semantics import MEMORY_EXPRESSION_KINDS
from proof.typed_admission_v7 import validate_typed_document
from proof.typed_corehir_v3 import validate_document

CONVERTER = "arukellt-typed-corehir-converter-v7"


def _verified_type(entry: dict[str, Any]) -> dict[str, Any]:
    result = v2._verified_type(entry)
    if entry["kind"] == "reference":
        result["pointee_type_id"] = int(entry["pointee_type_id"])
    return result


def _proof_expr(expression_id: int, expressions: dict[int, dict[str, Any]], seen: set[int]) -> dict[str, Any]:
    if expression_id in seen:
        raise ValueError(f"expression cycle at {expression_id}")
    seen = {*seen, expression_id}
    expression = expressions[expression_id]
    kind = str(expression["kind"])
    type_id = int(expression["type_id"])
    result: dict[str, Any] = {"id": expression_id, "kind": kind, "type_id": type_id}
    if kind == "local":
        result["local_id"] = int(expression["local_id"])
        return result
    if kind == "result":
        return result
    if kind == "constant":
        result["value"] = expression["value"]
        return result
    if kind in {"shl", "shr_s", "convert"}:
        raise ValueError(f"contract expression {kind!r} is outside TypedCoreHIR v3 proof expressions")
    if kind == "project":
        result["index"] = int(expression["index"])
    elif kind == "is_variant":
        result["variant_index"] = int(expression["variant_index"])
    elif kind == "variant_payload":
        result["variant_index"] = int(expression["variant_index"])
        result["payload_index"] = int(expression["payload_index"])
    elif kind == "construct" and "variant_index" in expression:
        result["variant_index"] = int(expression["variant_index"])
    elif kind == "load_field":
        result["field_index"] = int(expression["field_index"])
    allowed = {
        "construct", "project", "is_variant", "variant_payload",
        "is_null", "ref_eq", "load_field", "array_len", "array_get",
        "neg", "not", "add", "sub", "mul", "div", "mod",
        "eq", "ne", "lt", "le", "gt", "ge", "and", "or", "implies",
    }
    if kind not in allowed:
        raise ValueError(f"unsupported canonical proof expression {kind!r}")
    result["operands"] = [_proof_expr(int(child), expressions, seen) for child in expression.get("children", [])]
    return result


class _BodyLowerer(v2._BodyLowerer):
    def eval(self, expression_id: int, seen: set[int] | None = None) -> dict[str, Any] | None:
        seen = set() if seen is None else seen
        if expression_id in seen:
            raise ValueError(f"body expression cycle at {expression_id}")
        expression = self.expressions[expression_id]
        kind = str(expression["kind"])
        type_id = int(expression["type_id"])
        if kind in MEMORY_EXPRESSION_KINDS:
            next_seen = {*seen, expression_id}
            args = [self.eval(int(child), next_seen) for child in expression.get("children", [])]
            if any(value is None for value in args):
                raise ValueError(f"{kind}: missing value operand")
            extra: dict[str, Any] = {}
            if kind == "load_field":
                extra["field_index"] = int(expression["field_index"])
            return self.emit(kind, type_id, args, **extra)
        if kind in {"shl", "shr_s"}:
            next_seen = {*seen, expression_id}
            args = [self.eval(int(child), next_seen) for child in expression.get("children", [])]
            if any(value is None for value in args):
                raise ValueError(f"{kind}: missing value operand")
            return self.emit("binary", type_id, args, operator=kind)
        if kind == "convert":
            next_seen = {*seen, expression_id}
            args = [self.eval(int(child), next_seen) for child in expression.get("children", [])]
            if any(value is None for value in args):
                raise ValueError("convert: missing value operand")
            return self.emit("convert", type_id, args, conversion=str(expression["conversion"]))
        return super().eval(expression_id, seen)


def convert_document(value: Any) -> dict[str, Any]:
    source = validate_document(value)
    if source["schema_version"] in {1, 2}:
        return v2.convert_document(source)
    functions: list[dict[str, Any]] = []
    for function_index, function in enumerate(source["functions"]):
        if not function.get("contracts"):
            continue
        root, expressions = v2._index(function)
        lowerer = _BodyLowerer(function, expressions)
        returned = lowerer.eval(root)
        return_type = int(function["signature"]["return_type_id"])
        if return_type != 0 and returned is None:
            raise ValueError(f"$.functions[{function_index}].body: value return required")
        contracts: list[dict[str, Any]] = []
        for contract in function["contracts"]:
            rendered: dict[str, Any] = {
                "kind": str(contract["kind"]),
                "expression": _proof_expr(int(contract["expression_id"]), expressions, set()),
            }
            if contract.get("result_name"):
                rendered["result_name"] = str(contract["result_name"])
            elif contract["kind"] == "ensures":
                rendered["result_name"] = "result"
            contracts.append(rendered)
        terminator: dict[str, Any] = {"kind": "return"}
        if returned is not None:
            terminator["value"] = returned
        functions.append({
            "id": int(function["id"]),
            "name": str(function["name"]),
            "signature": copy.deepcopy(function["signature"]),
            "abi": copy.deepcopy(function["abi"]),
            "locals": lowerer.locals,
            "contracts": contracts,
            "body": {"entry_block": 0, "blocks": [{"id": 0, "parameters": [], "instructions": lowerer.instructions, "terminator": terminator}]},
        })
    if not functions:
        raise ValueError("$.functions: no contracted functions")
    result = {
        "schema": "arukellt-verified-core",
        "schema_version": 1,
        "generator": CONVERTER,
        "module": source["module"],
        "target_profile": copy.deepcopy(source["target_profile"]),
        "types": [_verified_type(entry) for entry in source["types"]],
        "proof_memory": copy.deepcopy(source["proof_memory"]),
        "functions": functions,
    }
    return validate_typed_document(result)


__all__ = ["CONVERTER", "convert_document"]
