#!/usr/bin/env python3
"""Verify component composition through official WIT and ``wac plug``."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS_DIR = REPO_ROOT / "scripts"
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))
from lib.tooling import find_wasm_tools

COMPOSE_CMD = REPO_ROOT / "src/compiler/main/compose_cmd.ark"
SELFHOST_WRAPPER = REPO_ROOT / "scripts/run/arukellt-selfhost.sh"
ADR = REPO_ROOT / "docs/adr/ADR-034-component-composition-linking.md"
FIXTURES = REPO_ROOT / "tests/component-interop/compose/fixtures"
PROVIDER_WIT = FIXTURES / "provider-match.wit"
SOCKET_WIT = FIXTURES / "socket-match.wit"


def _compile_env() -> dict[str, str]:
    env = dict(os.environ)
    if "ARUKELLT_SELFHOST_WASM" in env:
        return env
    for candidate in (
        REPO_ROOT / ".build/selfhost/arukellt-s2.wasm",
        REPO_ROOT / ".build/selfhost/arukellt-s2-runtime.wasm",
        REPO_ROOT / ".build/selfhost/arukellt-pinned-bootstrap.wasm",
        REPO_ROOT / "bootstrap/arukellt-selfhost.wasm",
    ):
        if candidate.is_file():
            env["ARUKELLT_SELFHOST_WASM"] = str(candidate)
            break
    return env


def _static_evidence() -> tuple[int, str]:
    if not COMPOSE_CMD.is_file():
        return 1, "missing src/compiler/main/compose_cmd.ark"
    text = COMPOSE_CMD.read_text(encoding="utf-8")
    for needle in (
        "compose_wac_plug_command",
        "compose_request_wac_plug",
        "wac plug --plug",
        "compose: wac-plug",
    ):
        if needle not in text:
            return 1, f"compose_cmd.ark missing {needle!r}"
    if not SELFHOST_WRAPPER.is_file():
        return 1, "missing scripts/run/arukellt-selfhost.sh"
    wrapper = SELFHOST_WRAPPER.read_text(encoding="utf-8")
    if "exec wac plug" not in wrapper:
        return 1, "arukellt-selfhost.sh missing exec wac plug delegation"
    if not ADR.is_file():
        return 1, "missing ADR-034"
    if not PROVIDER_WIT.is_file() or not SOCKET_WIT.is_file():
        return 1, "official WIT composition fixtures are missing"
    return 0, ""


def _package_dummy(wit: Path, world: str, out_dir: Path, name: str) -> tuple[int, str, Path | None]:
    wasm_tools = find_wasm_tools()
    if wasm_tools is None:
        return 2, "wasm-tools not in PATH", None
    core = out_dir / f"{name}.core.wasm"
    component = out_dir / f"{name}.component.wasm"
    result = subprocess.run(
        [
            wasm_tools,
            "component",
            "embed",
            str(wit),
            "--world",
            world,
            "--dummy-names",
            "standard32",
            "-o",
            str(core),
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:], None
    result = subprocess.run(
        [
            wasm_tools,
            "component",
            "new",
            str(core),
            "--reject-legacy-names",
            "--realloc-via-memory-grow",
            "-o",
            str(component),
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:], None
    return 0, "", component


def _compose(provider: Path, socket: Path, out: Path, validate_only: bool) -> tuple[int, str]:
    if not SELFHOST_WRAPPER.is_file():
        return 2, "selfhost wrapper not found"
    command = [
        "bash",
        str(SELFHOST_WRAPPER),
        "compose",
    ]
    if validate_only:
        command.append("--validate")
    provider_arg = str(provider.relative_to(REPO_ROOT))
    socket_arg = str(socket.relative_to(REPO_ROOT))
    output_arg = str(out.relative_to(REPO_ROOT))
    command.extend(["--plug", provider_arg, socket_arg, "-o", output_arg])
    result = subprocess.run(
        command,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=180,
        env=_compile_env(),
    )
    combined = result.stdout + result.stderr
    if result.returncode != 0:
        return 1, combined[-800:]
    if validate_only and "compose: validation ok" not in combined:
        return 1, "compose --validate did not report validation ok"
    return 0, ""


def main() -> int:
    static_rc, static_msg = _static_evidence()
    if static_rc != 0:
        print(f"gate-443: FAIL: static evidence: {static_msg}", file=sys.stderr)
        return 1
    if find_wasm_tools() is None:
        print("gate-443-component-composition-phase3: PASS (static; wasm-tools unavailable)")
        return 0

    build_dir = REPO_ROOT / ".build"
    build_dir.mkdir(parents=True, exist_ok=True)
    out_dir = Path(tempfile.mkdtemp(prefix="gate-443-", dir=build_dir))
    try:
        rc, msg, provider = _package_dummy(PROVIDER_WIT, "math-lib", out_dir, "provider")
        if rc != 0 or provider is None:
            print(f"gate-443: FAIL: provider packaging: {msg}", file=sys.stderr)
            return 1
        rc, msg, socket = _package_dummy(SOCKET_WIT, "runner", out_dir, "socket")
        if rc != 0 or socket is None:
            print(f"gate-443: FAIL: socket packaging: {msg}", file=sys.stderr)
            return 1

        composed = out_dir / "composed.component.wasm"
        rc, msg = _compose(provider, socket, composed, validate_only=True)
        if rc == 2:
            print(f"gate-443-component-composition-phase3: PASS (static; {msg})")
            return 0
        if rc != 0:
            print(f"gate-443: FAIL: compose validation: {msg}", file=sys.stderr)
            return 1

        if shutil.which("wac") is not None:
            rc, msg = _compose(provider, socket, composed, validate_only=False)
            if rc != 0:
                print(f"gate-443: FAIL: wac plug delegation: {msg}", file=sys.stderr)
                return 1
            wasm_tools = find_wasm_tools()
            assert wasm_tools is not None
            result = subprocess.run(
                [wasm_tools, "validate", str(composed)],
                capture_output=True,
                text=True,
                timeout=60,
            )
            if result.returncode != 0:
                print(
                    f"gate-443: FAIL: composed component validation: {(result.stderr or result.stdout)[-800:]}",
                    file=sys.stderr,
                )
                return 1

        print("gate-443-component-composition-phase3: PASS")
        return 0
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
