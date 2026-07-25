#!/usr/bin/env python3
"""Gate for #668 args/env slice — P2 environment bridge + GC assembly.

Proves:
1. Artifact imports wasi:cli/environment + get-arguments / get-environment
2. Guest exports args-sizes/arguments/environ-* (bridged P1-shaped ABI)
3. wasmtime run with CLI args prints user args (excluding argv[0])
4. wasmtime --env delivers env::var values
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


def _assert_environment_import_shape(path: Path) -> tuple[int, str]:
    data = path.read_bytes()
    if b"wasi:cli/environment@0.2.0" not in data:
        return 1, "artifact missing wasi:cli/environment@0.2.0"
    if b"get-arguments" not in data:
        return 1, "artifact missing get-arguments"
    if b"get-environment" not in data:
        return 1, "artifact missing get-environment"
    if b"args-sizes" not in data:
        return 1, "artifact missing args-sizes bridge export"
    if b"environ-get" not in data:
        return 1, "artifact missing environ-get bridge export"
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
        rc, msg = _compile_component("tests/fixtures/wasi_p2_native/args.ark", args_out)
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
        rc, msg = _compile_component("tests/fixtures/wasi_p2_native/env_var.ark", env_out)
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
