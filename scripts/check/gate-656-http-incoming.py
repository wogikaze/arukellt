#!/usr/bin/env python3
"""Close gate for issue #656 — HTTP incoming server on T3."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def _static_evidence() -> tuple[int, str]:
    required = [
        REPO_ROOT / "std/host/http.ark",
        REPO_ROOT / "src/compiler/wasm/import_indices.ark",
        REPO_ROOT / "tests/fixtures/host/http/incoming_smoke.ark",
        REPO_ROOT / "tests/fixtures/target_gating/t1_import_http.ark",
    ]
    for path in required:
        if not path.is_file():
            return 1, f"missing {path.relative_to(REPO_ROOT)}"
    imports = (REPO_ROOT / "src/compiler/wasm/sections_imports.ark").read_text(encoding="utf-8")
    if "wasi:http/incoming-handler@0.2.0" not in imports:
        return 1, "sections_imports.ark lacks wasi:http/incoming-handler import"
    if "http_serve" not in imports:
        return 1, "sections_imports.ark lacks arukellt_host http_serve import"
    gate = (REPO_ROOT / "src/compiler/resolver/target_gate.ark").read_text(encoding="utf-8")
    if "std::host::http" not in gate:
        return 1, "target_gate.ark lacks std::host::http T1 gate"
    manifest = (REPO_ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    if "t3-run:host/http/incoming_smoke.ark" not in manifest:
        return 1, "manifest missing t3-run:host/http/incoming_smoke.ark"
    text = (REPO_ROOT / "std/host/http.ark").read_text(encoding="utf-8")
    if "serve" not in text or "__intrinsic_http_serve" not in text:
        return 1, "std/host/http.ark lacks serve / __intrinsic_http_serve"
    return 0, ""


def _host_linker_incoming_test() -> tuple[int, str]:
    result = subprocess.run(
        ["cargo", "test", "--lib", "http_incoming_roundtrip", "--", "--nocapture"],
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
        ("host-linker incoming", _host_linker_incoming_test),
    ):
        rc, msg = fn()
        if rc != 0:
            failures.append(f"{name}: {msg}")
    if failures:
        print("gate-656-http-incoming: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-656-http-incoming: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
