#!/usr/bin/env python3
"""Ensure production call routers do not use callee-string semantic dispatch."""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ROUTERS = [
    ROOT / "src/compiler/wasm/call_dispatch_table.ark",
    ROOT / "src/compiler/wasm/call_dispatch.ark",
    ROOT / "src/compiler/wasm/inst_dispatch.ark",
]
PATTERN = re.compile(r"eq\(clone\(callee\)")


def main() -> int:
    violations: list[str] = []
    for path in ROUTERS:
        if not path.exists():
            continue
        count = len(PATTERN.findall(path.read_text(encoding="utf-8")))
        if count:
            rel = path.relative_to(ROOT)
            violations.append(f"{rel}: {count} callee-string semantic compares")
    if violations:
        print("FAIL: router callee-string semantic dispatch", file=sys.stderr)
        for item in violations:
            print(f"  {item}", file=sys.stderr)
        return 1
    print("PASS: call routers use FunctionId/CoreOp dispatch only")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
