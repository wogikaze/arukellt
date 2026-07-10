#!/usr/bin/env python3
"""Runtime Wasm debug smoke gate (#638)."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SMOKE_ARK = Path("tests/fixtures/selfhost/debug_smoke.ark")
BUILD_DIR = REPO_ROOT / ".build" / "debug-smoke"
BUILD_WASM = BUILD_DIR / "smoke.wasm"
BUILD_PATCHED = BUILD_DIR / "smoke.debug.wasm"
WRAPPER = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
HOST_LINKER = REPO_ROOT / "tools" / "host-linker"
PROBE = REPO_ROOT / "target" / "release" / "arukellt-debug-probe"
ADAPTER = REPO_ROOT / "target" / "release" / "arukellt-debug-adapter"
PREPARE = REPO_ROOT / "target" / "release" / "arukellt-debug-prepare"
BOOTSTRAP_WASM = REPO_ROOT / "bootstrap" / "arukellt-selfhost.wasm"


def _compile_smoke() -> Path:
    BUILD_DIR.mkdir(parents=True, exist_ok=True)
    env = {**os.environ}
    s2 = REPO_ROOT / ".build" / "selfhost" / "arukellt-s2-runtime.wasm"
    if s2.is_file():
        env["ARUKELLT_SELFHOST_WASM"] = str(s2)
    elif BOOTSTRAP_WASM.is_file():
        env["ARUKELLT_SELFHOST_WASM"] = str(BOOTSTRAP_WASM)
    build_wasm_rel = BUILD_WASM.relative_to(REPO_ROOT)
    r = subprocess.run(
        [str(WRAPPER), "compile", str(SMOKE_ARK), "-o", str(build_wasm_rel)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=300,
        env=env,
    )
    if r.returncode != 0:
        print("FAIL: compile smoke program", file=sys.stderr)
        print(r.stderr, file=sys.stderr)
        raise SystemExit(1)
    if not BUILD_WASM.is_file():
        print(f"FAIL: wasm not found at {BUILD_WASM}", file=sys.stderr)
        raise SystemExit(1)
    return BUILD_WASM


def _prepare_debug_wasm(wasm_path: Path) -> Path:
    ark_path = REPO_ROOT / SMOKE_ARK
    r = subprocess.run(
        [
            str(PREPARE),
            str(wasm_path),
            str(ark_path),
            "3",
            str(BUILD_PATCHED),
        ],
        capture_output=True,
        text=True,
        timeout=120,
    )
    if r.returncode != 0:
        print("FAIL: debug prepare", file=sys.stderr)
        print(r.stderr, file=sys.stderr)
        raise SystemExit(1)
    return BUILD_PATCHED


def _dap_frame(body: str) -> str:
    encoded = body.encode("utf-8")
    return f"Content-Length: {len(encoded)}\r\n\r\n{body}"


def _dap_request(seq: int, command: str, arguments: dict | None = None) -> str:
    payload = {
        "seq": seq,
        "type": "request",
        "command": command,
        "arguments": arguments or {},
    }
    return _dap_frame(json.dumps(payload))


def main() -> int:
    for tool in (PROBE, ADAPTER, PREPARE):
        if not tool.is_file():
            print("FAIL: build tools/host-linker (cargo build --release)", file=sys.stderr)
            return 1

    wasm_path = _compile_smoke()
    patched = _prepare_debug_wasm(wasm_path)
    data = patched.read_bytes()
    if b"metadata.debug.source_map" not in data:
        print("FAIL: metadata.debug.source_map section missing", file=sys.stderr)
        return 1

    ark_path = REPO_ROOT / SMOKE_ARK
    probe = subprocess.run(
        [str(PROBE), str(wasm_path), "3", str(REPO_ROOT), str(ark_path)],
        capture_output=True,
        text=True,
        timeout=120,
    )
    if probe.returncode != 0:
        print("FAIL: debug probe", file=sys.stderr)
        print(probe.stderr, file=sys.stderr)
        return 1
    payload = json.loads(probe.stdout)
    if not payload.get("locals") or payload["locals"][0].get("value") != "42":
        print(f"FAIL: expected live local x=42, got {payload}", file=sys.stderr)
        return 1

    smoke_abs = str(ark_path)
    dap_script = "".join(
        [
            _dap_request(1, "initialize"),
            _dap_request(2, "launch", {"program": smoke_abs}),
            _dap_request(
                3,
                "setBreakpoints",
                {
                    "source": {"path": smoke_abs},
                    "breakpoints": [{"line": 3}],
                },
            ),
            _dap_request(4, "configurationDone"),
            _dap_request(5, "variables", {"variablesReference": 1}),
        ]
    )
    dap = subprocess.run(
        [str(ADAPTER)],
        cwd=str(REPO_ROOT),
        input=dap_script,
        capture_output=True,
        text=True,
        timeout=300,
        env={"ARUKELLT_REPO_ROOT": str(REPO_ROOT), **os.environ},
    )
    if dap.returncode != 0:
        print("FAIL: debug-adapter", file=sys.stderr)
        print(dap.stderr, file=sys.stderr)
        return 1
    if '"value":"42"' not in dap.stdout and '"value": "42"' not in dap.stdout:
        print("FAIL: DAP variables missing live value 42", file=sys.stderr)
        print(dap.stdout, file=sys.stderr)
        return 1

    print("check-wasm-debug-smoke: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
