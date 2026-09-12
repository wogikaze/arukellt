#!/usr/bin/env python3
"""Check that filesystem access uses the official WASI P2 surface."""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "p2_fs_out.txt"
BUILD = ROOT / ".build" / "gate-076"
COMPONENT = BUILD / "wasi-fs-p2.component.wasm"


def fail(message: str) -> int:
    print(f"gate-076: FAIL: {message}", file=sys.stderr)
    return 1


def _compile_component(fixture: Path) -> tuple[int, str]:
    BUILD.mkdir(parents=True, exist_ok=True)
    COMPONENT.unlink(missing_ok=True)
    run = subprocess.run(
        [
            str(ROOT / "scripts/run/arukellt-selfhost.sh"),
            "compile",
            str(fixture.relative_to(ROOT)),
            "--target",
            "wasm32-gc",
            "--wasi-version",
            "p2",
            "--emit",
            "component",
            "-o",
            str(COMPONENT.relative_to(ROOT)),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=240,
    )
    if run.returncode != 0:
        return run.returncode, (run.stdout + run.stderr)[-1600:]
    if not COMPONENT.is_file():
        return 1, "component compile produced no output"
    return 0, ""


def _run_bare_wasmtime() -> tuple[int, str, str]:
    run = subprocess.run(
        [
            "wasmtime",
            "run",
            "--wasm",
            "gc",
            "--wasm",
            "function-references",
            f"--dir={ROOT}",
            str(COMPONENT),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=120,
    )
    return run.returncode, run.stdout, run.stderr


def main() -> int:
    fixture = ROOT / "tests/fixtures/wasi_fs_p2.ark"
    if not fixture.is_file() or "p2_fs_out.txt" not in fixture.read_text(encoding="utf-8"):
        return fail("disk-write fixture missing")

    imports = (ROOT / "src/compiler/wasm/sections_imports.ark").read_text(encoding="utf-8")
    p2_start = imports.find("fn emit_p2_import_entries(")
    if p2_start < 0:
        return fail("P2 import function missing")
    p2_end = imports.find("\nfn ", p2_start + 1)
    p2_block = imports[p2_start:] if p2_end < 0 else imports[p2_start:p2_end]
    if "arukellt:" in p2_block or "runtime/host" in p2_block:
        return fail("P2 still imports a repository-specific runtime module")
    for marker in (
        'p2_module_name("filesystem/types")',
        "descriptor.open-at",
        "descriptor.read",
        "descriptor.write",
        "descriptor_drop",
    ):
        if marker not in p2_block:
            return fail(f"official WASI filesystem import missing: {marker}")

    launcher = (ROOT / "scripts/run/arukellt-selfhost.sh").read_text(encoding="utf-8")
    if "wasmtime" not in launcher:
        return fail("selfhost launcher does not execute through Wasmtime")

    if os.environ.get("ARUKELLT_REQUIRE_RUNTIME_E2E") == "1":
        for tool in ("wasmtime", "wasm-tools"):
            if shutil.which(tool) is None:
                return fail(f"required E2E tool missing: {tool}")
        OUT.unlink(missing_ok=True)
        try:
            rc, detail = _compile_component(fixture)
            if rc != 0:
                return fail("branch-built P2 component compile failed: " + detail)
            rc, stdout, stderr = _run_bare_wasmtime()
            if rc != 0:
                return fail("bare Wasmtime disk-write fixture failed: " + stderr[-1600:])
            if stdout.strip() != "hello p2 fs":
                return fail(f"unexpected stdout: {stdout!r}")
            if not OUT.is_file() or OUT.read_text(encoding="utf-8") != "hello p2 fs":
                return fail("p2_fs_out.txt was not created with the expected contents")
        finally:
            OUT.unlink(missing_ok=True)
            COMPONENT.unlink(missing_ok=True)

    print("gate-076-wasi-p2-filesystem: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
