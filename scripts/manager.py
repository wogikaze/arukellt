#!/usr/bin/env python3
"""Arukellt scripts manager.

Usage:
    manager.py <domain> <subcommand> [options]

Domains:
    verify

Subcommands for verify:
    quick       Run the fast local gate checks (default behavior of verify-harness.sh).
    fixtures    Run the manifest-driven fixture harness.
    size        Run the hello.wasm binary size gate.
    wat         Run the WAT roundtrip gate.
    component   Run the component interop smoke test.

Global flags:
    --dry-run   Print intent but do not execute commands.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import os
import re
import subprocess
import sys
from pathlib import Path

# Ensure scripts/ is on sys.path so we can import lib and verify packages.
_SCRIPTS_DIR = Path(__file__).resolve().parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from lib.files import repo_root as _repo_root  # noqa: E402
from verify.fixtures import (  # noqa: E402
    count_fixtures,
    disk_fixture_paths,
    load_manifest,
)
from verify.harness import GREEN, NC, RED, YELLOW, Harness  # noqa: E402
from selfhost.checks import (  # noqa: E402
    SelfhostFixpointResult,
    run_diag_parity,
    run_fixpoint,
    run_fixture_parity,
    run_parity,
)
from docs_domain.checks import (  # noqa: E402
    run_consistency,
    run_examples,
    run_freshness,
    run_regenerate,
)
from perf.checks import (  # noqa: E402
    run_baseline,
    run_benchmarks,
    run_gate as run_perf_gate,
)
from gate_domain.checks import (  # noqa: E402
    run_local,
    run_pre_commit,
    run_pre_push,
    run_repro,
)

# ── helpers ──────────────────────────────────────────────────────────────────


def _run(cmd: list[str], *, cwd: Path, dry_run: bool) -> tuple[int, str, str]:
    if dry_run:
        print(f"DRY-RUN: {cmd}")
        return (0, "", "")
    result = subprocess.run(cmd, cwd=str(cwd), capture_output=True, text=True)
    return (result.returncode, result.stdout, result.stderr)


def _run_env(
    cmd: list[str], *, cwd: Path, dry_run: bool, env: dict[str, str]
) -> tuple[int, str, str]:
    if dry_run:
        print(f"DRY-RUN: {cmd}")
        return (0, "", "")
    result = subprocess.run(cmd, cwd=str(cwd), capture_output=True, text=True, env=env)
    return (result.returncode, result.stdout, result.stderr)


def _corehir_mir_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return frontend/typecheck node leaks across the CoreHIR -> MIR boundary."""
    patterns = [
        "src/compiler/corehir_raw*.ark",
        "src/compiler/corehir_mir*.ark",
        "src/compiler/mir_lower.ark",
        "src/compiler/mir_lower_entry.ark",
        "src/compiler/mir_lower_entry_decls.ark",
        "src/compiler/mir_lower_entry_fns.ark",
        "src/compiler/mir_lower_entry_methods.ark",
        "src/compiler/mir_lower_entry_emit*.ark",
        "src/compiler/mir_lower_entry_body_input*.ark",
        "src/compiler/mir_lower_entry_body_fallback.ark",
        "src/compiler/mir_lower_entry_fallback_body*.ark",
    ]
    forbidden = ("AstNode", "TypeCheckResult")
    violations: list[tuple[str, int, str]] = []
    for pattern in patterns:
        for path in sorted(root.glob(pattern)):
            if not path.is_file():
                continue
            for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if any(token in line for token in forbidden):
                    rel = str(path.relative_to(root))
                    violations.append((rel, line_no, line.strip()))
    return violations


def _corehir_mir_view_contract_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return raw CoreHIR builder-table leaks in the MIR-facing view contract."""
    path = root / "src" / "compiler" / "corehir_mir_view.ark"
    forbidden = ("CoreHirBodyTable", "body_table", "mir_view_body_table")
    violations: list[tuple[str, int, str]] = []
    if not path.is_file():
        return violations
    in_view_struct = False
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.strip()
        if stripped == "struct CoreHirMirView {":
            in_view_struct = True
        if in_view_struct and any(token in stripped for token in forbidden):
            violations.append((str(path.relative_to(root)), line_no, stripped))
        if in_view_struct and stripped == "}":
            in_view_struct = False
    return violations


def _corehir_param_shape_facade_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return parameter shape helper leaks in the CoreHIR parameter facade."""
    path = root / "src" / "compiler" / "corehir_param_shape.ark"
    forbidden = (
        "use corehir_decl_annotation_shape",
        "use corehir_frontend_ast_node",
        "use corehir_frontend_expr_kind",
        "use corehir_value_types",
        "fn param_is_ident",
        "fn param_has_type_ann",
        "fn param_type_ann",
        "fn param_named_local_type_name",
        "fn fn_param_value_type",
        "fn method_param_value_type",
        "fn fn_param_local_type_name",
        "fn method_param_local_type_name",
        "fn param_name",
        "fn param_is_self_name",
    )
    violations: list[tuple[str, int, str]] = []
    if not path.is_file():
        return violations
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.strip()
        if any(stripped.startswith(token) for token in forbidden):
            violations.append((str(path.relative_to(root)), line_no, stripped))
    return violations


