#!/usr/bin/env python3
"""Close gate for issue #648 — general canonical ABI umbrella (post-#121)."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

E0401_FIXTURES = (
    "export_unsupported_enum_status.ark",
    "export_unsupported_variant_payload_i32.ark",
    "export_unsupported_record_rect.ark",
    "export_unsupported_f32_multi.ark",
    "export_unsupported_string_multi_mixed.ark",
)

GENERAL_ADAPTER_SOURCES = (
    "src/compiler/component/export_shapes_f32_general.ark",
    "src/compiler/component/adapters_f32_general.ark",
    "src/compiler/component/export_shapes_string_general.ark",
    "src/compiler/component/adapters_string_general.ark",
)


def main() -> int:
    failures: list[str] = []

    manifest_path = REPO_ROOT / "tests" / "fixtures" / "manifest.txt"
    if not manifest_path.is_file():
        failures.append("missing tests/fixtures/manifest.txt")
        manifest = ""
    else:
        manifest = manifest_path.read_text(encoding="utf-8")

    for fixture in E0401_FIXTURES:
        rel = f"component/{fixture}"
        path = REPO_ROOT / "tests" / "fixtures" / rel
        if not path.is_file():
            failures.append(f"missing tests/fixtures/{rel}")
        entry = f"compile-error:component/{fixture}"
        if entry not in manifest:
            failures.append(f"manifest missing {entry}")

    for rel in GENERAL_ADAPTER_SOURCES:
        if not (REPO_ROOT / rel).is_file():
            failures.append(f"missing {rel}")

    current_state = (REPO_ROOT / "docs" / "current-state.md").read_text(encoding="utf-8")
    for needle in (
        "non-`Color` enums",
        "non-`Shape` payload variants",
        "export_unsupported_record_rect",
        "#659",
        "#660",
    ):
        if needle not in current_state:
            failures.append(f"docs/current-state.md missing boundary marker: {needle!r}")

    if failures:
        print("gate-648-component-export-general-abi: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1

    print("gate-648-component-export-general-abi: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
