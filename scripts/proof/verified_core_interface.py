"""Deterministic binding for modular-call proof interfaces."""

from __future__ import annotations

import hashlib
import json
from contextvars import ContextVar
from typing import Any

_UNBOUND_CALLS = ContextVar("arukellt_allow_unbound_call_interfaces", default=False)


def call_interfaces_may_be_unbound() -> bool:
    return bool(_UNBOUND_CALLS.get())


class _UnboundCallInterfaceScope:
    def __enter__(self):
        self._token = _UNBOUND_CALLS.set(True)
        return self

    def __exit__(self, exc_type, exc, traceback):
        _UNBOUND_CALLS.reset(self._token)
        return False


def allow_unbound_call_interfaces() -> _UnboundCallInterfaceScope:
    return _UnboundCallInterfaceScope()


def interface_payload(function: dict[str, Any]) -> dict[str, Any]:
    return {"signature": function["signature"], "abi": function["abi"], "contracts": function["contracts"]}


def interface_sha256(function: dict[str, Any]) -> str:
    encoded = json.dumps(interface_payload(function), sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def bind_call_interfaces(document: dict[str, Any]) -> dict[str, Any]:
    functions = {int(function["id"]): function for function in document["functions"]}
    digests = {function_id: interface_sha256(function) for function_id, function in functions.items()}
    for function in document["functions"]:
        for block in function["body"]["blocks"]:
            for instruction in block["instructions"]:
                if instruction.get("op") != "call": continue
                callee_id = int(instruction["callee_id"])
                if callee_id not in digests: raise ValueError(f"unknown callee id {callee_id}")
                instruction["callee_interface_sha256"] = digests[callee_id]
    return document


def validate_call_interface_binding(instruction: dict[str, Any], callee: dict[str, Any], path: str) -> None:
    actual = instruction.get("callee_interface_sha256")
    if actual is None and call_interfaces_may_be_unbound(): return
    if not isinstance(actual, str) or len(actual) != 64 or any(ch not in "0123456789abcdef" for ch in actual): raise ValueError(f"{path}.callee_interface_sha256: expected lowercase sha256")
    if actual != interface_sha256(callee): raise ValueError(f"{path}.callee_interface_sha256: callee interface digest mismatch")

__all__ = ["allow_unbound_call_interfaces", "bind_call_interfaces", "call_interfaces_may_be_unbound", "interface_payload", "interface_sha256", "validate_call_interface_binding"]
