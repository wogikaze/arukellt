#!/usr/bin/env python3
"""Close-gate helper for #657 sockets connect/read/write."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def _static_evidence() -> tuple[int, str]:
    required = [
        REPO_ROOT / "std/host/sockets.ark",
        REPO_ROOT / "src/compiler/wasm/import_indices.ark",
        REPO_ROOT / "src/compiler/wasm/intrinsic_sockets_vec.ark",
        REPO_ROOT / "tests/fixtures/host/sockets/connect_read_write.ark",
        REPO_ROOT / "tests/fixtures/target_gating/t1_import_sockets.ark",
    ]
    for path in required:
        if not path.is_file():
            return 1, f"missing {path.relative_to(REPO_ROOT)}"
    text = (REPO_ROOT / "src/compiler/wasm/sections_imports.ark").read_text(encoding="utf-8")
    if "sockets_read" not in text or "sockets_write" not in text:
        return 1, "sections_imports.ark lacks sockets_read/write imports"
    manifest = (REPO_ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    if "t3-run:host/sockets/connect_read_write.ark" not in manifest:
        return 1, "manifest missing t3-run:host/sockets/connect_read_write.ark"
    return 0, ""


def _host_linker_echo_test() -> tuple[int, str]:
    result = subprocess.run(
        ["cargo", "test", "--lib", "tcp_echo_roundtrip", "--", "--nocapture"],
        cwd=str(REPO_ROOT / "tools" / "host-linker"),
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def main() -> int:
    failures: list[str] = []
    for name, fn in (
        ("static evidence", _static_evidence),
        ("host-linker echo", _host_linker_echo_test),
    ):
        rc, msg = fn()
        if rc != 0:
            failures.append(f"{name}: {msg}")
    if failures:
        print("gate-657-sockets: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-657-sockets: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
