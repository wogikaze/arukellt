#!/usr/bin/env python3
"""Check the CoreHIR body-table construction/seal boundary."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COREHIR = ROOT / "src" / "compiler" / "corehir"
TABLE = COREHIR / "body_table.ark"
BODY = COREHIR / "body.ark"
ROOTS = COREHIR / "body_roots.ark"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise ValueError(f"missing {label}: {needle}")


def main() -> int:
    table = TABLE.read_text(encoding="utf-8")
    body = BODY.read_text(encoding="utf-8")
    roots = ROOTS.read_text(encoding="utf-8")

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
        start = table.index(mutation)
        end = table.find("\nfn ", start + 1)
        if end < 0:
            end = len(table)
        require(
            table[start:end],
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
