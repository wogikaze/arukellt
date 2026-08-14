#!/usr/bin/env python3
"""Close gate for #676: std::host fs/env/process production surface."""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def fail(message: str) -> int:
    print(f"gate-676: FAIL: {message}", file=sys.stderr)
    return 1


def require(path: str, markers: tuple[str, ...]) -> int:
    text = (ROOT / path).read_text(encoding="utf-8")
    for marker in markers:
        if marker not in text:
            return fail(f"{path}: missing {marker}")
    return 0


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
        launcher = ROOT / "scripts/run/arukellt-selfhost.sh"
        for rel, expected in (
            ("tests/fixtures/host/fs_env_process_surface.ark", "host surface ok"),
            ("tests/fixtures/host/fs_path_traversal.ark", "sandbox denied"),
        ):
            run = subprocess.run([str(launcher), "run", str(ROOT / rel)], cwd=ROOT, text=True, capture_output=True)
            if run.returncode != 0:
                return fail(f"E2E failed for {rel}: {run.stderr[-1200:]}")
            if expected not in run.stdout:
                return fail(f"E2E output for {rel} did not contain {expected!r}: {run.stdout!r}")

    print("gate-676-std-host-fs-env-process: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
