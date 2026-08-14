#!/usr/bin/env python3
"""Close gate for #672 — generated Ark bindings from WIT declarations."""
from __future__ import annotations
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PARSER = ROOT / "src/compiler/component/wit_parse_text.ark"
BINDINGS = ROOT / "src/compiler/component/wit_bindings.ark"
NAMES = ROOT / "src/compiler/component/wit_names_import.ark"
TYPE_MAP = ROOT / "src/compiler/resolver/wit_type_map.ark"
FIXTURE = ROOT / "tests/fixtures/wit_import/bindings/nested.wit"


def main() -> int:
    required = (PARSER, BINDINGS, NAMES, TYPE_MAP, FIXTURE)
    for path in required:
        if not path.is_file():
            print(f"gate-672-wit-type-binding-codegen: FAIL: missing {path.relative_to(ROOT)}", file=sys.stderr)
            return 1
    parser = PARSER.read_text(encoding="utf-8")
    bindings = BINDINGS.read_text(encoding="utf-8")
    names = NAMES.read_text(encoding="utf-8")
    fixture = FIXTURE.read_text(encoding="utf-8")
    for needle in ("WitParsedRecord", "WitParsedEnum", "WitParsedVariant", "case_payload_wit_types"):
        if needle not in parser:
            print(f"gate-672-wit-type-binding-codegen: FAIL: parser missing {needle}", file=sys.stderr)
            return 1
    for needle in ("pub struct ", "pub enum ", "Vec<", "Option<", "Result<", "tuple<", "wit-package:", "wit-interface:", "E0402 recursive WIT record binding", "resource handle field"):
        if needle not in bindings:
            print(f"gate-672-wit-type-binding-codegen: FAIL: renderer missing {needle}", file=sys.stderr)
            return 1
    if "std::wit::names" not in names or "pascal_case" not in names or "kebab_to_snake" not in names:
        print("gate-672-wit-type-binding-codegen: FAIL: stable naming is not shared with std::wit", file=sys.stderr)
        return 1
    for needle in ("display-name", "list<string>", "option<s32>", "result<s32, string>", "tuple<s32, s32>", "variant response"):
        if needle not in fixture:
            print(f"gate-672-wit-type-binding-codegen: FAIL: nested fixture missing {needle}", file=sys.stderr)
            return 1
    print("gate-672-wit-type-binding-codegen: PASS")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
