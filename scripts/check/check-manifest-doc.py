#!/usr/bin/env python3
"""Manifest-backed doc command gate (issue #456)."""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _compiler(root: Path) -> Path | None:
    sys.path.insert(0, str(root))
    from scripts.selfhost.checks import resolve_ide_gate_compiler_wasm

    return resolve_ide_gate_compiler_wasm(root)


def _run(compiler: Path, root: Path, args: list[str]) -> subprocess.CompletedProcess[str]:
    wasmtime = shutil.which("wasmtime")
    assert wasmtime is not None
    return subprocess.run(
        [wasmtime, "run", "--dir", str(root), str(compiler), "--", *args],
        cwd=str(root),
        capture_output=True,
        text=True,
        timeout=60,
    )


def main() -> int:
    root = _repo_root()
    compiler = _compiler(root)
    if compiler is None:
        print("error: no selfhost compiler wasm", file=sys.stderr)
        return 1

    failures = 0

    r = _run(compiler, root, ["doc", "println"])
    if r.returncode != 0 or "fn println" not in r.stdout:
        print("FAIL doc println", file=sys.stderr)
        failures += 1

    r = _run(compiler, root, ["doc", "--json", "println"])
    if r.returncode != 0:
        print("FAIL doc --json println exit", file=sys.stderr)
        failures += 1
    else:
        try:
            data = json.loads(r.stdout.strip())
            if data.get("kind") != "function" or data.get("name") != "println":
                print("FAIL doc --json println fields", file=sys.stderr)
                failures += 1
        except json.JSONDecodeError:
            print("FAIL doc --json println invalid JSON", file=sys.stderr)
            failures += 1

    r = _run(compiler, root, ["doc", "--target", "wasm32-gc", "std::host::http"])
    if r.returncode != 0 or "Functions:" not in r.stdout:
        print("FAIL doc module std::host::http", file=sys.stderr)
        failures += 1

    r = _run(compiler, root, ["doc", "--target", "wasm32", "std::host::sockets::connect"])
    out = r.stdout + r.stderr
    if r.returncode != 0 or "Not available" not in out:
        print("FAIL doc target filter sockets::connect", file=sys.stderr)
        failures += 1

    r = _run(compiler, root, ["doc", "std::core"])
    if r.returncode != 0 or "Functions:" not in r.stdout:
        print("FAIL doc module std::core", file=sys.stderr)
        failures += 1

    r = _run(compiler, root, ["doc", "httq_get"])
    if r.returncode == 0:
        print("FAIL doc missing symbol should fail", file=sys.stderr)
        failures += 1
    elif "Did you mean" not in r.stderr and "Did you mean" not in r.stdout:
        print("FAIL doc fuzzy match message", file=sys.stderr)
        failures += 1

    if failures:
        print(f"manifest-doc: {failures} failure(s)", file=sys.stderr)
        return 1
    print("manifest-doc: pass")
    return 0


if __name__ == "__main__":
    sys.exit(main())
