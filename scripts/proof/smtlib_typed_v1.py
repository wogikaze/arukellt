"""Typed VerifiedCore v1 to SMT-LIB adapter.

This is the public proof adapter. It performs semantic typed admission before
calling the syntax-oriented SMT renderer, so malformed TypeIds or operators
cannot reach the solver through another caller.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from proof.common import load_json
from proof.smtlib_v1 import UnsupportedVerifiedCore, generate_smtlib
from proof.verified_core_typed import validate_typed_document


class UnsupportedTypedVerifiedCore(UnsupportedVerifiedCore):
    """The subject failed typed admission before SMT generation."""


def generate_typed_smtlib(value: Any) -> str:
    try:
        document = validate_typed_document(value)
    except (ValueError, TypeError, KeyError) as exc:
        raise UnsupportedTypedVerifiedCore(str(exc)) from exc
    return generate_smtlib(document)


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
