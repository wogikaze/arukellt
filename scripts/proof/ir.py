"""Validation for Arukellt Proof IR v1.

This module intentionally depends only on the Python standard library so it can
run in bootstrap and repository-quality lanes before Why3 or an SMT solver is
installed. It validates the compiler/verifier boundary; it does not prove the
verification conditions themselves.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, NoReturn

SCHEMA = "arukellt-proof-ir"
SCHEMA_VERSION = 1
CONTRACT_KINDS = {"requires", "ensures", "invariant", "assert", "decreases"}


@dataclass(frozen=True)
class ValidationError(ValueError):
    path: str
    message: str

    def __str__(self) -> str:
        return f"{self.path}: {self.message}"


def _fail(path: str, message: str) -> NoReturn:
    raise ValidationError(path, message)


def _object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        _fail(path, "expected object")
    return value


def _array(value: Any, path: str) -> list[Any]:
    if not isinstance(value, list):
        _fail(path, "expected array")
    return value


def _string(value: Any, path: str, *, nonempty: bool = True) -> str:
    if not isinstance(value, str):
        _fail(path, "expected string")
    if nonempty and not value:
        _fail(path, "must not be empty")
    return value


def _integer(value: Any, path: str, *, minimum: int | None = None) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        _fail(path, "expected integer")
    if minimum is not None and value < minimum:
        _fail(path, f"must be >= {minimum}")
    return value


def _keys(obj: dict[str, Any], path: str, *, required: set[str], optional: set[str]) -> None:
    missing = required - obj.keys()
    if missing:
        _fail(path, f"missing field(s): {', '.join(sorted(missing))}")
    unknown = obj.keys() - required - optional
    if unknown:
        _fail(path, f"unknown field(s): {', '.join(sorted(unknown))}")


def _validate_span(value: Any, path: str) -> None:
    span = _object(value, path)
    _keys(span, path, required={"file", "start", "end"}, optional=set())
    _string(span["file"], f"{path}.file")
    start = _integer(span["start"], f"{path}.start", minimum=0)
    end = _integer(span["end"], f"{path}.end", minimum=0)
    if end < start:
        _fail(path, "end must be >= start")


def _validate_contract(value: Any, path: str) -> None:
    contract = _object(value, path)
    _keys(
        contract,
        path,
        required={"kind", "expression", "span"},
        optional={"label", "result_name"},
    )
    kind = _string(contract["kind"], f"{path}.kind")
    if kind not in CONTRACT_KINDS:
        _fail(f"{path}.kind", f"unsupported contract kind: {kind}")
    _string(contract["expression"], f"{path}.expression")
    _validate_span(contract["span"], f"{path}.span")
    if "label" in contract:
        _string(contract["label"], f"{path}.label")
    if "result_name" in contract:
        _string(contract["result_name"], f"{path}.result_name")
    if kind == "ensures" and "result_name" not in contract:
        _fail(path, "ensures contract requires result_name")
    if kind != "ensures" and "result_name" in contract:
        _fail(path, "result_name is only valid for ensures")


def _validate_parameter(value: Any, path: str) -> None:
    param = _object(value, path)
    _keys(param, path, required={"name", "type"}, optional=set())
    _string(param["name"], f"{path}.name")
    _string(param["type"], f"{path}.type")


def _validate_function(value: Any, path: str) -> None:
    function = _object(value, path)
    _keys(
        function,
        path,
        required={"id", "name", "parameters", "return_type", "contracts", "body"},
        optional={"type_parameters", "span"},
    )
    _integer(function["id"], f"{path}.id", minimum=0)
    _string(function["name"], f"{path}.name")
    for index, parameter in enumerate(_array(function["parameters"], f"{path}.parameters")):
        _validate_parameter(parameter, f"{path}.parameters[{index}]")
    _string(function["return_type"], f"{path}.return_type")
    for index, contract in enumerate(_array(function["contracts"], f"{path}.contracts")):
        _validate_contract(contract, f"{path}.contracts[{index}]")
    _object(function["body"], f"{path}.body")
    if "type_parameters" in function:
        for index, type_parameter in enumerate(
            _array(function["type_parameters"], f"{path}.type_parameters")
        ):
            _string(type_parameter, f"{path}.type_parameters[{index}]")
    if "span" in function:
        _validate_span(function["span"], f"{path}.span")


def validate_document(value: Any) -> dict[str, Any]:
    """Validate and return a Proof IR v1 document.

    Raises ValidationError at the first malformed boundary. Semantic proof
    obligations are deliberately left to the verifier backend.
    """

    document = _object(value, "$")
    _keys(
        document,
        "$",
        required={"schema", "schema_version", "module", "functions"},
        optional={"generator"},
    )
    if _string(document["schema"], "$.schema") != SCHEMA:
        _fail("$.schema", f"expected {SCHEMA!r}")
    if _integer(document["schema_version"], "$.schema_version") != SCHEMA_VERSION:
        _fail("$.schema_version", f"expected {SCHEMA_VERSION}")
    _string(document["module"], "$.module")
    if "generator" in document:
        _string(document["generator"], "$.generator")

    ids: set[int] = set()
    names: set[str] = set()
    functions = _array(document["functions"], "$.functions")
    for index, function_value in enumerate(functions):
        path = f"$.functions[{index}]"
        _validate_function(function_value, path)
        function = _object(function_value, path)
        function_id = function["id"]
        function_name = function["name"]
        if function_id in ids:
            _fail(f"{path}.id", f"duplicate function id: {function_id}")
        if function_name in names:
            _fail(f"{path}.name", f"duplicate function name: {function_name}")
        ids.add(function_id)
        names.add(function_name)
    return document
