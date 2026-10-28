#!/usr/bin/env python3
"""Gate for #668 args/env slice — official WASI P2 environment interfaces.

Proves:
1. The component imports wasi:cli/environment with the canonical functions
2. The artifact contains no P1-shaped or repository-specific compatibility ABI
3. wasmtime run with CLI args prints user args (excluding argv[0])
4. wasmtime --env delivers env::var values
5. `Env::arg_count` / `Env::arg_at` return the same values from the
   wasm32-gc P2 component and the wasm32 linear Preview 1 core for the
   same guest argv
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


def _compile(
    fixture_rel: str,
    out: Path,
    *,
    target: str,
    wasi_version: str,
    emit: str,
    memory_width: str | None = None,
) -> tuple[int, str]:
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
        target,
        "--wasi-version",
        wasi_version,
        "--emit",
        emit,
        "-o",
        out_arg,
    ]
    if memory_width is not None:
        cmd.extend(["--memory-width", memory_width])
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


def _assert_environment_import_shape(path: Path) -> tuple[int, str]:
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
        "import wasi:cli/environment@0.2.0",
        "get-arguments: func() -> list<string>",
        "get-environment: func() -> list<tuple<string, string>>",
    ):
        if marker not in text:
            return 1, f"component WIT missing official environment contract: {marker}"
    data = path.read_bytes()
    for marker in (b"wasi_snapshot_preview1", b"args-sizes", b"environ-get", b"arukellt:"):
        if marker in data:
            return 1, f"artifact contains retired compatibility marker {marker!r}"
    return 0, ""


def _wasmtime_run(
    path: Path,
    *,
    args: list[str] | None = None,
    env: dict[str, str] | None = None,
    expect_stdout: str = "",
) -> tuple[int, str]:
    cmd = [
        "wasmtime",
        "run",
        "--wasm",
        "gc",
        "--wasm",
        "function-references",
    ]
    if env:
        for key, value in env.items():
            cmd.extend(["--env", f"{key}={value}"])
    cmd.append(str(path))
    # Positional guest args only — a "--" separator is forwarded into
    # wasi:cli/environment get-arguments under wasmtime component run.
    if args:
        cmd.extend(args)
    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, f"wasmtime exit {result.returncode}: {(result.stderr or '')[-400:]}"
    if expect_stdout and expect_stdout not in (result.stdout or ""):
        return 1, f"expected stdout {expect_stdout!r}, got {result.stdout!r}"
    return 0, ""


def main() -> int:
    failures: list[str] = []
    out_dir = Path(tempfile.mkdtemp(prefix="gate-668-args-env-", dir=REPO_ROOT / ".build"))
    try:
        args_out = out_dir / "args.component.wasm"
        rc, msg = _compile(
            "tests/fixtures/wasi_p2_native/args.ark",
            args_out,
            target="wasm32-gc",
            wasi_version="wasi-p2",
            emit="component",
        )
        if rc != 0:
            failures.append(f"args compile: {msg}")
        else:
            for name, fn in (
                ("args validate", lambda: _validate(args_out)),
                ("args import shape", lambda: _assert_environment_import_shape(args_out)),
                (
                    "args wasmtime",
                    lambda: _wasmtime_run(
                        args_out,
                        args=["alpha", "beta"],
                        expect_stdout="alpha\nbeta\n",
                    ),
                ),
            ):
                rc, msg = fn()
                if rc != 0:
                    failures.append(f"{name}: {msg}")

        env_out = out_dir / "env_var.component.wasm"
        rc, msg = _compile(
            "tests/fixtures/wasi_p2_native/env_var.ark",
            env_out,
            target="wasm32-gc",
            wasi_version="wasi-p2",
            emit="component",
        )
        if rc != 0:
            failures.append(f"env_var compile: {msg}")
        else:
            for name, fn in (
                ("env_var validate", lambda: _validate(env_out)),
                ("env_var import shape", lambda: _assert_environment_import_shape(env_out)),
                (
                    "env_var wasmtime",
                    lambda: _wasmtime_run(
                        env_out,
                        env={"ARUKELLT_P2_ENV_PROOF": "env-ok"},
                        expect_stdout="env-ok\n",
                    ),
                ),
            ):
                rc, msg = fn()
                if rc != 0:
                    failures.append(f"{name}: {msg}")

        arg_at_expected = (
            REPO_ROOT / "tests/fixtures/wasi_p2_native/arg_at.expected"
        ).read_text(encoding="utf-8").strip()
        arg_at_component = out_dir / "arg_at.component.wasm"
        rc, msg = _compile(
            "tests/fixtures/wasi_p2_native/arg_at.ark",
            arg_at_component,
            target="wasm32-gc",
            wasi_version="wasi-p2",
            emit="component",
        )
        if rc != 0:
            failures.append(f"arg_at component compile: {msg}")
        else:
            for name, fn in (
                ("arg_at component validate", lambda: _validate(arg_at_component)),
                (
                    "arg_at component wasmtime",
                    lambda: _wasmtime_run(
                        arg_at_component,
                        args=["alpha", "beta"],
                        expect_stdout=arg_at_expected,
                    ),
                ),
            ):
                rc, msg = fn()
                if rc != 0:
                    failures.append(f"{name}: {msg}")

        arg_at_linear = out_dir / "arg_at.linear.wasm"
        rc, msg = _compile(
            "tests/fixtures/wasi_p2_native/arg_at.ark",
            arg_at_linear,
            target="wasm32",
            wasi_version="wasi-p1",
            emit="wasm",
            memory_width="memory32",
        )
        if rc != 0:
            failures.append(f"arg_at linear compile: {msg}")
        else:
            for name, fn in (
                ("arg_at linear validate", lambda: _validate(arg_at_linear)),
                (
                    "arg_at linear wasmtime",
                    lambda: _wasmtime_run(
                        arg_at_linear,
                        args=["alpha", "beta"],
                        expect_stdout=arg_at_expected,
                    ),
                ),
            ):
                rc, msg = fn()
                if rc != 0:
                    failures.append(f"{name}: {msg}")
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)

    if failures:
        print("gate-668-p2-args-env: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-668-p2-args-env: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
