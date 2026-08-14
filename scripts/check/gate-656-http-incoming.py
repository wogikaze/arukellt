#!/usr/bin/env python3
"""Close gate for #656 on the real-WASI runtime-adapter path."""
from __future__ import annotations
import sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]

def fail(msg: str) -> int:
    print(f"gate-656-http-incoming: FAIL: {msg}", file=sys.stderr); return 1

def main() -> int:
    for rel in ("std/host/http.ark", "tests/fixtures/host/http/incoming_smoke.ark"):
        if not (ROOT / rel).is_file(): return fail(f"missing {rel}")
    component = (ROOT / "src/compiler/wasm/component_p2_runtime.ark").read_text(encoding="utf-8")
    if "runtime-http-serve" not in component: return fail("component runtime HTTP serve import missing")
    adapter = (ROOT / "runtime/wasi-p2-adapter/src/lib.rs").read_text(encoding="utf-8")
    for marker in ("TcpListener", "http_serve_once"):
        if marker not in adapter: return fail(f"adapter marker missing: {marker}")
    host = (ROOT / "std/host/http.ark").read_text(encoding="utf-8")
    if "__runtime_abi_http_serve" not in host: return fail("std host serve does not use runtime ABI")
    if (ROOT / "tools/host-linker/src/host_http.rs").exists(): return fail("legacy host_http shim remains")
    manifest = (ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    if "t3-run:host/http/incoming_smoke.ark" not in manifest: return fail("incoming fixture missing from manifest")
    print("gate-656-http-incoming: PASS"); return 0

if __name__ == "__main__": raise SystemExit(main())
