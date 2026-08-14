#!/usr/bin/env python3
"""Close gate for #076: P2 filesystem is a real component-runtime path."""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "p2_fs_out.txt"


def fail(message: str) -> int:
    print(f"gate-076: FAIL: {message}", file=sys.stderr)
    return 1


def main() -> int:
    fixture = ROOT / "tests/fixtures/wasi_fs_p2.ark"
    if not fixture.is_file() or "p2_fs_out.txt" not in fixture.read_text(encoding="utf-8"):
        return fail("disk-write fixture missing")

    imports = (ROOT / "src/compiler/wasm/sections_imports.ark").read_text(encoding="utf-8")
    if "arukellt:fs@0.1.0" in imports:
        return fail("legacy arukellt:fs bridge is still imported")
    for marker in ("runtime_fs_open_at", "runtime_fs_read", "runtime_fs_write", "runtime_fs_close"):
        if marker not in imports:
            return fail(f"P2 runtime filesystem import missing: {marker}")

    adapter = (ROOT / "runtime/wasi-p2-adapter/src/lib.rs").read_text(encoding="utf-8")
    for marker in ("fs::read(", "fs::write(", "fs::read_dir(", "fs::metadata("):
        if marker not in adapter:
            return fail(f"WASI adapter filesystem implementation missing: {marker}")

    bridge = (ROOT / "runtime/wasi-p2-bridge/src/lib.rs").read_text(encoding="utf-8")
    if "runtime_fs_write" not in bridge or "host_fs_write_bytes" not in bridge:
        return fail("core compatibility bridge does not route disk writes")

    launcher = (ROOT / "scripts/run/arukellt-selfhost.sh").read_text(encoding="utf-8")
    if "plug_runtime_adapter_in_place" not in launcher or "wac plug --plug" not in launcher:
        return fail("compiled P2 components are not linked with the checked runtime adapter")

    # Branch-built E2E lane. The normal static close gate stays cheap; release
    # or explicit E2E CI sets ARUKELLT_REQUIRE_RUNTIME_E2E=1.
    if os.environ.get("ARUKELLT_REQUIRE_RUNTIME_E2E") == "1":
        for tool in ("wasmtime", "wac"):
            if shutil.which(tool) is None:
                return fail(f"required E2E tool missing: {tool}")
        OUT.unlink(missing_ok=True)
        run = subprocess.run(
            [str(ROOT / "scripts/run/arukellt-selfhost.sh"), "run", str(fixture)],
            cwd=ROOT,
            text=True,
            capture_output=True,
        )
        if run.returncode != 0:
            return fail("bare Wasmtime disk-write fixture failed: " + run.stderr[-1200:])
        if "hello p2 fs" not in run.stdout:
            return fail(f"unexpected stdout: {run.stdout!r}")
        if not OUT.is_file() or OUT.read_text(encoding="utf-8") != "hello p2 fs":
            return fail("p2_fs_out.txt was not created with the expected contents")
        OUT.unlink(missing_ok=True)

    print("gate-076-wasi-p2-filesystem: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
