"""TypedCoreHIR v2 canonical aggregate lowering to VerifiedCore v1."""
from __future__ import annotations
import copy
from typing import Any
from proof import typed_corehir_phase4_convert as phase4
from proof.typed_corehir_v2 import validate_document
from proof.typed_admission_v5_strict import validate_typed_document

CONVERTER = "arukellt-typed-corehir-converter-v5"


def _verified_type(entry: dict[str, Any]) -> dict[str, Any]:
    kind = entry["kind"]
    result = {"id": int(entry["id"]), "kind": kind, "name": str(entry["name"]), "representation": copy.deepcopy(entry["representation"])}
    if kind == "integer":
        result["bits"] = int(entry["bits"]); result["signed"] = bool(entry["signed"])
    elif kind == "tuple":
        result["elements"] = [int(value) for value in entry["elements"]]
    elif kind == "struct":
        result["fields"] = [{"name": str(field["name"]), "type_id": int(field["type_id"])} for field in entry["fields"]]
    elif kind == "enum":
        result["variants"] = [{"name": str(variant["name"]), "discriminant": int(variant["discriminant"]), "payload_type_ids": [int(value) for value in variant["payload_type_ids"]]} for variant in entry["variants"]]
    return result


def _index(function: dict[str, Any]) -> tuple[int, dict[int, dict[str, Any]]]:
    body = function["body"]
    return int(body["root_expression_id"]), {int(expression["id"]): expression for expression in body["expressions"]}


def _proof_expr(expression_id: int, expressions: dict[int, dict[str, Any]], seen: set[int]) -> dict[str, Any]:
    if expression_id in seen: raise ValueError(f"expression cycle at {expression_id}")
    seen = {*seen, expression_id}; expression = expressions[expression_id]; kind = str(expression["kind"]); type_id = int(expression["type_id"])
    result: dict[str, Any] = {"id": expression_id, "kind": kind, "type_id": type_id}
    if kind == "local": result["local_id"] = int(expression["local_id"]); return result
    if kind == "result": return result
    if kind == "constant": result["value"] = expression["value"]; return result
    if kind == "project": result["index"] = int(expression["index"])
    elif kind == "is_variant": result["variant_index"] = int(expression["variant_index"])
    elif kind == "variant_payload":
        result["variant_index"] = int(expression["variant_index"]); result["payload_index"] = int(expression["payload_index"])
    elif kind == "construct" and "variant_index" in expression: result["variant_index"] = int(expression["variant_index"])
    allowed = {"construct", "project", "is_variant", "variant_payload", "neg", "not", "add", "sub", "mul", "div", "mod", "eq", "ne", "lt", "le", "gt", "ge", "and", "or", "implies"}
    if kind not in allowed: raise ValueError(f"unsupported canonical proof expression {kind!r}")
    result["operands"] = [_proof_expr(int(child), expressions, seen) for child in expression.get("children", [])]
    return result


class _BodyLowerer:
    def __init__(self, function: dict[str, Any], expressions: dict[int, dict[str, Any]]):
        self.function = function; self.expressions = expressions
        self.locals = copy.deepcopy(function["locals"])
        self.next_local = max((int(local["id"]) for local in self.locals), default=-1) + 1
        self.next_instruction = 0; self.instructions: list[dict[str, Any]] = []
    def temp(self, type_id: int) -> int:
        local_id = self.next_local; self.next_local += 1
        self.locals.append({"id": local_id, "name": f"$p5_{local_id}", "type_id": type_id, "storage": "temporary"}); return local_id
    def emit(self, op: str, type_id: int, arguments: list[dict[str, Any]], **extra: Any) -> dict[str, Any]:
        local_id = self.temp(type_id); instruction = {"id": self.next_instruction, "op": op, "dest_local_id": local_id, "type_id": type_id, "arguments": arguments}; self.next_instruction += 1; instruction.update(extra); self.instructions.append(instruction)
        return {"kind": "local", "type_id": type_id, "local_id": local_id}
    def eval(self, expression_id: int, seen: set[int] | None = None) -> dict[str, Any] | None:
        seen = set() if seen is None else seen
        if expression_id in seen: raise ValueError(f"body expression cycle at {expression_id}")
        seen = {*seen, expression_id}; expression = self.expressions[expression_id]; kind = str(expression["kind"]); type_id = int(expression["type_id"])
        if kind == "block":
            value = None
            for child in expression.get("children", []): value = self.eval(int(child), seen)
            return value
        if kind == "local": return {"kind": "local", "type_id": type_id, "local_id": int(expression["local_id"])}
        if kind == "constant": return {"kind": "constant", "type_id": type_id, "value": expression["value"]}
        if kind in {"construct", "project", "is_variant", "variant_payload"}:
            args = [self.eval(int(child), seen) for child in expression.get("children", [])]
            if any(value is None for value in args): raise ValueError(f"{kind}: missing value operand")
            extra = {}
            if kind == "project": extra["index"] = int(expression["index"])
            if kind in {"is_variant", "variant_payload", "construct"} and "variant_index" in expression: extra["variant_index"] = int(expression["variant_index"])
            if kind == "variant_payload": extra["payload_index"] = int(expression["payload_index"])
            return self.emit(kind, type_id, args, **extra)
        if kind in {"neg", "not"}:
            args = [self.eval(int(child), seen) for child in expression.get("children", [])]
            return self.emit("unary", type_id, args, operator=kind)
        if kind in {"add", "sub", "mul", "div", "mod", "eq", "ne", "lt", "le", "gt", "ge", "and", "or", "implies"}:
            args = [self.eval(int(child), seen) for child in expression.get("children", [])]
            return self.emit("binary", type_id, args, operator=kind)
        raise ValueError(f"body expression {kind!r} is outside TypedCoreHIR v2 Phase 5")


def convert_document(value: Any) -> dict[str, Any]:
    source = validate_document(value)
    if source["schema_version"] == 1: return phase4.convert_document(source)
    functions = []
    for function_index, function in enumerate(source["functions"]):
        if not function.get("contracts"): continue
        root, expressions = _index(function); lowerer = _BodyLowerer(function, expressions); returned = lowerer.eval(root)
        return_type = int(function["signature"]["return_type_id"])
        if return_type != 0 and returned is None: raise ValueError(f"$.functions[{function_index}].body: value return required")
        contracts = []
        for contract in function["contracts"]:
            rendered = {"kind": str(contract["kind"]), "expression": _proof_expr(int(contract["expression_id"]), expressions, set())}
            if contract.get("result_name"): rendered["result_name"] = str(contract["result_name"])
            elif contract["kind"] == "ensures": rendered["result_name"] = "result"
            contracts.append(rendered)
        terminator = {"kind": "return"}
        if returned is not None: terminator["value"] = returned
        functions.append({"id": int(function["id"]), "name": str(function["name"]), "signature": copy.deepcopy(function["signature"]), "abi": copy.deepcopy(function["abi"]), "locals": lowerer.locals, "contracts": contracts, "body": {"entry_block": 0, "blocks": [{"id": 0, "parameters": [], "instructions": lowerer.instructions, "terminator": terminator}]}})
    if not functions: raise ValueError("$.functions: no contracted functions")
    result = {"schema": "arukellt-verified-core", "schema_version": 1, "generator": CONVERTER, "module": source["module"], "target_profile": copy.deepcopy(source["target_profile"]), "types": [_verified_type(entry) for entry in source["types"]], "functions": functions}
    return validate_typed_document(result)


__all__ = ["CONVERTER", "convert_document"]
