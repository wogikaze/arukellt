#!/usr/bin/env python3
"""Enforce construction-only mutation for the CoreHIR body artifact."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMPILER = ROOT / "src" / "compiler"
BODY_TABLE = COMPILER / "corehir" / "body_table.ark"
BODY_BUILDER = COMPILER / "corehir" / "body_builder.ark"
ALLOWED_BUILDER_IMPORTS = {
    "src/compiler/corehir/body.ark",
    "src/compiler/corehir/body_roots.ark",
}
MUTABLE_FIELDS = (
    ".exprs",
    ".fn_body_roots",
    ".method_body_roots",
    ".fn_contract_starts",
    ".fn_contract_counts",
    ".contract_roots",
    ".contract_kinds",
    ".contract_result_names",
)
FORBIDDEN_READ_API_TOKENS = (
    "CoreHirBodyTable_new",
    "corehir_body_table_push_",
    "corehir_body_table_begin_",
    "corehir_body_table_end_",
    "push(table.",
)


def relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def main() -> int:
    table = BODY_TABLE.read_text(encoding="utf-8")
    builder = BODY_BUILDER.read_text(encoding="utf-8")

    if "struct CoreHirBodyTable" not in table:
        raise ValueError("frozen CoreHirBodyTable record is missing")
    for token in FORBIDDEN_READ_API_TOKENS:
        if token in table:
            raise ValueError(f"body_table exposes mutation capability: {token}")
    if "fn CoreHirBodyBuilder_new" not in builder:
        raise ValueError("construction capability constructor is missing")
    if "fn corehir_body_builder_finish" not in builder:
        raise ValueError("builder-to-artifact handoff is missing")

    violations: list[str] = []
    for path in sorted(COMPILER.rglob("*.ark")):
        rel = relative(path)
        text = path.read_text(encoding="utf-8")
        if "use corehir::body_builder" in text and rel not in ALLOWED_BUILDER_IMPORTS:
            violations.append(f"{rel}: imports construction-only body_builder")
        if rel not in {
            "src/compiler/corehir/body_builder.ark",
            "src/compiler/corehir/body_table.ark",
        }:
            for field in MUTABLE_FIELDS:
                if field in text and "push(" in text:
                    violations.append(
                        f"{rel}: directly mutates CoreHIR body storage field {field}"
                    )

    for prefix in ("src/compiler/driver/", "src/compiler/mir/", "src/compiler/wasm/"):
        for path in sorted((ROOT / prefix).rglob("*.ark")):
            text = path.read_text(encoding="utf-8")
            for token in (
                "body_builder::",
                "corehir_body_builder_",
                "corehir_push_expr(",
                "corehir_push_fn_body_root(",
                "corehir_push_method_body_root(",
            ):
                if token in text:
                    violations.append(
                        f"{relative(path)}: downstream layer uses mutation token {token}"
                    )

    if violations:
        raise ValueError("\n".join(violations))
    print("corehir-body-boundary: PASS: builder -> frozen read-only artifact")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"corehir-body-boundary: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
