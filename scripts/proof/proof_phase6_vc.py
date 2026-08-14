"""Proof Phase 6 VC generation for checked signed machine integers."""
from __future__ import annotations

import copy
from typing import Any

from proof import proof_phase4_vc as phase4
from proof import proof_phase5_vc as phase5
from proof import smtlib_v1 as legacy
from proof.aggregate_semantics import has_aggregates
from proof.aggregate_types import is_aggregate_type, smt_constructor, smt_selector
from proof.machine_integer_semantics import PROFILE, integer_bounds
from proof.typed_admission_v6 import validate_typed_document

UnsupportedVerifiedCore = legacy.UnsupportedVerifiedCore
ENCODING = "arukellt-machine-int-range-v1"


def _types(document: dict[str, Any]) -> dict[int, dict[str, Any]]:
    return {int(entry["id"]): entry for entry in document["types"]}


def _bool_type_id(types: dict[int, dict[str, Any]]) -> int:
    matches = [type_id for type_id, entry in types.items() if entry.get("kind") == "bool"]
    if len(matches) != 1:
        raise UnsupportedVerifiedCore("$.types: expected exactly one bool TypeId")
    return matches[0]


def _range(term: str, type_id: int, types: dict[int, dict[str, Any]]) -> str:
    entry = types[type_id]
    if entry.get("kind") != "integer":
        raise UnsupportedVerifiedCore(f"$.types[id={type_id}]: expected integer for machine range")
    low, high = integer_bounds(entry)
    return f"(and (>= {term} {low}) (<= {term} {high}))"


def _walk_ids(expression: dict[str, Any], result: list[int]) -> None:
    if isinstance(expression, dict) and type(expression.get("id")) is int:
        result.append(int(expression["id"]))
        for operand in expression.get("operands", []):
            _walk_ids(operand, result)


def _next_expression_id(function: dict[str, Any]) -> list[int]:
    ids: list[int] = []
    for contract in function.get("contracts", []):
        _walk_ids(contract["expression"], ids)
    for block in function.get("body", {}).get("blocks", []):
        if "loop" in block:
            for expression in block["loop"].get("invariants", []):
                _walk_ids(expression, ids)
            _walk_ids(block["loop"].get("decreases", {}), ids)
    return [max(ids, default=-1) + 1]


def _new_id(counter: list[int]) -> int:
    value = counter[0]
    counter[0] += 1
    return value


def _atom(kind: str, type_id: int, counter: list[int], **extra: Any) -> dict[str, Any]:
    return {"id": _new_id(counter), "kind": kind, "type_id": type_id, **extra}


def _range_expression(
    *,
    type_id: int,
    bool_type_id: int,
    counter: list[int],
    types: dict[int, dict[str, Any]],
    local_id: int | None = None,
    result: bool = False,
) -> dict[str, Any]:
    low, high = integer_bounds(types[type_id])
    if result:
        left_a = _atom("result", type_id, counter)
        left_b = _atom("result", type_id, counter)
    else:
        if local_id is None:
            raise UnsupportedVerifiedCore("internal: local_id required")
        left_a = _atom("local", type_id, counter, local_id=local_id)
        left_b = _atom("local", type_id, counter, local_id=local_id)
    ge = {
        "id": _new_id(counter),
        "kind": "ge",
        "type_id": bool_type_id,
        "operands": [left_a, _atom("constant", type_id, counter, value=low)],
    }
    le = {
        "id": _new_id(counter),
        "kind": "le",
        "type_id": bool_type_id,
        "operands": [left_b, _atom("constant", type_id, counter, value=high)],
    }
    return {"id": _new_id(counter), "kind": "and", "type_id": bool_type_id, "operands": [ge, le]}


def _augment_machine_ranges(document: dict[str, Any]) -> dict[str, Any]:
    value = copy.deepcopy(document)
    types = _types(value)
    bool_type_id = _bool_type_id(types)
    for function in value["functions"]:
        counter = _next_expression_id(function)
        local_by_name = {str(local["name"]): local for local in function["locals"]}
        for parameter in function["signature"]["parameters"]:
            type_id = int(parameter["type_id"])
            if types[type_id].get("kind") != "integer":
                continue
            local = local_by_name[str(parameter["name"])]
            function["contracts"].append({
                "kind": "requires",
                "expression": _range_expression(
                    type_id=type_id,
                    bool_type_id=bool_type_id,
                    counter=counter,
                    types=types,
                    local_id=int(local["id"]),
                ),
            })
        return_type = int(function["signature"]["return_type_id"])
        if return_type in types and types[return_type].get("kind") == "integer":
            function["contracts"].append({
                "kind": "ensures",
                "result_name": "result",
                "expression": _range_expression(
                    type_id=return_type,
                    bool_type_id=bool_type_id,
                    counter=counter,
                    types=types,
                    result=True,
                ),
            })
        blocks = {int(block["id"]): block for block in function["body"]["blocks"]}
        local_types = {int(local["id"]): int(local["type_id"]) for local in function["locals"]}
        for block in function["body"]["blocks"]:
            if "loop" not in block:
                continue
            term = block["terminator"]
            if term.get("kind") != "branch":
                continue
            modified = phase4._modified_user_locals(function, blocks, int(term["then_target"]), int(block["id"]))
            for local_id in sorted(modified):
                type_id = local_types[local_id]
                if types[type_id].get("kind") != "integer":
                    continue
                block["loop"]["invariants"].append(
                    _range_expression(
                        type_id=type_id,
                        bool_type_id=bool_type_id,
                        counter=counter,
                        types=types,
                        local_id=local_id,
                    )
                )
    return value


