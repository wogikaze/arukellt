#!/usr/bin/env python3
"""Close gate for #676: std::host fs/env/process production surface."""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BUILD = ROOT / ".build" / "gate-676"


def fail(message: str) -> int:
    print(f"gate-676: FAIL: {message}", file=sys.stderr)
    return 1


def require(path: str, markers: tuple[str, ...]) -> int:
    text = (ROOT / path).read_text(encoding="utf-8")
    for marker in markers:
        if marker not in text:
            return fail(f"{path}: missing {marker}")
    return 0


def _compile_and_run(rel: str) -> tuple[int, str, str]:
    fixture = ROOT / rel
    BUILD.mkdir(parents=True, exist_ok=True)
    component = BUILD / (fixture.stem + ".component.wasm")
    component.unlink(missing_ok=True)
    compile_run = subprocess.run(
        [
            str(ROOT / "scripts/run/arukellt-selfhost.sh"),
            "compile",
            rel,
            "--target",
            "wasm32-gc",
            "--wasi-version",
            "p2",
            "--emit",
            "component",
            "-o",
            str(component.relative_to(ROOT)),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=240,
    )
    if compile_run.returncode != 0:
        return compile_run.returncode, "", (compile_run.stdout + compile_run.stderr)[-1600:]
    if not component.is_file():
        return 1, "", "component compile produced no output"
    run = subprocess.run(
        [
            "wasmtime",
            "run",
            "--wasm",
            "gc",
            "--wasm",
            "function-references",
            f"--dir={ROOT}",
            str(component),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=120,
    )
    component.unlink(missing_ok=True)
    return run.returncode, run.stdout, run.stderr


def main() -> int:
    if require("std/host/fs.ark", (
        "pub fn read_dir(", "pub fn metadata(", "pub fn remove_file(", "pub fn create_dir_all(",
        "__runtime_abi_fs_read_dir", "__runtime_abi_fs_metadata",
    )):
        return 1
    if require("std/host/env.ark", (
        "pub fn vars_snapshot(", "pub fn current_dir(", "__runtime_abi_env_vars", "__runtime_abi_env_current_dir",
    )):
        return 1
    if require("std/host/process.ark", (
        "pub fn id() -> Result<i32, String>", "process id is not defined by the WASI 0.2 process model",
    )):
        return 1
    if require("src/compiler/main/args_parse_flags.ark", ("--deny-process", "cli_set_deny_process")):
        return 1
    if require("src/compiler/main/compile_core.ark", ("source_uses_process_intrinsics", "--deny-process: this program uses process-control intrinsics")):
        return 1

    fixtures = (
        "tests/fixtures/host/fs_env_process_surface.ark",
        "tests/fixtures/host/fs_path_traversal.ark",
        "tests/fixtures/host/process/abort.ark",
        "tests/fixtures/deny_process_compile.ark",
        "tests/fixtures/deny_process_compile.flags",
        "tests/fixtures/deny_process_compile.diag",
    )
    for rel in fixtures:
        if not (ROOT / rel).is_file():
            return fail(f"required fixture missing: {rel}")

    registry = (ROOT / "data/core-ops.toml").read_text(encoding="utf-8")
    for op in (
        "runtime.fs_read_dir", "runtime.fs_metadata", "runtime.fs_remove_file",
        "runtime.fs_create_dir_all", "runtime.env_vars", "runtime.env_current_dir",
    ):
        if f'id = "{op}"' not in registry:
            return fail(f"CoreOp missing: {op}")

    if os.environ.get("ARUKELLT_REQUIRE_RUNTIME_E2E") == "1":
        for tool in ("wasmtime", "wac"):
            if shutil.which(tool) is None:
                return fail(f"required E2E tool missing: {tool}")
        for rel, expected in (
            ("tests/fixtures/host/fs_env_process_surface.ark", "host surface ok"),
            ("tests/fixtures/host/fs_path_traversal.ark", "sandbox denied"),
        ):
            rc, stdout, stderr = _compile_and_run(rel)
            if rc != 0:
                return fail(f"direct P2 E2E failed for {rel}: {stderr[-1600:]}")
            if expected not in stdout:
                return fail(f"E2E output for {rel} did not contain {expected!r}: {stdout!r}")

    print("gate-676-std-host-fs-env-process: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
