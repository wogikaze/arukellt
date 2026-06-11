#!/usr/bin/env python3
"""Compiler module boundary checks for CoreHIR / MIR separation."""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
COMPILER_ROOT = REPO_ROOT / "src" / "compiler"

COREHIR_PARSER_ALLOW_RE = re.compile(
    r"^frontend_ast_.*\.ark$|^frontend_.*_kind\.ark$|^frontend_kind_map.*\.ark$"
)
MIR_LOWER_PARSER_ALLOW = {"src/compiler/mir/lower/ast_node.ark"}
MIR_TOPLEVEL_ASTNODE_SIG_ALLOW = {
    "src/compiler/mir/legacy_decl.ark",
    "src/compiler/mir/mod.ark",
}
MIR_LOWER_ASTNODE_FORBIDDEN_PREFIXES = ("return_",)
FN_SIG_ASTNODE_RE = re.compile(r"\bfn\s+\w+\([^)]*\bAstNode\b")


def _rel(path: Path) -> str:
    return str(path.relative_to(REPO_ROOT))


def check_corehir_parser_deps() -> list[str]:
    violations: list[str] = []
    corehir_dir = COMPILER_ROOT / "corehir"
    if not corehir_dir.is_dir():
        return violations
    for path in sorted(corehir_dir.rglob("*.ark")):
        if COREHIR_PARSER_ALLOW_RE.match(path.name):
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("use parser") or "use parser::" in stripped:
                violations.append(f"{_rel(path)}:{line_no}: {stripped}")
    return violations


def check_mir_parser_deps() -> list[str]:
    violations: list[str] = []
    mir_dir = COMPILER_ROOT / "mir"
    if not mir_dir.is_dir():
        return violations
    for path in sorted(mir_dir.rglob("*.ark")):
        rel = _rel(path)
        if rel in MIR_LOWER_PARSER_ALLOW:
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("use parser") or "use parser::" in stripped:
                violations.append(f"{rel}:{line_no}: {stripped}")
    return violations


def check_mir_astnode_signatures() -> list[str]:
    violations: list[str] = []
    mir_dir = COMPILER_ROOT / "mir"
    if not mir_dir.is_dir():
        return violations
    for path in sorted(mir_dir.rglob("*.ark")):
        rel = _rel(path)
        in_lower = "/mir/lower/" in rel
        if in_lower:
            if not path.name.startswith(MIR_LOWER_ASTNODE_FORBIDDEN_PREFIXES):
                continue
        elif rel in MIR_TOPLEVEL_ASTNODE_SIG_ALLOW:
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if FN_SIG_ASTNODE_RE.search(line):
                violations.append(f"{rel}:{line_no}: {line.strip()}")
    return violations


def main() -> int:
    errors: list[str] = []
    for label, fn in (
        ("corehir parser dependency", check_corehir_parser_deps),
        ("mir parser dependency", check_mir_parser_deps),
        ("mir AstNode signature", check_mir_astnode_signatures),
    ):
        found = fn()
        if found:
            errors.append(f"{label} violations:")
            errors.extend(f"  {item}" for item in found)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("compiler boundary checks OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
