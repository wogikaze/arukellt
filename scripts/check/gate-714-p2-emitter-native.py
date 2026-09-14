#!/usr/bin/env python3
"""Close gate for issue #714 — emitter-native WASI P2 component output.

Proves:
1. The compiler emits a core module and the launcher packages it with official wasm-tools
2. The resulting component imports wasi:cli/stdout + wasi:io/streams
3. The artifact has no repository-specific or legacy compatibility ABI
4. wasm-tools validate + wasmtime run prints expected stdout
5. Exit-path fixture traps/non-zero on the same emitter-native path
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
    fixture = REPO_ROOT / fixture_rel
    if not fixture.is_file():
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


def _assert_import_shape(path: Path) -> tuple[int, str]:
    wit = subprocess.run(
        ["wasm-tools", "component", "wit", str(path)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if wit.returncode != 0:
        return 1, (wit.stderr or wit.stdout)[-800:]
    text = wit.stdout
    for marker in (
        "import wasi:cli/stdout@0.2.0",
        "import wasi:io/streams@0.2.0",
        "get-stdout: func() -> output-stream",
    ):
        if marker not in text:
            return 1, f"component WIT missing official stdout contract: {marker}"
    data = path.read_bytes()
    for marker in (
        b"arukellt:",
        b"runtime/host",
        b"wasi_snapshot_preview1",
        b"wasi:cli/stdout@0.2.0::write",
    ):
        if marker in data:
            return 1, f"artifact contains retired compatibility marker {marker!r}"
    return 0, ""


def _wasmtime_run(path: Path, expect_stdout: str) -> tuple[int, str]:
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
    if expect_stdout not in result.stdout:
        return 1, f"expected stdout {expect_stdout!r}, got {result.stdout!r}"
    return 0, ""


def _wasmtime_exit_path(path: Path, expect_stdout: str) -> tuple[int, str]:
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
    if expect_stdout not in (result.stdout or ""):
        return 1, f"expected stdout {expect_stdout!r}, got {result.stdout!r}"
    if result.returncode == 0:
        return 1, "expected non-zero exit from process::exit path, got 0"
    return 0, ""


def main() -> int:
    failures: list[str] = []

    out_dir = Path(tempfile.mkdtemp(prefix="gate-714-", dir=REPO_ROOT / ".build"))
    try:
        hello = out_dir / "hello.component.wasm"
        rc, msg = _compile_component("tests/fixtures/wasi_p2_native/hello.ark", hello)
        if rc != 0:
            failures.append(f"hello compile: {msg}")
        else:
            for name, fn in (
                ("hello validate", lambda: _validate(hello)),
                ("hello import shape", lambda: _assert_import_shape(hello)),
                ("hello wasmtime", lambda: _wasmtime_run(hello, "hello p2")),
            ):
                rc, msg = fn()
                if rc != 0:
                    failures.append(f"{name}: {msg}")

        exit_comp = out_dir / "exit.component.wasm"
        rc, msg = _compile_component(
            "tests/fixtures/wasi_p2_native/exit_code.ark", exit_comp
        )
        if rc != 0:
            failures.append(f"exit_code compile: {msg}")
        else:
            for name, fn in (
                ("exit validate", lambda: _validate(exit_comp)),
                ("exit import shape", lambda: _assert_import_shape(exit_comp)),
                ("exit wasmtime", lambda: _wasmtime_exit_path(exit_comp, "exit-marker")),
            ):
                rc, msg = fn()
                if rc != 0:
                    failures.append(f"{name}: {msg}")
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)

    if failures:
        print("gate-714-p2-emitter-native: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-714-p2-emitter-native: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
