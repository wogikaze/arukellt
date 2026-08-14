"""Proof Phase 7 VC generation for read-only references and heap reads."""
from __future__ import annotations

from typing import Any

from proof import proof_phase4_vc as phase4
from proof import proof_phase5_vc as phase5
from proof import smtlib_v1 as legacy
from proof.aggregate_types import aggregate_order, is_aggregate_type, smt_constructor, smt_selector, smt_sort
from proof.machine_integer_semantics import integer_bounds
from proof.proof_phase6_vc import (
    PROFILE as MACHINE_PROFILE,
    _augment_machine_ranges,
    _machine_helpers,
    _machine_instruction_term,
    _range,
)
from proof.readonly_memory_semantics import (
    MEMORY_EXPRESSION_KINDS,
    MEMORY_INSTRUCTION_OPS,
    MODEL,
    _reference_descriptors,
    function_uses_memory,
)
from proof.typed_admission_v7 import validate_typed_document

UnsupportedVerifiedCore = legacy.UnsupportedVerifiedCore
ENCODING = "arukellt-readonly-heap-smt-v1"


def _types(document: dict[str, Any]) -> dict[int, dict[str, Any]]:
    return {int(entry["id"]): entry for entry in document["types"]}


def _ref_sort(type_id: int) -> str:
    return f"ArukelltRefT{type_id}"


def _null(type_id: int) -> str:
    return f"arukellt_null_t{type_id}"


def _field_fun(reference_type: int, field_index: int) -> str:
    return f"arukellt_heap_t{reference_type}_f{field_index}"


def _length_fun(reference_type: int) -> str:
    return f"arukellt_heap_t{reference_type}_len"


def _get_fun(reference_type: int) -> str:
    return f"arukellt_heap_t{reference_type}_get"


def _sort(type_id: int, types: dict[int, dict[str, Any]]) -> str:
    entry = types[type_id]
    kind = entry.get("kind")
    if kind == "reference":
        return _ref_sort(type_id)
    if is_aggregate_type(entry):
        return smt_sort(type_id)
    return legacy._sort(str(kind), f"$.types[id={type_id}]")


def _non_null_constraint(term: str, reference_type: int) -> str:
    return f"(distinct {term} {_null(reference_type)})"


def _reference_nullable(type_id: int, types: dict[int, dict[str, Any]]) -> bool:
    return bool(types[type_id]["representation"]["nullable"])


def _value_semantic_axiom(term: str, type_id: int, types: dict[int, dict[str, Any]]) -> str | None:
    entry = types[type_id]
    if entry.get("kind") == "integer":
        return _range(term, type_id, types)
    if entry.get("kind") == "reference" and not _reference_nullable(type_id, types):
        return _non_null_constraint(term, type_id)
    return None


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
            constructors: list[str] = []
            for variant_index, variant in enumerate(entry["variants"]):
                fields = " ".join(
                    f"({smt_selector(type_id, variant_index, payload_index)} {_sort(int(member), types)})"
                    for payload_index, member in enumerate(variant["payload_type_ids"])
                )
                constructors.append(
                    f"({smt_constructor(type_id, variant_index)} {fields})" if fields else f"({smt_constructor(type_id, variant_index)})"
                )
            lines.append(f"(declare-datatypes () (({smt_sort(type_id)} {' '.join(constructors)})))")
    return lines


def _heap_declarations(document: dict[str, Any], types: dict[int, dict[str, Any]], descriptors: dict[int, dict[str, Any]]) -> list[str]:
    lines: list[str] = [f"; readonly-memory-model={MODEL}", f"; readonly-memory-encoding={ENCODING}"]
    for reference_type in sorted(descriptors):
        lines.append(f"(declare-sort {_ref_sort(reference_type)} 0)")
        lines.append(f"(declare-const {_null(reference_type)} {_ref_sort(reference_type)})")
    for reference_type in sorted(descriptors):
        descriptor = descriptors[reference_type]
        ref_sort = _ref_sort(reference_type)
        if descriptor["kind"] == "object":
            pointee = types[int(descriptor["pointee_type_id"])]
            for field_index, field in enumerate(pointee.get("fields", [])):
                field_type = int(field["type_id"])
                name = _field_fun(reference_type, field_index)
                lines.append(f"(declare-fun {name} ({ref_sort}) {_sort(field_type, types)})")
                semantic = _value_semantic_axiom(f"({name} r)", field_type, types)
                if semantic:
                    lines.append(
                        f"(assert (forall ((r {ref_sort})) (=> {_non_null_constraint('r', reference_type)} {semantic})))"
                    )
        else:
            element_type = int(descriptor["element_type_id"])
            length_type = int(descriptor["length_type_id"])
            length_name = _length_fun(reference_type)
            get_name = _get_fun(reference_type)
            lines.append(f"(declare-fun {length_name} ({ref_sort}) {_sort(length_type, types)})")
            lines.append(f"(declare-fun {get_name} ({ref_sort} {_sort(length_type, types)}) {_sort(element_type, types)})")
            length_term = f"({length_name} r)"
            length_range = _range(length_term, length_type, types)
            lines.append(
                f"(assert (forall ((r {ref_sort})) (=> {_non_null_constraint('r', reference_type)} (and (>= {length_term} 0) {length_range}))))"
            )
            semantic = _value_semantic_axiom(f"({get_name} r i)", element_type, types)
            if semantic:
                lines.append(
                    f"(assert (forall ((r {ref_sort}) (i {_sort(length_type, types)})) (=> (and {_non_null_constraint('r', reference_type)} (>= i 0) (< i ({length_name} r))) {semantic})))"
                )
    return lines