def _corehir_mir_signature_facade_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return fn/method projection leaks in the MIR signature source constructor facade."""
    path = root / "src" / "compiler" / "corehir_mir_signature_source.ark"
    forbidden = (
        "use corehir_mir_signature_fn",
        "use corehir_mir_signature_method",
        "mir_signature_fn_",
        "mir_signature_method_",
    )
    violations: list[tuple[str, int, str]] = []
    if not path.is_file():
        return violations
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.strip()
        if any(token in stripped for token in forbidden):
            violations.append((str(path.relative_to(root)), line_no, stripped))
    return violations


def _mir_entry_view_projection_facade_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return MIR entry view projection logic leaking back into the constructor facade."""
    facade = root / "src" / "compiler" / "mir_lower_entry_view.ark"
    facade_forbidden = (
        "use corehir_mir_body_source",
        "use corehir_mir_signature_fn",
        "use corehir_mir_signature_method",
        "use corehir_mir_type_source",
        "use mir_lower_return_types",
        "fn mir_entry_fn_",
        "pub fn mir_entry_fn_",
        "fn mir_entry_method_",
        "pub fn mir_entry_method_",
        "fn mir_entry_exprs",
        "pub fn mir_entry_exprs",
        "fn mir_entry_typed_return_tag_at",
        "pub fn mir_entry_typed_return_tag_at",
    )
    violations: list[tuple[str, int, str]] = []
    if facade.is_file():
        for line_no, line in enumerate(facade.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(stripped.startswith(token) for token in facade_forbidden):
                violations.append((str(facade.relative_to(root)), line_no, stripped))
    for path in sorted((root / "src" / "compiler").glob("mir_lower_entry*.ark")):
        if not path.is_file() or path.name == "mir_lower_entry_view.ark":
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if "mir_lower_entry_view::mir_entry_" in stripped and "mir_entry_view_from_core" not in stripped:
                violations.append((rel, line_no, stripped))
    return violations


def _corehir_layout_decl_facade_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return layout name/field/variant/payload projection leaks in the decl facade."""
    path = root / "src" / "compiler" / "corehir_layout_decl.ark"
    forbidden = (
        "fn layout_decl_name",
        "pub fn layout_decl_name",
        "fn layout_decl_field_",
        "pub fn layout_decl_field_",
        "fn layout_decl_variant_",
        "pub fn layout_decl_variant_",
        "fn layout_decl_has_payload",
        "pub fn layout_decl_has_payload",
    )
    violations: list[tuple[str, int, str]] = []
    if not path.is_file():
        return violations
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.strip()
        if any(stripped.startswith(token) for token in forbidden):
            violations.append((str(path.relative_to(root)), line_no, stripped))
    return violations


def _driver_module_state_facade_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return driver module state roles leaking into the constructor facade."""
    checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/driver_module_state.ark",
            (
                "LoadState_error",
                "LoadState_errors",
                "LoadState_modules",
                "LoadState_has_",
                "LoadState_push_",
                "LoadState_pop_",
                "LoadState_add_module",
                "ModuleDecls_decl",
                "vec_has",
            ),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, forbidden in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    for path in sorted((root / "src" / "compiler").glob("driver*.ark")):
        if not path.is_file() or path.name in ("driver_module_state.ark", "driver_modules.ark", "driver_load.ark"):
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if "driver_module_state::" in stripped:
                violations.append((rel, line_no, stripped))
    return violations


def _driver_module_source_lookup_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return parser/state registration leaks in driver module source lookup files."""
    lookup_files = (
        "src/compiler/driver_module_local_source.ark",
        "src/compiler/driver_module_stdlib_source.ark",
        "src/compiler/driver_module_file_load_local.ark",
        "src/compiler/driver_module_file_load_stdlib.ark",
    )
    forbidden = (
        "use driver_module_parse",
        "driver_module_parse::",
        "use compiler_session_parse",
        "compiler_session_parse::",
        "use parser",
        "use driver_module_state_",
        "LoadState_",
        "load_local_module_decls",
        "load_stdlib_module_decls",
    )
    violations: list[tuple[str, int, str]] = []
    for rel in lookup_files:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    graph = root / "src" / "compiler" / "driver_module_graph.ark"
    graph_forbidden = (
        "use driver_module_local_source",
        "driver_module_local_source::",
        "use driver_module_stdlib_source",
        "driver_module_stdlib_source::",
    )
    if graph.is_file():
        for line_no, line in enumerate(graph.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in graph_forbidden):
                violations.append((str(graph.relative_to(root)), line_no, stripped))
    return violations


def _driver_module_graph_relative_import_violations(root: Path) -> list[str]:
    """Return module graph loader patterns that would lose nested relative imports."""
    graph = root / "src" / "compiler" / "driver" / "module_graph.ark"
    if not graph.is_file():
        graph = root / "src" / "compiler" / "driver_module_graph.ark"
    if not graph.is_file():
        return ["driver module graph source not found"]

    text = graph.read_text(encoding="utf-8")
    required = (
        "fn load_imported_modules_at(",
        "current_dir: String",
        "root_dir: String",
        "module_base_dir_for_import(",
        'contains(use_path, String_from("::"))',
        "module_paths::parent_dir(module_local_decls::DriverLocalModuleDecls_path(loaded))",
        "load_imported_modules_at(module_local_decls::DriverLocalModuleDecls_decls(loaded), next_dir, root_dir, state)",
    )
    violations = [f"missing `{needle}`" for needle in required if needle not in text]
    stale_patterns = (
        "load_imported_modules(module_local_decls::DriverLocalModuleDecls_decls(loaded), clone(base_dir), state)",
        "load_imported_modules(module_local_decls::DriverLocalModuleDecls_decls(loaded), base_dir, state)",
    )
    violations.extend(f"stale recursive base_dir pattern `{needle}`" for needle in stale_patterns if needle in text)
    return violations


def _parser_expectation_diagnostic_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return parse diagnostic construction leaking into token expectation control."""
    checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser_expect.ark",
            (
                "use diagnostics_",
                "Diagnostic_new",
                "DIAG_PARSE_",
                "parser_push_error",
                "push(p.errors",
            ),
        ),
        (
            "src/compiler/parser_core_api_expect.ark",
            (
                "use diagnostics_",
                "Diagnostic_new",
                "DIAG_PARSE_",
                "parser_push_error",
                "push(p.errors",
            ),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, forbidden in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _parser_ast_factory_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return literal construction leaking into the generic AST factory."""
    path = root / "src" / "compiler" / "parser_core_ast_factory.ark"
    forbidden = (
        "use parser_kind_expr_atoms",
        "AstNode_int",
        "AstNode_float",
        "NK_INT_LIT",
        "NK_FLOAT_LIT",
    )
    violations: list[tuple[str, int, str]] = []
    if not path.is_file():
        return violations
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.strip()
        if any(token in stripped for token in forbidden):
            violations.append((str(path.relative_to(root)), line_no, stripped))
    return violations


def _parser_type_dispatch_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return unqualified parser helper leaks in the type dispatch facade."""
    path = root / "src" / "compiler" / "parser_types.ark"
    forbidden = (
        "skip_newlines(",
        "cur(",
        "error(",
        "tok_span(",
        "AstNode_leaf(",
    )
    violations: list[tuple[str, int, str]] = []
    if not path.is_file():
        return violations
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.strip()
        if any(stripped.startswith(token) for token in forbidden):
                    violations.append((str(path.relative_to(root)), line_no, stripped))
    return violations


def _parser_type_recursion_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return type parser recursion leaking out of the dispatch owner."""
    checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser_types.ark",
            (
                "use parser_type_fn",
                "use parser_type_tuple",
                "parser_type_fn::",
                "parser_type_tuple::",
            ),
        ),
        (
            "src/compiler/parser_type_named.ark",
            (
                "use parser_types",
                "parser_types::parse_type",
                "parse_type(",
                "parse_generic_type_args",
                "consume_closing_gt(",
            ),
        ),
    ]
    ambient_checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser_type_fn.ark",
            (
                "AstNode_new(",
                "tok_span(",
                "expect_token(",
                "at(",
                "at_eof(",
                "skip_newlines(",
                "peek_kind(",
                "parse_type(",
            ),
        ),
        (
            "src/compiler/parser_type_tuple.ark",
            (
                "AstNode_new(",
                "tok_span(",
                "expect_token(",
                "at(",
                "at_eof(",
                "skip_newlines(",
                "peek_kind(",
                "parse_type(",
            ),
        ),
        (
            "src/compiler/parser_type_params.ark",
            (
                "AstNode_leaf(",
                "tok_span(",
                "expect_token(",
                "at(",
                "at_eof(",
                "skip_newlines(",
                "peek_kind(",
                "consume_closing_gt(",
            ),
        ),
        (
            "src/compiler/parser_type_where.ark",
            (
                "skip_newlines(",
                "peek_kind(",
            ),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, forbidden in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//"):
                continue
            if any(stripped.startswith(token) or token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    for rel, forbidden in ambient_checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//"):
                continue
            if any(stripped.startswith(token) for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _parser_decl_facade_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return declaration parsing details leaking into struct/enum facades."""
    checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser_decl_struct.ark",
            (
                "use lexer_",
                "use parser_core_api_cursor",
                "use parser_core_api_expect",
                "use parser_core_ast_",
                "use parser_kind_",
                "use parser_token_state",
                "expect_token(",
                "skip_newlines(",
                "AstNode_",
                "Token_",
                "TK_",
                "NK_",
            ),
        ),
        (
            "src/compiler/parser_decl_enum.ark",
            (
                "use lexer_",
                "use parser_core_api_cursor",
                "use parser_core_api_expect",
                "use parser_core_ast_",
                "use parser_kind_",
                "use parser_token_state",
                "expect_token(",
                "skip_newlines(",
                "AstNode_",
                "Token_",
                "TK_",
                "NK_",
            ),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, forbidden in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//"):
                continue
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _parser_decl_dispatch_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return ambient parser helper calls leaking into declaration dispatch modules."""
    checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser_decl_dispatch.ark",
            (
                "peek_kind(",
                "skip_newlines(",
                "expect_token(",
                "parse_expr(",
                "parse_block(",
            ),
        ),
        (
            "src/compiler/parser_decl_module.ark",
            (
                "peek_kind(",
                "skip_newlines(",
                "at_eof(",
                "expect_token(",
            ),
        ),
        (
            "src/compiler/parser_decl_fns.ark",
            (
                "peek_kind(",
                "skip_newlines(",
                "at(",
                "at_eof(",
                "expect_token(",
                "parse_expr(",
                "parse_block(",
            ),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, forbidden in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//"):
                continue
            if any(stripped.startswith(token) for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _parser_fn_signature_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return ambient parser helper calls leaking into function signature modules."""
    checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser_fn_sig_decl.ark",
            (
                "expect_token(",
                "skip_newlines(",
                "AstNode_new(",
                "tok_span(",
            ),
        ),
        (
            "src/compiler/parser_fn_sig_clause.ark",
            (
                "expect_token(",
                "skip_newlines(",
                "AstNode_new(",
                "tok_span(",
            ),
        ),
        (
            "src/compiler/parser_fn_params_regular.ark",
            (
                "expect_token(",
                "skip_newlines(",
                "peek_kind(",
                "at(",
                "at_eof(",
                "AstNode_leaf(",
                "tok_span(",
            ),
        ),
        (
            "src/compiler/parser_fn_params_clause.ark",
            (
                "expect_token(",
                "skip_newlines(",
                "at(",
                "at_eof(",
            ),
        ),
        (
            "src/compiler/parser_fn_return.ark",
            (
                "peek_kind(",
                "skip_newlines(",
                "AstNode_new(",
            ),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, forbidden in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//"):
                continue
            if any(stripped.startswith(token) for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _parser_import_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return ambient parser helper calls leaking into import/use parsing modules."""
    checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser_imports_use.ark",
            (
                "expect_token(",
                "skip_newlines(",
                "peek_kind(",
                "AstNode_new(",
                "tok_span(",
            ),
        ),
        (
            "src/compiler/parser_imports_import.ark",
            (
                "expect_token(",
                "skip_newlines(",
                "AstNode_new(",
                "tok_span(",
            ),
        ),
        (
            "src/compiler/parser_imports_path.ark",
            ("expect_token(",),
        ),
        (
            "src/compiler/parser_imports_alias.ark",
            (
                "expect_token(",
                "skip_newlines(",
                "peek_kind(",
                "AstNode_leaf(",
                "tok_span(",
            ),
        ),
        (
            "src/compiler/parser_imports_group.ark",
            (
                "expect_token(",
                "skip_newlines(",
                "peek_kind(",
                "AstNode_new(",
                "tok_span(",
            ),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, forbidden in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//"):
                continue
            if any(stripped.startswith(token) for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _parser_stmt_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return statement facade entrypoints or let parsing helpers leaking ambient APIs."""
    required_public: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser/stmt.ark",
            (
                "pub fn parse_expr_or_block(",
                "pub fn parse_block(",
                "pub fn parse_stmt(",
            ),
        ),
        (
            "src/compiler/parser/stmt_block.ark",
            (
                "pub fn parse_expr_or_block(",
                "pub fn parse_block(",
            ),
        ),
        (
            "src/compiler/parser/stmt_control.ark",
            (
                "pub fn parse_return(",
                "pub fn parse_break(",
                "pub fn parse_continue(",
                "pub fn parse_while(",
                "pub fn parse_loop(",
                "pub fn parse_for(",
            ),
        ),
        (
            "src/compiler/parser/stmt_let.ark",
            ("pub fn parse_let(",),
        ),
        (
            "src/compiler/parser/stmt_let_type.ark",
            ("pub fn parse_let_type_ann(",),
        ),
    ]
    ambient_checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser/stmt_let.ark",
            (
                "expect_token(",
                "AstNode_new(",
                "tok_span(",
                "skip_newlines(",
                "peek_kind(",
            ),
        ),
        (
            "src/compiler/parser/stmt_let_type.ark",
            (
                "skip_newlines(",
                "peek_kind(",
            ),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, required in required_public:
        path = root / rel
        if not path.is_file():
            violations.append((rel, 0, "missing file"))
            continue
        lines = [line.strip() for line in path.read_text(encoding="utf-8").splitlines()]
        for token in required:
            if not any(line.startswith(token) for line in lines):
                violations.append((rel, 0, f"missing {token}"))
    for rel, forbidden in ambient_checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//"):
                continue
            if any(stripped.startswith(token) for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _parser_pratt_facade_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return Pratt expression facade entrypoints that lost their public contracts."""
    required_public: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser/expr_infix_result.ark",
            (
                "pub fn ParserInfixResult_no(",
                "pub fn ParserInfixResult_yes(",
                "pub fn ParserInfixResult_matched(",
                "pub fn ParserInfixResult_node(",
            ),
        ),
        (
            "src/compiler/parser/pratt.ark",
            (
                "pub fn is_expr_stop_token(",
                "pub fn should_parse_assignment(",
                "pub fn should_parse_range(",
                "pub fn prefix_bp(",
                "pub fn infix_bp_left(",
                "pub fn infix_bp_right(",
                "pub fn token_to_binop(",
                "pub fn token_to_unop(",
            ),
        ),
        (
            "src/compiler/parser/expr_bp_suffix.ark",
            ("pub fn parse_bp_suffix_step(",),
        ),
        (
            "src/compiler/parser/expr_suffix.ark",
            (
                "pub fn parse_dot_suffix(",
                "pub fn parse_index_suffix(",
                "pub fn parse_try_suffix(",
                "pub fn parse_call_suffix(",
                "pub fn parse_path_suffix(",
            ),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, required in required_public:
        path = root / rel
        if not path.is_file():
            violations.append((rel, 0, "missing file"))
            continue
        lines = [line.strip() for line in path.read_text(encoding="utf-8").splitlines()]
        for token in required:
            if not any(line.startswith(token) for line in lines):
                violations.append((rel, 0, f"missing {token}"))
    return violations


def _parser_prefix_facade_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return prefix expression facade entrypoints that lost their public contracts."""
    required_public: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser/expr_prefix_dispatch.ark",
            ("pub fn parse_prefix(",),
        ),
        (
            "src/compiler/parser/expr_prefix.ark",
            (
                "pub fn parse_ident_or_struct_lit(",
                "pub fn parse_tuple_or_group(",
                "pub fn parse_array_prefix(",
            ),
        ),
        (
            "src/compiler/parser/expr_prefix_atoms.ark",
            (
                "pub fn is_atom_prefix(",
                "pub fn parse_atom_prefix(",
            ),
        ),
        (
            "src/compiler/parser/expr_prefix_controls.ark",
            (
                "pub fn is_control_prefix(",
                "pub fn parse_control_prefix(",
            ),
        ),
        (
            "src/compiler/parser/expr_prefix_closures.ark",
            (
                "pub fn is_closure_prefix(",
                "pub fn parse_closure_prefix(",
            ),
        ),
        (
            "src/compiler/parser/expr_prefix_unary.ark",
            ("pub fn parse_unary_prefix(",),
        ),
        (
            "src/compiler/parser/expr_prefix_struct.ark",
            ("pub fn parse_ident_or_struct_lit(",),
        ),
        (
            "src/compiler/parser/expr_prefix_struct_probe.ark",
            ("pub fn next_prefix_is_struct_lit(",),
        ),
        (
            "src/compiler/parser/expr_prefix_struct_fields.ark",
            ("pub fn parse_struct_lit_fields(",),
        ),
        (
            "src/compiler/parser/expr_prefix_struct_named.ark",
            ("pub fn parse_struct_lit_named_field(",),
        ),
        (
            "src/compiler/parser/expr_prefix_struct_base.ark",
            ("pub fn parse_struct_lit_base_spread(",),
        ),
        (
            "src/compiler/parser/expr_prefix_tuple.ark",
            ("pub fn parse_tuple_or_group(",),
        ),
        (
            "src/compiler/parser/expr_prefix_array.ark",
            ("pub fn parse_array_prefix(",),
        ),
        (
            "src/compiler/parser/expr_prefix_array_tail.ark",
            ("pub fn parse_array_lit_tail(",),
        ),
        (
            "src/compiler/parser/expr_prefix_array_repeat.ark",
            ("pub fn parse_array_repeat_tail(",),
        ),
        (
            "src/compiler/parser/expr_prefix_array_comprehension.ark",
            ("pub fn parse_array_comprehension(",),
        ),
    ]
    ambient_checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/parser/expr_prefix_dispatch.ark",
            (
                "skip_newlines(",
                "cur(",
                "error(",
                "AstNode_leaf(",
                "tok_span(",
            ),
        ),
        (
            "src/compiler/parser/expr_prefix_controls.ark",
            ("cur(",),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, required in required_public:
        path = root / rel
        if not path.is_file():
            violations.append((rel, 0, "missing file"))
            continue
        lines = [line.strip() for line in path.read_text(encoding="utf-8").splitlines()]
        for token in required:
            if not any(line.startswith(token) for line in lines):
                violations.append((rel, 0, f"missing {token}"))
    for rel, forbidden in ambient_checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//"):
                continue
            if any(stripped.startswith(token) for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _corehir_frontend_accessor_facade_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return parser/type annotation internals leaking into CoreHIR accessor facades."""
    checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/corehir_frontend_ast_node.ark",
            ("use parser_ast", "parser_ast::"),
        ),
        (
            "src/compiler/corehir_expr_node_access.ark",
            ("use corehir_frontend_ast_node", "corehir_frontend_ast_node::"),
        ),
        (
            "src/compiler/corehir_decl_annotation_shape.ark",
            ("use corehir_decl_type_annotations", "use corehir_type_ann_node", "corehir_decl_type_annotations::", "corehir_type_ann_node::"),
        ),
        (
            "src/compiler/corehir_type_ann_node.ark",
            ("use corehir_frontend_ast_node", "corehir_frontend_ast_node::"),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, forbidden in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_lower_import_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return direct parser/typechecker imports outside MIR legacy AST adapters."""
    allowed_paths = {
        "src/compiler/mir/lower/ast_node.ark",
    }
    allowed_prefixes = (
        "src/compiler/mir_lower_ast_node_",
    )
    forbidden_imports = (
        "use parser",
        "use parser_",
        "use typechecker",
        "use typechecker_",
    )
    violations: list[tuple[str, int, str]] = []
    for path in sorted((root / "src" / "compiler").rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        if not (rel.startswith("src/compiler/mir/lower/") or Path(rel).name.startswith("mir_lower")):
            continue
        if rel in allowed_paths or rel.startswith(allowed_prefixes):
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(stripped.startswith(prefix) for prefix in forbidden_imports):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_lower_ast_adapter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split MIR lowering AST adapter files/imports."""
    compiler_root = root / "src" / "compiler"
    violations: list[tuple[str, int, str]] = []
    for path in sorted((compiler_root / "mir" / "lower").glob("ast_node_*.ark")):
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split MIR AST adapter file"))
    old_import = re.compile(r"^use\s+mir::lower::ast_node_(kind|type_ann|identity|literal|children)\s*$")
    old_call = re.compile(r"\bast_node_(kind|type_ann|identity|literal|children)::")
    for path in sorted((compiler_root / "mir" / "lower").glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_core_lowering_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return frontend or legacy body leaks inside CoreHIR-based MIR lowering."""
    patterns = (
        "mir_lower_core*.ark",
        "mir_lower_entry_body_core_source.ark",
        "mir_lower_entry_core_body*.ark",
    )
    forbidden = (
        "AstNode",
        "TypeCheckResult",
        "use parser",
        "use typechecker",
        "mir_lower_body",
        "MirLowerLegacyBody",
        "mir_lower_legacy_body",
    )
    violations: list[tuple[str, int, str]] = []
    compiler_root = root / "src" / "compiler"
    for pattern in patterns:
        for path in sorted(compiler_root.glob(pattern)):
            if not path.is_file():
                continue
            rel = str(path.relative_to(root))
            for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                stripped = line.strip()
                if any(token in stripped for token in forbidden):
                    violations.append((rel, line_no, stripped))
    return violations


def _compiler_table_like_file(path: Path) -> bool:
    """Return true for cohesive constant/opcode/tag tables.

    These files intentionally group numeric compiler vocabulary. Splitting them
    too aggressively hurts reviewability more than it helps local reasoning.
    """
    name = path.name
    parts = set(path.parts)
    table_tokens = (
        "constants",
        "kind",
        "kinds",
        "opcode",
        "opcodes",
        "tag",
        "tags",
        "tokens",
    )
    if any(token in name for token in table_tokens):
        return True
    if "lexer" in parts and name.startswith("kind_"):
        return True
    return False


def _compiler_root_layout_violations(root: Path) -> list[str]:
    """Return root-level compiler files that should live behind namespaces.

    The compiler root should be a small command/facade surface. Role-specific
    implementation files belong under namespaces such as parser/, mir/, wasm/,
    and component/ so humans can navigate by subsystem instead of prefix.
    """
    allowed = {
        "analysis.ark",
        "ark.toml",
        "component_emit.ark",
        "component_emitter.ark",
        "corehir.ark",
        "dap.ark",
        "diagnostics.ark",
        "driver.ark",
        "emit_wat.ark",
        "emitter.ark",
        "hir.ark",
        "lsp.ark",
        "main.ark",
        "mir_dump.ark",
        "mir_lower.ark",
        "parser.ark",
        "resolver.ark",
        "typechecker.ark",
    }
    compiler_root = root / "src" / "compiler"
    return [
        str(path.relative_to(root))
        for path in sorted(compiler_root.glob("*.ark"))
        if path.name not in allowed
    ]


def _compiler_namespace_layout_violations(root: Path) -> list[str]:
    """Return missing or empty compiler namespace directories."""
    required = (
        "compiler",
        "component",
        "corehir",
        "diagnostics",
        "driver",
        "hir",
        "lexer",
        "mir",
        "mir/lower",
        "parser",
        "resolver",
        "typechecker",
        "wasm",
        "wasm/intrinsics",
    )
    compiler_root = root / "src" / "compiler"
    violations: list[str] = []
    for rel in required:
        directory = compiler_root / rel
        if not directory.is_dir():
            violations.append(f"missing directory: src/compiler/{rel}/")
            continue
        if not any(directory.glob("*.ark")):
            violations.append(f"empty namespace: src/compiler/{rel}/")
    return violations


def _compiler_public_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return subsystem imports or pub APIs that bypass public mod.ark boundaries."""
    required_mods = (
        "src/compiler/component/mod.ark",
        "src/compiler/corehir/mod.ark",
        "src/compiler/wasm/mod.ark",
        "src/compiler/mir/mod.ark",
        "src/compiler/diagnostics/mod.ark",
        "src/compiler/parser/mod.ark",
        "src/compiler/resolver/mod.ark",
        "src/compiler/typechecker/mod.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for rel in required_mods:
        if not (root / rel).is_file():
            violations.append((rel, 1, "missing public boundary mod.ark"))

    public_mod_only_dirs = (
        "src/compiler/component",
        "src/compiler/wasm",
        "src/compiler/mir",
        "src/compiler/corehir",
        "src/compiler/resolver",
        "src/compiler/typechecker",
    )
    for rel_dir in public_mod_only_dirs:
        subsystem_root = root / rel_dir
        if not subsystem_root.is_dir():
            continue
        for path in sorted(subsystem_root.rglob("*.ark")):
            if path.name == "mod.ark":
                continue
            rel = str(path.relative_to(root))
            for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                stripped = line.strip()
                if stripped.startswith("pub fn "):
                    violations.append((rel, line_no, stripped))

    forbidden_imports = (
        "use component::emit",
        "use component::wit_text",
        "use component::contract",
        "use wasm::wasm",
        "use wasm::wat",
        "use mir::lower",
        "use mir::input",
        "use mir::fallback_source",
        "use mir::legacy_decl",
        "use mir::reachability",
        "use mir::dump_core",
        "use corehir::frontend_checked",
        "use corehir::mir_view",
        "use component_emit",
        "use component_emitter",
        "use emit_wat",
        "use emitter",
        "use mir_dump",
        "use mir_lower",
    )
    compiler_root = root / "src" / "compiler"
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        rel_from_compiler = str(path.relative_to(compiler_root))
        if rel_from_compiler.startswith(("component/", "corehir/", "wasm/", "mir/")):
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(stripped.startswith(prefix) for prefix in forbidden_imports):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_corehir_view_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return MIR modules that bypass the CoreHIR public view facade."""
    compiler_root = root / "src" / "compiler"
    violations: list[tuple[str, int, str]] = []
    for path in sorted((compiler_root / "mir").rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped == "use corehir::mir_view":
                violations.append((rel, line_no, stripped))
    return violations


def _mir_annotation_adapter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split MIR annotation wrapper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "ann_value_type.ark",
        "ann_local.ark",
        "ann_param.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split MIR annotation wrapper module"))

    old_import = re.compile(r"^use\s+mir::lower::ann_(local|param|value_type)\s*$")
    old_call = re.compile(r"\bann_(local|param|value_type)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_return_index_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split MIR return-index type-name helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "return_index_name_node.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split return-index type-name helper module"))

    old_import = re.compile(r"^use\s+mir::lower::return_index_name_node\s*$")
    old_call = re.compile(r"\breturn_index_name_node::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_stmt_let_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split MIR let finalization helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "stmt_let_annotation.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split let annotation helper module"))

    old_import = re.compile(r"^use\s+mir::lower::stmt_let_annotation\s*$")
    old_call = re.compile(r"\bstmt_let_annotation::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_literal_int_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split MIR integer literal classification helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "literal_int_kind.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split integer literal width helper module"))

    old_import = re.compile(r"^use\s+mir::lower::literal_int_kind\s*$")
    old_call = re.compile(r"\bliteral_int_kind::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_atomic_value_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split MIR atomic value helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "value_ident.ark",
        "value_string.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split atomic value helper module"))

    old_import = re.compile(r"^use\s+mir::lower::value_(ident|string)\s*$")
    old_call = re.compile(r"\bvalue_(ident|string)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_call_arg_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split MIR call-argument facade helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "call_arg_names.ark",
        "call_arg_stage.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split call argument facade helper module"))

    old_import = re.compile(r"^use\s+mir::lower::call_arg_(names|stage)\s*$")
    old_call = re.compile(r"\bcall_arg_(names|stage)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_core_call_arg_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split CoreHIR call-argument facade helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_names = (
        "core_call_arg_print.ark",
        "core_call_arg_stage.ark",
    )
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split CoreHIR call argument facade helper module"))

    old_import = re.compile(r"^use\s+mir::lower::core_call_arg_(print|stage)\s*$")
    old_call = re.compile(r"\bcore_call_arg_(print|stage)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_call_type_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split MIR call result type helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_names = (
        "call_type_string.ark",
        "call_type_names.ark",
    )
    for name in old_names:
        old_path = lower_root / name
        if old_path.is_file():
            violations.append((str(old_path.relative_to(root)), 1, "split call result type helper module"))

    old_import = re.compile(r"^use\s+mir::lower::call_type_(string|names)\s*$")
    old_call = re.compile(r"\bcall_type_(string|names)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_call_rewrite_type_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split MIR call rewrite record modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "call_rewrite_types.ark",
        "call_rewrite_string_types.ark",
        "call_rewrite_vec_types.ark",
        "call_rewrite_vec_push_types.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split call rewrite record module"))

    old_import = re.compile(r"^use\s+mir::lower::call_rewrite(_types|_string_types|_vec_types|_vec_push_types)\s*$")
    old_call = re.compile(r"\bcall_rewrite(_types|_string_types|_vec_types|_vec_push_types)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_loop_struct_iter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split struct-iterator loop type helper module/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "loop_struct_iter_types.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split struct-iterator loop type helper module"))

    old_import = re.compile(r"^use\s+mir::lower::loop_struct_iter_types\s*$")
    old_call = re.compile(r"\bloop_struct_iter_types::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_vec_call_rewrite_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split arity-only Vec call rewrite modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "call_rewrite_vec_basic.ark",
        "call_rewrite_vec_len.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split Vec call rewrite helper module"))

    old_import = re.compile(r"^use\s+mir::lower::call_rewrite_vec_(basic|len)\s*$")
    old_call = re.compile(r"\bcall_rewrite_vec_(basic|len)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_hof_step_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split HOF typed loop modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "hof_i32_step_advance.ark",
        "hof_i32_step_filter.ark",
        "hof_i32_step_fold.ark",
        "hof_i32_step_map.ark",
        "hof_i64_step_advance.ark",
        "hof_i64_step_filter.ark",
        "hof_i64_step_fold.ark",
        "hof_i64_step_map.ark",
        "hof_f64_step_advance.ark",
        "hof_f64_step_filter.ark",
        "hof_f64_step_map.ark",
        "hof_i32_steps.ark",
        "hof_i64_steps.ark",
        "hof_f64_steps.ark",
        "hof_i32_finish.ark",
        "hof_i64_finish.ark",
        "hof_f64_finish.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split HOF typed loop helper module"))

    old_import = re.compile(r"^use\s+mir::lower::hof_(i32|i64|f64)_(?:step_(advance|filter|fold|map)|steps|finish)\s*$")
    old_call = re.compile(r"\bhof_(i32|i64|f64)_(?:step_(advance|filter|fold|map)|steps|finish)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_hof_option_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split Option HOF phase modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "hof_option_alloc.ark",
        "hof_option_input.ark",
        "hof_option_tag.ark",
        "hof_option_some.ark",
        "hof_option_payload.ark",
        "hof_option_callback.ark",
        "hof_option_some_result.ark",
        "hof_option_none.ark",
        "hof_option_finish.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split Option HOF phase helper module"))

    old_import = re.compile(r"^use\s+mir::lower::hof_option_(alloc|input|tag|some|payload|callback|some_result|none|finish)\s*$")
    old_call = re.compile(r"\bhof_option_(alloc|input|tag|some|payload|callback|some_result|none|finish)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_hof_call_plan_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split HOF call-plan contract modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "hof_call_plan_record.ark",
        "hof_call_plan_factory.ark",
        "hof_call_plan_access.ark",
        "hof_call_plan_flags.ark",
        "hof_call_plan_query.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split HOF call-plan contract module"))

    old_import = re.compile(r"^use\s+mir::lower::hof_call_plan_(record|factory|access|flags|query)\s*$")
    old_call = re.compile(r"\bhof_call_plan_(record|factory|access|flags|query)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_hof_call_classifier_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split HOF call classifier modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "hof_call_i32.ark",
        "hof_call_i64.ark",
        "hof_call_f64.ark",
        "hof_call_option.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split HOF call classifier module"))

    old_import = re.compile(r"^use\s+mir::lower::hof_call_(i32|i64|f64|option)\s*$")
    old_call = re.compile(r"\bhof_call_(i32|i64|f64|option)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_hof_setup_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split HOF typed setup helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "hof_i32_setup.ark",
        "hof_i32_state.ark",
        "hof_i32_result.ark",
        "hof_i64_setup.ark",
        "hof_i64_state.ark",
        "hof_i64_result.ark",
        "hof_f64_setup.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split HOF typed setup helper module"))

    old_import = re.compile(r"^use\s+mir::lower::(?:hof_i(32|64)_(setup|state|result)|hof_f64_setup)\s*$")
    old_call = re.compile(r"\b(?:hof_i(32|64)_(setup|state|result)|hof_f64_setup)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_hof_frame_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split HOF typed frame wrapper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "hof_i32_frame.ark",
        "hof_i64_frame.ark",
        "hof_f64_frame.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split HOF typed frame wrapper module"))

    old_import = re.compile(r"^use\s+mir::lower::hof_(i32|i64|f64)_frame\s*$")
    old_call = re.compile(r"\bhof_(i32|i64|f64)_frame::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_hof_loop_step_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split shared HOF loop-step modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "hof_loop_advance.ark",
        "hof_loop_map.ark",
        "hof_loop_filter.ark",
        "hof_loop_fold.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split shared HOF loop-step module"))

    old_import = re.compile(r"^use\s+mir::lower::hof_loop_(advance|map|filter|fold)\s*$")
    old_call = re.compile(r"\bhof_loop_(advance|map|filter|fold)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_hof_result_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split shared HOF result modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "hof_result_emit.ark",
        "hof_result_vec.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split shared HOF result module"))

    old_import = re.compile(r"^use\s+mir::lower::hof_result_(emit|vec)\s*$")
    old_call = re.compile(r"\bhof_result_(emit|vec)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_hof_loop_scaffolding_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split shared HOF loop scaffolding modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "hof_loop_elem.ark",
        "hof_loop_frame.ark",
        "hof_loop_setup.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split shared HOF loop scaffolding module"))

    old_import = re.compile(r"^use\s+mir::lower::hof_loop_(elem|frame|setup)\s*$")
    old_call = re.compile(r"\bhof_loop_(elem|frame|setup)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_binary_opcode_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split binary opcode mapping modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "binary_opcode_arith.ark",
        "binary_opcode_compare.ark",
        "binary_opcode_logic.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split binary opcode mapping module"))

    old_import = re.compile(r"^use\s+mir::lower::binary_opcode_(arith|compare|logic)\s*$")
    old_call = re.compile(r"\bbinary_opcode_(arith|compare|logic)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_binary_emit_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split binary emission helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "binary_compare.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split binary comparison helper module"))

    old_import = re.compile(r"^use\s+mir::lower::binary_compare\s*$")
    old_call = re.compile(r"\bbinary_compare::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_body_dispatch_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split non-statement body dispatch modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "body_dispatch.ark",
        "body_dispatch_aggregate.ark",
        "body_dispatch_call.ark",
        "body_dispatch_control.ark",
        "body_dispatch_ops.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split body dispatch helper module"))

    old_import = re.compile(r"^use\s+mir::lower::(body_dispatch|body_dispatch_(aggregate|call|control|ops))\s*$")
    old_call = re.compile(r"\b(body_dispatch|body_dispatch_(aggregate|call|control|ops))::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_body_aggregate_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split aggregate literal body modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "body_aggregate_literal.ark",
        "body_array_lit.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split aggregate body helper module"))

    old_import = re.compile(r"^use\s+mir::lower::(body_aggregate_literal|body_array_lit)\s*$")
    old_call = re.compile(r"\b(body_aggregate_literal|body_array_lit)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_body_control_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split control-expression body modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "body_break_continue.ark",
        "body_return.ark",
        "body_try.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split control-expression body helper module"))

    old_import = re.compile(r"^use\s+mir::lower::(body_break_continue|body_return|body_try)\s*$")
    old_call = re.compile(r"\b(body_break_continue|body_return|body_try)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_body_unary_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split unary-expression body modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "body_unary.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split unary-expression body helper module"))

    old_import = re.compile(r"^use\s+mir::lower::body_unary\s*$")
    old_call = re.compile(r"\bbody_unary::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_body_match_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split legacy match dispatch helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "body_match_arm.ark",
        "body_match_arm_catchall.ark",
        "body_match_compare.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split body match dispatch helper module"))

    old_import = re.compile(r"^use\s+mir::lower::body_match_(arm|arm_catchall|compare)\s*$")
    old_call = re.compile(r"\bbody_match_(arm|arm_catchall|compare)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_body_stmt_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split legacy statement dispatch helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "body_stmt_basic.ark",
        "body_stmt_expr.ark",
        "body_stmt_for_iter.ark",
        "body_stmt_for_range.ark",
        "body_stmt_for_values.ark",
        "body_stmt_loop.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split body statement dispatch helper module"))

    old_import = re.compile(r"^use\s+mir::lower::body_stmt_(basic|expr|for_iter|for_range|for_values|loop)\s*$")
    old_call = re.compile(r"\bbody_stmt_(basic|expr|for_iter|for_range|for_values|loop)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_core_match_dispatch_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split CoreHIR match dispatch helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "core_match_arms.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split CoreHIR match dispatch helper module"))

    old_import = re.compile(r"^use\s+mir::lower::core_match_arms\s*$")
    old_call = re.compile(r"\bcore_match_arms::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_core_control_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split CoreHIR control-expression modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_names = (
        "core_expr_shape_control_block.ark",
        "core_try.ark",
    )
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split CoreHIR control helper module"))

    old_import = re.compile(r"^use\s+mir::lower::(core_expr_shape_control_block|core_try)\s*$")
    old_call = re.compile(r"\b(core_expr_shape_control_block|core_try)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_core_ops_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split CoreHIR operator-expression modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "core_unary.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split CoreHIR unary helper module"))

    old_import = re.compile(r"^use\s+mir::lower::core_unary\s*$")
    old_call = re.compile(r"\bcore_unary::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_core_call_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split CoreHIR call facade modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "core_call.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split CoreHIR call facade module"))

    old_import = re.compile(r"^use\s+mir::lower::core_call\s*$")
    old_call = re.compile(r"\bcore_call::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_entry_body_source_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split entry fallback body-source modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "entry_body_fallback_source.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split entry fallback body-source helper module"))

    old_import = re.compile(r"^use\s+mir::lower::entry_body_fallback_source\s*$")
    old_call = re.compile(r"\bentry_body_fallback_source::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_entry_core_body_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split entry CoreHIR body value-lowering modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_names = (
        "entry_body_support.ark",
        "entry_core_body_lower.ark",
        "entry_core_body_support.ark",
    )
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split entry CoreHIR body helper module"))

    old_import = re.compile(r"^use\s+mir::lower::(entry_body_support|entry_core_body_lower|entry_core_body_support)\s*$")
    old_call = re.compile(r"\b(entry_body_support|entry_core_body_lower|entry_core_body_support)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_entry_emit_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split entry body-emission facade modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_names = (
        "entry_emit.ark",
        "entry_frame.ark",
        "entry_view_type.ark",
    )
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split entry emission facade module"))

    old_import = re.compile(r"^use\s+mir::lower::(entry_emit|entry_frame|entry_view_type)\s*$")
    old_call = re.compile(r"\b(entry_emit|entry_frame|entry_view_type)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_entry_params_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split entry parameter facade modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_names = (
        "params.ark",
        "params_method_self.ark",
    )
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split entry parameter helper module"))

    old_import = re.compile(r"^use\s+mir::lower::(params|params_method_self)\s*$")
    old_call = re.compile(r"\b(params|params_method_self)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_return_type_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split return type facade modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "return_decl.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split declaration return type facade module"))

    old_import = re.compile(r"^use\s+mir::lower::return_decl\s*$")
    old_call = re.compile(r"\breturn_decl::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_method_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split method call facade modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "method_name.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split method callee naming helper module"))

    old_import = re.compile(r"^use\s+mir::lower::method_name\s*$")
    old_call = re.compile(r"\bmethod_name::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_registry_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split layout registry facade modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "registry.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split layout registry facade module"))

    old_import = re.compile(r"^use\s+mir::lower::registry\s*$")
    old_call = re.compile(r"\bregistry::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_ctx_fn_return_vt_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split function return-vt builtin fact modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "ctx_fn_builtin.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split function return-vt builtin fact module"))

    old_import = re.compile(r"^use\s+mir::lower::ctx_fn_builtin\s*$")
    old_call = re.compile(r"\bctx_fn_builtin::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_struct_lit_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split struct literal helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "struct_lit_field.ark",
        "struct_lit_new.ark",
        "struct_lit_result.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split struct literal helper module"))

    old_import = re.compile(r"^use\s+mir::lower::struct_lit_(field|new|result)\s*$")
    old_call = re.compile(r"\bstruct_lit_(field|new|result)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_variant_payload_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split variant payload tag helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "variant_payload_tag.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split variant payload tag helper module"))

    old_import = re.compile(r"^use\s+mir::lower::variant_payload_tag\s*$")
    old_call = re.compile(r"\bvariant_payload_tag::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_core_match_payload_info_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split CoreHIR match payload info record module/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_names = (
        "core_match_payload_info_types.ark",
        "core_match_payload_tag.ark",
    )
    for name in old_names:
        old_path = lower_root / name
        if old_path.is_file():
            violations.append((str(old_path.relative_to(root)), 1, "split CoreHIR match payload info helper module"))

    old_import = re.compile(r"^use\s+mir::lower::core_match_payload_(info_types|tag)\s*$")
    old_call = re.compile(r"\bcore_match_payload_(info_types|tag)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_match_payload_info_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split legacy match payload info record module/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "match_payload_info_types.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split legacy match payload info record module"))

    old_import = re.compile(r"^use\s+mir::lower::match_payload_info_types\s*$")
    old_call = re.compile(r"\bmatch_payload_info_types::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_match_payload_helper_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split legacy match payload binding/classification modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "match_payload_ident.ark",
        "match_payload_kinds.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split legacy match payload helper module"))

    old_import = re.compile(r"^use\s+mir::lower::match_payload_(ident|kinds)\s*$")
    old_call = re.compile(r"\bmatch_payload_(ident|kinds)::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_comprehension_type_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split list-comprehension element type helper module/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "comprehension_types.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split list-comprehension element type helper module"))

    old_import = re.compile(r"^use\s+mir::lower::comprehension_types\s*$")
    old_call = re.compile(r"\bcomprehension_types::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_comprehension_emit_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split list-comprehension emission/local helper modules/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    old_names = (
        "comprehension_emit_end.ark",
        "comprehension_emit_push.ark",
        "comprehension_emit_result.ark",
        "comprehension_guard.ark",
        "comprehension_iter.ark",
        "comprehension_locals_end.ark",
        "comprehension_locals_index.ark",
        "comprehension_locals_result.ark",
        "comprehension_locals_source.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = lower_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split list-comprehension helper module"))

    old_import = re.compile(r"^use\s+mir::lower::comprehension_(emit_(end|push|result)|guard|iter|locals_(end|index|result|source))\s*$")
    old_call = re.compile(r"\bcomprehension_(emit_(end|push|result)|guard|iter|locals_(end|index|result|source))::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_core_match_payload_type_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split CoreHIR match payload type helper module/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "core_match_payload_types.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split CoreHIR match payload type helper module"))

    old_import = re.compile(r"^use\s+mir::lower::core_match_payload_types\s*$")
    old_call = re.compile(r"\bcore_match_payload_types::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_stmt_let_type_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split let-statement type decision module/imports."""
    lower_root = root / "src" / "compiler" / "mir" / "lower"
    violations: list[tuple[str, int, str]] = []
    old_path = lower_root / "stmt_let_types.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split let-statement type decision module"))

    old_import = re.compile(r"^use\s+mir::lower::stmt_let_types\s*$")
    old_call = re.compile(r"\bstmt_let_types::")
    for path in sorted(lower_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_wit_decl_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return WIT generation files that bypass the component WIT decl view."""
    required = root / "src" / "compiler" / "component" / "wit_decl.ark"
    violations: list[tuple[str, int, str]] = []
    if not required.is_file():
        violations.append((str(required.relative_to(root)), 1, "missing WIT declaration view"))

    checks = (
        "src/compiler/component/wit_text.ark",
        "src/compiler/component/wit_type_defs.ark",
    )
    forbidden = (
        "use component::ast_node",
        "ast_node::",
        "use component::contract_helpers",
        "contract_helpers::",
        "use component::contract_export_filter",
        "contract_export_filter::",
    )
    for rel in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _component_type_node_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return component type predicate files that bypass the type-node adapter."""
    required = root / "src" / "compiler" / "component" / "type_node.ark"
    violations: list[tuple[str, int, str]] = []
    if not required.is_file():
        violations.append((str(required.relative_to(root)), 1, "missing component type-node view"))

    checks = (
        "src/compiler/component/contract_generic_types.ark",
        "src/compiler/component/contract_tuple_types.ark",
        "src/compiler/component/contract_collection_types.ark",
        "src/compiler/component/contract_type_support.ark",
        "src/compiler/component/contract_type_support_composite.ark",
        "src/compiler/component/contract_type_support_named.ark",
        "src/compiler/component/contract_numeric_predicates.ark",
        "src/compiler/component/contract_validation.ark",
        "src/compiler/component/wit_types.ark",
    )
    forbidden = (
        "use component::ast_node",
        "ast_node::",
    )
    for rel in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
            if "ty: AstNode" in stripped:
                violations.append((rel, line_no, stripped))
    return violations


def _component_contract_helper_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return component contract files that bypass function-decl helper views."""
    required = root / "src" / "compiler" / "component" / "contract_helpers.ark"
    violations: list[tuple[str, int, str]] = []
    if not required.is_file():
        violations.append((str(required.relative_to(root)), 1, "missing component contract helper view"))

    checks = (
        "src/compiler/component/contract_export_decl.ark",
        "src/compiler/component/contract_special_error.ark",
        "src/compiler/component/contract_special_support.ark",
    )
    forbidden = (
        "use component::ast_node",
        "ast_node::",
    )
    for rel in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _component_ast_node_adapter_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return component files that directly import AST outside approved adapters."""
    component_root = root / "src" / "compiler" / "component"
    allowed = {
        "contract_helpers.ark",
        "type_node.ark",
        "wit_decl.ark",
    }
    violations: list[tuple[str, int, str]] = []
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file() or path.name in allowed:
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped == "use component::ast_node":
                violations.append((rel, line_no, stripped))
    return violations


def _component_export_func_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return component predicate owners that still accept raw AST function nodes."""
    component_root = root / "src" / "compiler" / "component"
    checks = (
        "contract_string_fn.ark",
        "contract_record_fn.ark",
        "contract_collection_fn.ark",
        "contract_collection_types.ark",
        "contract_tuple_functions.ark",
        "contract_tuple_types.ark",
        "contract_numeric.ark",
        "contract_numeric_scan.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in checks:
        path = component_root / name
        if not path.is_file():
            violations.append((str(path.relative_to(root)), 1, "missing component function-view predicate owner"))
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if "f: AstNode" in stripped or "use component::ast_node" in stripped or "ast_node::" in stripped:
                violations.append((rel, line_no, stripped))
    return violations


def _component_string_predicate_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component string predicate modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "contract_string_fn_basic.ark",
        "contract_string_fn_narrow.ark",
        "contract_string_fn_numeric.ark",
        "contract_string_fn_scalar.ark",
    )
    violations: list[tuple[str, int, str]] = []
    if not (component_root / "contract_string_fn.ark").is_file():
        violations.append(("src/compiler/component/contract_string_fn.ark", 1, "missing consolidated string predicate module"))
    else:
        string_fn = component_root / "contract_string_fn.ark"
        rel = str(string_fn.relative_to(root))
        for line_no, line in enumerate(string_fn.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if "f: AstNode" in stripped or "use component::ast_node" in stripped or "ast_node::" in stripped:
                violations.append((rel, line_no, stripped))
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split string predicate module"))

    old_import = re.compile(r"^use\s+component::contract_string_fn_(basic|narrow|numeric|scalar)\s*$")
    old_call = re.compile(r"\bcontract_string_fn_(basic|narrow|numeric|scalar)::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_record_predicate_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component record predicate modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "contract_record_point_predicates.ark",
        "contract_record_color_predicates.ark",
        "contract_record_variant_predicates.ark",
    )
    violations: list[tuple[str, int, str]] = []
    if not (component_root / "contract_record_fn.ark").is_file():
        violations.append(("src/compiler/component/contract_record_fn.ark", 1, "missing consolidated record predicate module"))
    else:
        record_fn = component_root / "contract_record_fn.ark"
        rel = str(record_fn.relative_to(root))
        for line_no, line in enumerate(record_fn.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if "f: AstNode" in stripped or "use component::ast_node" in stripped or "ast_node::" in stripped:
                violations.append((rel, line_no, stripped))
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split record predicate module"))

    old_import = re.compile(r"^use\s+component::contract_record_(point|color|variant)_predicates\s*$")
    old_call = re.compile(r"\bcontract_record_(point|color|variant)_predicates::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_record_decl_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component record declaration shape modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "contract_record_point_shape.ark",
        "contract_record_color_shape.ark",
        "contract_record_variant_shape.ark",
    )
    violations: list[tuple[str, int, str]] = []
    if not (component_root / "contract_record_decl.ark").is_file():
        violations.append(("src/compiler/component/contract_record_decl.ark", 1, "missing consolidated record declaration module"))
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split record declaration shape module"))

    record_decl = component_root / "contract_record_decl.ark"
    if record_decl.is_file():
        rel = str(record_decl.relative_to(root))
        forbidden = ("use component::ast_node", "ast_node::")
        for line_no, line in enumerate(record_decl.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))

    old_import = re.compile(r"^use\s+component::contract_record_(point|color|variant)_shape\s*$")
    old_call = re.compile(r"\bcontract_record_(point|color|variant)_shape::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_collection_predicate_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component collection predicate modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "contract_collection_list_predicates.ark",
        "contract_collection_option_predicates.ark",
        "contract_collection_result_predicates.ark",
    )
    violations: list[tuple[str, int, str]] = []
    if not (component_root / "contract_collection_fn.ark").is_file():
        violations.append(("src/compiler/component/contract_collection_fn.ark", 1, "missing consolidated collection predicate module"))
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split collection predicate module"))

    old_import = re.compile(r"^use\s+component::contract_collection_(list|option|result)_predicates\s*$")
    old_call = re.compile(r"\bcontract_collection_(list|option|result)_predicates::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_collection_contract_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component option/result contract facades/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "contract_collection_option.ark",
        "contract_collection_option_param.ark",
        "contract_collection_option_return.ark",
        "contract_collection_result.ark",
        "contract_collection_result_param.ark",
        "contract_collection_result_return.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split option/result contract facade"))

    old_import = re.compile(r"^use\s+component::contract_collection_(option|result)(_(param|return))?\s*$")
    old_call = re.compile(r"\bcontract_collection_(option|result)(_(param|return))?::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_tiny_facade_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return tiny one-function component facades that should live in their owner."""
    component_root = root / "src" / "compiler" / "component"
    old_names = {
        "adapter_helpers.ark": "adapter_body.ark / adapter_memory.ark",
        "adapters_list.ark": "emit_list.ark",
        "adapters_option_projection_body.ark": "adapters_option.ark",
        "adapters_option_roundtrip_body.ark": "adapters_option.ark",
        "adapters_option_construct.ark": "emit_option_construct.ark",
        "adapters_output_body_user_call.ark": "adapter_body.ark",
        "adapters_pair_input_body_layout.ark": "adapter_body.ark",
        "adapters_result_input_roundtrip_body.ark": "adapters_result.ark",
        "adapters_result_output_body.ark": "adapters_result.ark",
        "adapters_string_input_copy.ark": "adapters_string_body.ark",
        "adapters_string_output_record.ark": "adapters_string_body.ark",
        "adapters_string_to_numeric.ark": "emit_string_numeric.ark",
        "adapters_tuple_return_body.ark": "adapters_tuple.ark",
        "adapters_string_output_record_alloc.ark": "adapters_string_body.ark",
        "adapters_string_output_record_fields.ark": "adapters_string_body.ark",
        "contract_allows_collection_build.ark": "contract_allows_scan_groups.ark",
        "contract_allows_collection_scan.ark": "contract_allows_scan_groups.ark",
        "contract_allows_scan.ark": "contract_allows.ark",
        "contract_allows_scan_records.ark": "contract_allows.ark",
        "component_base_header.ark": "component_base.ark",
        "constants_canon_encoding.ark": "constants_canon_resources.ark",
        "constants_canon_ops.ark": "constants_canon_resources.ark",
        "constants_canon_resources.ark": "constants_sections_linkage.ark",
        "constants_component_values.ark": "constants_wit_values.ark",
        "constants_sections_boundary.ark": "constants_sections_linkage.ark",
        "constants_sections_component.ark": "constants_sections_linkage.ark",
        "constants_sections_core.ark": "constants_sections_linkage.ark",
        "constants_sorts_component.ark": "constants_sections_linkage.ark",
        "constants_sorts_core.ark": "constants_sections_linkage.ark",
        "constants_sorts_exports.ark": "constants_sections_linkage.ark",
        "constants_wit_tags.ark": "constants_wit_values.ark",
        "core_wasm_sections.ark": "sections.ark",
        "contract_allows_numeric_build.ark": "contract_allows_numeric.ark",
        "contract_allows_numeric_query.ark": "contract_allows_numeric.ark",
        "contract_allows_numeric_record.ark": "contract_allows_numeric.ark",
        "contract_allows_numeric_root_query.ark": "contract_allows_numeric_tuple_query.ark",
        "contract_allows_numeric_scan.ark": "contract_allows_scan_groups.ark",
        "contract_allows_record_build.ark": "contract_allows.ark",
        "contract_allows_record_color_query.ark": "contract_allows_record_query.ark",
        "contract_allows_record_color_root_query.ark": "contract_allows_record_query.ark",
        "contract_allows_record_point_query.ark": "contract_allows_record_query.ark",
        "contract_allows_record_point_root_query.ark": "contract_allows_record_query.ark",
        "contract_allows_record_scan.ark": "contract_allows.ark",
        "contract_allows_record_shape_query.ark": "contract_allows_record_query.ark",
        "contract_allows_record_shape_root_query.ark": "contract_allows_record_query.ark",
        "contract_allows_result.ark": "contract_allows.ark",
        "contract_allows_string_basic_query.ark": "contract_allows_string_query.ark",
        "contract_allows_string_basic_root_query.ark": "contract_allows_string_query.ark",
        "contract_allows_string_build.ark": "contract_allows_scan_groups.ark",
        "contract_allows_string_numeric_query.ark": "contract_allows_string_query.ark",
        "contract_allows_string_numeric_root_query.ark": "contract_allows_string_query.ark",
        "contract_allows_string_scan.ark": "contract_allows_scan_groups.ark",
        "contract_allows_string_scalar_query.ark": "contract_allows_string_query.ark",
        "contract_allows_string_scalar_root_query.ark": "contract_allows_string_query.ark",
        "contract_allows_tuple_build.ark": "contract_allows_tuple.ark",
        "contract_allows_tuple_query.ark": "contract_allows_tuple.ark",
        "contract_allows_tuple_root_query.ark": "contract_allows_numeric_tuple_query.ark",
        "contract_allows_tuple_scan.ark": "contract_allows_scan_groups.ark",
        "contract_collection_list.ark": "contract_collection_list_scan.ark",
        "contract_export_decl.ark": "contract_helpers.ark",
        "contract_generic_types.ark": "contract_collection_types.ark",
        "contract_numeric_decls.ark": "contract_numeric.ark",
        "contract_numeric_predicates.ark": "contract_numeric_scan.ark",
        "contract_export_filter.ark": "contract_helpers.ark",
        "contract_record_color.ark": "contract_record_shape.ark",
        "contract_record_variant.ark": "contract_record_shape.ark",
        "contract_special_error.ark": "contract_helpers.ark",
        "contract_special_numeric_tuple.ark": "contract_special.ark",
        "contract_special_collection_option.ark": "contract_special_collection.ark",
        "contract_special_collection_result.ark": "contract_special_collection.ark",
        "contract_special_string_collection.ark": "contract_special.ark",
        "contract_special_string.ark": "contract_special.ark",
        "contract_special_support_collection.ark": "contract_special_support.ark",
        "contract_special_support_numeric_tuple.ark": "contract_special_support.ark",
        "contract_special_support_record.ark": "contract_special_support.ark",
        "contract_special_support_string.ark": "contract_special_support.ark",
        "contract_special_numeric.ark": "contract_special.ark",
        "contract_string_decls_basic.ark": "contract_string_scan.ark",
        "contract_string_decls_numeric.ark": "contract_string_decls_basic.ark",
        "contract_string_decls_param.ark": "contract_string_decls_basic.ark",
        "contract_string_decls_return.ark": "contract_string_decls_basic.ark",
        "contract_type_support_list.ark": "contract_type_support_composite.ark",
        "contract_type_support_option.ark": "contract_type_support_composite.ark",
        "contract_type_support_result.ark": "contract_type_support_composite.ark",
        "contract_type_support_tuple.ark": "contract_type_support_composite.ark",
        "contract_type_support.ark": "contract_validation.ark",
        "contract_special_tuple.ark": "contract_special.ark",
        "contract_tuple_function_params.ark": "contract_tuple_functions.ark",
        "contract_tuple_function_returns.ark": "contract_tuple_functions.ark",
        "contract_tuple_decls.ark": "contract_tuple_scan.ark",
        "contract_tuple_decls_param.ark": "contract_tuple_scan.ark",
        "contract_tuple_decls_return.ark": "contract_tuple_scan.ark",
        "contract_validation_func.ark": "contract_validation.ark",
        "contract_validation_params.ark": "contract_validation_func.ark",
        "contract_validation_returns.ark": "contract_validation_func.ark",
        "component_base_instance_sections.ark": "component_base.ark",
        "component_base_module_sections.ark": "component_base.ark",
        "export_shapes_color_select.ark": "export_shapes_color.ark",
        "export_shapes_color.ark": "emit_specialized_record.ark",
        "export_shapes_color_predicates.ark": "export_shapes_color_scan.ark",
        "export_shapes_list.ark": "export_shapes_match.ark",
        "export_shapes_list_predicates.ark": "export_shapes_match.ark",
        "export_shapes_numeric.ark": "export_shapes_match.ark",
        "export_shapes_numeric_predicates.ark": "export_shapes_match.ark",
        "export_shapes_numeric_signature.ark": "export_shapes_match.ark",
        "export_shapes_option.ark": "emit_specialized.ark",
        "export_shapes_option_construct.ark": "export_shapes_option_predicates.ark",
        "export_shapes_option_construct_predicates.ark": "export_shapes_option_predicates.ark",
        "export_shapes_option_project.ark": "export_shapes_option_predicates.ark",
        "export_shapes_option_project_predicates.ark": "export_shapes_option_predicates.ark",
        "export_shapes_presence_record.ark": "export_shapes_presence.ark",
        "export_shapes_record_predicates.ark": "export_shapes_record_scan.ark",
        "export_shapes_result_input_predicates.ark": "export_shapes_result_predicates.ark",
        "export_shapes_result_output_predicates.ark": "export_shapes_result_predicates.ark",
        "export_shapes_types.ark": "naming.ark",
        "export_shapes_shape_predicates.ark": "export_shapes_shape_scan.ark",
        "export_shapes_shape.ark": "emit_specialized.ark",
        "export_shapes_string_input_predicates.ark": "export_shapes_string_predicates.ark",
        "export_shapes_string_output_predicates.ark": "export_shapes_string_predicates.ark",
        "export_shapes_tuple_param_helpers.ark": "export_shapes_tuple_predicates.ark",
        "export_shapes_tuple_param_predicates.ark": "export_shapes_tuple_predicates.ark",
        "export_shapes_tuple_return_predicates.ark": "export_shapes_tuple_predicates.ark",
        "export_plan_append.ark": "export_plan.ark",
        "export_plan_predicates.ark": "export_plan.ark",
        "export_plan_record.ark": "export_plan.ark",
        "export_sections_alias_entry.ark": "export_sections_entries.ark",
        "export_sections_canon_entry.ark": "export_sections_entries.ark",
        "export_sections_export_entry.ark": "export_sections.ark",
        "export_types_color.ark": "export_types.ark",
        "export_shapes_shape_select.ark": "export_shapes_shape.ark",
        "export_shapes_presence_shape.ark": "export_shapes_presence.ark",
        "export_shapes_record_select.ark": "export_shapes_record.ark",
        "emit_i32_to_string.ark": "emit_string_numeric.ark",
        "emit_record.ark": "emit_specialized_record.ark",
        "emit_record_point.ark": "emit_record_ops.ark",
        "emit_string_unary.ark": "emit_specialized_string_basic.ark",
        "emit_string_scalar.ark": "emit_specialized_string_basic.ark",
        "emit_string_to_numeric.ark": "emit_string_numeric.ark",
        "emit_specialized_collection.ark": "emit_specialized.ark",
        "emit_specialized_list.ark": "emit_specialized.ark",
        "emit_specialized_option.ark": "emit_specialized.ark",
        "emit_specialized_result.ark": "emit_specialized.ark",
        "emit_specialized_shape.ark": "emit_specialized.ark",
        "emit_specialized_string.ark": "emit_specialized.ark",
        "emit_specialized_string_numeric.ark": "emit_specialized.ark",
        "emit_specialized_tuple_numeric.ark": "emit_specialized.ark",
        "record_point_type_bytes.ark": "type_section_entries.ark",
        "sections_export_entry.ark": "sections.ark",
        "sections_import_entry.ark": "sections.ark",
        "types_interner.ark": "export_types.ark",
        "types_interner_entries.ark": "export_types.ark",
        "types_wit_scalar.ark": "types_wit_compound.ark",
        "writer_leb.ark": "writer_core.ark",
        "writer_sections.ark": "writer_core.ark",
        "writer_string.ark": "writer_core.ark",
        "wasi_module_type.ark": "wasi_stub.ark / component_base.ark",
        "wasi.ark": "component_base.ark",
        "wit_defs.ark": "wit_text.ark",
        "wit_func_defs.ark": "wit_text.ark",
    }
    violations: list[tuple[str, int, str]] = []
    for name, owner in old_names.items():
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, f"tiny facade should be folded into {owner}"))

    old_import = re.compile(
        r"^use\s+component::("
        r"adapter_helpers|"
        r"adapters_list|"
        r"adapters_option_projection_body|"
        r"adapters_option_roundtrip_body|"
        r"adapters_option_construct|"
        r"adapters_output_body_user_call|"
        r"adapters_pair_input_body_layout|"
        r"adapters_result_input_roundtrip_body|"
        r"adapters_result_output_body|"
        r"adapters_string_input_copy|"
        r"adapters_string_output_record|"
        r"adapters_string_output_record_(alloc|fields)|"
        r"adapters_string_to_numeric|"
        r"adapters_tuple_return_body|"
        r"contract_allows_scan(_records)?|"
        r"contract_allows_(collection|numeric|record|string|tuple)_build|"
        r"contract_allows_(collection|numeric|record|string|tuple)_scan|"
        r"contract_allows_numeric_record|"
        r"contract_allows_(numeric|tuple)(_root)?_query|"
        r"contract_allows_record_(color|point|shape)(_root)?_query|"
        r"contract_allows_result|"
        r"contract_allows_string_(basic|numeric|scalar)(_root)?_query|"
        r"contract_collection_list|"
        r"contract_export_decl|"
        r"contract_generic_types|"
        r"contract_numeric_decls|"
        r"contract_numeric_predicates|"
        r"contract_export_filter|"
        r"contract_record_(color|variant)|"
        r"contract_special_error|"
        r"contract_special_(numeric|numeric_tuple|string|string_collection|tuple)|"
        r"contract_special_collection_(option|result)|"
        r"contract_special_support_(collection|numeric_tuple|record|string)|"
        r"contract_string_decls_(basic|numeric|param|return)|"
        r"contract_type_support_(list|option|result|tuple)|"
        r"contract_type_support|"
        r"contract_tuple_function_(params|returns)|"
        r"contract_tuple_decls(_(param|return))?|"
        r"contract_validation_(func|params|returns)|"
        r"constants_canon_encoding|"
        r"constants_canon_ops|"
        r"constants_canon_resources|"
        r"constants_component_values|"
        r"constants_sections_(boundary|component|core)|"
        r"constants_sorts_(component|core|exports)|"
        r"constants_wit_tags|"
        r"core_wasm_sections|"
        r"component_base_(header|instance_sections|module_sections)|"
        r"export_sections_export_entry|"
        r"export_shapes_(color|shape)_predicates|"
        r"export_shapes_(color|option|shape)|"
        r"export_shapes_(list|numeric)|"
        r"export_shapes_list_predicates|"
        r"export_shapes_numeric_predicates|"
        r"export_shapes_numeric_signature|"
        r"export_shapes_option_(construct|project)|"
        r"export_shapes_option_(construct|project)_predicates|"
        r"export_shapes_presence_record|"
        r"export_shapes_record_predicates|"
        r"export_shapes_result_(input|output)_predicates|"
        r"export_shapes_types|"
        r"export_shapes_(color|shape)_select|"
        r"export_shapes_string_(input|output)_predicates|"
        r"export_shapes_tuple_param_helpers|"
        r"export_shapes_tuple_(param|return)_predicates|"
        r"export_plan_append|"
        r"export_plan_predicates|"
        r"export_plan_record|"
        r"export_sections_(alias|canon)_entry|"
        r"export_types_color|"
        r"export_shapes_option_(construct|project)|"
        r"export_shapes_types|"
        r"export_shapes_presence_shape|"
        r"export_shapes_record_select|"
        r"export_shapes_tuple_param_helpers|"
        r"emit_i32_to_string|"
        r"emit_record|"
        r"emit_record_point|"
        r"emit_string_unary|"
        r"emit_string_to_numeric|"
        r"emit_specialized_(collection|list|option|result|shape|string)|"
        r"emit_specialized_string_numeric|"
        r"emit_specialized_tuple_numeric|"
        r"record_point_type_bytes|"
        r"sections_(export|import)_entry|"
        r"types_interner|"
        r"types_interner_entries|"
        r"types_wit_scalar|"
        r"writer_(leb|sections|string)|"
        r"wasi_module_type|"
        r"wasi|"
        r"wit_defs|"
        r"wit_func_defs"
        r")\s*$"
    )
    old_call = re.compile(
        r"\b("
        r"adapter_helpers|"
        r"adapters_list|"
        r"adapters_option_projection_body|"
        r"adapters_option_roundtrip_body|"
        r"adapters_option_construct|"
        r"adapters_output_body_user_call|"
        r"adapters_pair_input_body_layout|"
        r"adapters_result_input_roundtrip_body|"
        r"adapters_result_output_body|"
        r"adapters_string_input_copy|"
        r"adapters_string_output_record|"
        r"adapters_string_output_record_(alloc|fields)|"
        r"adapters_string_to_numeric|"
        r"adapters_tuple_return_body|"
        r"contract_allows_scan(_records)?|"
        r"contract_allows_(collection|numeric|record|string|tuple)_build|"
        r"contract_allows_(collection|numeric|record|string|tuple)_scan|"
        r"contract_allows_numeric_record|"
        r"contract_allows_(numeric|tuple)(_root)?_query|"
        r"contract_allows_record_(color|point|shape)(_root)?_query|"
        r"contract_allows_result|"
        r"contract_allows_string_(basic|numeric|scalar)(_root)?_query|"
        r"contract_collection_list|"
        r"contract_export_decl|"
        r"contract_generic_types|"
        r"contract_numeric_decls|"
        r"contract_numeric_predicates|"
        r"contract_export_filter|"
        r"contract_record_(color|variant)|"
        r"contract_special_error|"
        r"contract_special_(numeric|numeric_tuple|string|string_collection|tuple)|"
        r"contract_special_collection_(option|result)|"
        r"contract_special_support_(collection|numeric_tuple|record|string)|"
        r"contract_string_decls_(basic|numeric|param|return)|"
        r"contract_type_support_(list|option|result|tuple)|"
        r"contract_type_support|"
        r"contract_tuple_function_(params|returns)|"
        r"contract_tuple_decls(_(param|return))?|"
        r"contract_validation_(func|params|returns)|"
        r"constants_canon_encoding|"
        r"constants_canon_ops|"
        r"constants_canon_resources|"
        r"constants_component_values|"
        r"constants_sections_(boundary|component|core)|"
        r"constants_sorts_(component|core|exports)|"
        r"constants_wit_tags|"
        r"core_wasm_sections|"
        r"component_base_(header|instance_sections|module_sections)|"
        r"export_sections_export_entry|"
        r"export_shapes_(color|shape)_predicates|"
        r"export_shapes_(color|option|shape)|"
        r"export_shapes_(list|numeric)|"
        r"export_shapes_list_predicates|"
        r"export_shapes_numeric_predicates|"
        r"export_shapes_numeric_signature|"
        r"export_shapes_option_(construct|project)_predicates|"
        r"export_shapes_presence_record|"
        r"export_shapes_record_predicates|"
        r"export_shapes_result_(input|output)_predicates|"
        r"export_shapes_(color|shape)_select|"
        r"export_shapes_string_(input|output)_predicates|"
        r"export_shapes_tuple_(param|return)_predicates|"
        r"export_plan_append|"
        r"export_plan_predicates|"
        r"export_plan_record|"
        r"export_sections_(alias|canon)_entry|"
        r"export_types_color|"
        r"export_shapes_presence_shape|"
        r"export_shapes_record_select|"
        r"emit_i32_to_string|"
        r"emit_record|"
        r"emit_record_point|"
        r"record|"
        r"emit_string_scalar|"
        r"emit_string_unary|"
        r"emit_string_to_numeric|"
        r"emit_specialized_(collection|list|option|result|shape|string)|"
        r"emit_specialized_string_numeric|"
        r"emit_specialized_tuple_numeric|"
        r"specialized_(list|option|result|shape|string_numeric|tuple_numeric)|"
        r"record_point_type_bytes|"
        r"sections_(export|import)_entry|"
        r"types_interner|"
        r"types_interner_entries|"
        r"types_wit_scalar|"
        r"writer_(leb|sections|string)|"
        r"wasi_module_type|"
        r"wasi|"
        r"wit_defs|"
        r"wit_func_defs"
        r")::"
    )
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_list_adapter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component list adapter modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "adapters_list_input.ark",
        "adapters_list_output.ark",
        "adapters_list_input_sections.ark",
        "adapters_list_input_code_section.ark",
        "adapters_list_input_body.ark",
        "adapters_list_output_body.ark",
        "adapters_list_roundtrip_sections.ark",
        "adapters_list_roundtrip_code_section.ark",
        "adapters_list_roundtrip_body.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split list adapter module"))

    old_import = re.compile(r"^use\s+component::(adapters_list_(input|output)|adapters_list_(input|output|roundtrip)_(sections|code_section|body))\s*$")
    old_call = re.compile(r"\b(adapters_list_(input|output)|adapters_list_(input|output|roundtrip)_(sections|code_section|body))::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_shared_adapter_section_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return adapter section helper files/imports outside approved shared owners."""
    component_root = root / "src" / "compiler" / "component"
    allowed_section_modules = {
        "adapters_list_core_sections": {
            "emit_list.ark",
            "adapters_list_roundtrip.ark",
        },
        "adapters_single_export_sections": {
            "emit_list.ark",
            "adapters_option.ark",
            "emit_option_construct.ark",
            "adapters_record.ark",
            "adapters_result.ark",
            "emit_shape.ark",
            "adapters_tuple.ark",
        },
    }
    violations: list[tuple[str, int, str]] = []
    for path in sorted(component_root.glob("adapters_*sections.ark")):
        if path.stem not in allowed_section_modules:
            violations.append((str(path.relative_to(root)), 1, "unapproved adapter section helper module"))

    allowed_imports = set(allowed_section_modules)
    import_re = re.compile(r"^use\s+component::(adapters_[A-Za-z0-9_]+_sections)\s*$")
    call_re = re.compile(r"\b(adapters_[A-Za-z0-9_]+_sections)::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            imported = import_re.match(stripped)
            if imported:
                module = imported.group(1)
                if module not in allowed_imports or path.name not in allowed_section_modules[module]:
                    violations.append((rel, line_no, stripped))
                continue
            for module in call_re.findall(stripped):
                if module not in allowed_imports or path.name not in allowed_section_modules[module]:
                    violations.append((rel, line_no, stripped))
    return violations


def _component_string_adapter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component string adapter modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "adapters_string_numeric_body.ark",
        "adapters_string_unary_sections.ark",
        "adapters_string_unary_body.ark",
        "adapters_string_input_copy.ark",
        "adapters_string_output_record.ark",
        "adapters_string_to_numeric_body.ark",
        "adapters_string_numeric_sections.ark",
        "adapters_string_to_numeric_sections.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split string adapter module"))

    old_import = re.compile(r"^use\s+component::(adapters_string_(unary|numeric|to_numeric)_(body|sections)|adapters_string_(input_copy|output_record))\s*$")
    old_call = re.compile(r"\b(adapters_string_(unary|numeric|to_numeric)_(body|sections)|adapters_string_(input_copy|output_record))::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_numeric_adapter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component numeric adapter modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "adapters_numeric_wrapper_body.ark",
        "adapters_numeric_sections.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split numeric adapter module"))

    old_import = re.compile(r"^use\s+component::adapters_numeric_(sections|wrapper_body)\s*$")
    old_call = re.compile(r"\badapters_numeric_(sections|wrapper_body)::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_shape_adapter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component shape adapter modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "adapters_shape.ark",
        "adapters_shape_area.ark",
        "adapters_shape_area_body.ark",
        "adapters_shape_roundtrip_body.ark",
        "adapters_shape_roundtrip_sections.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split shape adapter module"))

    old_import = re.compile(r"^use\s+component::(adapters_shape|adapters_shape_area|adapters_shape_area_body|adapters_shape_roundtrip_body|adapters_shape_roundtrip_sections)\s*$")
    old_call = re.compile(r"\b(adapters_shape|adapters_shape_area|adapters_shape_area_body|adapters_shape_roundtrip_body|adapters_shape_roundtrip_sections)::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_result_string_adapter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component result/string adapter modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "adapters_result_string_input_sections.ark",
        "adapters_result_string_output_sections.ark",
        "adapters_result_input_roundtrip_body.ark",
        "adapters_result_output_body.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split result/string adapter module"))

    old_import = re.compile(r"^use\s+component::(adapters_result_string_(input|output)_sections|adapters_result_(input_roundtrip|output)_body)\s*$")
    old_call = re.compile(r"\b(adapters_result_string_(input|output)_sections|adapters_result_(input_roundtrip|output)_body)::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_option_adapter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component option adapter body modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "adapters_option_construct_body_output_common.ark",
        "adapters_option_construct.ark",
        "adapters_option_construct_bool_body.ark",
        "adapters_option_construct_i32_body.ark",
        "adapters_option_construct_i64_body.ark",
        "adapters_option_i32_body.ark",
        "adapters_option_i64_body.ark",
        "adapters_option_projection_body.ark",
        "adapters_option_roundtrip_body.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split option adapter body module"))

    old_import = re.compile(r"^use\s+component::adapters_option_(construct_(body_output_common|bool_body|i32_body|i64_body)|i(32|64)_body|projection_body|roundtrip_body)\s*$")
    old_call = re.compile(r"\badapters_option_(construct_(body_output_common|bool_body|i32_body|i64_body)|i(32|64)_body|projection_body|roundtrip_body)::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_pair_input_adapter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component pair-input adapter body modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "adapters_tuple_input_body.ark",
        "adapters_result_input_i32_body.ark",
        "adapters_pair_input_body_layout.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split pair-input adapter body module"))

    old_import = re.compile(r"^use\s+component::adapters_(tuple_input_body|result_input_i32_body|pair_input_body_layout)\s*$")
    old_call = re.compile(r"\badapters_(tuple_input_body|result_input_i32_body|pair_input_body_layout)::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_record_adapter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component record adapter body modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "adapters_record_add_body.ark",
        "adapters_record_pair_body.ark",
        "adapters_record_distance_body.ark",
        "adapters_record_roundtrip_body.ark",
        "adapters_record_add_sections.ark",
        "adapters_record_pair_sections.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split record adapter body module"))

    old_import = re.compile(r"^use\s+component::adapters_record_(add|pair|distance|roundtrip)_body\s*$|^use\s+component::adapters_record_(add|pair)_sections\s*$")
    old_call = re.compile(r"\badapters_record_(add|pair|distance|roundtrip)_body::|\badapters_record_(add|pair)_sections::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_wrapper_plan_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component wrapper adapter-plan modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "emit_list_plan.ark",
        "emit_numeric_plan.ark",
        "emit_option_plan.ark",
        "emit_option_construct_plan.ark",
        "emit_record_ops_plan.ark",
        "emit_result_plan.ark",
        "emit_result_input_plan.ark",
        "emit_shape_plan.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split component wrapper adapter-plan module"))

    old_import = re.compile(r"^use\s+component::emit_(list|numeric|option|option_construct|record_ops|result|result_input|shape)_plan\s*$")
    old_call = re.compile(r"\b(list_plan|numeric_plan|option_plan|option_construct_plan|record_ops_plan|result_plan|result_input_plan|shape_plan)::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_wrapper_section_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component wrapper section facade modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "emit_list_sections.ark",
        "emit_list_alias_sections.ark",
        "emit_list_canon_sections.ark",
        "emit_list_export_sections.ark",
        "emit_list_type_sections.ark",
        "emit_numeric_sections.ark",
        "emit_numeric_alias_sections.ark",
        "emit_numeric_canon_sections.ark",
        "emit_numeric_export_sections.ark",
        "emit_numeric_type_sections.ark",
        "emit_color_single.ark",
        "emit_color_enum.ark",
        "emit_color_alias_sections.ark",
        "emit_color_canon_sections.ark",
        "emit_color_export_sections.ark",
        "emit_color_func_type_sections.ark",
        "emit_color_type_sections.ark",
        "emit_tuple_input.ark",
        "emit_tuple_construct.ark",
        "emit_tuple_sections.ark",
        "emit_tuple_alias_sections.ark",
        "emit_tuple_canon_sections.ark",
        "emit_tuple_export_sections.ark",
        "emit_tuple_type_sections.ark",
        "emit_string_alias_sections.ark",
        "emit_string_canon_sections.ark",
        "emit_string_export_sections.ark",
        "emit_string_func_type_sections.ark",
        "emit_option_sections.ark",
        "emit_option_alias_sections.ark",
        "emit_option_canon_sections.ark",
        "emit_option_export_sections.ark",
        "emit_option_type_sections.ark",
        "emit_option_construct_sections.ark",
        "emit_option_construct_alias_sections.ark",
        "emit_option_construct_canon_sections.ark",
        "emit_option_construct_export_sections.ark",
        "emit_option_construct_type_sections.ark",
        "emit_result_sections.ark",
        "emit_result_input.ark",
        "emit_result_alias_sections.ark",
        "emit_result_canon_sections.ark",
        "emit_result_export_sections.ark",
        "emit_result_type_sections.ark",
        "emit_result_input_sections.ark",
        "emit_result_input_alias_sections.ark",
        "emit_result_input_canon_sections.ark",
        "emit_result_input_export_sections.ark",
        "emit_result_input_type_sections.ark",
        "emit_shape_sections.ark",
        "emit_shape_alias_sections.ark",
        "emit_shape_canon_sections.ark",
        "emit_shape_export_sections.ark",
        "emit_shape_func_type_sections.ark",
        "emit_shape_type_sections.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split component wrapper section facade module"))

    old_import = re.compile(r"^use\s+component::emit_(list|numeric|option|option_construct)_(alias_sections|canon_sections|export_sections|type_sections)\s*$|^use\s+component::emit_color_(alias_sections|canon_sections|export_sections|func_type_sections|type_sections|single|enum)\s*$|^use\s+component::emit_result_(alias_sections|canon_sections|export_sections|type_sections|input)\s*$|^use\s+component::emit_result_input_(alias_sections|canon_sections|export_sections|type_sections)\s*$|^use\s+component::emit_shape_(alias_sections|canon_sections|export_sections|func_type_sections|type_sections)\s*$|^use\s+component::emit_string_(alias_sections|canon_sections|export_sections|func_type_sections)\s*$|^use\s+component::emit_tuple_(alias_sections|canon_sections|export_sections|type_sections|input|construct)\s*$|^use\s+component::emit_(list|numeric|option|option_construct|result|result_input|shape|tuple)_sections\s*$")
    old_call = re.compile(r"\b(list_sections|list_alias_sections|list_canon_sections|list_export_sections|list_type_sections|numeric_sections|numeric_alias_sections|numeric_canon_sections|numeric_export_sections|numeric_type_sections|color_single|color_enum|color_alias_sections|color_canon_sections|color_export_sections|color_func_type_sections|color_type_sections|string_alias_sections|string_canon_sections|string_export_sections|string_func_type_sections|tuple_input|tuple_construct|tuple_sections|tuple_alias_sections|tuple_canon_sections|tuple_export_sections|tuple_type_sections|option_sections|option_alias_sections|option_canon_sections|option_export_sections|option_type_sections|option_construct_sections|option_construct_alias_sections|option_construct_canon_sections|option_construct_export_sections|option_construct_type_sections|result_input|result_sections|result_alias_sections|result_canon_sections|result_export_sections|result_type_sections|result_input_sections|result_input_alias_sections|result_input_canon_sections|result_input_export_sections|result_input_type_sections|shape_sections|shape_alias_sections|shape_canon_sections|shape_export_sections|shape_func_type_sections|shape_type_sections)::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_record_point_emitter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component record-point emitter modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "emit_record_point.ark",
        "emit_record_point_sections.ark",
        "emit_record_point_alias_sections.ark",
        "emit_record_point_canon_sections.ark",
        "emit_record_point_export_sections.ark",
        "emit_record_point_func_type_sections.ark",
        "emit_record_point_type_sections.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split record-point emitter module"))

    old_import = re.compile(r"^use\s+component::emit_record_point_(sections|alias_sections|canon_sections|export_sections|func_type_sections|type_sections)\s*$")
    old_call = re.compile(r"\brecord_point_(sections|alias_sections|canon_sections|export_sections|func_type_sections|type_sections)::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _component_record_ops_emitter_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return old split component record operation emitter modules/imports."""
    component_root = root / "src" / "compiler" / "component"
    old_names = (
        "emit_record_ops_sections.ark",
        "emit_record_ops_alias_sections.ark",
        "emit_record_ops_canon_sections.ark",
        "emit_record_ops_export_sections.ark",
        "emit_record_ops_func_type_sections.ark",
        "emit_record_ops_type_sections.ark",
    )
    violations: list[tuple[str, int, str]] = []
    for name in old_names:
        path = component_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split record operation emitter module"))

    old_import = re.compile(r"^use\s+component::emit_record_ops_(sections|alias_sections|canon_sections|export_sections|func_type_sections|type_sections)\s*$")
    old_call = re.compile(r"\brecord_ops_(sections|alias_sections|canon_sections|export_sections|func_type_sections|type_sections)::")
    for path in sorted(component_root.glob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


_CONST_FN_RE = re.compile(r"^(?:pub\s+)?fn\s+([A-Z][A-Z0-9_]*)\(\)\s*->\s*i32\s*\{\s*-?\d+\s*\}$")


def _compiler_constant_function_layout_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return enum/constant function shims outside dedicated table modules."""
    canonical_table_names = {
        "constants.ark",
        "kinds.ark",
        "lower_kinds.ark",
        "opcodes.ark",
        "tokens.ark",
        "type_tags.ark",
    }
    allowed_component_prefixes = (
        "constants_",
        "emit_",
        "export_shapes_",
        "sorts_",
        "wit_tags_",
        "wit_values_",
    )
    violations: list[tuple[str, int, str]] = []
    compiler_root = root / "src" / "compiler"
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        in_component = "component" in path.relative_to(compiler_root).parts
        name = path.name
        dedicated = (
            name in canonical_table_names
            or (in_component and name.startswith(allowed_component_prefixes))
        )
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if _CONST_FN_RE.match(stripped) and not dedicated:
                violations.append((rel, line_no, stripped))
    return violations


def _compiler_fragmented_constant_table_violations(root: Path) -> list[tuple[str, int]]:
    """Return tiny table-only files that should be merged into subsystem tables."""
    minimum_table_constants = 12
    violations: list[tuple[str, int]] = []
    compiler_root = root / "src" / "compiler"
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        lines = [line.strip() for line in path.read_text(encoding="utf-8").splitlines()]
        code = [line for line in lines if line and not line.startswith("//")]
        if not code:
            continue
        const_lines = [line for line in code if _CONST_FN_RE.match(line)]
        other_code = [
            line
            for line in code
            if line not in const_lines and not line.startswith("use ")
        ]
        if const_lines and not other_code and len(const_lines) < minimum_table_constants:
            rel_parts = path.relative_to(compiler_root).parts
            # Component constants are split by binary encoding subdomain; keep
            # that domain-specific layout independent from compiler vocabulary.
            if rel_parts and rel_parts[0] == "component":
                continue
            violations.append((str(path.relative_to(root)), len(const_lines)))
    return violations


def _diagnostics_code_table_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return split diagnostics code-table modules/imports."""
    diagnostics_root = root / "src" / "compiler" / "diagnostics"
    violations: list[tuple[str, int, str]] = []
    if not (diagnostics_root / "codes.ark").is_file():
        violations.append(("src/compiler/diagnostics/codes.ark", 1, "missing consolidated diagnostics code table"))
    for path in sorted(diagnostics_root.glob("codes_*.ark")):
        violations.append((str(path.relative_to(root)), 1, "split diagnostics code table"))

    compiler_root = root / "src" / "compiler"
    old_import = re.compile(r"^use\s+diagnostics::codes_[a-z0-9_]+\s*$")
    old_call = re.compile(r"\bcodes_[a-z0-9_]+::")
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _diagnostics_json_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return split diagnostics JSON wrapper/helper modules/imports."""
    diagnostics_root = root / "src" / "compiler" / "diagnostics"
    violations: list[tuple[str, int, str]] = []
    if not (diagnostics_root / "json.ark").is_file():
        violations.append(("src/compiler/diagnostics/json.ark", 1, "missing consolidated diagnostics JSON module"))
    for name in ("api_json.ark", "json_escape.ark"):
        path = diagnostics_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split diagnostics JSON module"))

    compiler_root = root / "src" / "compiler"
    old_import = re.compile(r"^use\s+diagnostics::(api_json|json_escape)\s*$")
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _diagnostics_severity_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return split diagnostics severity facade modules/imports."""
    diagnostics_root = root / "src" / "compiler" / "diagnostics"
    violations: list[tuple[str, int, str]] = []
    if not (diagnostics_root / "severity.ark").is_file():
        violations.append(("src/compiler/diagnostics/severity.ark", 1, "missing diagnostics severity module"))
    old_path = diagnostics_root / "api_severity.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split diagnostics severity facade"))

    compiler_root = root / "src" / "compiler"
    old_import = re.compile(r"^use\s+diagnostics::api_severity\s*$")
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _diagnostics_phase_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return unused split diagnostics phase facade modules/imports."""
    diagnostics_root = root / "src" / "compiler" / "diagnostics"
    violations: list[tuple[str, int, str]] = []
    old_path = diagnostics_root / "api_phase.ark"
    if old_path.is_file():
        violations.append((str(old_path.relative_to(root)), 1, "split diagnostics phase facade"))

    compiler_root = root / "src" / "compiler"
    old_import = re.compile(r"^use\s+diagnostics::api_phase\s*$")
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _diagnostics_span_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return split diagnostics span facade modules/imports."""
    diagnostics_root = root / "src" / "compiler" / "diagnostics"
    violations: list[tuple[str, int, str]] = []
    if not (diagnostics_root / "record.ark").is_file():
        violations.append(("src/compiler/diagnostics/record.ark", 1, "missing diagnostics record module"))
    for name in ("api_span.ark", "span_record.ark"):
        old_path = diagnostics_root / name
        if old_path.is_file():
            violations.append((str(old_path.relative_to(root)), 1, "split diagnostics span facade"))

    compiler_root = root / "src" / "compiler"
    old_import = re.compile(r"^use\s+diagnostics::(api_span|span_record)\s*$")
    old_call = re.compile(r"(?<![A-Za-z0-9_])span_record::")
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _diagnostics_record_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return split diagnostics record facade modules/imports."""
    diagnostics_root = root / "src" / "compiler" / "diagnostics"
    violations: list[tuple[str, int, str]] = []
    if not (diagnostics_root / "record.ark").is_file():
        violations.append(("src/compiler/diagnostics/record.ark", 1, "missing diagnostics record module"))
    for name in ("api_diagnostic.ark", "types.ark"):
        old_path = diagnostics_root / name
        if old_path.is_file():
            violations.append((str(old_path.relative_to(root)), 1, "split diagnostics record facade"))

    compiler_root = root / "src" / "compiler"
    old_import = re.compile(r"^use\s+diagnostics::(api_diagnostic|types)\s*$")
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or "api_diagnostic::" in stripped:
                violations.append((rel, line_no, stripped))
    return violations


def _diagnostics_source_map_fragmentation_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return split diagnostics source-map facade modules/imports."""
    diagnostics_root = root / "src" / "compiler" / "diagnostics"
    violations: list[tuple[str, int, str]] = []
    if not (diagnostics_root / "source_map_offsets.ark").is_file():
        violations.append(("src/compiler/diagnostics/source_map_offsets.ark", 1, "missing diagnostics source-map module"))
    for name in ("source_map_spans.ark", "source_map_text.ark"):
        path = diagnostics_root / name
        if path.is_file():
            violations.append((str(path.relative_to(root)), 1, "split diagnostics source-map facade"))

    compiler_root = root / "src" / "compiler"
    old_import = re.compile(r"^use\s+diagnostics::(source_map_spans|source_map_text)\s*$")
    old_call = re.compile(r"\b(source_map_spans|source_map_text)::")
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if old_import.search(stripped) or old_call.search(stripped):
                violations.append((rel, line_no, stripped))
    return violations


