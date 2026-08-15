#!/usr/bin/env python3
"""Close gate for #841: product network paths use the real-WASI adapter."""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BUILD = ROOT / ".build" / "gate-841"


def fail(message: str) -> int:
    print(f"gate-841: FAIL: {message}", file=sys.stderr)
    return 1


def _compile_component(fixture: Path, out: Path) -> tuple[int, str]:
    BUILD.mkdir(parents=True, exist_ok=True)
    out.unlink(missing_ok=True)
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
            str(out.relative_to(ROOT)),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=240,
    )
    if run.returncode != 0:
        return run.returncode, (run.stdout + run.stderr)[-1600:]
    if not out.is_file():
        return 1, "component compile produced no output"
    return 0, ""


def _run_bare_wasmtime(component: Path) -> tuple[int, str, str]:
    run = subprocess.run(
        [
            "wasmtime",
            "run",
            "--wasm",
            "gc",
            "--wasm",
            "function-references",
            "-S",
            "http=y",
            "-S",
            "inherit-network=y",
            "-S",
            "allow-ip-name-lookup=y",
            f"--dir={ROOT}",
            str(component),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=120,
    )
    return run.returncode, run.stdout, run.stderr


def _network_e2e() -> tuple[int, str]:
    cases = (
        (ROOT / "tests/fixtures/wasi_http_p2.ark", BUILD / "http.component.wasm", None),
        (ROOT / "tests/fixtures/host/sockets/connect_dns_fail.ark", BUILD / "sockets.component.wasm", "dns error"),
    )
    for fixture, component, expected in cases:
        if not fixture.is_file():
            return 1, f"missing network fixture: {fixture.relative_to(ROOT)}"
        rc, detail = _compile_component(fixture, component)
        if rc != 0:
            return 1, f"compile {fixture.relative_to(ROOT)}: {detail}"
        rc, stdout, stderr = _run_bare_wasmtime(component)
        component.unlink(missing_ok=True)
        if rc != 0:
            return 1, f"bare Wasmtime {fixture.relative_to(ROOT)}: {stderr[-1600:]}"
        if "unexpected ok" in stdout:
            return 1, f"network negative fixture unexpectedly succeeded: {stdout!r}"
        if expected is not None and expected not in stdout:
            return 1, f"expected {expected!r} from {fixture.relative_to(ROOT)}, got {stdout!r}"
        if expected is None and not stdout.strip():
            return 1, f"HTTP DNS failure produced no error output: {stdout!r}"
    return 0, ""


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
    ark_sources = "\n".join(
        p.read_text(encoding="utf-8", errors="ignore")
        for p in (ROOT / "src").rglob("*.ark")
    )
    if "needs_arukellt_host" in ark_sources:
        return fail("legacy needs_arukellt_host flag remains")

    if os.environ.get("ARUKELLT_REQUIRE_RUNTIME_E2E") == "1":
        for tool in ("wasmtime", "wac"):
            if shutil.which(tool) is None:
                return fail(f"required E2E tool missing: {tool}")
        rc, detail = _network_e2e()
        if rc != 0:
            return fail(detail)

    print("gate-841-real-wasi-network-abi: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
