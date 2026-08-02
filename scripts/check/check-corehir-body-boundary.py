#!/usr/bin/env python3
"""Enforce a versioned builder-to-frozen CoreHIR body boundary."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMPILER = ROOT / "src" / "compiler"
BODY_TABLE = COMPILER / "corehir" / "body_table.ark"
BODY_BUILDER = COMPILER / "corehir" / "body_builder.ark"
BODY_VALIDATOR = COMPILER / "corehir" / "body_validator.ark"
MIR_BODY_SOURCE = COMPILER / "corehir" / "mir_body_source.ark"
ALLOWED_BUILDER_IMPORTS = {
    "src/compiler/corehir/body.ark",
    "src/compiler/corehir/body_roots.ark",
}
STORAGE_FIELDS = (
    "exprs",
    "fn_body_roots",
    "method_body_roots",
    "fn_contract_starts",
    "fn_contract_counts",
    "contract_roots",
    "contract_kinds",
    "contract_result_names",
)
FORBIDDEN_READ_API_TOKENS = (
    "CoreHirBodyTable_new",
    "corehir_body_table_push_",
    "corehir_body_table_begin_",
    "corehir_body_table_end_",
    "push(table.",
    "fn corehir_body_table_exprs",
    "fn corehir_body_table_fn_body_roots",
    "fn corehir_body_table_method_body_roots",
)
FORBIDDEN_VECTOR_FACADE_TOKENS = (
    "fn body_exprs(",
    "fn body_fn_roots(",
    "fn body_method_roots(",
    "body::body_exprs(",
    "body::body_fn_roots(",
    "body::body_method_roots(",
)


def relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise ValueError(f"missing {label}: {needle}")


def main() -> int:
    table = BODY_TABLE.read_text(encoding="utf-8")
    builder = BODY_BUILDER.read_text(encoding="utf-8")
    validator = BODY_VALIDATOR.read_text(encoding="utf-8")
    mir_body_source = MIR_BODY_SOURCE.read_text(encoding="utf-8")

    require(table, "struct CoreHirBodyTable", "frozen body artifact record")
    require(table, "schema_version: i32", "body artifact schema version field")
    require(
        table,
        "fn corehir_body_table_schema_version",
        "body artifact schema version accessor",
    )
    require(table, "fn corehir_body_table_expr_count", "expression count accessor")
    require(table, "fn corehir_body_table_expr_at", "expression indexed accessor")
    for token in FORBIDDEN_READ_API_TOKENS:
        if token in table:
            raise ValueError(f"body_table exposes mutation or storage alias: {token}")

    require(builder, "fn CoreHirBodyBuilder_new", "construction capability constructor")
    require(builder, "schema_version: 1", "body artifact v1 construction")
    require(builder, "fn corehir_body_builder_finish", "builder-to-artifact handoff")
    require(
        builder,
        "body_validator::corehir_body_artifact_valid(builder)",
        "independent validator invocation",
    )
    require(builder, "process::exit(1)", "fail-closed invalid artifact action")

    require(
        validator,
        "fn corehir_body_artifact_valid",
        "independent body artifact validator",
    )
    require(
        validator,
        "corehir_body_table_schema_version(table) != 1",
        "body artifact version validation",
    )
    require(
        validator,
        "expected_start != flat_contract_count",
        "contract range coverage validation",
    )
    require(
        validator,
        "root < 0 || root >= expr_count",
        "contract root validation",
    )

    require(
        mir_body_source,
        "fn mir_body_source_copy_expr",
        "detached expression copy",
    )
    require(
        mir_body_source,
        "let exprs = Vec::new<CoreHirExpr>()",
        "detached expression vector",
    )
    require(
        mir_body_source,
        "let children = Vec::new<i32>()",
        "detached child vector",
    )
    require(
        mir_body_source,
        "corehir_body_expr_count(table)",
        "count-based body snapshot",
    )
    require(
        mir_body_source,
        "corehir_body_expr_at(table, expr_index)",
        "indexed body snapshot",
    )

    violations: list[str] = []
    for path in sorted(COMPILER.rglob("*.ark")):
        rel = relative(path)
        text = path.read_text(encoding="utf-8")
        if "use corehir::body_builder" in text and rel not in ALLOWED_BUILDER_IMPORTS:
            violations.append(f"{rel}: imports construction-only body_builder")
        if rel != "src/compiler/corehir/body_builder.ark":
            for field in STORAGE_FIELDS:
                for prefix in ("push(table.", "push(builder.", "set(table.", "set(builder."):
                    if f"{prefix}{field}" in text:
                        violations.append(
                            f"{rel}: directly mutates CoreHIR body storage field {field}"
                        )
        for token in FORBIDDEN_VECTOR_FACADE_TOKENS:
            if token in text:
                violations.append(f"{rel}: exposes or consumes body storage vector: {token}")

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
    print(
        "corehir-body-boundary: PASS: "
        "version=1 builder -> validator -> frozen artifact -> detached MIR snapshot"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"corehir-body-boundary: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
