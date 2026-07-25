#!/usr/bin/env python3
"""Gate for #668 stderr slice — P2 eprintln via wasi:cli/stderr.

Proves:
1. In-tree `--emit component` validates
2. Artifact imports wasi:cli/stderr + get-stderr (bridged path)
3. wasmtime run prints expected text on stderr (not only stdout)
4. Existing hello stdout path still green
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def _compiler() -> Path | None:
    wrapper = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    if wrapper.is_file():
        return wrapper
    return None


def _compile_component(fixture_rel: str, out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "missing scripts/run/arukellt-selfhost.sh"
    if not (REPO_ROOT / fixture_rel).is_file():
        return 1, f"missing {fixture_rel}"
    try:
        out_arg = str(out.relative_to(REPO_ROOT))
    except ValueError:
        out_arg = str(out)
    cmd = [
        "bash",
        str(compiler),
        "compile",
        fixture_rel,
        "--target",
        "wasm32-gc",
        "--wasi-version",
        "wasi-p2",
        "--emit",
        "component",
        "-o",
        out_arg,
    ]
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    if not out.is_file():
        return 1, f"missing output {out}"
    return 0, ""


def _validate(path: Path) -> tuple[int, str]:
    result = subprocess.run(
        ["wasm-tools", "validate", str(path)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    return 0, ""


def _assert_stderr_import_shape(path: Path) -> tuple[int, str]:
    data = path.read_bytes()
    if b"wasi:cli/stderr@0.2.0" not in data:
        return 1, "artifact missing wasi:cli/stderr@0.2.0"
    if b"get-stderr" not in data:
        return 1, "artifact missing get-stderr"
    if b"wasi:cli/stdout@0.2.0" not in data:
        return 1, "artifact missing wasi:cli/stdout@0.2.0"
    return 0, ""


def _wasmtime_run(
    path: Path, *, expect_stderr: str = "", expect_stdout: str = ""
) -> tuple[int, str]:
    result = subprocess.run(
        [
            "wasmtime",
            "run",
            "--wasm",
            "gc",
            "--wasm",
            "function-references",
            str(path),
        ],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, f"wasmtime exit {result.returncode}: {(result.stderr or '')[-400:]}"
    if expect_stdout and expect_stdout not in (result.stdout or ""):
        return 1, f"expected stdout {expect_stdout!r}, got {result.stdout!r}"
    if expect_stderr and expect_stderr not in (result.stderr or ""):
        return 1, f"expected stderr {expect_stderr!r}, got {result.stderr!r}"
    if expect_stderr and expect_stderr in (result.stdout or ""):
        return 1, f"stderr text leaked to stdout: {result.stdout!r}"
    return 0, ""


def main() -> int:
    failures: list[str] = []
    out_dir = Path(tempfile.mkdtemp(prefix="gate-668-stderr-", dir=REPO_ROOT / ".build"))
    try:
        hello = out_dir / "hello.component.wasm"
        rc, msg = _compile_component("tests/fixtures/wasi_p2_native/hello.ark", hello)
        if rc != 0:
            failures.append(f"hello compile: {msg}")
        else:
            for name, fn in (
                ("hello validate", lambda: _validate(hello)),
                ("hello wasmtime", lambda: _wasmtime_run(hello, expect_stdout="hello p2")),
            ):
                rc, msg = fn()
                if rc != 0:
                    failures.append(f"{name}: {msg}")

        err = out_dir / "eprintln.component.wasm"
        rc, msg = _compile_component(
            "tests/fixtures/wasi_p2_native/eprintln_stderr.ark", err
        )
        if rc != 0:
            failures.append(f"eprintln compile: {msg}")
        else:
            for name, fn in (
                ("eprintln validate", lambda: _validate(err)),
                ("eprintln import shape", lambda: _assert_stderr_import_shape(err)),
                (
                    "eprintln wasmtime",
                    lambda: _wasmtime_run(err, expect_stderr="hello stderr"),
                ),
            ):
                rc, msg = fn()
                if rc != 0:
                    failures.append(f"{name}: {msg}")
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)

    if failures:
        print("gate-668-p2-stderr: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-668-p2-stderr: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
