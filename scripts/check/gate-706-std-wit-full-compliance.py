#!/usr/bin/env python3
"""Close gate for #706 — std::wit owns WIT 1.0 parsing/naming/lowering primitives."""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "check"))
from _gate_open_skip import skip_if_any_open


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
    skipped = skip_if_any_open(["706"], "gate-706-std-wit-full-compliance")
    if skipped is not None:
        return skipped
    checks = (
        ("std/wit/ast.ark", ("WitNode::Package", "WitNode::World", "WitNode::Interface", "WitNode::Record", "WitNode::Enum", "WitNode::Flags", "WitNode::Variant", "WitNode::Resource", "WitNode::TypeAlias", "WitNode::Use", "fn parse(source: String) -> WitNode", "fn import_metadata(source: String) -> String")),
        ("std/wit/names.ark", ("WitName", "to_kebab", "to_snake", "to_pascal")),
        ("std/wit/types.ark", ("WitType", "WitType::list", "WitType::option", "wit_text")),
        ("std/wit/parser.ark", ("WitNode::parse", "WitNode::import_metadata", "migration anchor")),
        ("src/compiler/component/wit_names.ark", ("std::wit::names",)),
        ("src/compiler/component/wit_names_import.ark", ("std::wit::names",)),
        ("src/compiler/component/wit_parse_text_scan.ark", ("std::wit::scan",)),
        ("src/compiler/component/wit_parse_import.ark", ("std::wit::ast", "ast::WitNode::import_metadata(source)")),
        ("src/compiler/resolver/wit_import_bind.ark", ("component::wit_parse_text",)),
    )
    for path, needles in checks:
        error = require(path, needles)
        if error:
            print(f"gate-706-std-wit-full-compliance: FAIL: {error}", file=sys.stderr)
            return 1
    parser = (ROOT / "std/wit/parser.ark").read_text(encoding="utf-8")
    ast = (ROOT / "std/wit/ast.ark").read_text(encoding="utf-8")
    import_adapter = (ROOT / "src/compiler/component/wit_parse_import.ark").read_text(encoding="utf-8")
    if "pub fn parse_full" in parser or "pub fn parse_wit" in parser or "pub fn parse_full_import_text" in parser:
        print("gate-706-std-wit-full-compliance: FAIL: retired parser free entry point still exists", file=sys.stderr)
        return 1
    if "let parsed = parse_wit_document(source)" not in ast or "parsed.import_serialized()" not in ast:
        print("gate-706-std-wit-full-compliance: FAIL: canonical AST is not sealed as scalar metadata", file=sys.stderr)
        return 1
    if ast.count("let parsed = parse_wit_document(source)") != 1:
        print("gate-706-std-wit-full-compliance: FAIL: canonical parser adapter count changed", file=sys.stderr)
        return 1
    if import_adapter.count("ast::WitNode::import_metadata(source)") != 1:
        print("gate-706-std-wit-full-compliance: FAIL: compiler import adapter does not parse canonical metadata exactly once", file=sys.stderr)
        return 1
    if "fn parse_wit_import_text" in (ROOT / "src/compiler/component/wit_parse_text.ark").read_text(encoding="utf-8"):
        print("gate-706-std-wit-full-compliance: FAIL: compiler-local WIT parser still exists", file=sys.stderr)
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
