#!/usr/bin/env python3
"""Compile ark_native_runtime.c as C99 with warnings-as-errors."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUNTIME_C = ROOT / "src/compiler/native_c/runtime/ark_native_runtime.c"
RUNTIME_I = ROOT / "src/compiler/native_c/runtime"
CC = os.environ.get("ARUKELLT_CC", "clang-16")


def main() -> int:
    if not RUNTIME_C.is_file():
        print(f"FAIL: missing {RUNTIME_C}", file=sys.stderr)
        return 1
    with tempfile.TemporaryDirectory(prefix="native-runtime-werror-") as tmp:
        obj = Path(tmp) / "ark_native_runtime.o"
        # Match the native-executor C99 dialect. Allow unused static helpers that
        # remain after page-map GC migration without changing runtime ABI hash.
        cmd = [
            CC,
            "-std=c99",
            "-c",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-Wno-unused-function",
            f"-I{RUNTIME_I}",
            str(RUNTIME_C),
            "-o",
            str(obj),
        ]
        result = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=False)
        if result.returncode != 0:
            print("FAIL: native runtime C99 -Werror compile", file=sys.stderr)
            print(result.stderr[-4000:], file=sys.stderr)
            return 1
    print("native-runtime-c99-werror: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
