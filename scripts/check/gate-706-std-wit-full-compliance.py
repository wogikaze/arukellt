#!/usr/bin/env python3
"""Close gate for #706 — std::wit owns WIT 1.0 parsing/naming/lowering primitives."""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def require(path: str, needles: tuple[str, ...]) -> str | None:
    p = ROOT / path
    if not p.is_file():
        return f"missing {path}"
    text = p.read_text(encoding="utf-8")
    for needle in needles:
        if needle not in text:
            return f"{path} missing {needle!r}"
    return None


def main() -> int:
    checks = (
        ("std/wit/ast.ark", ("WitNode::Package", "WitNode::World", "WitNode::Interface", "WitNode::Record", "WitNode::Enum", "WitNode::Flags", "WitNode::Variant", "WitNode::Resource", "WitNode::TypeAlias", "WitNode::Use", "pub fn parse(source: String)")),
        ("std/wit/names.ark", ("kebab_name", "kebab_to_snake", "pascal_case")),
        ("std/wit/types.ark", ("WitType", "wit_type_from_ast")),
        ("std/wit/parser.ark", ("parse_wit", "parse_full", "ast::parse")),
        ("src/compiler/component/wit_names.ark", ("std::wit::names",)),
        ("src/compiler/component/wit_names_import.ark", ("std::wit::names",)),
        ("src/compiler/component/wit_parse_text_scan.ark", ("std::wit::scan",)),
        ("src/compiler/component/wit_parse_import.ark", ("std::wit::parser", "parser::parse_full")),
        ("src/compiler/resolver/wit_import_bind.ark", ("component::wit_parse_text",)),
    )
    for path, needles in checks:
        error = require(path, needles)
        if error:
            print(f"gate-706-std-wit-full-compliance: FAIL: {error}", file=sys.stderr)
            return 1
    if (ROOT / "src/compiler/component/wit_parse_types.ark").exists():
        print("gate-706-std-wit-full-compliance: FAIL: duplicate compiler WIT parsed type model still exists", file=sys.stderr)
        return 1
    parser_files = sorted((ROOT / "src/compiler/component").glob("wit_parse_*.ark"))
    for path in parser_files:
        text = path.read_text(encoding="utf-8")
        if path.name == "wit_parse_text_scan.ark" and "std::wit::scan" not in text:
            print("gate-706-std-wit-full-compliance: FAIL: compiler scanner is not a std::wit facade", file=sys.stderr)
            return 1
    print("gate-706-std-wit-full-compliance: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
