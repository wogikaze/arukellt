"""Typed VerifiedCore v1 to SMT-LIB adapter through Phase 4."""
from __future__ import annotations
from pathlib import Path
from typing import Any
from proof.common import load_json
from proof.proof_phase4_vc import UnsupportedVerifiedCore, generate_smtlib
from proof.verified_core_typed import validate_typed_document

class UnsupportedTypedVerifiedCore(UnsupportedVerifiedCore):
    """The subject failed typed admission before SMT generation."""

def _interface_comments(document: dict[str, Any]) -> list[str]:
    comments = []
    for function in document["functions"]:
        for block in function["body"]["blocks"]:
            for instruction in block["instructions"]:
                if instruction.get("op") == "call":
                    comments.append(f"; callee-interface-sha256 caller={function['id']} callee={instruction['callee_id']} sha256={instruction['callee_interface_sha256']}")
    return comments

def generate_typed_smtlib(value: Any) -> str:
    try:
        document = validate_typed_document(value)
    except (ValueError, TypeError, KeyError) as exc:
        raise UnsupportedTypedVerifiedCore(str(exc)) from exc
    rendered = generate_smtlib(document)
    comments = _interface_comments(document)
    return ("\n".join(comments) + "\n" if comments else "") + rendered

def generate_typed_smtlib_file(subject_path: Path, output_path: Path) -> int:
    rendered = generate_typed_smtlib(load_json(subject_path))
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(rendered, encoding="utf-8")
    return rendered.count("(check-sat)")

__all__ = ["UnsupportedTypedVerifiedCore", "generate_typed_smtlib", "generate_typed_smtlib_file"]
