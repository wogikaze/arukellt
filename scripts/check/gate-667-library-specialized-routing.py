#!/usr/bin/env python3
"""Verify that component packaging has one official wasm-tools owner.

The former gate protected the in-tree scalar/canonical-ABI adapter tree. That
tree was a second component implementation and is intentionally gone. The
remaining contract is the standard P2 core plus official wasm-tools
``component embed``/``component new`` packaging.
"""

from __future__ import annotations

import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    retired_sources = (
        "src/compiler/component/emit.ark",
        "src/compiler/component_emitter.ark",
        "src/compiler/component_emit.ark",
        "src/compiler/wasm/library_component_emit.ark",
        "src/compiler/wasm/wasi_p1_component_stub.ark",
        "src/compiler/wasm/component_string_adapter.ark",
    )
    for rel in retired_sources:
        if (ROOT / rel).exists():
            print(f"gate-667: FAIL: retired component encoder remains: {rel}", file=sys.stderr)
            return 1

    wrapper = (ROOT / "scripts/run/arukellt-selfhost.sh").read_text(encoding="utf-8")
    required = (
        'component embed',
        'component new',
        '--reject-legacy-names',
        'WASI_P2_WIT_DIR',
    )
    for needle in required:
        if needle not in wrapper:
            print(f"gate-667: FAIL: launcher missing official packaging marker {needle!r}", file=sys.stderr)
            return 1
    if "--adapt" in wrapper:
        print("gate-667: FAIL: launcher still uses a Preview 1 adapter", file=sys.stderr)
        return 1

    driver = (ROOT / "src/compiler/driver/emit.ark").read_text(encoding="utf-8")
    if "component::emit_component" in driver or "p1-component" in driver:
        print("gate-667: FAIL: compiler still contains an in-tree component wrapper", file=sys.stderr)
        return 1
    adr = (ROOT / "docs/adr/ADR-054-host-linker-and-rust-runtime-retirement.md").read_text(encoding="utf-8")
    for needle in ("wasm-tools component embed", "no repository-specific bridge", "--reject-legacy-names"):
        if needle not in adr:
            print(f"gate-667: FAIL: ADR-054 missing packaging decision {needle!r}", file=sys.stderr)
            return 1

    print("gate-667-library-specialized-routing: PASS (in-tree encoder retired; official wasm-tools owns packaging)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