def _machine_helpers() -> list[str]:
    return [
        "; signed machine integer semantics: truncation toward zero",
        "(define-fun arukellt_mi_abs ((x Int)) Int (ite (< x 0) (- x) x))",
        "(define-fun arukellt_mi_sdiv ((a Int) (b Int)) Int (let ((q (div (arukellt_mi_abs a) (arukellt_mi_abs b)))) (ite (xor (< a 0) (< b 0)) (- q) q)))",
        "(define-fun arukellt_mi_srem ((a Int) (b Int)) Int (- a (* b (arukellt_mi_sdiv a b))))",
    ]


def _render_expression_raw(
    expression: dict[str, Any],
    env: dict[int, str],
    result_symbol: str,
    types: dict[int, dict[str, Any]],
    path: str,
) -> tuple[str, list[str]]:
    kind = str(expression["kind"])
    type_id = int(expression["type_id"])
    if kind == "local":
        local_id = int(expression["local_id"])
        if local_id not in env:
            raise UnsupportedVerifiedCore(f"{path}.local_id: undefined local")
        return env[local_id], []
    if kind == "result":
        return result_symbol, []
    if kind == "constant":
        value = expression["value"]
        if type(value) is bool:
            return ("true" if value else "false"), []
        if type(value) is int:
            return str(value), []
        raise UnsupportedVerifiedCore(f"{path}.value: unsupported constant")

    rendered = [
        _render_expression_raw(operand, env, result_symbol, types, f"{path}.operands[{index}]")
        for index, operand in enumerate(expression.get("operands", []))
    ]
    operands = [item[0] for item in rendered]
    guards = [guard for item in rendered for guard in item[1]]

    if kind == "construct":
        variant = int(expression.get("variant_index", 0))
        return f"({smt_constructor(type_id, variant)}{' ' if operands else ''}{' '.join(operands)})", guards
    if kind == "project":
        source_type = int(expression["operands"][0]["type_id"])
        return f"({smt_selector(source_type, int(expression['index']))} {operands[0]})", guards
    if kind == "is_variant":
        source_type = int(expression["operands"][0]["type_id"])
        variant = int(expression["variant_index"])
        return f"((_ is {smt_constructor(source_type, variant)}) {operands[0]})", guards
    if kind == "variant_payload":
        source_type = int(expression["operands"][0]["type_id"])
        variant = int(expression["variant_index"])
        payload = int(expression["payload_index"])
        return f"({smt_selector(source_type, variant, payload)} {operands[0]})", guards

    if kind == "neg":
        term = f"(- {operands[0]})"
        guards.append(_range(term, type_id, types))
        return term, guards
    if kind in {"add", "sub", "mul"}:
        symbol = {"add": "+", "sub": "-", "mul": "*"}[kind]
        term = f"({symbol} {operands[0]} {operands[1]})"
        guards.append(_range(term, type_id, types))
        return term, guards
    if kind == "div":
        source_type = int(expression["operands"][0]["type_id"])
        low, _ = integer_bounds(types[source_type])
        term = f"(arukellt_mi_sdiv {operands[0]} {operands[1]})"
        guards.extend([
            f"(distinct {operands[1]} 0)",
            f"(not (and (= {operands[0]} {low}) (= {operands[1]} (- 1))))",
            _range(term, type_id, types),
        ])
        return term, guards
    if kind == "mod":
        term = f"(arukellt_mi_srem {operands[0]} {operands[1]})"
        guards.extend([f"(distinct {operands[1]} 0)", _range(term, type_id, types)])
        return term, guards
    if kind == "not":
        return f"(not {operands[0]})", guards
    operators = {
        "eq": "=", "ne": "distinct", "lt": "<", "le": "<=", "gt": ">", "ge": ">=",
        "and": "and", "or": "or", "implies": "=>",
    }
    if kind not in operators:
        raise UnsupportedVerifiedCore(f"{path}.kind: unsupported machine expression {kind!r}")
    return f"({operators[kind]} {operands[0]} {operands[1]})", guards


def _machine_contract_expression(
    expression: dict[str, Any],
    env: dict[int, str],
    sorts: dict[int, str],
    result_symbol: str,
    path: str,
    types: dict[int, dict[str, Any]],
) -> str:
    del sorts
    term, guards = _render_expression_raw(expression, env, result_symbol, types, path)
    if guards:
        if types[int(expression["type_id"])].get("kind") != "bool":
            raise UnsupportedVerifiedCore(f"{path}: arithmetic guards require a boolean proof root")
        return f"(and {' '.join(guards)} {term})"
    return term


