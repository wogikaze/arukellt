"""Compatibility facade for semantic VerifiedCore admission."""

from __future__ import annotations

import copy
from typing import Any

from proof import verified_core_typed_impl as _impl
from proof.verified_core import validate_document

SCHEMA = _impl.SCHEMA
VERSION = _impl.VERSION
TypedVerifiedCoreError = _impl.TypedVerifiedCoreError


def _expression_nodes(expression: dict[str, Any]) -> list[dict[str, Any]]:
    nodes = [expression]
    for operand in expression.get("operands", []):
        nodes.extend(_expression_nodes(operand))
    return nodes


def _prepare_contract_namespaces(value: Any) -> Any:
    prepared = copy.deepcopy(value)
    for function in prepared.get("functions", []):
        ids = [
            int(node["id"])
            for contract in function.get("contracts", [])
            for node in _expression_nodes(contract["expression"])
        ]
        offset = max(ids, default=-1) + 1
        for contract in function.get("contracts", []):
            if contract.get("kind") != "requires":
                continue
            for node in _expression_nodes(contract["expression"]):
                node["id"] = int(node["id"]) + offset
    return prepared


def _precheck_legacy_diagnostics(value: Any) -> None:
    if not isinstance(value, dict):
        return
    for function_index, function in enumerate(value.get("functions", [])):
        path = f"$.functions[{function_index}]"
        return_type = int(function.get("signature", {}).get("return_type_id", 0))
        locals_by_name = {str(local.get("name")): local for local in function.get("locals", [])}
        parameter_names = {str(parameter.get("name")) for parameter in function.get("signature", {}).get("parameters", [])}
        for parameter in function.get("signature", {}).get("parameters", []):
            name = str(parameter.get("name"))
            local = locals_by_name.get(name)
            if local is not None and local.get("storage") != "parameter":
                raise TypedVerifiedCoreError(
                    f"{path}.locals: parameter {name!r} mismatch: signature parameter is not storage=parameter"
                )
        for local_index, local in enumerate(function.get("locals", [])):
            if local.get("storage") == "parameter" and str(local.get("name")) not in parameter_names:
                raise TypedVerifiedCoreError(
                    f"{path}.locals[{local_index}]: parameter local is absent from signature"
                )
        for contract_index, contract in enumerate(function.get("contracts", [])):
            allow_result = contract.get("kind") == "ensures"
            stack = [contract.get("expression")]
            while stack:
                expression = stack.pop()
                if not isinstance(expression, dict):
                    continue
                if expression.get("kind") == "result":
                    if not allow_result:
                        raise TypedVerifiedCoreError(
                            f"{path}.contracts[{contract_index}].expression: result is only valid in ensures contracts"
                        )
                    if int(expression.get("type_id", -1)) != return_type:
                        raise TypedVerifiedCoreError(
                            f"{path}.contracts[{contract_index}].expression.type_id: result type does not match function return type"
                        )
                stack.extend(expression.get("operands", []))


def _compat_message(message: str) -> str:
    replacements = {
        "arithmetic type mismatch": "arithmetic type mismatch: arithmetic must preserve operand TypeId",
        "logical type mismatch": "logical operands must be bool",
        "contract must be bool": "contract must have type bool",
        "undeclared parameter local": "parameter local is absent from signature",
    }
    for old, new in replacements.items():
        if old in message:
            return message.replace(old, new)
    return message


def validate_typed_document(value: Any) -> dict[str, Any]:
    _precheck_legacy_diagnostics(value)
    prepared = _prepare_contract_namespaces(value)
    try:
        _impl.validate_typed_document(prepared)
    except TypedVerifiedCoreError as exc:
        raise TypedVerifiedCoreError(_compat_message(str(exc))) from exc
    return validate_document(value)


__all__ = ["SCHEMA", "VERSION", "TypedVerifiedCoreError", "validate_typed_document"]
