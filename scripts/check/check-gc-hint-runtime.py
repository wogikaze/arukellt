#!/usr/bin/env python3
"""Exercise gc_hint at opt-level 2 and emit a translation receipt."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "tests" / "proof" / "gc-hint-runtime.ark"
OUTPUT = ROOT / ".build" / "proof" / "gc-hint-runtime.wasm"
RECEIPT = ROOT / ".build" / "proof" / "gc-hint-translation-receipt.json"


def main() -> int:
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    command = [
        str(ROOT / "scripts" / "run" / "arukellt-selfhost.sh"),
        "compile",
        str(SOURCE.relative_to(ROOT)),
        "--opt-level",
        "2",
        "--dump-phases",
        "mir",
        "-o",
        str(OUTPUT.relative_to(ROOT)),
    ]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    combined = result.stdout + "\n" + result.stderr
    if result.returncode != 0:
        print(f"gc-hint-runtime: FAIL: compile exited {result.returncode}", file=sys.stderr)
        print(combined, file=sys.stderr)
        return 1
    if not OUTPUT.is_file() or OUTPUT.stat().st_size == 0:
        print("gc-hint-runtime: FAIL: compiler output missing", file=sys.stderr)
        print(combined, file=sys.stderr)
        return 1

    struct_new_count = combined.count("struct.new ->")
    hint_count = combined.count("gc.hint ->")
    if struct_new_count == 0:
        print("gc-hint-runtime: FAIL: fixture did not lower a struct allocation", file=sys.stderr)
        print(combined, file=sys.stderr)
        return 1
    if hint_count == 0:
        print("gc-hint-runtime: FAIL: optimized MIR contains no canonical gc.hint", file=sys.stderr)
        print(combined, file=sys.stderr)
        return 1

    receipt = {
        "schema": "arukellt-translation-validation-receipt",
        "schema_version": 1,
        "pass": "gc_hint",
        "fixture": str(SOURCE.relative_to(ROOT)),
        "opt_level": 2,
        "observed": {
            "struct_new_count": struct_new_count,
            "canonical_gc_hint_count": hint_count,
        },
        "validator": {
            "policy": "non-hint instructions exact; inserted hints canonical",
            "failure_action": "restore original block",
        },
        "status": "passed",
    }
    RECEIPT.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        "gc-hint-runtime: PASS: "
        f"canonical_hints={hint_count} struct_news={struct_new_count}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
