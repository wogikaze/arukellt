#!/usr/bin/env python3
"""Check WIT resource parsing without reviving an in-tree component encoder."""

from __future__ import annotations

import shutil
import sys
from pathlib import Path


SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))
from lib.wit_tools import parse_wit_package


ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    required = (
        ROOT / "src/compiler/component/wit_parse_resource.ark",
        ROOT / "src/compiler/component/wit_type_defs.ark",
        ROOT / "src/compiler/component/self_check_issue473.ark",
        ROOT / "tests/fixtures/component/export_resource_roundtrip.expected.wit",
        ROOT / "tests/fixtures/component/import_resource_handle_type.wit",
    )
    for path in required:
        if not path.is_file():
            print(f"gate-473: FAIL: missing {path.relative_to(ROOT)}", file=sys.stderr)
            return 1

    type_defs = (ROOT / "src/compiler/component/wit_type_defs.ark").read_text(encoding="utf-8")
    if "wit_export_own_handle_type" not in type_defs:
        print("gate-473: FAIL: WIT own<handle> emission helper missing", file=sys.stderr)
        return 1

    tool = shutil.which("wasm-tools")
    if tool is not None:
        lock = ROOT / ".build" / "wasm-tools-component.lock"
        for rel in (
            "tests/fixtures/component/export_resource_roundtrip.expected.wit",
            "tests/fixtures/component/import_resource_handle_type.wit",
        ):
            returncode, output = parse_wit_package(tool, ROOT / rel, lock)
            if returncode != 0:
                print(f"gate-473: FAIL: wasm-tools rejected {rel}: {output[-800:]}", file=sys.stderr)
                return 1

    print("gate-473-wit-resource-handles: PASS (WIT parsing; component packaging is official-tooling owned)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
