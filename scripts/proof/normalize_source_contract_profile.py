"""Normalize source-emitted VerifiedCore to the mathematical proof profile."""

from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

from proof.verified_core import validate_document


class UnsupportedSourceContractProfile(ValueError):
    pass


_ARITHMETIC_KINDS = {"add", "sub", "mul", "div", "mod", "neg"}


def _reject_contract_arithmetic(expression: dict[str, Any], path: str) -> None:
    if expression["kind"] in _ARITHMETIC_KINDS:
        raise UnsupportedSourceContractProfile(
            f"{path}.kind: machine arithmetic cannot be normalized: {expression['kind']}"
        )
    for index, operand in enumerate(expression.get("operands", [])):
        _reject_contract_arithmetic(operand, f"{path}.operands[{index}]")


def _reject_body_arithmetic(document: dict[str, Any]) -> None:
    for function_index, function in enumerate(document["functions"]):
        for block_index, block in enumerate(function["body"]["blocks"]):
            for instruction_index, instruction in enumerate(block["instructions"]):
                path = f"$.functions[{function_index}].body.blocks[{block_index}].instructions[{instruction_index}]"
                if instruction["op"] == "unary" and instruction.get("operator") == "neg":
                    raise UnsupportedSourceContractProfile(
                        f"{path}.operator: machine arithmetic cannot be normalized: neg"
                    )
                if instruction["op"] == "binary" and instruction.get("operator") in {"add", "sub", "mul", "div", "mod"}:
                    raise UnsupportedSourceContractProfile(
                        f"{path}.operator: machine arithmetic cannot be normalized: {instruction['operator']}"
                    )


def normalize_document(value: Any) -> dict[str, Any]:
    document = validate_document(value)
    profile = document["target_profile"]
    if profile["floating_point"] not in {"unsupported", "ieee754"}:
        raise UnsupportedSourceContractProfile(
            "$.target_profile.floating_point: unknown source floating-point profile"
        )
    for function_index, function in enumerate(document["functions"]):
        for contract_index, contract in enumerate(function["contracts"]):
            _reject_contract_arithmetic(
                contract["expression"],
                f"$.functions[{function_index}].contracts[{contract_index}].expression",
            )
    _reject_body_arithmetic(document)
    normalized = copy.deepcopy(document)
    normalized["generator"] = str(document.get("generator", "unknown")) + "+proof-profile-normalizer-v2"
    normalized["target_profile"] = {
        "integer_model": "mathematical",
        "overflow": "checked",
        "floating_point": "unsupported",
        "pointer_width": profile["pointer_width"],
    }
    return validate_document(normalized)


def normalize_file(input_path: Path, output_path: Path) -> None:
    value = json.loads(input_path.read_text(encoding="utf-8"))
    normalized = normalize_document(value)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(normalized, indent=2, sort_keys=True) + "\n", encoding="utf-8")


__all__ = ["UnsupportedSourceContractProfile", "normalize_document", "normalize_file"]
