#!/usr/bin/env python3
"""Restore ADR-040 module splits that were lost before commit 77856e2e2."""
from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PARENT = "b1a7376d0"


def git_show(path: str) -> str:
    return subprocess.check_output(
        ["git", "show", f"{PARENT}:{path}"], text=True, cwd=ROOT
    )


def write(relpath: str, content: str) -> None:
    path = ROOT / relpath
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def restore_intrinsics_splits() -> None:
    names = [
        "helpers_core_calls",
        "helpers_core_heap",
        "helpers_core_locals",
        "helpers_core_misc",
        "helpers_core_stack",
        "helpers_memory_access",
        "helpers_memory_stack",
        "helpers_numeric_i32_arith",
        "helpers_numeric_i32_bitwise",
        "helpers_numeric_i32_compare",
        "helpers_numeric_i32_const",
        "helpers_numeric_i32_convert",
        "helpers_control_branch",
        "helpers_control_loop",
        "helpers_control_structured",
    ]
    for name in names:
        write(
            f"src/compiler/wasm/intrinsics/{name}.ark",
            git_show(f"src/compiler/wasm/intrinsics/{name}.ark"),
        )
    write("src/compiler/wasm/writer_leb.ark", git_show("src/compiler/wasm/writer_leb.ark"))


def main() -> None:
    restore_intrinsics_splits()
    print("restored intrinsics splits; run full restore from session script for remaining modules")


if __name__ == "__main__":
    main()