def _machine_phase5_expression(
    expression: dict[str, Any],
    env: dict[int, str],
    result_symbol: str,
    types: dict[int, dict[str, Any]],
    path: str,
) -> str:
    term, guards = _render_expression_raw(expression, env, result_symbol, types, path)
    if guards:
        if types[int(expression["type_id"])].get("kind") != "bool":
            raise UnsupportedVerifiedCore(f"{path}: arithmetic guards require a boolean proof root")
        return f"(and {' '.join(guards)} {term})"
    return term


def _machine_instruction_term(
    instruction: dict[str, Any],
    env: dict[int, str],
    sorts: dict[int, str],
    path: str,
    types: dict[int, dict[str, Any]],
    original_instruction_term,
) -> tuple[str, list[str]]:
    op = str(instruction["op"])
    type_id = int(instruction["type_id"])
    if op == "convert":
        argument = legacy._typed_value(instruction["arguments"][0], env, sorts, f"{path}.arguments[0]")
        conversion = str(instruction["conversion"])
        if conversion == "signed-widen":
            return argument, []
        if conversion == "checked-narrow":
            return argument, [_range(argument, type_id, types)]
        raise UnsupportedVerifiedCore(f"{path}.conversion: unsupported {conversion!r}")
    if op == "binary" and instruction.get("operator") in {"shl", "shr_s"}:
        left = legacy._typed_value(instruction["arguments"][0], env, sorts, f"{path}.arguments[0]")
        raw_count = int(instruction["arguments"][1]["value"])
        bits = int(types[type_id]["bits"])
        count = raw_count % bits
        factor = 1 << count
        if instruction["operator"] == "shl":
            term = f"(* {left} {factor})"
            return term, [_range(term, type_id, types)]
        return f"(div {left} {factor})", []

    term, side = original_instruction_term(instruction, env, sorts, path)
    if op == "unary" and instruction.get("operator") == "neg":
        return term, side + [_range(term, type_id, types)]
    if op == "binary":
        operator = str(instruction.get("operator"))
        if operator in {"add", "sub", "mul"}:
            return term, side + [_range(term, type_id, types)]
        if operator in {"div", "mod"}:
            left = legacy._typed_value(instruction["arguments"][0], env, sorts, f"{path}.arguments[0]")
            right = legacy._typed_value(instruction["arguments"][1], env, sorts, f"{path}.arguments[1]")
            if operator == "div":
                low, _ = integer_bounds(types[type_id])
                machine = f"(arukellt_mi_sdiv {left} {right})"
                return machine, [
                    f"(distinct {right} 0)",
                    f"(not (and (= {left} {low}) (= {right} (- 1))))",
                    _range(machine, type_id, types),
                ]
            machine = f"(arukellt_mi_srem {left} {right})"
            return machine, [f"(distinct {right} 0)", _range(machine, type_id, types)]
    return term, side


def generate_smtlib(value: Any) -> str:
    document = validate_typed_document(value)
    document = _augment_machine_ranges(document)
    types = _types(document)
    aggregate_ids = {type_id for type_id, entry in types.items() if is_aggregate_type(entry)}
    sorts = {type_id: phase5._sort(type_id, types) for type_id, entry in types.items() if entry.get("kind") != "unit"}
    functions = {int(function["id"]): function for function in document["functions"]}

    original_contract = legacy._contract_expression
    original_instruction = legacy._instruction_term
    original_phase5_expression = phase5._expression
    original_validator = phase5.validate_typed_document
    try:
        legacy._contract_expression = lambda expression, env, local_sorts, result_symbol, path: _machine_contract_expression(
            expression, env, local_sorts, result_symbol, path, types
        )
        legacy._instruction_term = lambda instruction, env, local_sorts, path: _machine_instruction_term(
            instruction, env, local_sorts, path, types, original_instruction
        )
        phase5._expression = lambda expression, env, result_symbol, local_types, path: _machine_phase5_expression(
            expression, env, result_symbol, local_types, path
        )
        phase5.validate_typed_document = validate_typed_document

        lines = [
            "(set-logic ALL)",
            f"; machine-int-profile={PROFILE}",
            f"; machine-int-encoding={ENCODING}",
        ]
        lines.extend(_machine_helpers())
        if aggregate_ids:
            lines.extend(phase5._datatype_declarations(document, types))
        for function_index, function in enumerate(document["functions"]):
            if aggregate_ids and phase5._uses_aggregate(function, aggregate_ids):
                lines.extend(phase5._aggregate_function_vcs(function, sorts, types, function_index))
            else:
                lines.extend(phase4._function_vcs(function, functions, sorts, function_index))
        if not any(line == "(check-sat)" for line in lines):
            raise UnsupportedVerifiedCore("$.functions: no proof obligations generated")
        lines.append("(exit)")
        return "\n".join(lines) + "\n"
    finally:
        legacy._contract_expression = original_contract
        legacy._instruction_term = original_instruction
        phase5._expression = original_phase5_expression
        phase5.validate_typed_document = original_validator


__all__ = ["ENCODING", "UnsupportedVerifiedCore", "generate_smtlib"]
