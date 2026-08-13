"""Normalize comparison-only source contracts to the mathematical SMT profile."""

from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

from proof.verified_core import validate_document


class UnsupportedSourceContractProfile(ValueError):
    """The compiler artifact cannot be soundly normalized to mathematical integers."""


_ARITHMETIC_KINDS = {"add", "sub", "mul", "div", "mod", "neg"}


def _reject_arithmetic(expression: dict[str, Any], path: str) -> None:
    kind = expression["kind"]
    if kind in _ARITHMETIC_KINDS:
        raise UnsupportedSourceContractProfile(
            f"{path}.kind: machine arithmetic cannot be normalized: {kind}"
        )
    for index, operand in enumerate(expression.get("operands", [])):
        _reject_arithmetic(operand, f"{path}.operands[{index}]")


def normalize_document(value: Any) -> dict[str, Any]:
    document = validate_document(value)
    profile = document["target_profile"]
    if profile["floating_point"] not in {"unsupported", "ieee754"}:
        raise UnsupportedSourceContractProfile(
            "$.target_profile.floating_point: unknown source floating-point profile"
        )

    for function_index, function in enumerate(document["functions"]):
        for contract_index, contract in enumerate(function["contracts"]):
            _reject_arithmetic(
                contract["expression"],
                f"$.functions[{function_index}].contracts[{contract_index}].expression",
            )

    normalized = copy.deepcopy(document)
    normalized["generator"] = (
        str(document["generator"]) + "+comparison-profile-normalizer-v1"
    )
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
    output_path.write_text(
        json.dumps(normalized, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


__all__ = [
    "UnsupportedSourceContractProfile",
    "normalize_document",
    "normalize_file",
]
