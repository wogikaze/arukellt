#!/usr/bin/env python3
"""Exercise gc_hint at opt-level 2 and require a post-pass MIR hint."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "tests" / "proof" / "gc-hint-runtime.ark"
OUTPUT = ROOT / ".build" / "proof" / "gc-hint-runtime.wasm"


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
    if "struct.new ->" not in combined:
        print("gc-hint-runtime: FAIL: fixture did not lower a struct allocation", file=sys.stderr)
        print(combined, file=sys.stderr)
        return 1
    if "gc.hint ->" not in combined:
        print("gc-hint-runtime: FAIL: optimized MIR contains no canonical gc.hint", file=sys.stderr)
        print(combined, file=sys.stderr)
        return 1
    print("gc-hint-runtime: PASS: canonical hint observed in optimized MIR")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
