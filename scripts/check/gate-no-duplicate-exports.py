#!/usr/bin/env python3
"""Gate: the selfhost compiler never emits duplicate WASM export names.

Compiles a representative Ark fixture (``tests/fixtures/component/multi_export.ark``)
to ``wasm32`` with the current selfhost compiler and verifies that the export
section contains no duplicate export names.

Exit codes:
  0  — passed (or skipped because wasmtime / wasm-tools / compiler is missing)
  1  — compilation failed or duplicate export names were found
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = REPO_ROOT / "tests" / "fixtures" / "component" / "multi_export.ark"
BUILD_DIR = REPO_ROOT / ".build" / "gate-no-duplicate-exports"
OUTPUT_WASM = BUILD_DIR / "multi_export.wasm"

COMPILE_TIMEOUT = 120
PRINT_TIMEOUT = 30


def find_selfhost_wasm() -> Path | None:
    """Resolve the selfhost compiler wasm, matching scripts/run/arukellt-selfhost.sh."""
    env = os.environ.get("ARUKELLT_SELFHOST_WASM")
    if env:
        path = Path(env)
        if path.is_file():
            return path
    for candidate in (
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s2-runtime.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s3.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s2.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-pinned-bootstrap.wasm",
        REPO_ROOT / "bootstrap" / "arukellt-selfhost.wasm",
    ):
        if candidate.is_file():
            return candidate
    return None


def find_wasmtime() -> str | None:
    return shutil.which("wasmtime")


def find_wasm_tools() -> str | None:
    cargo = Path.home() / ".cargo" / "bin" / "wasm-tools"
    if cargo.is_file():
        return str(cargo)
    return shutil.which("wasm-tools")


def main() -> int:
    wasmtime = find_wasmtime()
    if not wasmtime:
        print("gate-no-duplicate-exports: SKIP (wasmtime not found)")
        return 0

    compiler = find_selfhost_wasm()
    if compiler is None:
        print("gate-no-duplicate-exports: SKIP (selfhost compiler wasm not found)")
        return 0

    wasm_tools = find_wasm_tools()
    if not wasm_tools:
        print("gate-no-duplicate-exports: SKIP (wasm-tools not found)")
        return 0

    if not FIXTURE.is_file():
        print(f"gate-no-duplicate-exports: FAIL (fixture missing: {FIXTURE})", file=sys.stderr)
        return 1

    BUILD_DIR.mkdir(parents=True, exist_ok=True)

    wrapper = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    if not wrapper.is_file():
        print("gate-no-duplicate-exports: SKIP (compiler wrapper not found)", file=sys.stderr)
        return 0

    fixture_rel = str(FIXTURE.relative_to(REPO_ROOT))
    output_rel = str(OUTPUT_WASM.relative_to(REPO_ROOT))
    env = dict(os.environ)
    if "ARUKELLT_SELFHOST_WASM" not in env:
        env["ARUKELLT_SELFHOST_WASM"] = str(compiler)

    compile_result = subprocess.run(
        ["bash", str(wrapper), "compile", fixture_rel, "--target", "wasm32", "-o", output_rel],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=COMPILE_TIMEOUT,
        env=env,
    )

    if not (OUTPUT_WASM.is_file() and OUTPUT_WASM.stat().st_size > 0):
        print("gate-no-duplicate-exports: FAIL (compilation did not produce output)", file=sys.stderr)
        if compile_result.stdout:
            print(compile_result.stdout, file=sys.stderr)
        if compile_result.stderr:
            print(compile_result.stderr, file=sys.stderr)
        return 1

    print_result = subprocess.run(
        [wasm_tools, "print", str(OUTPUT_WASM)],
        capture_output=True,
        text=True,
        timeout=PRINT_TIMEOUT,
    )
    if print_result.returncode != 0:
        print("gate-no-duplicate-exports: FAIL (wasm-tools print failed)", file=sys.stderr)
        if print_result.stderr:
            print(print_result.stderr, file=sys.stderr)
        return 1

    names = re.findall(r'\(export "([^"]+)"', print_result.stdout)
    seen: dict[str, int] = {}
    duplicates: dict[str, int] = {}
    for name in names:
        seen[name] = seen.get(name, 0) + 1
        if seen[name] >= 2:
            duplicates[name] = seen[name]

    if duplicates:
        print("gate-no-duplicate-exports: FAIL (duplicate export names)", file=sys.stderr)
        for name, count in sorted(duplicates.items()):
            print(f"  '{name}' appears {count} times", file=sys.stderr)
        return 1

    print(f"gate-no-duplicate-exports: PASS ({len(names)} unique export names)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
