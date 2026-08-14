"""Phase 5 deterministic SMT datatype VC rendering."""
from __future__ import annotations
from typing import Any
from proof import smtlib_v1 as legacy
from proof.aggregate_semantics import AGGREGATE_EXPRESSION_KINDS, has_aggregates
from proof.aggregate_types import aggregate_order, is_aggregate_type, smt_constructor, smt_selector, smt_sort
from proof.proof_phase4_vc import UnsupportedVerifiedCore, _function_vcs as phase4_function_vcs, generate_smtlib as generate_phase4_smtlib
from proof.verified_core_typed import validate_typed_document

ENCODING = "arukellt-smt-datatype-v1"


def _sort(type_id: int, types: dict[int, dict[str, Any]]) -> str:
    entry = types[type_id]
    if is_aggregate_type(entry): return smt_sort(type_id)
    return legacy._sort(str(entry["kind"]), f"$.types[id={type_id}]")


def _datatype_declarations(document: dict[str, Any], types: dict[int, dict[str, Any]]) -> list[str]:
    lines: list[str] = []
    for type_id in aggregate_order(document):
        entry = types[type_id]
        if entry["kind"] in {"tuple", "struct"}:
            members = [int(value) for value in entry["elements"]] if entry["kind"] == "tuple" else [int(field["type_id"]) for field in entry["fields"]]
            fields = " ".join(f"({smt_selector(type_id, index)} {_sort(member, types)})" for index, member in enumerate(members))
            ctor = f"({smt_constructor(type_id)} {fields})" if fields else f"({smt_constructor(type_id)})"
            lines.append(f"(declare-datatypes () (({smt_sort(type_id)} {ctor})))")
        else:
            constructors = []
            for variant_index, variant in enumerate(entry["variants"]):
                fields = " ".join(f"({smt_selector(type_id, variant_index, payload_index)} {_sort(int(member), types)})" for payload_index, member in enumerate(variant["payload_type_ids"]))
                constructors.append(f"({smt_constructor(type_id, variant_index)} {fields})" if fields else f"({smt_constructor(type_id, variant_index)})")
            lines.append(f"(declare-datatypes () (({smt_sort(type_id)} {' '.join(constructors)})))")
    return lines


def _expression(expression: dict[str, Any], env: dict[int, str], result: str, types: dict[int, dict[str, Any]], path: str) -> str:
    kind = expression["kind"]
    if kind == "local": return env[int(expression["local_id"])]
    if kind == "result": return result
    if kind == "constant":
        value = expression["value"]
        if type(value) is bool: return "true" if value else "false"
        if type(value) is int: return str(value)
        raise UnsupportedVerifiedCore(f"{path}: unsupported constant")
    operands = [_expression(value, env, result, types, f"{path}.operands[{index}]") for index, value in enumerate(expression.get("operands", []))]
    type_id = int(expression["type_id"])
    if kind == "construct":
        variant = int(expression.get("variant_index", 0))
        return f"({smt_constructor(type_id, variant)}{' ' if operands else ''}{' '.join(operands)})"
    if kind == "project":
        source_type = int(expression["operands"][0]["type_id"])
        return f"({smt_selector(source_type, int(expression['index']))} {operands[0]})"
    if kind == "is_variant":
        source_type = int(expression["operands"][0]["type_id"]); variant = int(expression["variant_index"])
        return f"((_ is {smt_constructor(source_type, variant)}) {operands[0]})"
    if kind == "variant_payload":
        source_type = int(expression["operands"][0]["type_id"]); variant = int(expression["variant_index"]); payload = int(expression["payload_index"])
        return f"({smt_selector(source_type, variant, payload)} {operands[0]})"
    if kind == "neg": return f"(- {operands[0]})"
    if kind == "not": return f"(not {operands[0]})"
    operators = {"add": "+", "sub": "-", "mul": "*", "div": "div", "mod": "mod", "eq": "=", "ne": "distinct", "lt": "<", "le": "<=", "gt": ">", "ge": ">=", "and": "and", "or": "or", "implies": "=>"}
    if kind not in operators: raise UnsupportedVerifiedCore(f"{path}.kind: unsupported expression {kind!r}")
    return f"({operators[kind]} {' '.join(operands)})"


def _instruction_term(instruction: dict[str, Any], env: dict[int, str], sorts: dict[int, str], types: dict[int, dict[str, Any]], path: str) -> tuple[str, list[str]]:
    op = instruction["op"]
    if op not in {"construct", "project", "is_variant", "variant_payload"}:
        return legacy._instruction_term(instruction, env, sorts, path)
    args = [legacy._typed_value(value, env, sorts, f"{path}.arguments[{index}]") for index, value in enumerate(instruction.get("arguments", []))]
    if op == "construct":
        type_id = int(instruction["type_id"]); variant = int(instruction.get("variant_index", 0))
        return f"({smt_constructor(type_id, variant)}{' ' if args else ''}{' '.join(args)})", []
    source_type = int(instruction["arguments"][0]["type_id"])
    if op == "project": return f"({smt_selector(source_type, int(instruction['index']))} {args[0]})", []
    if op == "is_variant": return f"((_ is {smt_constructor(source_type, int(instruction['variant_index']))}) {args[0]})", []
    return f"({smt_selector(source_type, int(instruction['variant_index']), int(instruction['payload_index']))} {args[0]})", []


