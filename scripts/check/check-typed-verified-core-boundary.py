#!/usr/bin/env python3
"""Reject untyped or name-inferred TypedCoreHIR to VerifiedCore paths."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CLI = ROOT / "scripts" / "gen" / "convert-typed-corehir.py"
SMT_CLI = ROOT / "scripts" / "gen" / "write-smt-vcs.py"
BOUNDARY = ROOT / "scripts" / "proof" / "typed_corehir_typed_convert.py"
SEMANTICS = ROOT / "scripts" / "proof" / "verified_core_typed.py"
TYPED_SMT = ROOT / "scripts" / "proof" / "smtlib_typed_v1.py"
WORKFLOW = ROOT / ".github" / "workflows" / "typed-corehir-proof-pipeline.yml"


def require(text: str, token: str, label: str) -> None:
    if token not in text:
        raise ValueError(f"missing {label}: {token}")


def main() -> int:
    cli = CLI.read_text(encoding="utf-8")
    smt_cli = SMT_CLI.read_text(encoding="utf-8")
    boundary = BOUNDARY.read_text(encoding="utf-8")
    semantics = SEMANTICS.read_text(encoding="utf-8")
    typed_smt = TYPED_SMT.read_text(encoding="utf-8")
    workflow = WORKFLOW.read_text(encoding="utf-8")

    require(cli, "from proof.typed_corehir_typed_convert import", "v2 CLI import")
    require(cli, "convert_typed_document", "v2 CLI call")
    if "from proof.typed_corehir_convert import" in cli:
        raise ValueError("public converter CLI bypasses explicit typed v2 boundary")

    for token, label in (
        ('"bits"', "explicit integer bit width"),
        ('"signed"', "explicit integer signedness"),
        ("legacy[\"name\"] = expected", "metadata-derived legacy normalization"),
        ("rendered[\"name\"] = explicit[\"name\"]", "source identity preservation"),
        ("return validate_typed_document(converted)", "semantic admission"),
    ):
        require(boundary, token, label)

    forbidden_name_inference = (
        'explicit["name"] == "i32"',
        'explicit["name"] == "i64"',
        'raw.get("name") == "i32"',
        'raw.get("name") == "i64"',
        'startswith(explicit["name"]',
    )
    for token in forbidden_name_inference:
        if token in boundary:
            raise ValueError(f"source type-name inference reintroduced: {token}")

    for token, label in (
        ("contract must have type bool", "contract root typing"),
        ("result type does not match function return type", "result typing"),
        ("binary operands must have identical TypeId", "binary TypeId equality"),
        ("arithmetic must preserve operand TypeId", "arithmetic preservation"),
        ("signature parameter", "parameter/local correspondence"),
        ("integer constant requires an integer value", "constant payload typing"),
    ):
        require(semantics, token, label)

    require(
        smt_cli,
        "from proof.smtlib_typed_v1 import",
        "typed SMT CLI import",
    )
    if "from proof.smtlib_v1 import" in smt_cli:
        raise ValueError("SMT CLI bypasses typed VerifiedCore admission")
    for token, label in (
        ("validate_typed_document(value)", "semantic admission before SMT"),
        ("return generate_smtlib(document)", "post-admission SMT rendering"),
    ):
        require(typed_smt, token, label)

    for token, label in (
        ("scripts.tests.test_verified_core_typed", "semantic negative tests"),
        ("scripts.tests.test_smtlib_typed_v1", "typed SMT negative tests"),
        ("verified_core_typed.py", "semantic validator TrustManifest component"),
        ("typed_corehir_typed_convert.py", "v2 converter TrustManifest component"),
        ("smtlib_typed_v1.py", "typed SMT TrustManifest component"),
        ("check-typed-verified-core.py", "standalone typed admission"),
        ("write-typed-verified-core-boundary-receipt.py", "versioned boundary receipt"),
        ("check-typed-verified-core-boundary.py", "source gate execution"),
    ):
        require(workflow, token, label)

    print(
        "typed-verified-core-boundary: PASS: "
        "explicit logical types and semantic admission enforced through SMT"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"typed-verified-core-boundary: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
