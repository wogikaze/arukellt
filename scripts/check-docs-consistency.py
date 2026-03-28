#!/usr/bin/env python3
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main() -> int:
    cmd = [sys.executable, str(ROOT / "scripts" / "generate-docs.py"), "--check"]
    result = subprocess.run(cmd, cwd=ROOT)
    if result.returncode != 0:
        return result.returncode
    print("docs consistency OK (generated docs up to date)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
