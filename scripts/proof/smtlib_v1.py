"""Fail-closed VerifiedCore v1 to SMT-LIB VC renderer for proof phases 1-3."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

from proof.common import load_json
from proof.verified_core_typed import validate_typed_document


class UnsupportedVerifiedCore(ValueError):
    pass


_SYMBOL_RE = re.compile(r"[^A-Za-z0-9_.$!?~-]")


def _symbol(raw: str) -> str:
    value = _SYMBOL_RE.sub("_", raw) or "value"
    return "v_" + value if value[0].isdigit() else value


def _sort(kind: str, path: str) -> str:
    if kind == "integer":
        return "Int"
    if kind == "bool":
        return "Bool"
    raise UnsupportedVerifiedCore(f"{path}: unsupported SMT type kind {kind!r}")


def _constant(value: Any, sort: str, path: str) -> str:
    if sort == "Bool":
        if value is True:
            return "true"
        if value is False:
            return "false"
        raise UnsupportedVerifiedCore(f"{path}: expected boolean")
    if sort == "Int":
        if isinstance(value, bool) or not isinstance(value, int):
            raise UnsupportedVerifiedCore(f"{path}: expected integer")
        return f"(- {abs(value)})" if value < 0 else str(value)
    raise UnsupportedVerifiedCore(f"{path}: unsupported constant sort")


def _typed_value(value: dict[str, Any], env: dict[int, str], sorts: dict[int, str], path: str) -> str:
    if value["kind"] == "local":
        local_id = int(value["local_id"])
        if local_id not in env:
            raise UnsupportedVerifiedCore(f"{path}.local_id: undefined on this path")
        return env[local_id]
    return _constant(value["value"], sorts[int(value["type_id"])], f"{path}.value")


def _contract_expression(expr: dict[str, Any], env: dict[int, str], sorts: dict[int, str], result_symbol: str, path: str) -> str:
    kind = expr["kind"]
    if kind == "local":
        local_id = int(expr["local_id"])
        if local_id not in env:
            raise UnsupportedVerifiedCore(f"{path}.local_id: undefined local")
        return env[local_id]
    if kind == "result":
        return result_symbol
    if kind == "constant":
        return _constant(expr["value"], sorts[int(expr["type_id"])], f"{path}.value")
    operands = [
        _contract_expression(operand, env, sorts, result_symbol, f"{path}.operands[{index}]")
        for index, operand in enumerate(expr.get("operands", []))
    ]
    unary = {"not": "not", "neg": "-"}
    binary = {
        "add": "+", "sub": "-", "mul": "*", "div": "div", "mod": "mod",
        "eq": "=", "ne": "distinct", "lt": "<", "le": "<=", "gt": ">", "ge": ">=",
        "and": "and", "or": "or", "implies": "=>",
    }
    if kind in unary:
        return f"({unary[kind]} {operands[0]})"
    if kind in binary:
        return f"({binary[kind]} {operands[0]} {operands[1]})"
    raise UnsupportedVerifiedCore(f"{path}.kind: unsupported expression {kind!r}")


def _instruction_term(instruction: dict[str, Any], env: dict[int, str], sorts: dict[int, str], path: str) -> tuple[str, list[str]]:
    op = instruction["op"]
    if op == "const":
        return _constant(instruction["value"], sorts[int(instruction["type_id"])], f"{path}.value"), []
    args = [_typed_value(value, env, sorts, f"{path}.arguments[{index}]") for index, value in enumerate(instruction["arguments"])]
    if op == "copy":
        return args[0], []
    if op == "unary":
        operator = {"neg": "-", "not": "not"}[instruction["operator"]]
        return f"({operator} {args[0]})", []
    if op == "binary":
        operator = instruction["operator"]
        smt = {
            "add": "+", "sub": "-", "mul": "*", "div": "div", "mod": "mod",
            "eq": "=", "ne": "distinct", "lt": "<", "le": "<=", "gt": ">", "ge": ">=",
            "and": "and", "or": "or", "implies": "=>",
        }[operator]
        side = [f"(distinct {args[1]} 0)"] if operator in {"div", "mod"} else []
        return f"({smt} {args[0]} {args[1]})", side
    raise UnsupportedVerifiedCore(f"{path}.op: unsupported non-call instruction {op!r}")


def _obligation(lines: list[str], assumptions: list[str], claim: str, label: str) -> None:
    lines.append(f"; obligation {label}")
    lines.append("(push 1)")
    for assumption in assumptions:
        lines.append(f"(assert {assumption})")
    lines.append(f"(assert (not {claim}))")
    lines.append("(check-sat)")
    lines.append("(pop 1)")


def _callee_env(callee: dict[str, Any], argument_terms: list[str]) -> dict[int, str]:
    by_name = {str(local["name"]): int(local["id"]) for local in callee["locals"] if local["storage"] == "parameter"}
    env: dict[int, str] = {}
    for parameter, term in zip(callee["signature"]["parameters"], argument_terms):
        name = str(parameter["name"])
        if name not in by_name:
            raise UnsupportedVerifiedCore(f"callee {callee['name']}: parameter local missing for {name!r}")
        env[by_name[name]] = term
    return env


def _function_vcs(function: dict[str, Any], functions: dict[int, dict[str, Any]], sorts: dict[int, str], function_index: int) -> list[str]:
    path = f"$.functions[{function_index}]"
    prefix = _symbol(f"f{function['id']}_{function['name']}")
    lines: list[str] = []
    initial_env: dict[int, str] = {}
    parameter_names = {parameter["name"] for parameter in function["signature"]["parameters"]}
    for local in function["locals"]:
        if local["storage"] == "parameter" and local["name"] in parameter_names:
            symbol = _symbol(f"{prefix}_arg_{local['id']}_{local['name']}")
            initial_env[int(local["id"])] = symbol
            lines.append(f"(declare-const {symbol} {sorts[int(local['type_id'])]})")

    placeholder = _symbol(f"{prefix}_result")
    requires = [
        _contract_expression(contract["expression"], initial_env, sorts, placeholder, f"{path}.contracts[{index}].expression")
        for index, contract in enumerate(function["contracts"])
        if contract["kind"] == "requires"
    ]
    ensures = [(index, contract) for index, contract in enumerate(function["contracts"]) if contract["kind"] == "ensures"]
    if not ensures:
        raise UnsupportedVerifiedCore(f"{path}.contracts: at least one ensures is required")

    blocks = {int(block["id"]): block for block in function["body"]["blocks"]}
    entry = int(function["body"]["entry_block"])
    call_counter = [0]

    def visit(block_id: int, env: dict[int, str], path_conditions: list[str], facts: list[str], trace: tuple[int, ...]) -> None:
        if block_id in trace:
            raise UnsupportedVerifiedCore(f"{path}.body: cyclic path escaped typed admission")
        block = blocks[block_id]
        local_env = dict(env)
        local_facts = list(facts)

        for instruction_index, instruction in enumerate(block["instructions"]):
            instruction_path = f"{path}.body.blocks[id={block_id}].instructions[{instruction_index}]"
            assumptions = requires + path_conditions + local_facts
            if instruction["op"] == "call":
                callee_id = int(instruction["callee_id"])
                callee = functions[callee_id]
                argument_terms = [
                    _typed_value(value, local_env, sorts, f"{instruction_path}.arguments[{index}]")
                    for index, value in enumerate(instruction["arguments"])
                ]
                result_symbol = _symbol(f"{prefix}_b{block_id}_i{instruction_index}_call{call_counter[0]}")
                call_counter[0] += 1
                lines.append(f"(declare-const {result_symbol} {sorts[int(instruction['type_id'])]})")
                callee_env = _callee_env(callee, argument_terms)
                for contract_index, contract in enumerate(callee["contracts"]):
                    if contract["kind"] != "requires":
                        continue
                    claim = _contract_expression(
                        contract["expression"], callee_env, sorts, result_symbol,
                        f"{instruction_path}.callee.requires[{contract_index}]",
                    )
                    _obligation(lines, assumptions, claim, f"{prefix}.b{block_id}.i{instruction_index}.callee-requires[{contract_index}]")
                for contract_index, contract in enumerate(callee["contracts"]):
                    if contract["kind"] != "ensures":
                        continue
                    local_facts.append(
                        _contract_expression(
                            contract["expression"], callee_env, sorts, result_symbol,
                            f"{instruction_path}.callee.ensures[{contract_index}]",
                        )
                    )
                local_env[int(instruction["dest_local_id"])] = result_symbol
                continue

            term, side_conditions = _instruction_term(instruction, local_env, sorts, instruction_path)
            for side_index, side_condition in enumerate(side_conditions):
                _obligation(lines, assumptions, side_condition, f"{prefix}.b{block_id}.i{instruction_index}.side[{side_index}]")
            local_env[int(instruction["dest_local_id"])] = term

        terminator = block["terminator"]
        assumptions = requires + path_conditions + local_facts
        kind = terminator["kind"]
        if kind == "return":
            if "value" not in terminator:
                raise UnsupportedVerifiedCore(f"{path}.body.blocks[id={block_id}].terminator: value return required")
            returned = _typed_value(terminator["value"], local_env, sorts, f"{path}.body.blocks[id={block_id}].terminator.value")
            for contract_index, contract in ensures:
                claim = _contract_expression(contract["expression"], local_env, sorts, returned, f"{path}.contracts[{contract_index}].expression")
                _obligation(lines, assumptions, claim, f"{prefix}.b{block_id}.ensures[{contract_index}]")
            return
        if kind == "goto":
            target = int(terminator["target"])
            target_block = blocks[target]
            values = [
                _typed_value(value, local_env, sorts, f"{path}.body.blocks[id={block_id}].terminator.arguments[{index}]")
                for index, value in enumerate(terminator["arguments"])
            ]
            next_env = dict(local_env)
            for parameter, rendered in zip(target_block["parameters"], values):
                next_env[int(parameter["local_id"])] = rendered
            visit(target, next_env, list(path_conditions), local_facts, trace + (block_id,))
            return
        if kind == "branch":
            condition = _typed_value(terminator["condition"], local_env, sorts, f"{path}.body.blocks[id={block_id}].terminator.condition")
            for side, condition_term in (("then", condition), ("else", f"(not {condition})")):
                target = int(terminator[f"{side}_target"])
                target_block = blocks[target]
                values = [
                    _typed_value(value, local_env, sorts, f"{path}.body.blocks[id={block_id}].terminator.{side}_arguments[{index}]")
                    for index, value in enumerate(terminator[f"{side}_arguments"])
                ]
                next_env = dict(local_env)
                for parameter, rendered in zip(target_block["parameters"], values):
                    next_env[int(parameter["local_id"])] = rendered
                visit(target, next_env, path_conditions + [condition_term], local_facts, trace + (block_id,))
            return
        raise UnsupportedVerifiedCore(f"{path}.body.blocks[id={block_id}].terminator.kind: unsupported {kind!r}")

    visit(entry, initial_env, [], [], ())
    return lines


def generate_smtlib(value: Any) -> str:
    document = validate_typed_document(value)
    profile = document["target_profile"]
    if profile["integer_model"] != "mathematical" or profile["overflow"] != "checked" or profile["floating_point"] != "unsupported":
        raise UnsupportedVerifiedCore("$.target_profile: proof phases 0-3 require mathematical/checked/no-float profile")
    kinds = {int(entry["id"]): str(entry["kind"]) for entry in document["types"]}
    sorts = {type_id: _sort(kind, f"$.types[id={type_id}]") for type_id, kind in kinds.items() if kind != "unit"}
    functions = {int(function["id"]): function for function in document["functions"]}
    lines = ["(set-logic QF_NIA)", "; generated from arukellt-verified-core v1 proof phase 3"]
    for function_index, function in enumerate(document["functions"]):
        lines.extend(_function_vcs(function, functions, sorts, function_index))
    if not any(line == "(check-sat)" for line in lines):
        raise UnsupportedVerifiedCore("$.functions: no proof obligations generated")
    lines.append("(exit)")
    return "\n".join(lines) + "\n"


def generate_smtlib_file(subject_path: Path, output_path: Path) -> int:
    rendered = generate_smtlib(load_json(subject_path))
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(rendered, encoding="utf-8")
    return rendered.count("(check-sat)")


__all__ = ["UnsupportedVerifiedCore", "generate_smtlib", "generate_smtlib_file"]
