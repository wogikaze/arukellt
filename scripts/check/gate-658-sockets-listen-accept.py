#!/usr/bin/env python3
"""Close gate for #658 on the real-WASI runtime-adapter path."""
from __future__ import annotations
import sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]

def fail(msg: str) -> int:
    print(f"gate-658-sockets: FAIL: {msg}", file=sys.stderr); return 1

def main() -> int:
    for rel in ("std/host/sockets.ark", "tests/fixtures/host/sockets/listen_accept.ark"):
        if not (ROOT / rel).is_file(): return fail(f"missing {rel}")
    component = (ROOT / "src/compiler/wasm/component_p2_runtime.ark").read_text(encoding="utf-8")
    for marker in ("runtime-socket-listen", "runtime-socket-accept"):
        if marker not in component: return fail(f"component runtime socket import missing: {marker}")
    adapter = (ROOT / "runtime/wasi-p2-adapter/src/lib.rs").read_text(encoding="utf-8")
    for marker in ("TcpListener", "socket_listen", "socket_accept"):
        if marker not in adapter: return fail(f"adapter marker missing: {marker}")
    if (ROOT / "tools/host-linker/src/host_sockets.rs").exists(): return fail("legacy host_sockets shim remains")
    if (ROOT / "src/compiler/wasm/intrinsic_sockets.ark").exists(): return fail("legacy intrinsic_sockets emitter remains")
    manifest = (ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    if "t3-run:host/sockets/listen_accept.ark" not in manifest: return fail("listen/accept fixture missing from manifest")
    print("gate-658-sockets: PASS"); return 0

if __name__ == "__main__": raise SystemExit(main())
