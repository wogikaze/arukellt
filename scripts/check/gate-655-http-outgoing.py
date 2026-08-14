#!/usr/bin/env python3
"""Close gate for #655 on the real-WASI runtime-adapter path."""
from __future__ import annotations
import sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]

def fail(msg: str) -> int:
    print(f"gate-655-http-outgoing: FAIL: {msg}", file=sys.stderr); return 1

def main() -> int:
    for rel in ("std/host/http.ark", "tests/fixtures/wasi_http_p2.ark", "tests/fixtures/host/http/get_err_dns.ark"):
        if not (ROOT / rel).is_file(): return fail(f"missing {rel}")
    component = (ROOT / "src/compiler/wasm/component_p2_runtime.ark").read_text(encoding="utf-8")
    if "runtime-http-get" not in component or "runtime-http-request" not in component:
        return fail("component runtime HTTP imports missing")
    adapter = (ROOT / "runtime/wasi-p2-adapter/src/lib.rs").read_text(encoding="utf-8")
    for marker in ("wasip2::http::outgoing_handler", "OutgoingRequest", "http_get"):
        if marker not in adapter: return fail(f"adapter marker missing: {marker}")
    if (ROOT / "tools/host-linker/src/host_http.rs").exists(): return fail("legacy host_http shim remains")
    manifest = (ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    for entry in ("component-compile:wasi_http_p2.ark", "t3-run:host/http/get_err_dns.ark"):
        if entry not in manifest: return fail(f"manifest missing {entry}")
    print("gate-655-http-outgoing: PASS"); return 0

if __name__ == "__main__": raise SystemExit(main())
