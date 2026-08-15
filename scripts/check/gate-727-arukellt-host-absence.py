#!/usr/bin/env python3
"""Absence gate for #727 after #841 real-WASI productionization."""
from __future__ import annotations
import subprocess
import sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]

def fail(msg: str) -> int:
    print(f"gate-727-arukellt-host-absence: FAIL: {msg}", file=sys.stderr); return 1

def main() -> int:
    for rel in ("tools/host-linker/src/host_http.rs", "tools/host-linker/src/host_sockets.rs"):
        if (ROOT / rel).exists(): return fail(f"legacy shim remains: {rel}")
    for rel in ("src/compiler/wasm/sections_imports.ark", "src/compiler/wasm/component_p2_runtime.ark", "scripts/run/arukellt-selfhost.sh"):
        text = (ROOT / rel).read_text(encoding="utf-8")
        if '"arukellt_host"' in text or "'arukellt_host'" in text:
            return fail(f"legacy arukellt_host module remains in {rel}")
    gate = ROOT / "scripts/check/gate-841-real-wasi-network-abi.py"
    result = subprocess.run([sys.executable, str(gate)], cwd=ROOT, capture_output=True, text=True, timeout=60)
    if result.returncode != 0: return fail((result.stdout + result.stderr)[-800:])
    print("gate-727-arukellt-host-absence: PASS (real-WASI path)"); return 0

if __name__ == "__main__": raise SystemExit(main())
