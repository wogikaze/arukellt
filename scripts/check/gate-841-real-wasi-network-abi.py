#!/usr/bin/env python3
"""Close gate for #841: product network paths use the real-WASI adapter."""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def fail(message: str) -> int:
    print(f"gate-841: FAIL: {message}", file=sys.stderr)
    return 1


def main() -> int:
    for path in (ROOT / "tools/host-linker/src/host_http.rs", ROOT / "tools/host-linker/src/host_sockets.rs"):
        if path.exists():
            return fail(f"legacy host shim still exists: {path.relative_to(ROOT)}")

    linker = (ROOT / "tools/host-linker/src/lib.rs").read_text(encoding="utf-8")
    if "register_http_host_fns" in linker or "register_sockets_host_fns" in linker:
        return fail("host-linker still registers HTTP/socket bridge functions")

    adapter = (ROOT / "runtime/wasi-p2-adapter/src/lib.rs").read_text(encoding="utf-8")
    required_adapter = (
        "wasip2::http::outgoing_handler",
        "OutgoingRequest",
        "TcpStream",
        "TcpListener",
    )
    for marker in required_adapter:
        if marker not in adapter:
            return fail(f"real-WASI adapter marker missing: {marker}")

    component = (ROOT / "src/compiler/wasm/component_p2_runtime.ark").read_text(encoding="utf-8")
    if "runtime-http-get" not in component or "runtime-socket-connect" not in component:
        return fail("component canonical runtime imports missing")
    launcher = (ROOT / "scripts/run/arukellt-selfhost.sh").read_text(encoding="utf-8")
    if "wac plug --plug" not in launcher:
        return fail("product component path does not plug the checked adapter")
    if "arukellt-run-hosted.sh" in launcher and "http_get|http_request|http_serve" in launcher:
        return fail("network runtime still falls back to custom hosted runner")
    if "needs_arukellt_host" in "\n".join(
        p.read_text(encoding="utf-8", errors="ignore")
        for p in (ROOT / "src").rglob("*.ark")
    ):
        return fail("legacy needs_arukellt_host flag remains")

    if os.environ.get("ARUKELLT_REQUIRE_RUNTIME_E2E") == "1":
        for tool in ("wasmtime", "wac"):
            if shutil.which(tool) is None:
                return fail(f"required E2E tool missing: {tool}")
        fixture = ROOT / "tests/fixtures/host/http_get.ark"
        if fixture.exists():
            run = subprocess.run(
                [str(ROOT / "scripts/run/arukellt-selfhost.sh"), "run", str(fixture)],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            if run.returncode != 0:
                return fail("bare Wasmtime HTTP fixture failed: " + run.stderr[-800:])

    print("gate-841-real-wasi-network-abi: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
