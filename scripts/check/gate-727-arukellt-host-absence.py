#!/usr/bin/env python3
"""Close gate for issue #727 — retire `arukellt_host` import module (bridged).

Proves:
1. HTTP fixture imports `wasi:http/outgoing-handler@0.2.0` (not `arukellt_host`)
2. Sockets fixture imports `wasi:sockets/tcp@0.2.0` (not `arukellt_host`)
3. `wasm-tools validate` on both artifacts
4. HTTP DNS Err runs under host-linker with WIT-shaped modules (bridged guest ABI)

Real WASI method ABI + bare `wasmtime run` + delete `host_http`/`host_sockets`
remain #841 (same bridged-close pattern as #714 → #668).
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

HTTP_FIXTURE = "tests/fixtures/host/http/get_err_dns.ark"
SOCK_FIXTURE = "tests/fixtures/host/sockets/connect_dns_fail.ark"


def _compiler() -> Path | None:
    wrapper = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    return wrapper if wrapper.is_file() else None


def _compile(fixture_rel: str, out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "missing scripts/run/arukellt-selfhost.sh"
    if not (REPO_ROOT / fixture_rel).is_file():
        return 1, f"missing {fixture_rel}"
    try:
        out_arg = str(out.relative_to(REPO_ROOT))
    except ValueError:
        out_arg = str(out)
    result = subprocess.run(
        [
            "bash",
            str(compiler),
            "compile",
            fixture_rel,
            "--target",
            "wasm32-gc",
            "--wasi-version",
            "wasi-p2",
            "-o",
            out_arg,
        ],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    if not out.is_file():
        return 1, f"missing output {out}"
    return 0, ""


def _validate(path: Path) -> tuple[int, str]:
    result = subprocess.run(
        ["wasm-tools", "validate", str(path)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    return 0, ""


def _assert_no_arukellt_host(path: Path) -> tuple[int, str]:
    data = path.read_bytes()
    if b"arukellt_host" in data:
        return 1, f"{path.name}: contains arukellt_host"
    return 0, ""


def _assert_http_imports(path: Path) -> tuple[int, str]:
    data = path.read_bytes()
    if b"wasi:http/outgoing-handler@0.2.0" not in data:
        return 1, f"{path.name}: missing wasi:http/outgoing-handler@0.2.0"
    if b"http_get" not in data:
        return 1, f"{path.name}: missing bridged http_get (guest ABI)"
    return 0, ""


def _assert_sockets_imports(path: Path) -> tuple[int, str]:
    data = path.read_bytes()
    if b"wasi:sockets/tcp@0.2.0" not in data:
        return 1, f"{path.name}: missing wasi:sockets/tcp@0.2.0"
    if b"sockets_connect" not in data:
        return 1, f"{path.name}: missing bridged sockets_connect (guest ABI)"
    return 0, ""


def _host_linker_run_dns(path: Path) -> tuple[int, str]:
    runner = REPO_ROOT / "scripts" / "run" / "arukellt-run-hosted.sh"
    if not runner.is_file():
        return 1, "missing scripts/run/arukellt-run-hosted.sh"
    result = subprocess.run(
        ["bash", str(runner), str(path)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
    )
    combined = (result.stdout or "") + (result.stderr or "")
    if "dns:" not in combined.lower() and "not found" not in combined.lower():
        return 1, f"expected DNS Err output, got exit={result.returncode}: {combined[-600:]}"
    return 0, ""


def _static_source_evidence() -> tuple[int, str]:
    imports = (REPO_ROOT / "src/compiler/wasm/sections_imports.ark").read_text(encoding="utf-8")
    if "wasi:http/outgoing-handler@0.2.0" not in imports:
        return 1, "sections_imports.ark lacks wasi:http/outgoing-handler@0.2.0"
    if "wasi:sockets/tcp@0.2.0" not in imports:
        return 1, "sections_imports.ark lacks wasi:sockets/tcp@0.2.0"
    if '"arukellt_host"' in imports or "'arukellt_host'" in imports:
        return 1, "sections_imports.ark still emits arukellt_host module string"
    host_http = (REPO_ROOT / "tools/host-linker/src/host_http.rs").read_text(encoding="utf-8")
    if "wasi:http/outgoing-handler@0.2.0" not in host_http:
        return 1, "host_http.rs not bound to WIT module names"
    if 'func_wrap(\n            "arukellt_host"' in host_http or '"arukellt_host"' in host_http:
        return 1, "host_http.rs still references arukellt_host module string"
    host_sockets = (REPO_ROOT / "tools/host-linker/src/host_sockets.rs").read_text(encoding="utf-8")
    if "wasi:sockets/tcp@0.2.0" not in host_sockets:
        return 1, "host_sockets.rs not bound to WIT module names"
    if '"arukellt_host"' in host_sockets:
        return 1, "host_sockets.rs still references arukellt_host module string"
    return 0, ""


def main() -> int:
    failures: list[str] = []

    rc, msg = _static_source_evidence()
    if rc != 0:
        failures.append(f"static source: {msg}")

    with tempfile.TemporaryDirectory(prefix="gate-727-", dir=str(REPO_ROOT / ".build")) as tmp:
        tmp_path = Path(tmp)
        http_out = tmp_path / "get_err_dns.wasm"
        sock_out = tmp_path / "connect_dns_fail.wasm"

        for name, fixture, out, shape_fn in (
            ("http compile", HTTP_FIXTURE, http_out, _assert_http_imports),
            ("sockets compile", SOCK_FIXTURE, sock_out, _assert_sockets_imports),
        ):
            rc, msg = _compile(fixture, out)
            if rc != 0:
                failures.append(f"{name}: {msg}")
                continue
            for check_name, check_fn in (
                ("no arukellt_host", _assert_no_arukellt_host),
                ("import shape", shape_fn),
                ("wasm-tools validate", _validate),
            ):
                rc, msg = check_fn(out)
                if rc != 0:
                    failures.append(f"{name}/{check_name}: {msg}")

        if http_out.is_file() and not any(f.startswith("http compile") for f in failures):
            rc, msg = _host_linker_run_dns(http_out)
            if rc != 0:
                failures.append(f"host-linker dns run: {msg}")

    if failures:
        print("gate-727-arukellt-host-absence: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-727-arukellt-host-absence: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
