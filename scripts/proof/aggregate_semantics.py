"""Phase 5 pure aggregate semantic admission."""
from __future__ import annotations
from typing import Any
from proof import verified_core_typed_impl as legacy
from proof.aggregate_types import is_aggregate_type

AGGREGATE_EXPRESSION_KINDS = {"construct", "project", "is_variant", "variant_payload"}
AGGREGATE_INSTRUCTION_OPS = {"construct", "project", "is_variant", "variant_payload"}


def _types(document: dict[str, Any]) -> dict[int, dict[str, Any]]:
    return {int(entry["id"]): entry for entry in document["types"]}


def _bool_type(types: dict[int, dict[str, Any]]) -> int:
    matches = [type_id for type_id, entry in types.items() if entry["kind"] == "bool"]
    if len(matches) != 1:
        raise ValueError("$.types: exactly one bool type is required")
    return matches[0]


def _member_types(entry: dict[str, Any], variant_index: int | None = None) -> list[int]:
    if entry["kind"] == "tuple":
        return [int(value) for value in entry["elements"]]
    if entry["kind"] == "struct":
        return [int(field["type_id"]) for field in entry["fields"]]
    if entry["kind"] == "enum":
        if type(variant_index) is not int or variant_index < 0 or variant_index >= len(entry["variants"]):
            raise ValueError("enum variant index is out of range")
        return [int(value) for value in entry["variants"][variant_index]["payload_type_ids"]]
    raise ValueError("aggregate type required")


def _value_type(value: dict[str, Any], local_types: dict[int, int], types: dict[int, dict[str, Any]], path: str) -> int:
    type_id = int(value["type_id"])
    if type_id not in types:
        raise ValueError(f"{path}.type_id: unknown type")
    if value["kind"] == "local":
        if local_types.get(int(value["local_id"])) != type_id:
            raise ValueError(f"{path}.local_id: local type mismatch")
    elif value["kind"] == "constant":
        if is_aggregate_type(types[type_id]):
            raise ValueError(f"{path}: aggregate constants require construct")
        legacy._check_constant(value["value"], type_id, {tid: str(t["kind"]) for tid, t in types.items()}, f"{path}.value")
    else:
        raise ValueError(f"{path}.kind: unsupported value kind")
    return type_id