def _render_expression_raw(
    expression: dict[str, Any],
    env: dict[int, str],
    result_symbol: str,
    types: dict[int, dict[str, Any]],
    descriptors: dict[int, dict[str, Any]],
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
        _render_expression_raw(operand, env, result_symbol, types, descriptors, f"{path}.operands[{index}]")
        for index, operand in enumerate(expression.get("operands", []))
    ]
    operands = [item[0] for item in rendered]
    guards = [guard for item in rendered for guard in item[1]]

    if kind == "is_null":
        reference_type = int(expression["operands"][0]["type_id"])
        return f"(= {operands[0]} {_null(reference_type)})", guards
    if kind == "ref_eq":
        return f"(= {operands[0]} {operands[1]})", guards
    if kind == "load_field":
        reference_type = int(expression["operands"][0]["type_id"])
        guards.append(_non_null_constraint(operands[0], reference_type))
        return f"({_field_fun(reference_type, int(expression['field_index']))} {operands[0]})", guards
    if kind == "array_len":
        reference_type = int(expression["operands"][0]["type_id"])
        guards.append(_non_null_constraint(operands[0], reference_type))
        term = f"({_length_fun(reference_type)} {operands[0]})"
        guards.append(_range(term, type_id, types))
        return term, guards
    if kind == "array_get":
        reference_type = int(expression["operands"][0]["type_id"])
        descriptor = descriptors[reference_type]
        index = operands[1]
        guards.extend([
            _non_null_constraint(operands[0], reference_type),
            f"(>= {index} 0)",
            f"(< {index} ({_length_fun(reference_type)} {operands[0]}))",
        ])
        return f"({_get_fun(reference_type)} {operands[0]} {index})", guards

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
        raise UnsupportedVerifiedCore(f"{path}.kind: unsupported Phase 7 expression {kind!r}")
    return f"({operators[kind]} {operands[0]} {operands[1]})", guards


def _contract_expression(
    expression: dict[str, Any],
    env: dict[int, str],
    sorts: dict[int, str],
    result_symbol: str,
    path: str,
    types: dict[int, dict[str, Any]],
    descriptors: dict[int, dict[str, Any]],
) -> str:
    del sorts
    term, guards = _render_expression_raw(expression, env, result_symbol, types, descriptors, path)
    if guards:
        if types[int(expression["type_id"])].get("kind") != "bool":
            raise UnsupportedVerifiedCore(f"{path}: guarded memory/arithmetic expression requires boolean proof root")
        return f"(and {' '.join(guards)} {term})"
    return term


def _memory_instruction_term(
    instruction: dict[str, Any],
    env: dict[int, str],
    sorts: dict[int, str],
    path: str,
    types: dict[int, dict[str, Any]],
    descriptors: dict[int, dict[str, Any]],
    original_instruction_term,
) -> tuple[str, list[str]]:
    op = str(instruction["op"])
    if op not in MEMORY_INSTRUCTION_OPS:
        return _machine_instruction_term(instruction, env, sorts, path, types, original_instruction_term)
    arguments = [legacy._typed_value(value, env, sorts, f"{path}.arguments[{index}]") for index, value in enumerate(instruction.get("arguments", []))]
    if op == "is_null":
        reference_type = int(instruction["arguments"][0]["type_id"])
        return f"(= {arguments[0]} {_null(reference_type)})", []
    if op == "ref_eq":
        return f"(= {arguments[0]} {arguments[1]})", []
    if op == "load_field":
        reference_type = int(instruction["arguments"][0]["type_id"])
        return (
            f"({_field_fun(reference_type, int(instruction['field_index']))} {arguments[0]})",
            [_non_null_constraint(arguments[0], reference_type)],
        )
    if op == "array_len":
        reference_type = int(instruction["arguments"][0]["type_id"])
        term = f"({_length_fun(reference_type)} {arguments[0]})"
        return term, [_non_null_constraint(arguments[0], reference_type), _range(term, int(instruction["type_id"]), types)]
    reference_type = int(instruction["arguments"][0]["type_id"])
    index = arguments[1]
    term = f"({_get_fun(reference_type)} {arguments[0]} {index})"
    return term, [
        _non_null_constraint(arguments[0], reference_type),
        f"(>= {index} 0)",
        f"(< {index} ({_length_fun(reference_type)} {arguments[0]}))",
    ]


