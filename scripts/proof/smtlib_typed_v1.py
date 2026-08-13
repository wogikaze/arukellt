"""Typed VerifiedCore v1 to SMT-LIB adapter.

Semantic admission runs before rendering. Modular calls also record the exact
callee interface digest in the generated solver input so receipts bind the
contract interface that was actually consumed by VC generation.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from proof.common import load_json
from proof.smtlib_v1 import UnsupportedVerifiedCore, generate_smtlib
from proof.verified_core_typed import validate_typed_document


class UnsupportedTypedVerifiedCore(UnsupportedVerifiedCore):
    """The subject failed typed admission before SMT generation."""


def _interface_comments(document: dict[str, Any]) -> list[str]:
    comments: list[str] = []
    for function in document["functions"]:
        for block in function["body"]["blocks"]:
            for instruction in block["instructions"]:
                if instruction.get("op") != "call":
                    continue
                comments.append(
                    "; callee-interface-sha256 "
                    f"caller={function['id']} callee={instruction['callee_id']} "
                    f"sha256={instruction['callee_interface_sha256']}"
                )
    return comments


def generate_typed_smtlib(value: Any) -> str:
    try:
        document = validate_typed_document(value)
    except (ValueError, TypeError, KeyError) as exc:
        raise UnsupportedTypedVerifiedCore(str(exc)) from exc
    rendered = generate_smtlib(document)
    comments = _interface_comments(document)
    if not comments:
        return rendered
    return "\n".join(comments) + "\n" + rendered


def generate_typed_smtlib_file(subject_path: Path, output_path: Path) -> int:
    rendered = generate_typed_smtlib(load_json(subject_path))
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(rendered, encoding="utf-8")
    return rendered.count("(check-sat)")


__all__ = [
    "UnsupportedTypedVerifiedCore",
    "generate_typed_smtlib",
    "generate_typed_smtlib_file",
]