def _expression_type(
    expression: dict[str, Any],
    *,
    path: str,
    types: dict[int, dict[str, Any]],
    local_types: dict[int, int],
    return_type: int,
    allow_result: bool,
    seen_ids: set[int],
) -> int:
    if not isinstance(expression, dict):
        raise ValueError(f"{path}: expected expression")
    expression_id = int(expression["id"])
    if expression_id in seen_ids:
        raise ValueError(f"{path}.id: duplicate expression id {expression_id}")
    seen_ids.add(expression_id)
    kind = str(expression["kind"])
    type_id = int(expression["type_id"])
    if type_id not in types:
        raise ValueError(f"{path}.type_id: unknown TypeId")
    operands = expression.get("operands", [])
    if not isinstance(operands, list):
        raise ValueError(f"{path}.operands: expected array")

    if kind == "local":
        if operands or local_types.get(int(expression["local_id"])) != type_id:
            raise ValueError(f"{path}: local type mismatch")
        return type_id
    if kind == "result":
        if not allow_result or operands or return_type == 0 or type_id != return_type:
            raise ValueError(f"{path}: invalid result expression")
        return type_id
    if kind == "constant":
        if operands:
            raise ValueError(f"{path}.operands: constant must be leaf")
        if is_aggregate_type(types[type_id]):
            raise ValueError(f"{path}: aggregate constants require construct")
        legacy._check_constant(expression["value"], type_id, {tid: str(t["kind"]) for tid, t in types.items()}, f"{path}.value")
        return type_id

    operand_types = [
        _expression_type(
            operand,
            path=f"{path}.operands[{index}]",
            types=types,
            local_types=local_types,
            return_type=return_type,
            allow_result=allow_result,
            seen_ids=seen_ids,
        )
        for index, operand in enumerate(operands)
    ]

    if kind == "construct":
        entry = types[type_id]
        if not is_aggregate_type(entry):
            raise ValueError(f"{path}.type_id: construct destination must be aggregate")
        variant_index = expression.get("variant_index")
        if entry["kind"] == "enum":
            expected = _member_types(entry, variant_index)
        else:
            if variant_index is not None:
                raise ValueError(f"{path}.variant_index: only enum constructors use variant index")
            expected = _member_types(entry)
        if operand_types != expected:
            raise ValueError(f"{path}.operands: constructor type mismatch")
        return type_id

    if kind == "project":
        if len(operand_types) != 1:
            raise ValueError(f"{path}.operands: projection requires one source")
        source = types[operand_types[0]]
        if source["kind"] not in {"tuple", "struct"}:
            raise ValueError(f"{path}: projection requires tuple/struct")
        index = expression.get("index")
        members = _member_types(source)
        if type(index) is not int or index < 0 or index >= len(members) or type_id != members[index]:
            raise ValueError(f"{path}.index: projection type mismatch")
        return type_id

    if kind == "is_variant":
        if len(operand_types) != 1 or types[operand_types[0]]["kind"] != "enum" or type_id != _bool_type(types):
            raise ValueError(f"{path}: variant test type mismatch")
        _member_types(types[operand_types[0]], expression.get("variant_index"))
        return type_id

    if kind == "variant_payload":
        if len(operand_types) != 1 or types[operand_types[0]]["kind"] != "enum":
            raise ValueError(f"{path}: variant payload requires enum")
        members = _member_types(types[operand_types[0]], expression.get("variant_index"))
        payload_index = expression.get("payload_index")
        if type(payload_index) is not int or payload_index < 0 or payload_index >= len(members) or type_id != members[payload_index]:
            raise ValueError(f"{path}.payload_index: variant payload type mismatch")
        return type_id

    if kind in {"neg", "not"}:
        if len(operand_types) != 1:
            raise ValueError(f"{path}.operands: unary arity")
        source_type = operand_types[0]
        if kind == "neg":
            if types[source_type]["kind"] != "integer" or type_id != source_type:
                raise ValueError(f"{path}: neg type mismatch")
        elif types[source_type]["kind"] != "bool" or types[type_id]["kind"] != "bool":
            raise ValueError(f"{path}: not type mismatch")
        return type_id

    binary = {"add", "sub", "mul", "div", "mod", "eq", "ne", "lt", "le", "gt", "ge", "and", "or", "implies"}
    if kind not in binary or len(operand_types) != 2:
        raise ValueError(f"{path}.kind: unsupported expression {kind!r}")
    left, right = operand_types
    if left != right:
        raise ValueError(f"{path}.operands: TypeId mismatch")
    source_kind = types[left]["kind"]
    if kind in {"add", "sub", "mul", "div", "mod"}:
        if source_kind != "integer" or type_id != left:
            raise ValueError(f"{path}: arithmetic type mismatch")
    elif kind in {"lt", "le", "gt", "ge"}:
        if source_kind != "integer" or type_id != _bool_type(types):
            raise ValueError(f"{path}: comparison type mismatch")
    elif kind in {"eq", "ne"}:
        if source_kind not in {"integer", "bool"} and not is_aggregate_type(types[left]):
            raise ValueError(f"{path}: equality type mismatch")
        if type_id != _bool_type(types):
            raise ValueError(f"{path}: equality result must be bool")
    else:
        if source_kind != "bool" or type_id != _bool_type(types):
            raise ValueError(f"{path}: logical type mismatch")
    return type_id


def has_aggregates(document: dict[str, Any]) -> bool:
    return any(is_aggregate_type(entry) for entry in document.get("types", []))


__all__ = [
    "AGGREGATE_EXPRESSION_KINDS",
    "AGGREGATE_INSTRUCTION_OPS",
    "_bool_type",
    "_expression_type",
    "_member_types",
    "_types",
    "_value_type",
    "has_aggregates",
]