def _memory_function_vcs(
    function: dict[str, Any],
    sorts: dict[int, str],
    types: dict[int, dict[str, Any]],
    descriptors: dict[int, dict[str, Any]],
    function_index: int,
) -> list[str]:
    path = f"$.functions[{function_index}]"
    block = function["body"]["blocks"][0]
    prefix = legacy._symbol(f"f{function['id']}_{function['name']}")
    lines: list[str] = []
    env: dict[int, str] = {}
    parameter_names = {str(parameter["name"]) for parameter in function["signature"]["parameters"]}
    implicit: list[str] = []
    for local in function["locals"]:
        if local["storage"] != "parameter" or str(local["name"]) not in parameter_names:
            continue
        type_id = int(local["type_id"])
        symbol = legacy._symbol(f"{prefix}_arg_{local['id']}_{local['name']}")
        env[int(local["id"])] = symbol
        lines.append(f"(declare-const {symbol} {sorts[type_id]})")
        if types[type_id].get("kind") == "reference" and not _reference_nullable(type_id, types):
            implicit.append(_non_null_constraint(symbol, type_id))

    result_placeholder = legacy._symbol(f"{prefix}_result")
    requires = [
        _contract_expression(contract["expression"], env, sorts, result_placeholder, f"{path}.contracts[{index}].expression", types, descriptors)
        for index, contract in enumerate(function["contracts"])
        if contract["kind"] == "requires"
    ] + implicit

    for instruction_index, instruction in enumerate(block["instructions"]):
        instruction_path = f"{path}.body.blocks[0].instructions[{instruction_index}]"
        term, side_conditions = phase5._instruction_term(instruction, env, sorts, types, instruction_path)
        for side_index, condition in enumerate(side_conditions):
            legacy._obligation(lines, requires, condition, f"{prefix}.i{instruction_index}.side[{side_index}]")
        env[int(instruction["dest_local_id"])] = term

    terminator = block["terminator"]
    if terminator["kind"] != "return" or "value" not in terminator:
        raise UnsupportedVerifiedCore(f"{path}.body: Phase 7 memory function requires value return")
    returned = legacy._typed_value(terminator["value"], env, sorts, f"{path}.body.blocks[0].terminator.value")
    for contract_index, contract in enumerate(function["contracts"]):
        if contract["kind"] != "ensures":
            continue
        claim = _contract_expression(contract["expression"], env, sorts, returned, f"{path}.contracts[{contract_index}].expression", types, descriptors)
        legacy._obligation(lines, requires, claim, f"{prefix}.ensures[{contract_index}]")
    return_type = int(function["signature"]["return_type_id"])
    if types[return_type].get("kind") == "reference" and not _reference_nullable(return_type, types):
        legacy._obligation(lines, requires, _non_null_constraint(returned, return_type), f"{prefix}.return-nonnull")
    return lines


def generate_smtlib(value: Any) -> str:
    document = validate_typed_document(value)
    document = _augment_machine_ranges(document)
    types = _types(document)
    descriptors = _reference_descriptors(document)
    aggregate_ids = {type_id for type_id, entry in types.items() if is_aggregate_type(entry)}
    sorts = {type_id: _sort(type_id, types) for type_id, entry in types.items() if entry.get("kind") != "unit"}
    functions = {int(function["id"]): function for function in document["functions"]}

    original_contract = legacy._contract_expression
    original_instruction = legacy._instruction_term
    original_phase5_expression = phase5._expression
    original_phase5_instruction = phase5._instruction_term
    try:
        legacy._contract_expression = lambda expression, env, local_sorts, result_symbol, path: _contract_expression(
            expression, env, local_sorts, result_symbol, path, types, descriptors
        )
        legacy._instruction_term = lambda instruction, env, local_sorts, path: _memory_instruction_term(
            instruction, env, local_sorts, path, types, descriptors, original_instruction
        )
        phase5._expression = lambda expression, env, result_symbol, local_types, path: _contract_expression(
            expression, env, {}, result_symbol, path, local_types, descriptors
        )
        phase5._instruction_term = lambda instruction, env, local_sorts, local_types, path: (
            original_phase5_instruction(instruction, env, local_sorts, local_types, path)
            if instruction.get("op") in {"construct", "project", "is_variant", "variant_payload"}
            else legacy._instruction_term(instruction, env, local_sorts, path)
        )

        lines = [
            "(set-logic ALL)",
            f"; machine-int-profile={MACHINE_PROFILE}",
            f"; readonly-memory-model={MODEL}",
        ]
        lines.extend(_machine_helpers())
        if aggregate_ids:
            lines.extend(_datatype_declarations(document, types))
        lines.extend(_heap_declarations(document, types, descriptors))
        for function_index, function in enumerate(document["functions"]):
            if function_uses_memory(function, types):
                lines.extend(_memory_function_vcs(function, sorts, types, descriptors, function_index))
            elif aggregate_ids and phase5._uses_aggregate(function, aggregate_ids):
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
        phase5._instruction_term = original_phase5_instruction


__all__ = ["ENCODING", "UnsupportedVerifiedCore", "generate_smtlib"]
