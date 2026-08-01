"""Fail-closed VerifiedCore v1 to SMT-LIB verification-condition adapter."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

from proof.common import load_json
from proof.verified_core import validate_document as validate_verified_core


class UnsupportedVerifiedCore(ValueError):
    """Raised when VerifiedCore is valid but outside the proved SMT subset."""


_SYMBOL_RE = re.compile(r"[^A-Za-z0-9_.$!?~-]")


def _symbol(raw: str) -> str:
    value = _SYMBOL_RE.sub("_", raw)
    if not value:
        value = "value"
    if value[0].isdigit():
        value = "v_" + value
    return value


def _sort(type_entry: dict[str, Any], path: str) -> str:
    kind = type_entry["kind"]
    if kind == "integer":
        return "Int"
    if kind == "bool":
        return "Bool"
    raise UnsupportedVerifiedCore(f"{path}: unsupported SMT type kind: {kind}")


def _constant(value: Any, sort: str, path: str) -> str:
    if sort == "Bool":
        if value is True:
            return "true"
        if value is False:
            return "false"
        raise UnsupportedVerifiedCore(f"{path}: expected boolean constant")
    if sort == "Int":
        if isinstance(value, bool) or not isinstance(value, int):
            raise UnsupportedVerifiedCore(f"{path}: expected integer constant")
        if value < 0:
            return f"(- {abs(value)})"
        return str(value)
    raise UnsupportedVerifiedCore(f"{path}: unsupported constant sort: {sort}")


def _typed_value(
    value: dict[str, Any],
    local_symbols: dict[int, str],
    type_sorts: dict[int, str],
    path: str,
) -> str:
    kind = value["kind"]
    type_id = value["type_id"]
    sort = type_sorts[type_id]
    if kind == "local":
        local_id = value["local_id"]
        try:
            return local_symbols[local_id]
        except KeyError as exc:
            raise UnsupportedVerifiedCore(f"{path}.local_id: unknown local {local_id}") from exc
    if kind == "constant":
        return _constant(value["value"], sort, f"{path}.value")
    raise UnsupportedVerifiedCore(f"{path}.kind: unsupported return value kind: {kind}")


def _expression(
    expr: dict[str, Any],
    local_symbols: dict[int, str],
    type_sorts: dict[int, str],
    result_symbol: str,
    path: str,
) -> str:
    kind = expr["kind"]
    type_id = expr["type_id"]
    sort = type_sorts[type_id]
    operands = expr.get("operands", [])

    if kind == "local":
        local_id = expr.get("local_id")
        if local_id not in local_symbols:
            raise UnsupportedVerifiedCore(f"{path}.local_id: unknown local {local_id}")
        return local_symbols[local_id]
    if kind == "result":
        return result_symbol
    if kind == "constant":
        return _constant(expr.get("value"), sort, f"{path}.value")

    unary = {"not": "not", "neg": "-"}
    if kind in unary:
        if len(operands) != 1:
            raise UnsupportedVerifiedCore(f"{path}.operands: {kind} requires one operand")
        rendered = _expression(
            operands[0], local_symbols, type_sorts, result_symbol, f"{path}.operands[0]"
        )
        return f"({unary[kind]} {rendered})"

    binary = {
        "add": "+",
        "sub": "-",
        "mul": "*",
        "div": "div",
        "mod": "mod",
        "eq": "=",
        "ne": "distinct",
        "lt": "<",
        "le": "<=",
        "gt": ">",
        "ge": ">=",
        "and": "and",
        "or": "or",
        "implies": "=>",
    }
    if kind in binary:
        if len(operands) != 2:
            raise UnsupportedVerifiedCore(f"{path}.operands: {kind} requires two operands")
        left = _expression(
            operands[0], local_symbols, type_sorts, result_symbol, f"{path}.operands[0]"
        )
        right = _expression(
            operands[1], local_symbols, type_sorts, result_symbol, f"{path}.operands[1]"
        )
        return f"({binary[kind]} {left} {right})"

    raise UnsupportedVerifiedCore(f"{path}.kind: unsupported proof expression: {kind}")


def _single_return(
    function: dict[str, Any],
    local_symbols: dict[int, str],
    type_sorts: dict[int, str],
    path: str,
) -> str:
    body = function["body"]
    blocks = body["blocks"]
    if len(blocks) != 1:
        raise UnsupportedVerifiedCore(f"{path}.body.blocks: only one-block bodies are supported")
    block = blocks[0]
    if block["id"] != body["entry_block"]:
        raise UnsupportedVerifiedCore(f"{path}.body.entry_block: entry block must be the only block")
    if block["parameters"]:
        raise UnsupportedVerifiedCore(f"{path}.body.blocks[0].parameters: block parameters unsupported")
    if block["instructions"]:
        raise UnsupportedVerifiedCore(f"{path}.body.blocks[0].instructions: instructions unsupported")
    terminator = block["terminator"]
    if terminator["kind"] != "return" or "value" not in terminator:
        raise UnsupportedVerifiedCore(f"{path}.body.blocks[0].terminator: value return required")
    return _typed_value(
        terminator["value"],
        local_symbols,
        type_sorts,
        f"{path}.body.blocks[0].terminator.value",
    )


def _function_vcs(
    function: dict[str, Any],
    type_sorts: dict[int, str],
    function_index: int,
) -> list[str]:
    path = f"$.functions[{function_index}]"
    prefix = _symbol(f"f{function['id']}_{function['name']}")
    local_symbols: dict[int, str] = {}
    declarations: list[str] = []
    for local in function["locals"]:
        local_id = local["id"]
        symbol = _symbol(f"{prefix}_local_{local_id}_{local['name']}")
        local_symbols[local_id] = symbol
        declarations.append(f"(declare-const {symbol} {type_sorts[local['type_id']]})")

    return_type_id = function["signature"]["return_type_id"]
    result_symbol = _symbol(f"{prefix}_result")
    declarations.append(f"(declare-const {result_symbol} {type_sorts[return_type_id]})")
    returned = _single_return(function, local_symbols, type_sorts, path)

    requires: list[str] = []
    ensures: list[tuple[int, str]] = []
    for contract_index, contract in enumerate(function["contracts"]):
        contract_path = f"{path}.contracts[{contract_index}]"
        rendered = _expression(
            contract["expression"],
            local_symbols,
            type_sorts,
            result_symbol,
            f"{contract_path}.expression",
        )
        if contract["kind"] == "requires":
            requires.append(rendered)
        elif contract["kind"] == "ensures":
            ensures.append((contract_index, rendered))
        else:
            raise UnsupportedVerifiedCore(
                f"{contract_path}.kind: only requires/ensures are supported"
            )
    if not ensures:
        raise UnsupportedVerifiedCore(f"{path}.contracts: at least one ensures contract is required")

    output: list[str] = declarations
    for contract_index, ensured in ensures:
        output.append(f"; obligation {prefix}.ensures[{contract_index}]")
        output.append("(push 1)")
        output.append(f"(assert (= {result_symbol} {returned}))")
        for required in requires:
            output.append(f"(assert {required})")
        output.append(f"(assert (not {ensured}))")
        output.append("(check-sat)")
        output.append("(pop 1)")
    return output


def generate_smtlib(value: Any) -> str:
    document = validate_verified_core(value)
    profile = document["target_profile"]
    if profile["integer_model"] != "mathematical":
        raise UnsupportedVerifiedCore("$.target_profile.integer_model: only mathematical is supported")
    if profile["overflow"] != "checked":
        raise UnsupportedVerifiedCore("$.target_profile.overflow: only checked is supported")
    if profile["floating_point"] != "unsupported":
        raise UnsupportedVerifiedCore("$.target_profile.floating_point: floats must be unsupported")

    type_entries = {entry["id"]: entry for entry in document["types"]}
    type_sorts = {
        type_id: _sort(entry, f"$.types[{type_id}]")
        for type_id, entry in type_entries.items()
        if entry["kind"] != "unit"
    }

    lines = ["(set-logic QF_LIA)", "; generated from arukellt-verified-core v1"]
    obligations = 0
    for function_index, function in enumerate(document["functions"]):
        function_lines = _function_vcs(function, type_sorts, function_index)
        obligations += sum(1 for line in function_lines if line == "(check-sat)")
        lines.extend(function_lines)
    if obligations == 0:
        raise UnsupportedVerifiedCore("$.functions: no proof obligations generated")
    lines.append("(exit)")
    return "\n".join(lines) + "\n"


def generate_smtlib_file(subject_path: Path, output_path: Path) -> int:
    rendered = generate_smtlib(load_json(subject_path))
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(rendered, encoding="utf-8")
    return rendered.count("(check-sat)")


__all__ = ["UnsupportedVerifiedCore", "generate_smtlib", "generate_smtlib_file"]
