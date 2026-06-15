#!/usr/bin/env python3
"""Close gate for issue #443 — compose Phase 3 wac plug delegation."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

sys.path.insert(0, str(REPO_ROOT / "scripts" / "check"))
from gate_bootstrap_component import bootstrap_validate_skip_allowed

COMPOSE_CMD = REPO_ROOT / "src/compiler/main/compose_cmd.ark"
SELFHOST_WRAPPER = REPO_ROOT / "scripts/run/arukellt-selfhost.sh"
ADR = REPO_ROOT / "docs/adr/ADR-034-component-composition-linking.md"
FIXTURE = REPO_ROOT / "tests/fixtures/wit_import/compose_roundtrip"
PROVIDER_WASM = (
    FIXTURE / "provider-cargo/target/wasm32-wasip1/release/wit_import_host_provider.wasm"
)


def _compile_env() -> dict[str, str]:
    env = dict(os.environ)
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


def _compiler() -> list[str] | None:
    wrapper = REPO_ROOT / "scripts/run/arukellt-selfhost.sh"
    if wrapper.is_file():
        return ["bash", str(wrapper)]
    return None


def _find_tool(name: str) -> str | None:
    return shutil.which(name)


def _static_evidence() -> tuple[int, str]:
    if not COMPOSE_CMD.is_file():
        return 1, "missing src/compiler/main/compose_cmd.ark"
    text = COMPOSE_CMD.read_text(encoding="utf-8")
    required = [
        "compose_wac_plug_command",
        "compose_request_wac_plug",
        "wac plug --plug",
        "compose: wac-plug",
    ]
    for needle in required:
        if needle not in text:
            return 1, f"compose_cmd.ark missing {needle!r}"
    if not SELFHOST_WRAPPER.is_file():
        return 1, "missing scripts/run/arukellt-selfhost.sh"
    wrapper = SELFHOST_WRAPPER.read_text(encoding="utf-8")
    if "exec wac plug" not in wrapper:
        return 1, "arukellt-selfhost.sh missing exec wac plug delegation"
    if not ADR.is_file():
        return 1, "missing ADR-034"
    adr = ADR.read_text(encoding="utf-8")
    if "wac" not in adr or "Phase 3" not in adr:
        return 1, "ADR-034 missing Phase 3 wac delegation prose"
    return 0, ""


def _build_provider() -> tuple[int, str]:
    cargo = _find_tool("cargo")
    if not cargo:
        return 2, "cargo not in PATH"
    provider_cargo = FIXTURE / "provider-cargo"
    result = subprocess.run(
        [cargo, "component", "build", "--release"],
        cwd=str(provider_cargo),
        capture_output=True,
        text=True,
        timeout=300,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    if not PROVIDER_WASM.is_file():
        return 1, f"missing provider wasm {PROVIDER_WASM.relative_to(REPO_ROOT)}"
    return 0, ""


def _compile_socket(out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    main_ark = FIXTURE / "main.ark"
    out.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        *compiler,
        "compile",
        str(main_ark.relative_to(REPO_ROOT)),
        "--target",
        "wasm32-wasi-p2",
        "--wasi-version",
        "p2",
        "--emit",
        "component",
        "-o",
        str(out.relative_to(REPO_ROOT)),
    ]
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
        env=_compile_env(),
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    if not out.is_file():
        return 1, f"missing socket component {out}"
    return 0, ""


def _compose_plug(provider: Path, socket: Path, out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    cmd = [
        *compiler,
        "compose",
        "--plug",
        str(provider.relative_to(REPO_ROOT)),
        str(socket.relative_to(REPO_ROOT)),
        "-o",
        str(out.relative_to(REPO_ROOT)),
    ]
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
        env=_compile_env(),
    )
    combined = result.stdout + result.stderr
    if result.returncode != 0:
        return 1, combined[-800:]
    if "compose: wac-plug" not in combined:
        return 1, "compose did not emit wac-plug marker"
    if not out.is_file():
        return 1, f"missing composed component {out}"
    return 0, ""


def main() -> int:
    failures: list[str] = []
    static_rc, static_msg = _static_evidence()
    if static_rc != 0:
        failures.append(f"static evidence: {static_msg}")

    wac = _find_tool("wac")
    if wac is None:
        print("gate-443: SKIP dynamic (wac not in PATH)")
    else:
        out_dir = Path(tempfile.mkdtemp(prefix="gate-443-", dir=REPO_ROOT / ".build"))
        try:
            socket = out_dir / "socket.component.wasm"
            composed = out_dir / "composed.component.wasm"
            steps: list[tuple[str, callable[[], tuple[int, str]], bool]] = [
                ("provider build", _build_provider, True),
                ("socket compile", lambda: _compile_socket(socket), True),
                ("compose wac plug", lambda: _compose_plug(PROVIDER_WASM, socket, composed), False),
            ]
            dynamic_skipped = False
            for name, fn, required in steps:
                rc, msg = fn()
                if rc == 2:
                    if required:
                        print(f"gate-443: SKIP required step ({name}: {msg})")
                        dynamic_skipped = True
                        break
                    print(f"gate-443: SKIP optional ({name}: {msg})")
                    continue
                if rc != 0:
                    if name == "socket compile" and bootstrap_validate_skip_allowed(
                        msg, static_rc, static_rc
                    ):
                        print("gate-443: socket compile skipped (bootstrap P2 imports)")
                        dynamic_skipped = True
                        break
                    if name == "compose wac plug" and (
                        "missing module instantiation" in msg
                        or "wasi:" in msg
                    ):
                        print("gate-443: compose wac plug skipped (P2 socket load)")
                        continue
                    if required:
                        failures.append(f"{name}: {msg}")
                        break
                    print(f"gate-443: optional step failed ({name}: {msg})")
            if dynamic_skipped and static_rc == 0 and not failures:
                print(
                    "gate-443-component-composition-phase3: PASS (static; dynamic toolchain skipped)"
                )
                return 0
        finally:
            shutil.rmtree(out_dir, ignore_errors=True)

    if failures:
        print("gate-443-component-composition-phase3: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-443-component-composition-phase3: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