def _uses_aggregate(function: dict[str, Any], aggregate_ids: set[int]) -> bool:
    if int(function["signature"]["return_type_id"]) in aggregate_ids: return True
    if any(int(parameter["type_id"]) in aggregate_ids for parameter in function["signature"]["parameters"]): return True
    if any(int(local["type_id"]) in aggregate_ids for local in function["locals"]): return True
    return any(instruction["op"] in {"construct", "project", "is_variant", "variant_payload"} for block in function["body"]["blocks"] for instruction in block["instructions"])


def _aggregate_function_vcs(function: dict[str, Any], sorts: dict[int, str], types: dict[int, dict[str, Any]], function_index: int) -> list[str]:
    path = f"$.functions[{function_index}]"; blocks = function["body"]["blocks"]
    if len(blocks) != 1 or int(function["body"]["entry_block"]) != int(blocks[0]["id"]): raise UnsupportedVerifiedCore(f"{path}.body: phase 5 aggregate functions currently require one straight-line block")
    block = blocks[0]
    if "loop" in block or any(instruction["op"] == "call" for instruction in block["instructions"]): raise UnsupportedVerifiedCore(f"{path}.body: aggregate loops/calls are outside phase 5")
    prefix = legacy._symbol(f"f{function['id']}_{function['name']}"); lines: list[str] = []; env: dict[int, str] = {}
    parameter_names = {parameter["name"] for parameter in function["signature"]["parameters"]}
    for local in function["locals"]:
        if local["storage"] == "parameter" and local["name"] in parameter_names:
            symbol = legacy._symbol(f"{prefix}_arg_{local['id']}_{local['name']}"); env[int(local["id"])] = symbol; lines.append(f"(declare-const {symbol} {sorts[int(local['type_id'])]})")
    placeholder = legacy._symbol(f"{prefix}_result")
    requires = [_expression(contract["expression"], env, placeholder, types, f"{path}.contracts[{index}].expression") for index, contract in enumerate(function["contracts"]) if contract["kind"] == "requires"]
    facts: list[str] = []
    for instruction_index, instruction in enumerate(block["instructions"]):
        term, side_conditions = _instruction_term(instruction, env, sorts, types, f"{path}.body.blocks[0].instructions[{instruction_index}]")
        for side_index, condition in enumerate(side_conditions): legacy._obligation(lines, requires + facts, condition, f"{prefix}.i{instruction_index}.side[{side_index}]")
        env[int(instruction["dest_local_id"])] = term
    terminator = block["terminator"]
    if terminator["kind"] != "return" or "value" not in terminator: raise UnsupportedVerifiedCore(f"{path}.body: aggregate proof function requires value return")
    returned = legacy._typed_value(terminator["value"], env, sorts, f"{path}.body.blocks[0].terminator.value")
    for contract_index, contract in enumerate(function["contracts"]):
        if contract["kind"] != "ensures": continue
        claim = _expression(contract["expression"], env, returned, types, f"{path}.contracts[{contract_index}].expression")
        legacy._obligation(lines, requires + facts, claim, f"{prefix}.ensures[{contract_index}]")
    return lines


def generate_smtlib(value: Any) -> str:
    document = validate_typed_document(value)
    if not has_aggregates(document): return generate_phase4_smtlib(document)
    types = {int(entry["id"]): entry for entry in document["types"]}; aggregate_ids = {type_id for type_id, entry in types.items() if is_aggregate_type(entry)}
    sorts = {type_id: _sort(type_id, types) for type_id, entry in types.items() if entry["kind"] != "unit"}
    functions = {int(function["id"]): function for function in document["functions"]}
    lines = ["(set-logic ALL)", f"; aggregate-encoding={ENCODING}"] + _datatype_declarations(document, types)
    for function_index, function in enumerate(document["functions"]):
        if _uses_aggregate(function, aggregate_ids): lines.extend(_aggregate_function_vcs(function, sorts, types, function_index))
        else: lines.extend(phase4_function_vcs(function, functions, sorts, function_index))
    if not any(line == "(check-sat)" for line in lines): raise UnsupportedVerifiedCore("$.functions: no proof obligations generated")
    lines.append("(exit)")
    return "\n".join(lines) + "\n"


__all__ = ["ENCODING", "UnsupportedVerifiedCore", "generate_smtlib"]