def _compiler_file_size_violations(root: Path) -> list[tuple[str, int]]:
    """Return compiler Ark files that exceed the context-sized module limit."""
    default_limit = 249
    table_limit = 500
    violations: list[tuple[str, int]] = []
    for path in sorted((root / "src" / "compiler").rglob("*.ark")):
        if not path.is_file():
            continue
        limit = table_limit if _compiler_table_like_file(path) else default_limit
        line_count = len(path.read_text(encoding="utf-8").splitlines())
        if line_count > limit:
            violations.append((str(path.relative_to(root)), line_count))
    return violations


def _compiler_function_size_violations(root: Path) -> list[tuple[str, int, int, str]]:
    """Return compiler Ark functions that exceed the local reasoning limit."""
    limit = 60
    violations: list[tuple[str, int, int, str]] = []
    for path in sorted((root / "src" / "compiler").rglob("*.ark")):
        if not path.is_file():
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        starts: list[int] = []
        for idx, line in enumerate(lines):
            stripped = line.strip()
            if stripped.startswith("fn ") or stripped.startswith("pub fn "):
                starts.append(idx)
        for pos, start in enumerate(starts):
            end = starts[pos + 1] if pos + 1 < len(starts) else len(lines)
            line_count = end - start
            if line_count > limit:
                violations.append(
                    (str(path.relative_to(root)), start + 1, line_count, lines[start].strip())
                )
    return violations


