#!/usr/bin/env python3
"""Check the CoreHIR body-table construction, seal, and admission boundary."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COREHIR = ROOT / "src" / "compiler" / "corehir"
DRIVER = ROOT / "src" / "compiler" / "driver"
TABLE = COREHIR / "body_table.ark"
BODY = COREHIR / "body.ark"
ROOTS = COREHIR / "body_roots.ark"
CHECKED_PROGRAM = DRIVER / "checked_program.ark"
PIPELINE = DRIVER / "pipeline_backend.ark"
ERRORS = DRIVER / "errors_phase.ark"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise ValueError(f"missing {label}: {needle}")


def function_body(text: str, signature: str) -> str:
    start = text.index(signature)
    next_private = text.find("\nfn ", start + 1)
    next_public = text.find("\npub fn ", start + 1)
    candidates = [index for index in (next_private, next_public) if index >= 0]
    end = min(candidates) if candidates else len(text)
    return text[start:end]


def main() -> int:
    table = TABLE.read_text(encoding="utf-8")
    body = BODY.read_text(encoding="utf-8")
    roots = ROOTS.read_text(encoding="utf-8")
    checked_program = CHECKED_PROGRAM.read_text(encoding="utf-8")
    pipeline = PIPELINE.read_text(encoding="utf-8")
    errors = ERRORS.read_text(encoding="utf-8")

    require(table, "sealed: bool", "sealed state")
    require(table, "mutation_violation: bool", "mutation violation state")
    require(
        table,
        "fn corehir_body_table_reject_mutation_if_sealed",
        "shared mutation guard",
    )
    for mutation in (
        "fn corehir_body_table_push_expr",
        "fn corehir_body_table_push_fn_body_root",
        "fn corehir_body_table_push_method_body_root",
    ):
        require(
            function_body(table, mutation),
            "corehir_body_table_reject_mutation_if_sealed(table)",
            f"guard in {mutation}",
        )

    require(table, "fn corehir_body_table_seal", "seal operation")
    require(table, "fn corehir_body_table_is_sealed", "seal query")
    require(
        table,
        "fn corehir_body_table_has_mutation_violation",
        "violation query",
    )
    require(body, "fn corehir_seal_body_table", "public construction wrapper")

    seal_at = roots.index("body::corehir_seal_body_table(table)")
    return_at = roots.index("    table\n}", seal_at)
    if seal_at >= return_at:
        raise ValueError("body table is not sealed before construction returns")

    admission = function_body(
        checked_program,
        "fn checked_program_corehir_body_table_is_admissible",
    )
    require(admission, "corehir_body_table_is_sealed(table)", "sealed admission check")
    require(
        admission,
        "!body::corehir_body_table_has_mutation_violation(table)",
        "mutation-free admission check",
    )

    pipeline_gate = function_body(pipeline, "fn run_backend")
    gate_call = "checked_program::checked_program_corehir_body_table_is_admissible(program)"
    require(pipeline_gate, gate_call, "pipeline admission call")
    require(
        pipeline_gate,
        "return errors_phase::corehir_body_table_admission_failed_result()",
        "fail-closed pipeline return",
    )
    if pipeline_gate.index(gate_call) >= pipeline_gate.index("lower::lower_checked_program"):
        raise ValueError("CoreHIR admission must run before MIR lowering")
    require(
        errors,
        "pub fn corehir_body_table_admission_failed_result",
        "CoreHIR admission diagnostic",
    )

    direct_mutation_names = (
        "corehir_body_table_push_expr(",
        "corehir_body_table_push_fn_body_root(",
        "corehir_body_table_push_method_body_root(",
    )
    allowed = {TABLE.resolve(), BODY.resolve()}
    violations: list[str] = []
    for path in COREHIR.glob("*.ark"):
        if path.resolve() in allowed:
            continue
        text = path.read_text(encoding="utf-8")
        for name in direct_mutation_names:
            if name in text:
                violations.append(f"{path.relative_to(ROOT)}: {name}")
    if violations:
        raise ValueError("direct body-table mutation bypasses wrapper: " + "; ".join(violations))

    print("corehir-body-table-seal: PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"corehir-body-table-seal: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
