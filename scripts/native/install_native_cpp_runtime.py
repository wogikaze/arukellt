#!/usr/bin/env python3
"""Install private native-cpp runtime files into an optional prefix layout."""

from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUNTIME_SRC = ROOT / "src" / "compiler" / "native_c" / "runtime"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--prefix", type=Path, required=True)
    args = parser.parse_args()
    dest = args.prefix / "lib" / "arukellt" / "native-cpp"
    dest.mkdir(parents=True, exist_ok=True)
    for name in ("ark_native_runtime.c", "ark_native_runtime.h"):
        src = RUNTIME_SRC / name
        if not src.is_file():
            print(f"missing {src}", file=sys.stderr)
            return 1
        shutil.copy2(src, dest / name)
    print(dest)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
