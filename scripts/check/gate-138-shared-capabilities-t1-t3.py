#!/usr/bin/env python3
"""Close gate for issue #138 — std::host shared capabilities T1/T3 smoke matrix."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

MODULES = ("stdio", "fs", "env", "process", "clock", "random")
FIXTURE_BY_MODULE = {
    "stdio": "wasi_stdio.ark",
    "fs": "wasi_fs.ark",
    "env": "wasi_args.ark",
    "process": "wasi_process.ark",
    "clock": "wasi_clock.ark",
    "random": "wasi_random.ark",
}


def main() -> int:
    failures: list[str] = []
    manifest_path = REPO_ROOT / "tests" / "fixtures" / "manifest.txt"
    if not manifest_path.is_file():
        failures.append("missing tests/fixtures/manifest.txt")
        manifest = ""
    else:
        manifest = manifest_path.read_text(encoding="utf-8")

    for module in MODULES:
        host_path = REPO_ROOT / "std" / "host" / f"{module}.ark"
        if not host_path.is_file():
            failures.append(f"missing std/host/{module}.ark")

        fixture_name = FIXTURE_BY_MODULE[module]
        fixture_path = REPO_ROOT / "tests" / "fixtures" / "stdlib_host" / fixture_name
        if not fixture_path.is_file():
            failures.append(f"missing tests/fixtures/stdlib_host/{fixture_name}")

        rel = f"stdlib_host/{fixture_name}"
        for kind in ("run", "t3-run"):
            entry = f"{kind}:{rel}"
            if entry not in manifest:
                failures.append(f"manifest missing {entry}")

    if failures:
        print("gate-138-shared-capabilities-t1-t3: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1

    print("gate-138-shared-capabilities-t1-t3: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