def _compiler_public_api_violations(root: Path) -> list[tuple[str, int]]:
    """Return compiler Ark files with too many public functions."""
    default_limit = 8
    table_limit = 128
    violations: list[tuple[str, int]] = []
    for path in sorted((root / "src" / "compiler").rglob("*.ark")):
        if not path.is_file():
            continue
        limit = table_limit if _compiler_table_like_file(path) else default_limit
        pub_count = sum(
            1
            for line in path.read_text(encoding="utf-8").splitlines()
            if line.strip().startswith("pub fn ")
        )
        if pub_count > limit:
            violations.append((str(path.relative_to(root)), pub_count))
    return violations


def _compiler_dependency_direction_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return imports that point against the compiler pipeline direction."""
    rules: list[tuple[str, tuple[str, ...]]] = [
        (
            "corehir*.ark",
            ("use mir", "use mir::", "use mir_lower", "use emitter", "use emit_", "use component", "use driver"),
        ),
        (
            "mir_lower*.ark",
            ("use emitter", "use emit_", "use component", "use driver"),
        ),
        (
            "emit*.ark",
            ("use mir_lower", "use component", "use driver", "use parser", "use typechecker"),
        ),
        (
            "emitter.ark",
            ("use mir_lower", "use component", "use driver", "use parser", "use typechecker"),
        ),
        (
            "component*.ark",
            ("use mir_lower", "use driver", "use parser", "use typechecker"),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    compiler_root = root / "src" / "compiler"
    for pattern, forbidden in rules:
        paths = list(compiler_root.glob(pattern))
        if pattern == "corehir*.ark":
            paths.extend((compiler_root / "corehir").rglob("*.ark"))
        for path in sorted(set(paths)):
            if not path.is_file():
                continue
            rel = str(path.relative_to(root))
            for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                stripped = line.strip()
                if any(stripped.startswith(prefix) for prefix in forbidden):
                    violations.append((rel, line_no, stripped))
    return violations


def _compiler_import_targets(compiler_root: Path, path: Path) -> list[Path]:
    """Return compiler-local imports from one Ark source file."""
    targets: list[Path] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped.startswith("use "):
            continue
        module = stripped[4:].split()[0]
        if module.startswith("std::"):
            continue
        module_path = compiler_root / f"{module.replace('::', '/')}.ark"
        if not module_path.is_file():
            module_path = compiler_root / f"{module.split('::')[0]}.ark"
        if module_path.is_file():
            targets.append(module_path)
    return targets


def _compiler_import_cycle_violations(root: Path) -> list[list[str]]:
    """Return compiler import cycles as relative file paths."""
    compiler_root = root / "src" / "compiler"
    graph: dict[Path, list[Path]] = {}
    for path in sorted(compiler_root.rglob("*.ark")):
        if path.is_file():
            graph[path] = _compiler_import_targets(compiler_root, path)

    visiting: set[Path] = set()
    visited: set[Path] = set()
    stack: list[Path] = []
    cycles: list[list[str]] = []

    def visit(path: Path) -> None:
        if path in visited:
            return
        if path in visiting:
            start = stack.index(path)
            cycle_paths = stack[start:] + [path]
            cycles.append([str(p.relative_to(root)) for p in cycle_paths])
            return
        visiting.add(path)
        stack.append(path)
        for dep in graph.get(path, []):
            visit(dep)
        stack.pop()
        visiting.remove(path)
        visited.add(path)

    for path in sorted(graph):
        if path not in visited:
            visit(path)
    return cycles


def _compiler_import_fanout_violations(root: Path) -> list[tuple[str, int]]:
    """Return compiler Ark files with too many direct compiler imports."""
    limit = 13
    violations: list[tuple[str, int]] = []
    compiler_root = root / "src" / "compiler"
    for path in sorted(compiler_root.rglob("*.ark")):
        if not path.is_file():
            continue
        import_count = len(_compiler_import_targets(compiler_root, path))
        if import_count > limit:
            violations.append((str(path.relative_to(root)), import_count))
    return violations


def _compiler_production_test_reachability_violations(root: Path) -> list[str]:
    """Return test-only modules reachable from the compiler production entry."""
    compiler_root = root / "src" / "compiler"
    entry = compiler_root / "main.ark"
    seen: set[Path] = set()
    stack: list[Path] = [entry]

    while stack:
        path = stack.pop()
        if path in seen or not path.is_file():
            continue
        seen.add(path)
        for module_path in _compiler_import_targets(compiler_root, path):
            stack.append(module_path)

    violations: list[str] = []
    for path in sorted(seen):
        rel = str(path.relative_to(root))
        name = path.name
        if "smoke" in name or "fixture" in name or "self_check" in name:
            violations.append(rel)
    return violations


def _mir_legacy_body_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return direct legacy AST-body lowering calls outside the adapter."""
    allowed = "src/compiler/mir_lower_legacy_body_lower.ark"
    violations: list[tuple[str, int, str]] = []
    for path in sorted((root / "src" / "compiler").glob("mir_lower*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        if rel == allowed:
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped == "use mir_lower_body" or "mir_lower_body::lower_expr" in stripped:
                violations.append((rel, line_no, stripped))
    return violations


def _mir_entry_fallback_source_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return direct fallback-source usage outside the entry body-source adapter."""
    allowed = {
        "src/compiler/mir_lower_entry_body_fallback_source.ark",
        "src/compiler/mir_lower_entry_body_source.ark",
    }
    forbidden = (
        "MirLowerFallbackSource",
        "mir_lower_fallback_source",
        "mir_fallback_source_",
    )
    violations: list[tuple[str, int, str]] = []
    for path in sorted((root / "src" / "compiler").glob("mir_lower_entry*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        if rel in allowed:
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_entry_body_value_partition_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return CoreHIR/fallback body value lowering cross-contamination."""
    checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/mir_lower_entry_body_input.ark",
            ("mir_entry_body_input_can_emit_body", "mir_lower_entry_core_body_support", "mir_lower_entry_fallback_body"),
        ),
        (
            "src/compiler/mir_lower_entry_body_support.ark",
            ("mir_lower_entry_fallback_body", "MirLowerEntryFallbackBody", "mir_entry_body_input_fallback"),
        ),
        (
            "src/compiler/mir_lower_entry_body_core_value.ark",
            ("MirLowerEntryBodyInput", "mir_lower_entry_body_input", "MirLowerEntryFallbackBody", "mir_lower_legacy_body"),
        ),
        (
            "src/compiler/mir_lower_entry_body_fallback.ark",
            ("MirLowerEntryBodyInput", "mir_lower_entry_body_input", "MirLowerEntryCoreBody", "mir_lower_entry_core_body"),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, forbidden in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_fallback_source_legacy_decl_boundary_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return AST decl-shape leaks outside the legacy declaration adapter."""
    checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/mir_lower_fallback_source.ark",
            (
                "AstNode",
                "corehir_decl_entry_shape",
                "decl_entry_child_at",
                "decl_entry_impl_method_at",
                "mir_lower_legacy_body",
            ),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, forbidden in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))

    allowed_shape_adapter = "src/compiler/mir_lower_legacy_decl.ark"
    for path in sorted((root / "src" / "compiler").glob("mir_lower*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        if rel == allowed_shape_adapter:
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if "corehir_decl_entry_shape" in stripped:
                violations.append((rel, line_no, stripped))
    return violations


def _driver_legacy_decl_adapter_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return driver-side legacy declaration use outside the fallback adapter."""
    allowed = "src/compiler/driver_lower_fallback_source.ark"
    forbidden = (
        "MirLowerLegacyDecl",
        "mir_lower_legacy_decl",
    )
    violations: list[tuple[str, int, str]] = []
    for path in sorted((root / "src" / "compiler").glob("driver*.ark")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(root))
        if rel == allowed:
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


def _mir_lower_input_contract_violations(root: Path) -> list[tuple[str, int, str]]:
    """Return MIR lowering facade/caller leaks around the bundled input contract."""
    checks: list[tuple[str, tuple[str, ...]]] = [
        (
            "src/compiler/mir_lower.ark",
            ("CoreHirRawProgram", "MirLowerFallbackSource", "fallback_source"),
        ),
        (
            "src/compiler/mir_lower_input.ark",
            ("CoreHirRawProgram", "mir_lower_input_program", "mir_lower_input_fallback_source"),
        ),
        (
            "src/compiler/mir_lower_entry_input.ark",
            ("CoreHirRawProgram", "MirLowerFallbackSource", "fallback_source", "mir_view_from_program", "corehir_mir_view"),
        ),
        (
            "src/compiler/driver_lower.ark",
            (
                "corehir_frontend_checked",
                "driver_lower_fallback_source",
                "CoreHirRawProgram",
                "MirLowerFallbackSource",
                "fallback_source",
            ),
        ),
        (
            "src/compiler/driver_lower_component.ark",
            (
                "corehir_frontend_checked",
                "driver_lower_fallback_source",
                "CoreHirRawProgram",
                "MirLowerFallbackSource",
                "fallback_source",
            ),
        ),
    ]
    violations: list[tuple[str, int, str]] = []
    for rel, forbidden in checks:
        path = root / rel
        if not path.is_file():
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if any(token in stripped for token in forbidden):
                violations.append((rel, line_no, stripped))
    return violations


# ── verify subcommands ────────────────────────────────────────────────────────


def cmd_verify_quick(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    # ── Fixture manifest completeness check ──────────────────────────────────
    print(f"\n{YELLOW}[manifest] Checking fixture manifest completeness...{NC}")
    manifest_file = root / "tests" / "fixtures" / "manifest.txt"
    manifest_ok = True
    fixture_count = 0
    if not manifest_file.exists():
        h.check_fail(
            "Fixture manifest not found: tests/fixtures/manifest.txt",
            category="fixture",
            command="python3 scripts/manager.py verify quick",
            primary_path="tests/fixtures/manifest.txt",
        )
        manifest_ok = False
    else:
        fixture_count = count_fixtures(manifest_file)
        fixtures_root = root / "tests" / "fixtures"
        disk_paths = disk_fixture_paths(fixtures_root)
        manifest_entries = sorted(
            {
                e["path"]
                for e in load_manifest(manifest_file)
                if e["kind"] != "bench"
            }
        )
        if disk_paths != manifest_entries:
            h.check_fail(
                "Fixture manifest out of sync with disk",
                category="fixture",
                command="python3 scripts/manager.py verify quick",
                primary_path="tests/fixtures/manifest.txt",
            )
            disk_set = set(disk_paths)
            manifest_set = set(manifest_entries)
            for p in sorted(disk_set - manifest_set)[:10]:
                print(f"  < {p}")
            for p in sorted(manifest_set - disk_set)[:10]:
                print(f"  > {p}")
            manifest_ok = False

    if manifest_ok:
        h.check_pass(f"Fixture manifest completeness ({fixture_count} entries)")

    # ── Background checks ─────────────────────────────────────────────────────
    print(f"\n{YELLOW}[bg] Running background checks in parallel...{NC}")

    def _shell(cmd_str: str) -> tuple[int, str]:
        """Run a bash command string; return (rc, combined output)."""
        if dry_run:
            return (0, f"DRY-RUN: {cmd_str}")
        result = subprocess.run(
            ["bash", "-lc", cmd_str],
            cwd=str(root),
            capture_output=True,
            text=True,
        )
        return (result.returncode, result.stdout + result.stderr)

    bg_checks: list[tuple[str, str]] = [
        (
            "Documentation structure OK",
            "test -f AGENTS.md && test -f docs/process/agent-harness.md "
            "&& test -d docs/adr && test -d issues/open && test -d issues/done "
            "&& test -d docs/language && test -d docs/platform && test -d docs/stdlib "
            "&& test -d docs/process",
        ),
        (
            "All required ADRs decided",
            "for f in docs/adr/ADR-002-memory-model.md docs/adr/ADR-003-generics-strategy.md "
            "docs/adr/ADR-004-trait-strategy.md docs/adr/ADR-005-llvm-scope.md "
            "docs/adr/ADR-006-abi-policy.md; do "
            'test -f "$f" || exit 1; grep -q \'DECIDED\\|決定\' "$f" || exit 1; done',
        ),
        (
            "Language specification OK",
            "test -f docs/language/memory-model.md && test -f docs/language/type-system.md "
            "&& test -f docs/language/syntax.md",
        ),
        (
            "Platform specification OK",
            "test -f docs/platform/wasm-features.md && test -f docs/platform/abi.md "
            "&& test -f docs/platform/wasi-resource-model.md",
        ),
        (
            "Stdlib specification OK",
            "test -f docs/stdlib/README.md && test -f docs/stdlib/core.md "
            "&& test -f docs/stdlib/io.md",
        ),
        (
            "docs consistency",
            "python3 scripts/check/check-docs-consistency.py",
        ),
        (
            "docs freshness (project-state.toml vs manifest.txt)",
            "python3 scripts/check/check-docs-freshness.py",
        ),
        (
            "stdlib manifest check",
            "bash scripts/check/check-stdlib-manifest.sh",
        ),
        (
            "issues/done/ has no unchecked checkboxes",
            "files=$(grep -rl '\\- \\[ \\]' issues/done/ 2>/dev/null | grep '\\.md$' || true); "
            'if [ -n "$files" ]; then echo "Files in done/ with unchecked items:"; '
            'printf \'%s\\n\' "$files"; exit 1; fi',
        ),
        (
            "no panic/unwrap in user-facing crates",
            "bash scripts/check/check-panic-audit.sh",
        ),
        (
            "asset naming convention (snake_case)",
            "bash scripts/check/check-asset-naming.sh",
        ),
        (
            "repository structure (root scripts, scripts/ layout)",
            "bash scripts/check/check-repo-structure.sh",
        ),
        (
            "generated file boundary check",
            "bash scripts/check/check-generated-files.sh",
        ),
        (
            "doc example check (ark blocks in docs/)",
            "python3 scripts/check/check-doc-examples.py docs/",
        ),
        (
            "selfhost analysis API gate (#568)",
            "python3 scripts/check/check-analysis-api.py",
        ),
        (
            "selfhost LSP lifecycle gate (#569)",
            "python3 scripts/check/check-lsp-lifecycle.py",
        ),
        (
            "selfhost DAP lifecycle gate (#571)",
            "python3 scripts/check/check-dap-lifecycle.py",
        ),
    ]

    bg_results: list[tuple[str, str, int, str]] = []
    with concurrent.futures.ThreadPoolExecutor() as executor:
        futures = {
            executor.submit(_shell, cmd_str): (label, cmd_str)
            for label, cmd_str in bg_checks
        }
        for future in concurrent.futures.as_completed(futures):
            label, cmd_str = futures[future]
            rc, out = future.result()
            bg_results.append((label, cmd_str, rc, out))

    print(f"\n{YELLOW}[bg] Collecting background check results...{NC}")
    for label, cmd_str, rc, out in bg_results:
        if rc == 0:
            h.check_pass(label)
        else:
            if "package-workspace" in label:
                category = "package-workspace"
                primary_path = "tests/package-workspace/"
            elif "LSP" in label or "DAP" in label:
                category = "editor-tooling"
                primary_path = "tests/fixtures/selfhost/"
            else:
                category = "verification-hygiene"
                primary_path = "scripts/manager.py"
            h.check_fail(
                label,
                category=category,
                command=cmd_str,
                primary_path=primary_path,
            )
            for line in out.splitlines()[-30:]:
                print(line)

    # Static pass
    boundary_violations = _corehir_mir_boundary_violations(root)
    if boundary_violations:
        h.check_fail(
            "CoreHIR -> MIR boundary has frontend/typecheck node leaks",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/corehir_mir_view.ark",
        )
        for rel, line_no, line in boundary_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("CoreHIR -> MIR boundary excludes frontend/typecheck nodes")

    mir_view_contract_violations = _corehir_mir_view_contract_violations(root)
    if mir_view_contract_violations:
        h.check_fail(
            "CoreHIR MIR view stores raw body builder tables",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/corehir_mir_view.ark",
        )
        for rel, line_no, line in mir_view_contract_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("CoreHIR MIR view stores body DTOs, not raw builder tables")

    param_shape_facade_violations = _corehir_param_shape_facade_violations(root)
    if param_shape_facade_violations:
        h.check_fail(
            "CoreHIR parameter shape facade owns frontend helper details",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/corehir_param_shape.ark",
        )
        for rel, line_no, line in param_shape_facade_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("CoreHIR parameter shape facade delegates helper details")

    signature_facade_violations = _corehir_mir_signature_facade_violations(root)
    if signature_facade_violations:
        h.check_fail(
            "CoreHIR MIR signature source facade owns fn/method projection details",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/corehir_mir_signature_source.ark",
        )
        for rel, line_no, line in signature_facade_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("CoreHIR MIR signature source facade stays constructor-only")

    entry_view_facade_violations = _mir_entry_view_projection_facade_violations(root)
    if entry_view_facade_violations:
        h.check_fail(
            "MIR entry view facade owns projection details",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir_lower_entry_view.ark",
        )
        for rel, line_no, line in entry_view_facade_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR entry view facade delegates role-specific projections")

    layout_decl_facade_violations = _corehir_layout_decl_facade_violations(root)
    if layout_decl_facade_violations:
        h.check_fail(
            "CoreHIR layout decl facade owns projection details",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/corehir_layout_decl.ark",
        )
        for rel, line_no, line in layout_decl_facade_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("CoreHIR layout decl facade delegates role-specific projections")

    module_state_facade_violations = _driver_module_state_facade_violations(root)
    if module_state_facade_violations:
        h.check_fail(
            "driver module state facade owns role-specific state details",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/driver_module_state.ark",
        )
        for rel, line_no, line in module_state_facade_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("driver module state facade delegates role-specific state APIs")

    module_source_lookup_violations = _driver_module_source_lookup_violations(root)
    if module_source_lookup_violations:
        h.check_fail(
            "driver module source lookup owns parsing or state registration",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/driver_module_local_source.ark",
        )
        for rel, line_no, line in module_source_lookup_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("driver module source lookup stays separate from parsing/state registration")

    module_graph_relative_import_violations = _driver_module_graph_relative_import_violations(root)
    if module_graph_relative_import_violations:
        h.check_fail(
            "driver module graph loses nested relative import roots",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/driver/module_graph.ark",
        )
        for violation in module_graph_relative_import_violations[:20]:
            print(f"  {violation}")
    else:
        h.check_pass("driver module graph preserves absolute root and nested relative import bases")

    parser_expectation_diagnostic_violations = _parser_expectation_diagnostic_boundary_violations(root)
    if parser_expectation_diagnostic_violations:
        h.check_fail(
            "parser expectation control owns parse diagnostic construction",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/parser_expect.ark",
        )
        for rel, line_no, line in parser_expectation_diagnostic_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("parser expectation control delegates parse diagnostic construction")

    parser_ast_factory_violations = _parser_ast_factory_boundary_violations(root)
    if parser_ast_factory_violations:
        h.check_fail(
            "generic parser AST factory owns literal construction details",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/parser_core_ast_factory.ark",
        )
        for rel, line_no, line in parser_ast_factory_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("generic parser AST factory excludes literal construction details")

    parser_type_dispatch_violations = _parser_type_dispatch_boundary_violations(root)
    if parser_type_dispatch_violations:
        h.check_fail(
            "parser type dispatch facade uses ambient parser helpers",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/parser_types.ark",
        )
        for rel, line_no, line in parser_type_dispatch_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("parser type dispatch facade uses explicit parser APIs")

    parser_type_recursion_violations = _parser_type_recursion_boundary_violations(root)
    if parser_type_recursion_violations:
        h.check_fail(
            "parser type recursion leaks out of dispatch owner",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/parser_types.ark",
        )
        for rel, line_no, line in parser_type_recursion_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("parser type recursion stays owned by dispatch")

    parser_decl_facade_violations = _parser_decl_facade_boundary_violations(root)
    if parser_decl_facade_violations:
        h.check_fail(
            "parser declaration facade owns header/body parsing details",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/parser_decl_struct.ark",
        )
        for rel, line_no, line in parser_decl_facade_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("parser declaration facades delegate header/body parsing details")

    parser_decl_dispatch_violations = _parser_decl_dispatch_boundary_violations(root)
    if parser_decl_dispatch_violations:
        h.check_fail(
            "parser declaration dispatch uses ambient parser helpers",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/parser_decl_dispatch.ark",
        )
        for rel, line_no, line in parser_decl_dispatch_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("parser declaration dispatch uses explicit parser APIs")

    parser_fn_signature_violations = _parser_fn_signature_boundary_violations(root)
    if parser_fn_signature_violations:
        h.check_fail(
            "parser function signatures use ambient parser helpers",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/parser_fn_sig_decl.ark",
        )
        for rel, line_no, line in parser_fn_signature_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("parser function signatures use explicit parser APIs")

    parser_import_violations = _parser_import_boundary_violations(root)
    if parser_import_violations:
        h.check_fail(
            "parser import declarations use ambient parser helpers",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/parser_imports_use.ark",
        )
        for rel, line_no, line in parser_import_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("parser import declarations use explicit parser APIs")

    parser_stmt_violations = _parser_stmt_boundary_violations(root)
    if parser_stmt_violations:
        h.check_fail(
            "parser statement facades hide entrypoints or use ambient helpers",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/parser/stmt.ark",
        )
        for rel, line_no, line in parser_stmt_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("parser statement facades expose explicit entrypoints")

    parser_pratt_violations = _parser_pratt_facade_violations(root)
    if parser_pratt_violations:
        h.check_fail(
            "parser Pratt facades hide expression loop contracts",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/parser/expr_bp_loop.ark",
        )
        for rel, line_no, line in parser_pratt_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("parser Pratt facades expose expression loop contracts")

    parser_prefix_violations = _parser_prefix_facade_violations(root)
    if parser_prefix_violations:
        h.check_fail(
            "parser prefix facades hide expression prefix contracts",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/parser/expr_prefix_dispatch.ark",
        )
        for rel, line_no, line in parser_prefix_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("parser prefix facades expose expression prefix contracts")

    accessor_facade_violations = _corehir_frontend_accessor_facade_violations(root)
    if accessor_facade_violations:
        h.check_fail(
            "CoreHIR frontend accessor facades depend on parser/type internals",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/corehir_frontend_ast_node.ark",
        )
        for rel, line_no, line in accessor_facade_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("CoreHIR frontend accessor facades use role-specific contracts")

    import_violations = _mir_lower_import_boundary_violations(root)
    if import_violations:
        h.check_fail(
            "MIR lowering imports parser/typechecker outside legacy adapters",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir_lower.ark",
        )
        for rel, line_no, line in import_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR lowering parser/typechecker imports stay behind adapters")

    ast_adapter_violations = _mir_lower_ast_adapter_fragmentation_violations(root)
    if ast_adapter_violations:
        h.check_fail(
            "MIR lowering AST adapter is split across role-specific files",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/ast_node.ark",
        )
        for rel, line_no, line in ast_adapter_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR lowering AST adapter stays consolidated")

    annotation_adapter_violations = _mir_annotation_adapter_fragmentation_violations(root)
    if annotation_adapter_violations:
        h.check_fail(
            "MIR annotation adapters are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/ann_types.ark",
        )
        for rel, line_no, line in annotation_adapter_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR annotation adapters stay grouped by role")

    return_index_fragmentation_violations = _mir_return_index_fragmentation_violations(root)
    if return_index_fragmentation_violations:
        h.check_fail(
            "MIR return-index type helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/return_index_name.ark",
        )
        for rel, line_no, line in return_index_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR return-index type helpers stay grouped by role")

    stmt_let_fragmentation_violations = _mir_stmt_let_fragmentation_violations(root)
    if stmt_let_fragmentation_violations:
        h.check_fail(
            "MIR let finalization helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/stmt_let_finish.ark",
        )
        for rel, line_no, line in stmt_let_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR let finalization helpers stay grouped by role")

    literal_int_fragmentation_violations = _mir_literal_int_fragmentation_violations(root)
    if literal_int_fragmentation_violations:
        h.check_fail(
            "MIR integer literal helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/literal_int.ark",
        )
        for rel, line_no, line in literal_int_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR integer literal helpers stay grouped by role")

    atomic_value_fragmentation_violations = _mir_atomic_value_fragmentation_violations(root)
    if atomic_value_fragmentation_violations:
        h.check_fail(
            "MIR atomic value helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/values.ark",
        )
        for rel, line_no, line in atomic_value_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR atomic value helpers stay grouped by role")

    call_arg_fragmentation_violations = _mir_call_arg_fragmentation_violations(root)
    if call_arg_fragmentation_violations:
        h.check_fail(
            "MIR call argument facade helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/call_args.ark",
        )
        for rel, line_no, line in call_arg_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR call argument facade helpers stay grouped by role")

    core_call_arg_fragmentation_violations = _mir_core_call_arg_fragmentation_violations(root)
    if core_call_arg_fragmentation_violations:
        h.check_fail(
            "MIR CoreHIR call argument facade helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/core_call_args.ark",
        )
        for rel, line_no, line in core_call_arg_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR CoreHIR call argument facade helpers stay grouped by role")

    call_type_fragmentation_violations = _mir_call_type_fragmentation_violations(root)
    if call_type_fragmentation_violations:
        h.check_fail(
            "MIR call result type helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/call_types.ark",
        )
        for rel, line_no, line in call_type_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR call result type helpers stay grouped by role")

    call_rewrite_type_fragmentation_violations = _mir_call_rewrite_type_fragmentation_violations(root)
    if call_rewrite_type_fragmentation_violations:
        h.check_fail(
            "MIR call rewrite record helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/call_rewrite_string.ark",
        )
        for rel, line_no, line in call_rewrite_type_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR call rewrite record helpers stay with their owners")

    loop_struct_iter_fragmentation_violations = _mir_loop_struct_iter_fragmentation_violations(root)
    if loop_struct_iter_fragmentation_violations:
        h.check_fail(
            "MIR struct-iterator loop helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/loop_struct_iter_locals.ark",
        )
        for rel, line_no, line in loop_struct_iter_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR struct-iterator loop helpers stay grouped by owner")

    vec_call_rewrite_fragmentation_violations = _mir_vec_call_rewrite_fragmentation_violations(root)
    if vec_call_rewrite_fragmentation_violations:
        h.check_fail(
            "MIR Vec call rewrite helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/call_rewrite_vec.ark",
        )
        for rel, line_no, line in vec_call_rewrite_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR Vec call rewrite helpers stay grouped by owner")

    hof_step_fragmentation_violations = _mir_hof_step_fragmentation_violations(root)
    if hof_step_fragmentation_violations:
        h.check_fail(
            "MIR HOF typed loop helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/hof_i32.ark",
        )
        for rel, line_no, line in hof_step_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR HOF typed loop helpers stay grouped by owner")

    hof_option_fragmentation_violations = _mir_hof_option_fragmentation_violations(root)
    if hof_option_fragmentation_violations:
        h.check_fail(
            "MIR Option HOF phase helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/hof_option.ark",
        )
        for rel, line_no, line in hof_option_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR Option HOF phase helpers stay grouped by owner")

    hof_call_plan_fragmentation_violations = _mir_hof_call_plan_fragmentation_violations(root)
    if hof_call_plan_fragmentation_violations:
        h.check_fail(
            "MIR HOF call-plan contract helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/hof_call_plan.ark",
        )
        for rel, line_no, line in hof_call_plan_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR HOF call-plan contract helpers stay grouped by owner")

    hof_call_classifier_fragmentation_violations = _mir_hof_call_classifier_fragmentation_violations(root)
    if hof_call_classifier_fragmentation_violations:
        h.check_fail(
            "MIR HOF call classifier helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/hof_call_classify.ark",
        )
        for rel, line_no, line in hof_call_classifier_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR HOF call classifier helpers stay grouped by owner")

    hof_setup_fragmentation_violations = _mir_hof_setup_fragmentation_violations(root)
    if hof_setup_fragmentation_violations:
        h.check_fail(
            "MIR HOF typed setup helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/hof_i32_setup.ark",
        )
        for rel, line_no, line in hof_setup_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR HOF typed setup helpers stay grouped by owner")

    hof_frame_fragmentation_violations = _mir_hof_frame_fragmentation_violations(root)
    if hof_frame_fragmentation_violations:
        h.check_fail(
            "MIR HOF typed frame wrappers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/hof_loop_frame.ark",
        )
        for rel, line_no, line in hof_frame_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR HOF typed frame wrappers stay folded into loop owners")

    hof_loop_step_fragmentation_violations = _mir_hof_loop_step_fragmentation_violations(root)
    if hof_loop_step_fragmentation_violations:
        h.check_fail(
            "MIR shared HOF loop-step helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/hof_loop_steps.ark",
        )
        for rel, line_no, line in hof_loop_step_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR shared HOF loop-step helpers stay grouped by owner")

    hof_result_fragmentation_violations = _mir_hof_result_fragmentation_violations(root)
    if hof_result_fragmentation_violations:
        h.check_fail(
            "MIR shared HOF result helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/hof_results.ark",
        )
        for rel, line_no, line in hof_result_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR shared HOF result helpers stay grouped by owner")

    hof_loop_scaffolding_fragmentation_violations = _mir_hof_loop_scaffolding_fragmentation_violations(root)
    if hof_loop_scaffolding_fragmentation_violations:
        h.check_fail(
            "MIR shared HOF loop scaffolding helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/hof_loop.ark",
        )
        for rel, line_no, line in hof_loop_scaffolding_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR shared HOF loop scaffolding helpers stay grouped by owner")

    binary_opcode_fragmentation_violations = _mir_binary_opcode_fragmentation_violations(root)
    if binary_opcode_fragmentation_violations:
        h.check_fail(
            "MIR binary opcode mapping helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/binary_opcode.ark",
        )
        for rel, line_no, line in binary_opcode_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR binary opcode mapping helpers stay grouped by owner")

    binary_emit_fragmentation_violations = _mir_binary_emit_fragmentation_violations(root)
    if binary_emit_fragmentation_violations:
        h.check_fail(
            "MIR binary emission helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/binary_emit.ark",
        )
        for rel, line_no, line in binary_emit_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR binary emission helpers stay grouped by owner")

    body_dispatch_fragmentation_violations = _mir_body_dispatch_fragmentation_violations(root)
    if body_dispatch_fragmentation_violations:
        h.check_fail(
            "MIR body dispatch helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/body_dispatch_expr.ark",
        )
        for rel, line_no, line in body_dispatch_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR body dispatch helpers stay grouped by owner")

    body_aggregate_fragmentation_violations = _mir_body_aggregate_fragmentation_violations(root)
    if body_aggregate_fragmentation_violations:
        h.check_fail(
            "MIR body aggregate helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/body_aggregate.ark",
        )
        for rel, line_no, line in body_aggregate_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR body aggregate helpers stay grouped by owner")

    body_control_fragmentation_violations = _mir_body_control_fragmentation_violations(root)
    if body_control_fragmentation_violations:
        h.check_fail(
            "MIR body control helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/body_control.ark",
        )
        for rel, line_no, line in body_control_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR body control helpers stay grouped by owner")

    body_unary_fragmentation_violations = _mir_body_unary_fragmentation_violations(root)
    if body_unary_fragmentation_violations:
        h.check_fail(
            "MIR body unary helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/body_binary.ark",
        )
        for rel, line_no, line in body_unary_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR body unary helpers stay grouped by owner")

    body_match_fragmentation_violations = _mir_body_match_fragmentation_violations(root)
    if body_match_fragmentation_violations:
        h.check_fail(
            "MIR body match helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/body_match.ark",
        )
        for rel, line_no, line in body_match_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR body match helpers stay grouped by owner")

    body_stmt_fragmentation_violations = _mir_body_stmt_fragmentation_violations(root)
    if body_stmt_fragmentation_violations:
        h.check_fail(
            "MIR body statement helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/body_stmt.ark",
        )
        for rel, line_no, line in body_stmt_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR body statement helpers stay grouped by owner")

    core_match_dispatch_fragmentation_violations = _mir_core_match_dispatch_fragmentation_violations(root)
    if core_match_dispatch_fragmentation_violations:
        h.check_fail(
            "MIR CoreHIR match dispatch helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/core_match.ark",
        )
        for rel, line_no, line in core_match_dispatch_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR CoreHIR match dispatch helpers stay grouped by owner")

    core_control_fragmentation_violations = _mir_core_control_fragmentation_violations(root)
    if core_control_fragmentation_violations:
        h.check_fail(
            "MIR CoreHIR control helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/core_control.ark",
        )
        for rel, line_no, line in core_control_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR CoreHIR control helpers stay grouped by owner")

    core_ops_fragmentation_violations = _mir_core_ops_fragmentation_violations(root)
    if core_ops_fragmentation_violations:
        h.check_fail(
            "MIR CoreHIR operator helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/core_dispatch_ops.ark",
        )
        for rel, line_no, line in core_ops_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR CoreHIR operator helpers stay grouped by owner")

    core_call_fragmentation_violations = _mir_core_call_fragmentation_violations(root)
    if core_call_fragmentation_violations:
        h.check_fail(
            "MIR CoreHIR call facades are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/core_dispatch_call.ark",
        )
        for rel, line_no, line in core_call_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR CoreHIR call facades stay grouped by owner")

    entry_body_source_fragmentation_violations = _mir_entry_body_source_fragmentation_violations(root)
    if entry_body_source_fragmentation_violations:
        h.check_fail(
            "MIR entry body-source helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/entry_body_source.ark",
        )
        for rel, line_no, line in entry_body_source_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR entry body-source helpers stay grouped by owner")

    entry_core_body_fragmentation_violations = _mir_entry_core_body_fragmentation_violations(root)
    if entry_core_body_fragmentation_violations:
        h.check_fail(
            "MIR entry CoreHIR body helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/entry_body_core_value.ark",
        )
        for rel, line_no, line in entry_core_body_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR entry CoreHIR body helpers stay grouped by owner")

    entry_emit_fragmentation_violations = _mir_entry_emit_fragmentation_violations(root)
    if entry_emit_fragmentation_violations:
        h.check_fail(
            "MIR entry emission facades are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/entry_fns_mono.ark",
        )
        for rel, line_no, line in entry_emit_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR entry emission facades stay grouped by owner")

    entry_params_fragmentation_violations = _mir_entry_params_fragmentation_violations(root)
    if entry_params_fragmentation_violations:
        h.check_fail(
            "MIR entry parameter helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/entry_params.ark",
        )
        for rel, line_no, line in entry_params_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR entry parameter helpers stay grouped by owner")

    return_type_fragmentation_violations = _mir_return_type_fragmentation_violations(root)
    if return_type_fragmentation_violations:
        h.check_fail(
            "MIR return type helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/return_types.ark",
        )
        for rel, line_no, line in return_type_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR return type helpers stay grouped by owner")

    method_fragmentation_violations = _mir_method_fragmentation_violations(root)
    if method_fragmentation_violations:
        h.check_fail(
            "MIR method call helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/method.ark",
        )
        for rel, line_no, line in method_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR method call helpers stay grouped by owner")

    registry_fragmentation_violations = _mir_registry_fragmentation_violations(root)
    if registry_fragmentation_violations:
        h.check_fail(
            "MIR layout registry facades are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/registry_view.ark",
        )
        for rel, line_no, line in registry_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR layout registry facades stay grouped by owner")

    ctx_fn_return_vt_fragmentation_violations = _mir_ctx_fn_return_vt_fragmentation_violations(root)
    if ctx_fn_return_vt_fragmentation_violations:
        h.check_fail(
            "MIR function return-vt helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/ctx_fn_return_vt.ark",
        )
        for rel, line_no, line in ctx_fn_return_vt_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR function return-vt helpers stay grouped by owner")

    struct_lit_fragmentation_violations = _mir_struct_lit_fragmentation_violations(root)
    if struct_lit_fragmentation_violations:
        h.check_fail(
            "MIR struct literal helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/struct_lit.ark",
        )
        for rel, line_no, line in struct_lit_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR struct literal helpers stay grouped by owner")

    variant_payload_fragmentation_violations = _mir_variant_payload_fragmentation_violations(root)
    if variant_payload_fragmentation_violations:
        h.check_fail(
            "MIR variant payload helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/variant_payload.ark",
        )
        for rel, line_no, line in variant_payload_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR variant payload helpers stay grouped by owner")

    core_match_payload_info_fragmentation_violations = _mir_core_match_payload_info_fragmentation_violations(root)
    if core_match_payload_info_fragmentation_violations:
        h.check_fail(
            "MIR CoreHIR match payload info helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/core_match_payload_info.ark",
        )
        for rel, line_no, line in core_match_payload_info_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR CoreHIR match payload info helpers stay grouped by owner")

    match_payload_info_fragmentation_violations = _mir_match_payload_info_fragmentation_violations(root)
    if match_payload_info_fragmentation_violations:
        h.check_fail(
            "MIR legacy match payload info helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/match_payload_prepare.ark",
        )
        for rel, line_no, line in match_payload_info_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR legacy match payload info helpers stay grouped by owner")

    match_payload_helper_fragmentation_violations = _mir_match_payload_helper_fragmentation_violations(root)
    if match_payload_helper_fragmentation_violations:
        h.check_fail(
            "MIR legacy match payload bind helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/match_payload_bind.ark",
        )
        for rel, line_no, line in match_payload_helper_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR legacy match payload bind helpers stay grouped by owner")

    comprehension_type_fragmentation_violations = _mir_comprehension_type_fragmentation_violations(root)
    if comprehension_type_fragmentation_violations:
        h.check_fail(
            "MIR comprehension type helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/comprehension_begin.ark",
        )
        for rel, line_no, line in comprehension_type_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR comprehension type helpers stay grouped by owner")

    comprehension_emit_fragmentation_violations = _mir_comprehension_emit_fragmentation_violations(root)
    if comprehension_emit_fragmentation_violations:
        h.check_fail(
            "MIR comprehension emission helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/comprehension_emit.ark",
        )
        for rel, line_no, line in comprehension_emit_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR comprehension emission helpers stay grouped by owner")

    core_match_payload_type_fragmentation_violations = _mir_core_match_payload_type_fragmentation_violations(root)
    if core_match_payload_type_fragmentation_violations:
        h.check_fail(
            "MIR CoreHIR match payload type helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/core_match_payload_bind_core.ark",
        )
        for rel, line_no, line in core_match_payload_type_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR CoreHIR match payload type helpers stay grouped by owner")

    stmt_let_type_fragmentation_violations = _mir_stmt_let_type_fragmentation_violations(root)
    if stmt_let_type_fragmentation_violations:
        h.check_fail(
            "MIR let type decision helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir/lower/stmt_let.ark",
        )
        for rel, line_no, line in stmt_let_type_fragmentation_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR let type decision helpers stay grouped by owner")

    core_lowering_violations = _mir_core_lowering_boundary_violations(root)
    if core_lowering_violations:
        h.check_fail(
            "CoreHIR MIR body lowering leaks frontend or legacy body nodes",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir_lower_core_body.ark",
        )
        for rel, line_no, line in core_lowering_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("CoreHIR MIR body lowering excludes frontend and legacy body nodes")

    root_layout_violations = _compiler_root_layout_violations(root)
    if root_layout_violations:
        h.check_fail(
            "compiler root contains role-specific implementation files",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/",
        )
        print(f"  root-level implementation files: {len(root_layout_violations)}")
        for rel in root_layout_violations[:30]:
            print(f"  {rel}")
    else:
        h.check_pass("compiler root contains only entrypoint/facade files")

    namespace_layout_violations = _compiler_namespace_layout_violations(root)
    if namespace_layout_violations:
        h.check_fail(
            "compiler namespace directories are incomplete",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/",
        )
        for violation in namespace_layout_violations[:30]:
            print(f"  {violation}")
    else:
        h.check_pass("compiler subsystems are organized into namespace directories")

    public_boundary_violations = _compiler_public_boundary_violations(root)
    if public_boundary_violations:
        h.check_fail(
            "compiler subsystem public boundaries are bypassed",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/",
        )
        for rel, line_no, line in public_boundary_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("compiler subsystem public boundaries go through mod.ark facades")

    mir_corehir_view_violations = _mir_corehir_view_boundary_violations(root)
    if mir_corehir_view_violations:
        h.check_fail(
            "MIR lowering bypasses the CoreHIR public view facade",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/corehir/mod.ark",
        )
        for rel, line_no, line in mir_corehir_view_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR lowering consumes CoreHIR view through public facade")

    component_wit_decl_violations = _component_wit_decl_boundary_violations(root)
    if component_wit_decl_violations:
        h.check_fail(
            "component WIT generation bypasses declaration view",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/wit_decl.ark",
        )
        for rel, line_no, line in component_wit_decl_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component WIT generation stays behind declaration view")

    component_type_node_violations = _component_type_node_boundary_violations(root)
    if component_type_node_violations:
        h.check_fail(
            "component type predicates bypass type-node view",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/type_node.ark",
        )
        for rel, line_no, line in component_type_node_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component type predicates stay behind type-node view")

    component_contract_helper_violations = _component_contract_helper_boundary_violations(root)
    if component_contract_helper_violations:
        h.check_fail(
            "component contract files bypass helper views",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/contract_helpers.ark",
        )
        for rel, line_no, line in component_contract_helper_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component contract files use function-decl helper views")

    component_ast_node_adapter_violations = _component_ast_node_adapter_boundary_violations(root)
    if component_ast_node_adapter_violations:
        h.check_fail(
            "component files import AST outside approved adapters",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/ast_node.ark",
        )
        for rel, line_no, line in component_ast_node_adapter_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component AST access stays behind approved adapters")

    component_export_func_violations = _component_export_func_boundary_violations(root)
    if component_export_func_violations:
        h.check_fail(
            "component function predicates bypass export function view",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/contract_helpers.ark",
        )
        for rel, line_no, line in component_export_func_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component function predicates stay behind export function view")

    component_string_predicate_violations = _component_string_predicate_fragmentation_violations(root)
    if component_string_predicate_violations:
        h.check_fail(
            "component string predicates are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/contract_string_fn.ark",
        )
        for rel, line_no, line in component_string_predicate_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component string predicates stay grouped by domain")

    component_record_predicate_violations = _component_record_predicate_fragmentation_violations(root)
    if component_record_predicate_violations:
        h.check_fail(
            "component record predicates are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/contract_record_fn.ark",
        )
        for rel, line_no, line in component_record_predicate_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component record predicates stay grouped by domain")

    component_record_decl_violations = _component_record_decl_fragmentation_violations(root)
    if component_record_decl_violations:
        h.check_fail(
            "component record declaration shapes are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/contract_record_decl.ark",
        )
        for rel, line_no, line in component_record_decl_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component record declaration shapes stay grouped by domain")

    component_collection_predicate_violations = _component_collection_predicate_fragmentation_violations(root)
    if component_collection_predicate_violations:
        h.check_fail(
            "component collection predicates are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/contract_collection_fn.ark",
        )
        for rel, line_no, line in component_collection_predicate_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component collection predicates stay grouped by domain")

    component_collection_contract_violations = _component_collection_contract_fragmentation_violations(root)
    if component_collection_contract_violations:
        h.check_fail(
            "component option/result contracts are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/contract_collection_option.ark",
        )
        for rel, line_no, line in component_collection_contract_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component option/result contracts stay grouped by domain")

    component_tiny_facade_violations = _component_tiny_facade_fragmentation_violations(root)
    if component_tiny_facade_violations:
        h.check_fail(
            "component one-function facades are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/export_plan.ark",
        )
        for rel, line_no, line in component_tiny_facade_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component one-function facades stay folded into owners")

    component_list_adapter_violations = _component_list_adapter_fragmentation_violations(root)
    if component_list_adapter_violations:
        h.check_fail(
            "component list adapters are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/adapters_list_input.ark",
        )
        for rel, line_no, line in component_list_adapter_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component list adapters stay grouped by adapter")

    component_shared_adapter_section_violations = _component_shared_adapter_section_boundary_violations(root)
    if component_shared_adapter_section_violations:
        h.check_fail(
            "component shared adapter section helpers leak outside approved owners",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/adapters_single_export_sections.ark",
        )
        for rel, line_no, line in component_shared_adapter_section_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component shared adapter section helpers stay behind approved owners")

    component_string_adapter_violations = _component_string_adapter_fragmentation_violations(root)
    if component_string_adapter_violations:
        h.check_fail(
            "component string adapters are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/adapters_string.ark",
        )
        for rel, line_no, line in component_string_adapter_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component string adapters stay grouped by adapter")

    component_numeric_adapter_violations = _component_numeric_adapter_fragmentation_violations(root)
    if component_numeric_adapter_violations:
        h.check_fail(
            "component numeric adapters are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/adapters_numeric.ark",
        )
        for rel, line_no, line in component_numeric_adapter_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component numeric adapters stay grouped by adapter")

    component_shape_adapter_violations = _component_shape_adapter_fragmentation_violations(root)
    if component_shape_adapter_violations:
        h.check_fail(
            "component shape adapters are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/adapters_shape_roundtrip.ark",
        )
        for rel, line_no, line in component_shape_adapter_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component shape adapters stay grouped by adapter")

    component_result_string_adapter_violations = _component_result_string_adapter_fragmentation_violations(root)
    if component_result_string_adapter_violations:
        h.check_fail(
            "component result/string adapters are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/adapters_result_string.ark",
        )
        for rel, line_no, line in component_result_string_adapter_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component result/string adapters stay grouped by adapter")

    component_option_adapter_violations = _component_option_adapter_fragmentation_violations(root)
    if component_option_adapter_violations:
        h.check_fail(
            "component option adapters are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/adapters_option.ark",
        )
        for rel, line_no, line in component_option_adapter_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component option adapters stay grouped by adapter")

    component_pair_input_adapter_violations = _component_pair_input_adapter_fragmentation_violations(root)
    if component_pair_input_adapter_violations:
        h.check_fail(
            "component pair-input adapters are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/adapters_tuple.ark",
        )
        for rel, line_no, line in component_pair_input_adapter_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component pair-input adapters stay grouped by adapter")

    component_record_adapter_violations = _component_record_adapter_fragmentation_violations(root)
    if component_record_adapter_violations:
        h.check_fail(
            "component record adapters are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/adapters_record.ark",
        )
        for rel, line_no, line in component_record_adapter_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component record adapters stay grouped by adapter")

    component_wrapper_plan_violations = _component_wrapper_plan_fragmentation_violations(root)
    if component_wrapper_plan_violations:
        h.check_fail(
            "component wrapper adapter plans are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/emit_list.ark",
        )
        for rel, line_no, line in component_wrapper_plan_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component wrapper adapter plans stay grouped by wrapper")

    component_wrapper_section_violations = _component_wrapper_section_fragmentation_violations(root)
    if component_wrapper_section_violations:
        h.check_fail(
            "component wrapper section facades are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/emit_list.ark",
        )
        for rel, line_no, line in component_wrapper_section_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component wrapper section facades stay grouped by wrapper")

    component_record_point_emitter_violations = _component_record_point_emitter_fragmentation_violations(root)
    if component_record_point_emitter_violations:
        h.check_fail(
            "component record-point emitter is over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/emit_record_ops.ark",
        )
        for rel, line_no, line in component_record_point_emitter_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component record-point emitter stays grouped by adapter")

    component_record_ops_emitter_violations = _component_record_ops_emitter_fragmentation_violations(root)
    if component_record_ops_emitter_violations:
        h.check_fail(
            "component record operation emitter is over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/component/emit_record_ops.ark",
        )
        for rel, line_no, line in component_record_ops_emitter_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("component record operation emitter stays grouped by adapter")

    const_layout_violations = _compiler_constant_function_layout_violations(root)
    if const_layout_violations:
        h.check_fail(
            "compiler constant-like functions live outside dedicated table modules",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/",
        )
        for rel, line_no, line in const_layout_violations[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("compiler constant-like functions stay in dedicated table modules")

    fragmented_tables = _compiler_fragmented_constant_table_violations(root)
    if fragmented_tables:
        h.check_fail(
            "compiler constant tables are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/",
        )
        for rel, const_count in fragmented_tables[:30]:
            print(f"  {rel}: {const_count} constant functions")
    else:
        h.check_pass("compiler constant tables are grouped at subsystem scale")

    diagnostics_code_tables = _diagnostics_code_table_fragmentation_violations(root)
    if diagnostics_code_tables:
        h.check_fail(
            "diagnostics code table is over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/diagnostics/codes.ark",
        )
        for rel, line_no, line in diagnostics_code_tables[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("diagnostics code table stays consolidated")

    diagnostics_json_modules = _diagnostics_json_fragmentation_violations(root)
    if diagnostics_json_modules:
        h.check_fail(
            "diagnostics JSON helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/diagnostics/json.ark",
        )
        for rel, line_no, line in diagnostics_json_modules[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("diagnostics JSON helpers stay consolidated")

    diagnostics_severity_modules = _diagnostics_severity_fragmentation_violations(root)
    if diagnostics_severity_modules:
        h.check_fail(
            "diagnostics severity helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/diagnostics/severity.ark",
        )
        for rel, line_no, line in diagnostics_severity_modules[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("diagnostics severity helpers stay consolidated")

    diagnostics_phase_modules = _diagnostics_phase_fragmentation_violations(root)
    if diagnostics_phase_modules:
        h.check_fail(
            "diagnostics phase helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/diagnostics/",
        )
        for rel, line_no, line in diagnostics_phase_modules[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("diagnostics phase helpers stay consolidated")

    diagnostics_span_modules = _diagnostics_span_fragmentation_violations(root)
    if diagnostics_span_modules:
        h.check_fail(
            "diagnostics span helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/diagnostics/span_record.ark",
        )
        for rel, line_no, line in diagnostics_span_modules[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("diagnostics span helpers stay consolidated")

    diagnostics_record_modules = _diagnostics_record_fragmentation_violations(root)
    if diagnostics_record_modules:
        h.check_fail(
            "diagnostics record helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/diagnostics/record.ark",
        )
        for rel, line_no, line in diagnostics_record_modules[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("diagnostics record helpers stay consolidated")

    diagnostics_source_map_modules = _diagnostics_source_map_fragmentation_violations(root)
    if diagnostics_source_map_modules:
        h.check_fail(
            "diagnostics source-map helpers are over-fragmented",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/diagnostics/source_map_offsets.ark",
        )
        for rel, line_no, line in diagnostics_source_map_modules[:30]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("diagnostics source-map helpers stay consolidated")

    size_violations = _compiler_file_size_violations(root)
    if size_violations:
        h.check_fail(
            "compiler Ark files exceed 249 line context limit",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/",
        )
        for rel, line_count in size_violations[:20]:
            print(f"  {rel}: {line_count} lines")
    else:
        h.check_pass("compiler Ark files stay under configured line limits")

    function_size_violations = _compiler_function_size_violations(root)
    if function_size_violations:
        h.check_fail(
            "compiler Ark functions exceed 60 line context limit",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/",
        )
        for rel, line_no, line_count, signature in function_size_violations[:20]:
            print(f"  {rel}:{line_no}: {line_count} lines: {signature}")
    else:
        h.check_pass("compiler Ark functions stay under 60 lines")

    api_violations = _compiler_public_api_violations(root)
    if api_violations:
        h.check_fail(
            "compiler Ark files expose too many public functions",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/",
        )
        for rel, pub_count in api_violations[:20]:
            print(f"  {rel}: {pub_count} pub fn")
    else:
        h.check_pass("compiler Ark public API count stays bounded")

    direction_violations = _compiler_dependency_direction_violations(root)
    if direction_violations:
        h.check_fail(
            "compiler dependency direction gate",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/",
        )
        for rel, line_no, line in direction_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("compiler dependency direction stays layered")

    import_cycle_violations = _compiler_import_cycle_violations(root)
    if import_cycle_violations:
        h.check_fail(
            "compiler import graph has cycles",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/",
        )
        for cycle in import_cycle_violations[:10]:
            print("  " + " -> ".join(cycle))
    else:
        h.check_pass("compiler import graph is acyclic")

    import_fanout_violations = _compiler_import_fanout_violations(root)
    if import_fanout_violations:
        h.check_fail(
            "compiler Ark files exceed direct import fan-out limit",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/",
        )
        for rel, import_count in import_fanout_violations[:20]:
            print(f"  {rel}: {import_count} imports")
    else:
        h.check_pass("compiler Ark direct import fan-out stays bounded")

    test_reachability_violations = _compiler_production_test_reachability_violations(root)
    if test_reachability_violations:
        h.check_fail(
            "compiler production graph reaches test-only modules",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/main.ark",
        )
        for rel in test_reachability_violations[:20]:
            print(f"  {rel}")
    else:
        h.check_pass("compiler production graph excludes test-only modules")

    legacy_body_violations = _mir_legacy_body_boundary_violations(root)
    if legacy_body_violations:
        h.check_fail(
            "MIR legacy body lowering is used outside adapter",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir_lower_legacy_body_lower.ark",
        )
        for rel, line_no, line in legacy_body_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR legacy body lowering stays behind adapter")

    entry_fallback_violations = _mir_entry_fallback_source_boundary_violations(root)
    if entry_fallback_violations:
        h.check_fail(
            "MIR entry fallback source leaks outside body-source adapter",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir_lower_entry_body_source.ark",
        )
        for rel, line_no, line in entry_fallback_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR entry fallback source stays behind body-source adapter")

    body_value_partition_violations = _mir_entry_body_value_partition_violations(root)
    if body_value_partition_violations:
        h.check_fail(
            "MIR entry CoreHIR/fallback body value lowering is cross-coupled",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir_lower_entry_body_value.ark",
        )
        for rel, line_no, line in body_value_partition_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR entry body value lowering keeps CoreHIR and fallback paths split")

    legacy_decl_violations = _mir_fallback_source_legacy_decl_boundary_violations(root)
    if legacy_decl_violations:
        h.check_fail(
            "MIR fallback source leaks legacy decl shape",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir_lower_legacy_decl.ark",
        )
        for rel, line_no, line in legacy_decl_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR fallback source uses legacy decl adapter only")

    driver_legacy_decl_violations = _driver_legacy_decl_adapter_violations(root)
    if driver_legacy_decl_violations:
        h.check_fail(
            "driver legacy decl conversion leaks outside fallback adapter",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/driver_lower_fallback_source.ark",
        )
        for rel, line_no, line in driver_legacy_decl_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("driver legacy decl conversion stays in fallback adapter")

    lower_input_violations = _mir_lower_input_contract_violations(root)
    if lower_input_violations:
        h.check_fail(
            "MIR lowering facade/callers bypass bundled input contract",
            category="compiler-boundary",
            command="python3 scripts/manager.py verify quick",
            primary_path="src/compiler/mir_lower_input.ark",
        )
        for rel, line_no, line in lower_input_violations[:20]:
            print(f"  {rel}:{line_no}: {line}")
    else:
        h.check_pass("MIR lowering facade/callers use bundled input contract")

    h.check_pass("Perf policy documented (check<=10%, compile<=20%; heavy perf separated)")

    # ── Stdlib fixture registration checks ───────────────────────────────────
    fixtures_root = root / "tests" / "fixtures"
    manifest_text = manifest_file.read_text(encoding="utf-8") if manifest_file.exists() else ""

    stdlib_missing = 0
    for stdlib_dir in sorted(fixtures_root.glob("stdlib_*")):
        if not stdlib_dir.is_dir():
            continue
        for ark in sorted(stdlib_dir.glob("*.ark")):
            rel_path = str(ark.relative_to(fixtures_root))
            if rel_path not in manifest_text:
                print(f"  Missing from manifest.txt: {rel_path}")
                stdlib_missing += 1
    if stdlib_missing == 0:
        h.check_pass("all stdlib fixtures registered in manifest.txt")
    else:
        h.check_fail(
            f"stdlib fixtures missing from manifest.txt ({stdlib_missing})",
            category="fixture",
            command="python3 scripts/manager.py verify quick",
            primary_path="tests/fixtures/manifest.txt",
        )

    stdlib_fixture_count = manifest_text.count("stdlib_")
    if stdlib_fixture_count >= 5:
        h.check_pass(f"v3 stdlib fixtures registered ({stdlib_fixture_count} entries in manifest)")
    else:
        h.check_fail(
            f"v3 stdlib fixtures insufficient ({stdlib_fixture_count} < 5)",
            category="fixture",
            command="python3 scripts/manager.py verify quick",
            primary_path="tests/fixtures/manifest.txt",
        )

    # ── Internal link integrity ───────────────────────────────────────────────
    links_script = root / "scripts" / "check" / "check-links.sh"
    if links_script.exists():
        rc, _, _ = _run(["bash", str(links_script)], cwd=root, dry_run=dry_run)
        if rc == 0:
            h.check_pass("internal link integrity")
        else:
            h.check_fail(
                "broken internal links detected (run scripts/check/check-links.sh)",
                category="docs",
                command="bash scripts/check/check-links.sh",
                primary_path="docs/",
            )

    # ── Diagnostic codes check ────────────────────────────────────────────────
    diag_script = root / "scripts" / "check" / "check-diagnostic-codes.sh"
    if diag_script.exists():
        rc, _, _ = _run(["bash", str(diag_script)], cwd=root, dry_run=dry_run)
        if rc == 0:
            h.check_pass("diagnostic codes aligned")
        else:
            h.check_fail(
                "diagnostic codes out of sync (run scripts/check/check-diagnostic-codes.sh)",
                category="diagnostics-snapshot",
                command="bash scripts/check/check-diagnostic-codes.sh",
                primary_path="src/compiler/diagnostics.ark",
            )

    # ── Summary ───────────────────────────────────────────────────────────────
    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}========================================{NC}")
    print(f"{YELLOW}Summary{NC}")
    print(f"{YELLOW}========================================{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")

    if failed == 0:
        print(f"\n{GREEN}\u2713 All selected harness checks passed{NC}")
    else:
        print(f"\n{RED}\u2717 Some harness checks failed ({failed} checks failed){NC}")

    return h.exit_code()


def cmd_verify_fixtures(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[fixtures] Running selfhost fixture parity...{NC}")

    if dry_run:
        print("DRY-RUN: cmd_verify_fixtures (selfhost fixture-parity)")
        h.check_pass("selfhost fixture parity (dry-run)")
        total, passed, skipped, failed = h.summary()
        print(f"\n{YELLOW}Summary{NC}")
        print(f"Total checks: {total}")
        print(f"Passed: {GREEN}{passed}{NC}")
        print(f"Skipped: {YELLOW}{skipped}{NC}")
        print(f"Failed: {RED}{failed}{NC}")
        return h.exit_code()

    rc, output = run_fixture_parity(root, dry_run=False)
    print(output)

    if rc == 0:
        h.check_pass("selfhost fixture parity")
    else:
        h.check_fail(
            "selfhost fixture parity",
            category="fixture",
            command="python3 scripts/manager.py selfhost parity --mode --fixture",
            primary_path="tests/fixtures/manifest.txt",
        )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_verify_size(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[size] Checking hello.wasm binary size gate...{NC}")

    arukellt_bin = os.environ.get("ARUKELLT_BIN", "")
    if not arukellt_bin:
        selfhost = root / "scripts" / "run" / "arukellt-selfhost.sh"
        debug = root / "target" / "debug" / "arukellt"
        release = root / "target" / "release" / "arukellt"
        if selfhost.exists():
            arukellt_bin = str(selfhost)
        elif release.exists():
            arukellt_bin = str(release)
        elif debug.exists():
            arukellt_bin = str(debug)

    HELLO_WASM_OUT = "hello_perfgate.wasm"
    HELLO_SIZE_MAX = 5120

    if dry_run:
        print(f"DRY-RUN: would compile tests/fixtures/hello/hello.ark via {arukellt_bin!r}")
        h.check_pass("hello.wasm binary size (dry-run)")
        total, passed, skipped, failed = h.summary()
        print(f"\n{YELLOW}Summary{NC}")
        print(f"Total checks: {total}")
        print(f"Passed: {GREEN}{passed}{NC}")
        print(f"Skipped: {YELLOW}{skipped}{NC}")
        print(f"Failed: {RED}{failed}{NC}")
        return h.exit_code()

    if not arukellt_bin:
        h.check_fail(
            "hello.wasm size gate (arukellt binary not found — build first)",
            category="target-contract",
            command="python3 scripts/manager.py verify size",
            primary_path="tests/fixtures/hello/hello.ark",
        )
    else:
        compile_cmd = [
            arukellt_bin,
            "compile",
            "tests/fixtures/hello/hello.ark",
            "--target", "wasm32-wasi-p2",
            "--opt-level", "1",
            "-o", HELLO_WASM_OUT,
        ]
        result = subprocess.run(compile_cmd, cwd=str(root), capture_output=True)
        wasm_path = root / HELLO_WASM_OUT
        try:
            if result.returncode == 0 and wasm_path.exists():
                size = wasm_path.stat().st_size
                wasm_path.unlink(missing_ok=True)
                if size <= HELLO_SIZE_MAX:
                    h.check_pass(f"hello.wasm binary size: {size} bytes (<= {HELLO_SIZE_MAX})")
                else:
                    h.check_fail(
                        f"hello.wasm binary size: {size} bytes (> {HELLO_SIZE_MAX} threshold)",
                        category="target-contract",
                        command=" ".join(compile_cmd),
                        primary_path="tests/fixtures/hello/hello.ark",
                    )
            else:
                wasm_path.unlink(missing_ok=True)
                h.check_fail(
                    "hello.wasm compilation failed",
                    category="target-contract",
                    command=" ".join(compile_cmd),
                    primary_path="tests/fixtures/hello/hello.ark",
                )
        except Exception:
            wasm_path.unlink(missing_ok=True)
            h.check_fail(
                "hello.wasm compilation failed",
                category="target-contract",
                command=" ".join(compile_cmd),
                primary_path="tests/fixtures/hello/hello.ark",
            )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_verify_wat(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[wat] Running WAT roundtrip verification...{NC}")

    rc, _, _ = _run(
        ["bash", "scripts/run/wat-roundtrip.sh"],
        cwd=root,
        dry_run=dry_run,
    )
    if rc == 0:
        h.check_pass("WAT roundtrip (wasm2wat \u21c4 wat2wasm)")
    else:
        h.check_fail(
            "WAT roundtrip (wasm2wat \u21c4 wat2wasm)",
            category="target-contract",
            command="bash scripts/run/wat-roundtrip.sh",
            primary_path="scripts/run/wat-roundtrip.sh",
        )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_verify_component(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[component] Component interop smoke test...{NC}")

    # Check for wasmtime
    wasmtime_check = subprocess.run(
        ["which", "wasmtime"], capture_output=True
    )
    if wasmtime_check.returncode != 0:
        h.check_skip("component interop (wasmtime not found)")
    else:
        interop_dir = root / "tests" / "component-interop" / "jco"
        run_scripts = sorted(interop_dir.glob("*/run.sh"))
        if not run_scripts:
            h.check_skip("component interop scripts not found")
        else:
            component_env = os.environ.copy()
            pinned_selfhost = root / "bootstrap" / "arukellt-selfhost.wasm"
            if "ARUKELLT_SELFHOST_WASM" not in component_env and pinned_selfhost.exists():
                component_env["ARUKELLT_SELFHOST_WASM"] = str(pinned_selfhost)
            for run_sh in run_scripts:
                fixture_name = run_sh.parent.name
                rc, _, _ = _run_env(
                    ["bash", str(run_sh)],
                    cwd=root,
                    dry_run=dry_run,
                    env=component_env,
                )
                if rc == 0:
                    h.check_pass(f"component interop: {fixture_name} (wasmtime)")
                else:
                    h.check_fail(
                        f"component interop: {fixture_name} (wasmtime)",
                        category="component-interop",
                        command=f"bash {run_sh}",
                        primary_path=str(run_sh.relative_to(root)),
                    )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_verify_selfhost_parity(args: argparse.Namespace) -> int:
    """Aggregate selfhost parity gate (#530): CLI parity + diagnostic parity.

    Delegates to the existing ``selfhost parity --mode --cli`` and
    ``selfhost diag-parity`` runners without modifying their behavior. Returns
    non-zero if either underlying check fails.
    """
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[selfhost-parity] CLI parity + diagnostic parity gates (#530)...{NC}")

    rc_cli, out_cli = run_parity(root, dry_run, mode="--cli")
    if rc_cli == 0:
        h.check_pass("selfhost CLI parity")
    else:
        h.check_fail(
            "selfhost CLI parity",
            category="bootstrap",
            command="python3 scripts/manager.py selfhost parity --mode --cli",
            primary_path="tests/snapshots/selfhost/",
        )
        for line in out_cli.splitlines()[-30:]:
            print(line)

    rc_diag, out_diag = run_diag_parity(root, dry_run)
    if rc_diag == 0:
        h.check_pass("selfhost diagnostic parity")
    else:
        h.check_fail(
            "selfhost diagnostic parity",
            category="bootstrap",
            command="python3 scripts/manager.py selfhost diag-parity",
            primary_path="tests/fixtures/",
        )
        for line in out_diag.splitlines()[-30:]:
            print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


# ── CLI wiring ────────────────────────────────────────────────────────────────

# Flags not yet migrated to manager.py (Phase 1 out-of-scope).
_VERIFY_OUT_OF_SCOPE_FLAGS = {
    "--baseline", "--fixpoint", "--selfhost-fixture-parity",
    "--selfhost-diag-parity", "--lsp-perf", "--memory-gate", "--repro",
    "--opt-equiv", "--perf-gate",
}


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="manager.py",
        description="Arukellt scripts manager",
    )
    parser.add_argument("--dry-run", action="store_true", help="Print intent but do not execute.")

    sub_domain = parser.add_subparsers(dest="domain", metavar="<domain>")
    sub_domain.required = True

    # verify domain
    verify_parser = sub_domain.add_parser("verify", help="Verification commands")


    verify_parser.add_argument("--dry-run", action="store_true", help="Print intent but do not execute.")
    verify_parser.add_argument("--quick",     action="store_true", help="Run the fast local gate checks (default)")
    verify_parser.add_argument("--fixtures",  action="store_true", help="Run the manifest-driven fixture harness")
    verify_parser.add_argument("--size",      action="store_true", help="Run the hello.wasm binary size gate")
    verify_parser.add_argument("--wat",       action="store_true", help="Run the WAT roundtrip gate")
    verify_parser.add_argument("--component", action="store_true", help="Run the component interop smoke test")
    verify_parser.add_argument("--docs",      action="store_true", help="[Phase 1 stub] skipped — not yet migrated")
    verify_parser.add_argument(
        "--selfhost-parity", action="store_true",
        help="Run selfhost CLI parity + diagnostic parity gates (#530)",
    )
    verify_parser.add_argument(
        "--full", action="store_true",
        help="Run quick + fixtures + size + wat + component + selfhost-parity sequentially",
    )

    # ── Positional subcommand interface (legacy, preserved) ───────────────────
    sub_verify = verify_parser.add_subparsers(dest="subcommand", metavar="<subcommand>")
    sub_verify.required = False

    for name, help_text in [
        ("quick",     "Run the fast local gate checks"),
        ("fixtures",  "Run the manifest-driven fixture harness"),
        ("size",      "Run the hello.wasm binary size gate"),
        ("wat",       "Run the WAT roundtrip gate"),
        ("component", "Run the component interop smoke test"),
    ]:
        p = sub_verify.add_parser(name, help=help_text)
        p.add_argument("--dry-run", action="store_true", help="Print intent but do not execute.")

    _build_selfhost_subparser(sub_domain)
    _build_docs_subparser(sub_domain)
    _build_perf_subparser(sub_domain)
    _build_orchestration_subparser(sub_domain)
    _build_gate_subparser(sub_domain)

    return parser


def _build_selfhost_subparser(sub_domain: argparse._SubParsersAction) -> None:  # type: ignore[type-arg]
    sh = sub_domain.add_parser("selfhost", help="Selfhost check commands")
    sh.add_argument("--dry-run", action="store_true")
    sub = sh.add_subparsers(dest="subcommand", metavar="<subcommand>")
    sub.required = True
    p = sub.add_parser("fixpoint", help="Run selfhost fixpoint check")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--build", action="store_true", default=False, help="Build before check")
    for name, help_text in [
        ("fixture-parity", "Run selfhost fixture parity"),
        ("diag-parity", "Run selfhost diagnostic parity"),
    ]:
        q = sub.add_parser(name, help=help_text)
        q.add_argument("--dry-run", action="store_true")
    p_par = sub.add_parser("parity", help="Run selfhost parity (fixture/cli/diag)")
    p_par.add_argument("--dry-run", action="store_true")
    p_par.add_argument("--mode", choices=["", "--fixture", "--cli", "--diag"], default="")


def _build_docs_subparser(sub_domain: argparse._SubParsersAction) -> None:  # type: ignore[type-arg]
    dp = sub_domain.add_parser("docs", help="Documentation commands")
    dp.add_argument("--dry-run", action="store_true")
    sub = dp.add_subparsers(dest="subcommand", metavar="<subcommand>")
    sub.required = True
    for name, help_text in [("check", "Run docs checks"), ("regenerate", "Regenerate docs")]:
        p = sub.add_parser(name, help=help_text)
        p.add_argument("--dry-run", action="store_true")
    # regenerate extra flag
    sub.choices["regenerate"].add_argument(
        "--check-only", dest="check_only", action="store_true",
        help="Check only, do not write",
    )


def _build_perf_subparser(sub_domain: argparse._SubParsersAction) -> None:  # type: ignore[type-arg]
    pp = sub_domain.add_parser("perf", help="Performance commands")
    pp.add_argument("--dry-run", action="store_true")
    sub = pp.add_subparsers(dest="subcommand", metavar="<subcommand>")
    sub.required = True
    for name, help_text in [
        ("gate", "Run perf gate"),
        ("baseline", "Collect perf baseline"),
        ("benchmarks", "Run benchmarks"),
    ]:
        p = sub.add_parser(name, help=help_text)
        p.add_argument("--dry-run", action="store_true")
    sub.choices["gate"].add_argument("--update", action="store_true", help="Update baseline")
    sub.choices["benchmarks"].add_argument(
        "--no-quick", dest="no_quick", action="store_true", help="Full (not quick) benchmarks"
    )


def _build_orchestration_subparser(sub_domain: argparse._SubParsersAction) -> None:  # type: ignore[type-arg]
    op = sub_domain.add_parser("orchestration", help="Orchestration commands (autonomous dev)")
    op.add_argument("--dry-run", action="store_true")
    sub = op.add_subparsers(dest="subcommand", metavar="<subcommand>")
    sub.required = True
    for name, help_text in [
        ("agent-state", "Check agent worktree state"),
        ("issue-health", "Check issue metadata health"),
        ("repo-smoke", "Quick repository smoke check"),
        ("reference-coverage", "Generate reference coverage report (stub)"),
        ("gen-issues", "Generate issues from coverage gaps (stub)"),
    ]:
        p = sub.add_parser(name, help=help_text)
        p.add_argument("--dry-run", action="store_true")
    # agent-state extra: path argument
    sub.choices["agent-state"].add_argument(
        "worktree_path", nargs="?", help="Path to worktree directory"
    )
    sub.choices["agent-state"].add_argument(
        "--list-all", action="store_true", help="List all worktrees"
    )
    # reference-coverage extra flags
    sub.choices["reference-coverage"].add_argument(
        "--limit", type=int, default=100, help="Coverage limit"
    )
    sub.choices["reference-coverage"].add_argument(
        "--detail", action="store_true", help="Detailed output"
    )
    # gen-issues extra flags
    sub.choices["gen-issues"].add_argument(
        "--suite", default="test262", help="Reference suite name"
    )


def _build_gate_subparser(sub_domain: argparse._SubParsersAction) -> None:  # type: ignore[type-arg]
    gp = sub_domain.add_parser("gate", help="Gate checks")
    gp.add_argument("--dry-run", action="store_true")
    sub = gp.add_subparsers(dest="subcommand", metavar="<subcommand>")
    sub.required = True
    for name, help_text in [
        ("local", "Run full local CI gate"),
        ("pre-commit", "Run pre-commit verification"),
        ("pre-push", "Run pre-push verification"),
        ("repro", "Run reproducible build check"),
    ]:
        p = sub.add_parser(name, help=help_text)
        p.add_argument("--dry-run", action="store_true")
    sub.choices["local"].add_argument("--skip-ext", dest="skip_ext", action="store_true")
    for flag, dest, help_text in [
        ("--fixture", "fixture", "Fixture path"),
        ("--target", "target", "Target triple"),
    ]:
        sub.choices["repro"].add_argument(flag, dest=dest, default="", help=help_text)
    sub.choices["repro"].add_argument("--verbose", action="store_true")


def main() -> int:
    argv = list(sys.argv[1:])

    # Normalize selfhost parity mode values so documented invocations like
    # `selfhost parity --mode --cli` work under argparse.
    if len(argv) >= 4 and argv[0] == "selfhost" and argv[1] == "parity":
        for i in range(len(argv) - 1):
            if argv[i] == "--mode" and argv[i + 1] in {"--fixture", "--cli", "--diag"}:
                argv[i] = f"--mode={argv[i + 1]}"
                del argv[i + 1]
                break

    # Pre-scan argv for out-of-scope flags and give a clear error before argparse
    # touches anything, so we don't get confusing "unrecognized arguments" messages.
    if len(argv) > 0 and argv[0] == "verify":
        for raw in argv[1:]:
            flag = raw.split("=")[0]  # strip =value if any
            if flag in _VERIFY_OUT_OF_SCOPE_FLAGS:
                print(
                    f"error: flag not yet migrated to manager.py (Phase 1 scope: "
                    f"quick/fixtures/size/wat/component): {flag}",
                    file=sys.stderr,
                )
                return 2

    parser = build_parser()
    args = parser.parse_args(argv)

    # Propagate global --dry-run down (argparse already puts both in same namespace
    # for nested parsers, but guard in case of future restructuring).
    dry_run: bool = getattr(args, "dry_run", False)

    dispatch_positional = {
        "quick":     cmd_verify_quick,
        "fixtures":  cmd_verify_fixtures,
        "size":      cmd_verify_size,
        "wat":       cmd_verify_wat,
        "component": cmd_verify_component,
    }

    if args.domain == "verify":
        subcommand: str | None = getattr(args, "subcommand", None)

        # ── Positional subcommand takes priority when present ─────────────────
        if subcommand:
            handler = dispatch_positional.get(subcommand)
            if handler is None:
                print(f"{RED}error: unknown subcommand: {subcommand}{NC}", file=sys.stderr)
                return 1
            return handler(args)

        # ── Flag-based dispatch ───────────────────────────────────────────────
        # Expand --full into the individual Phase-1 flags.
        if args.full:
            args.quick = args.fixtures = args.size = args.wat = args.component = True
            args.selfhost_parity = True

        # Collect requested steps in a deterministic order.
        steps: list[tuple[str, object]] = []
        for flag, fn in [
            ("quick",     cmd_verify_quick),
            ("fixtures",  cmd_verify_fixtures),
            ("size",      cmd_verify_size),
            ("wat",       cmd_verify_wat),
            ("component", cmd_verify_component),
            ("selfhost_parity", cmd_verify_selfhost_parity),
        ]:
            if getattr(args, flag, False):
                steps.append((flag, fn))

        # --docs: Phase 1 stub — skip with a message.
        docs_requested = getattr(args, "docs", False)

        # Default: no flags given → run quick.
        if not steps and not docs_requested:
            return cmd_verify_quick(args)

        overall_rc = 0
        for flag, fn in steps:
            rc = fn(args)  # type: ignore[operator]
            if rc != 0:
                overall_rc = rc

        if docs_requested:
            print("[verify docs] skipped — not yet migrated (see Issue #534)")
            # exit 0 for docs stub; don't override a real failure.

        return overall_rc

    if args.domain == "selfhost":
        _sh_dispatch = {
            "fixpoint":       cmd_selfhost_fixpoint,
            "fixture-parity": cmd_selfhost_fixture_parity,
            "diag-parity":    cmd_selfhost_diag_parity,
            "parity":         cmd_selfhost_parity,
        }
        subcommand = getattr(args, "subcommand", None)
        handler = _sh_dispatch.get(subcommand or "")
        if handler is None:
            print(f"{RED}error: unknown selfhost subcommand: {subcommand}{NC}", file=sys.stderr)
            return 1
        return handler(args)

    if args.domain == "docs":
        _docs_dispatch = {
            "check":      cmd_docs_check,
            "regenerate": cmd_docs_regenerate,
        }
        subcommand = getattr(args, "subcommand", None)
        handler = _docs_dispatch.get(subcommand or "")
        if handler is None:
            print(f"{RED}error: unknown docs subcommand: {subcommand}{NC}", file=sys.stderr)
            return 1
        return handler(args)

    if args.domain == "perf":
        _perf_dispatch = {
            "gate":       cmd_perf_gate,
            "baseline":   cmd_perf_baseline,
            "benchmarks": cmd_perf_benchmarks,
        }
        subcommand = getattr(args, "subcommand", None)
        handler = _perf_dispatch.get(subcommand or "")
        if handler is None:
            print(f"{RED}error: unknown perf subcommand: {subcommand}{NC}", file=sys.stderr)
            return 1
        return handler(args)

    if args.domain == "orchestration":
        _orchestration_dispatch = {
            "agent-state":   cmd_orch_agent_state,
            "issue-health":  cmd_orch_issue_health,
            "repo-smoke":    cmd_orch_repo_smoke,
            "reference-coverage": cmd_orch_reference_coverage,
            "gen-issues":    cmd_orch_gen_issues,
        }
        subcommand = getattr(args, "subcommand", None)
        handler = _orchestration_dispatch.get(subcommand or "")
        if handler is None:
            print(f"{RED}error: unknown orchestration subcommand: {subcommand}{NC}", file=sys.stderr)
            return 1
        return handler(args)

    if args.domain == "gate":
        _gate_dispatch = {
            "local":      cmd_gate_local,
            "pre-commit": cmd_gate_pre_commit,
            "pre-push":   cmd_gate_pre_push,
            "repro":      cmd_gate_repro,
        }
        subcommand = getattr(args, "subcommand", None)
        handler = _gate_dispatch.get(subcommand or "")
        if handler is None:
            print(f"{RED}error: unknown gate subcommand: {subcommand}{NC}", file=sys.stderr)
            return 1
        return handler(args)

    print(f"{RED}error: unknown domain: {args.domain}{NC}", file=sys.stderr)
    return 1
# ── selfhost subcommands ──────────────────────────────────────────────────────


def cmd_selfhost_fixpoint(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    no_build: bool = not getattr(args, "build", False)
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[selfhost] Running selfhost fixpoint check...{NC}")
    res: SelfhostFixpointResult = run_fixpoint(root, dry_run, no_build=no_build)

    if res.passed:
        h.check_pass("selfhost fixpoint reached")
    elif res.skipped:
        h.check_skip(f"selfhost fixpoint not yet reached (exit {res.exit_code})")
    else:
        h.check_fail(
            "selfhost fixpoint check failed",
            category="bootstrap",
            command="python3 scripts/manager.py selfhost fixpoint --build",
            primary_path="src/compiler/main.ark",
        )
        for line in res.output.splitlines()[-30:]:
            print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_selfhost_fixture_parity(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[selfhost] Running selfhost fixture parity check...{NC}")
    rc, out = run_fixture_parity(root, dry_run)
    if rc == 0:
        h.check_pass("selfhost fixture parity")
    else:
        h.check_fail(
            "selfhost fixture parity",
            category="bootstrap",
            command="python3 scripts/manager.py selfhost fixture-parity",
            primary_path="tests/fixtures/manifest.txt",
        )
        for line in out.splitlines()[-30:]:
            print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_selfhost_diag_parity(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[selfhost] Running selfhost diagnostic parity check...{NC}")
    rc, out = run_diag_parity(root, dry_run)
    if rc == 0:
        h.check_pass("selfhost diagnostic parity")
    else:
        h.check_fail(
            "selfhost diagnostic parity",
            category="bootstrap",
            command="python3 scripts/manager.py selfhost diag-parity",
            primary_path="tests/fixtures/",
        )
        for line in out.splitlines()[-30:]:
            print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_selfhost_parity(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    mode: str = getattr(args, "mode", "") or ""
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[selfhost] Running selfhost parity check (mode={mode!r})...{NC}")
    rc, out = run_parity(root, dry_run, mode=mode)
    if rc == 0:
        h.check_pass(f"selfhost parity{' ' + mode if mode else ''}")
    else:
        h.check_fail(
            f"selfhost parity{' ' + mode if mode else ''}",
            category="bootstrap",
            command=f"python3 scripts/manager.py selfhost parity{(' --mode ' + mode) if mode else ''}",
            primary_path="bootstrap/arukellt-selfhost.wasm",
        )
        for line in out.splitlines()[-30:]:
            print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


# ── docs subcommands ──────────────────────────────────────────────────────────


def cmd_docs_check(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[docs check] Running docs checks in parallel...{NC}")
    import concurrent.futures as _cf

    checks = [
        ("docs consistency", run_consistency),
        ("docs freshness", run_freshness),
        ("doc examples", run_examples),
    ]
    with _cf.ThreadPoolExecutor() as executor:
        futures = {executor.submit(fn, root, dry_run): label for label, fn in checks}
        for future in _cf.as_completed(futures):
            label = futures[future]
            rc, out = future.result()
            if rc == 0:
                h.check_pass(label)
            else:
                h.check_fail(
                    label,
                    category="docs",
                    command=f"python3 scripts/manager.py docs check ({label})",
                    primary_path="docs/",
                )
                for line in out.splitlines()[-20:]:
                    print(line)

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_docs_regenerate(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    check_only: bool = getattr(args, "check_only", False)
    h = Harness(repo_root=root, dry_run=dry_run)

    label = "docs regenerate (check)" if check_only else "docs regenerate"
    print(f"\n{YELLOW}[docs regenerate] {label}...{NC}")
    rc, out = run_regenerate(root, dry_run=dry_run, check_only=check_only)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass(label)
    else:
        h.check_fail(
            label,
            category="docs",
            command="python3 scripts/manager.py docs regenerate",
            primary_path="docs/",
        )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


# ── perf subcommands ──────────────────────────────────────────────────────────


def cmd_perf_gate(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    update: bool = getattr(args, "update", False)
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[perf gate] Running performance gate...{NC}")
    rc, out = run_perf_gate(root, dry_run=dry_run, update=update)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("perf gate")
    else:
        h.check_fail(
            "perf gate",
            category="perf",
            command="python3 scripts/manager.py perf gate",
            primary_path="tests/baselines/perf/baselines.json",
        )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_perf_baseline(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[perf baseline] Collecting perf baseline...{NC}")
    rc, out = run_baseline(root, dry_run=dry_run)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("perf baseline")
    else:
        h.check_fail(
            "perf baseline",
            category="perf",
            command="python3 scripts/manager.py perf baseline",
            primary_path="tests/baselines/",
        )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_perf_benchmarks(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    quick: bool = not getattr(args, "no_quick", False)
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[perf benchmarks] Running benchmarks (quick={quick})...{NC}")
    rc, out = run_benchmarks(root, dry_run=dry_run, quick=quick)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("perf benchmarks")
    else:
        h.check_fail(
            "perf benchmarks",
            category="perf",
            command="python3 scripts/manager.py perf benchmarks",
            primary_path="benchmarks/",
        )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


# ── gate subcommands ──────────────────────────────────────────────────────────


def cmd_gate_local(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    skip_ext: bool = getattr(args, "skip_ext", False)
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[gate local] Running full local CI gate...{NC}")
    rc, out = run_local(root, dry_run=dry_run, skip_ext=skip_ext)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("gate local")
    else:
        h.check_fail(
            "gate local",
            category="local-ci",
            command="python3 scripts/manager.py gate local",
            primary_path="scripts/gate_domain/checks.py",
        )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_gate_pre_commit(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[gate pre-commit] Running pre-commit verification...{NC}")
    rc, out = run_pre_commit(root, dry_run=dry_run)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("gate pre-commit")
    else:
        h.check_fail(
            "gate pre-commit",
            category="local-ci",
            command="python3 scripts/manager.py gate pre-commit",
            primary_path="scripts/gate/pre-commit-verify.sh",
        )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_gate_pre_push(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[gate pre-push] Running pre-push verification...{NC}")
    rc, out = run_pre_push(root, dry_run=dry_run)
    if out:
        print(out, end="")
    if rc == 0:
        h.check_pass("gate pre-push")
    else:
        h.check_fail(
            "gate pre-push",
            category="local-ci",
            command="python3 scripts/manager.py gate pre-push",
            primary_path="scripts/gate_domain/checks.py",
        )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_gate_repro(args: argparse.Namespace) -> int:
    root = _repo_root()
    dry_run: bool = args.dry_run
    h = Harness(repo_root=root, dry_run=dry_run)

    print(f"\n{YELLOW}[gate repro] Running reproducible build check...{NC}")
    rc, out = run_repro(
        root,
        dry_run=dry_run,
        fixture=getattr(args, "fixture", ""),
        target=getattr(args, "target", ""),
        verbose=getattr(args, "verbose", False),
    )
    if out:
        print(out, end="")
    if rc == 2:
        h.check_skip("gate repro (prereqs missing)")
    elif rc == 0:
        h.check_pass("gate repro")
    else:
        h.check_fail(
            "gate repro",
            category="determinism",
            command="python3 scripts/manager.py gate repro",
            primary_path="docs/examples/hello.ark",
        )

    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


# ── orchestration subcommands ──────────────────────────────────────────────


def cmd_orch_agent_state(args: argparse.Namespace) -> int:
    check_script = _SCRIPTS_DIR / "check" / "check-agent-state.py"
    if not check_script.exists():
        print(f"{RED}error: {check_script} not found{NC}", file=sys.stderr)
        return 1
    cmd = [sys.executable, str(check_script)]
    if args.list_all:
        cmd.append("--list-all")
    elif args.worktree_path:
        cmd.append(args.worktree_path)
    else:
        print(f"{RED}error: specify worktree_path or --list-all{NC}", file=sys.stderr)
        return 1
    result = subprocess.run(cmd, cwd=str(_repo_root()))
    return result.returncode


def cmd_orch_issue_health(args: argparse.Namespace) -> int:
    check_script = _SCRIPTS_DIR / "check" / "check-issue-health.py"
    if not check_script.exists():
        print(f"{RED}error: {check_script} not found{NC}", file=sys.stderr)
        return 1
    result = subprocess.run(
        [sys.executable, str(check_script)],
        cwd=str(_repo_root()),
    )
    return result.returncode


def cmd_orch_repo_smoke(args: argparse.Namespace) -> int:
    check_script = _SCRIPTS_DIR / "check" / "check-repo-smoke.py"
    if not check_script.exists():
        print(f"{RED}error: {check_script} not found{NC}", file=sys.stderr)
        return 1
    result = subprocess.run(
        [sys.executable, str(check_script)],
        cwd=str(_repo_root()),
    )
    return result.returncode


def cmd_orch_reference_coverage(args: argparse.Namespace) -> int:
    """Stub: generate reference coverage report.
    
    In a full implementation, this would run reference test suites (test262,
    spec tests, etc.) and report coverage gaps.
    """
    h = Harness(repo_root=_repo_root(), dry_run=args.dry_run)
    print(f"\n{YELLOW}[orchestration reference-coverage]{NC}")
    print("  NOTE: reference-coverage is a stub.")
    print("  A full implementation would run reference suites and report gaps.")
    print(f"  Requested limit: {getattr(args, 'limit', 100)}")
    print(f"  Detail: {getattr(args, 'detail', False)}")
    h.check_skip("reference-coverage (stub)")
    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


def cmd_orch_gen_issues(args: argparse.Namespace) -> int:
    """Stub: generate issues from reference coverage gaps.
    
    In a full implementation, this would parse coverage gaps and create issue
    files under issues/open/ with appropriate frontmatter.
    """
    h = Harness(repo_root=_repo_root(), dry_run=args.dry_run)
    print(f"\n{YELLOW}[orchestration gen-issues]{NC}")
    print("  NOTE: gen-issues is a stub.")
    print("  A full implementation would create issue files from coverage gaps.")
    print(f"  Suite: {getattr(args, 'suite', 'test262')}")
    h.check_skip("gen-issues (stub)")
    total, passed, skipped, failed = h.summary()
    print(f"\n{YELLOW}Summary{NC}")
    print(f"Total checks: {total}")
    print(f"Passed: {GREEN}{passed}{NC}")
    print(f"Skipped: {YELLOW}{skipped}{NC}")
    print(f"Failed: {RED}{failed}{NC}")
    return h.exit_code()


if __name__ == "__main__":
    sys.exit(main())
