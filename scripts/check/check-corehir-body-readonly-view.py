#!/usr/bin/env python3
"""Check that sealed CoreHIR body-table vectors do not escape their module."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COREHIR = ROOT / "src" / "compiler" / "corehir"
TABLE = COREHIR / "body_table.ark"
BODY = COREHIR / "body.ark"
SOURCE = COREHIR / "mir_body_source.ark"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise ValueError(f"missing {label}: {needle}")


def reject(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise ValueError(f"forbidden {label}: {needle}")


def main() -> int:
    table = TABLE.read_text(encoding="utf-8")
    body = BODY.read_text(encoding="utf-8")
    source = SOURCE.read_text(encoding="utf-8")

    for forbidden in (
        "fn corehir_body_table_exprs(",
        "fn corehir_body_table_fn_body_roots(",
        "fn corehir_body_table_method_body_roots(",
    ):
        reject(table, forbidden, "raw table-vector export")

    for forbidden in (
        "fn body_exprs(",
        "fn body_fn_roots(",
        "fn body_method_roots(",
    ):
        reject(body, forbidden, "raw body-vector wrapper")

    require(table, "fn corehir_body_table_expr_count", "expression count accessor")
    require(table, "fn corehir_body_table_expr_at", "expression element accessor")
    require(body, "fn corehir_body_expr_count", "body expression count wrapper")
    require(body, "fn corehir_body_expr_at", "body expression element wrapper")

    require(source, "let exprs = Vec::new<CoreHirExpr>()", "detached expression snapshot")
    require(source, "body::corehir_body_expr_count(table)", "expression count snapshot loop")
    require(source, "body::corehir_body_expr_at(table, expr_index)", "expression element snapshot")
    require(source, "let fn_body_roots = Vec::new<i32>()", "detached function-root snapshot")
    require(source, "let method_body_roots = Vec::new<i32>()", "detached method-root snapshot")

    raw_export_names = (
        "corehir_body_table_exprs(",
        "corehir_body_table_fn_body_roots(",
        "corehir_body_table_method_body_roots(",
    )
    violations: list[str] = []
    for path in ROOT.joinpath("src", "compiler").rglob("*.ark"):
        text = path.read_text(encoding="utf-8")
        for name in raw_export_names:
            if name in text:
                violations.append(f"{path.relative_to(ROOT)}: {name}")
    if violations:
        raise ValueError("raw table storage escape remains: " + "; ".join(violations))

    print("corehir-body-readonly-view: PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"corehir-body-readonly-view: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
