"""Compatibility facade for semantic VerifiedCore admission."""

from __future__ import annotations

from typing import Any

from proof import verified_core_typed_impl as _impl

SCHEMA = _impl.SCHEMA
VERSION = _impl.VERSION
TypedVerifiedCoreError = _impl.TypedVerifiedCoreError


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
    try:
        return _impl.validate_typed_document(value)
    except TypedVerifiedCoreError as exc:
        raise TypedVerifiedCoreError(_compat_message(str(exc))) from exc


__all__ = ["SCHEMA", "VERSION", "TypedVerifiedCoreError", "validate_typed_document"]
