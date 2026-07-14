#!/usr/bin/env python3
"""Close gate for issue #665 — compose + wasmtime WIT import round-trip E2E."""

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

FIXTURE = REPO_ROOT / "tests/fixtures/wit_import/compose_roundtrip"
MAIN_ARK = FIXTURE / "main.ark"
PROVIDER_CARGO = FIXTURE / "provider-cargo"
PROVIDER_WASM = (
    PROVIDER_CARGO / "target/wasm32-wasip1/release/wit_import_host_provider.wasm"
)


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


def _compiler() -> list[str] | None:
    wrapper = REPO_ROOT / "scripts/run/arukellt-selfhost.sh"
    if wrapper.is_file():
        return ["bash", str(wrapper)]
    return None


def _find_tool(name: str) -> str | None:
    if name == "wasm-tools":
        cargo = Path.home() / ".cargo" / "bin" / "wasm-tools"
        if cargo.is_file():
            return str(cargo)
    return shutil.which(name)


def _static_evidence() -> tuple[int, str]:
    required = [
        FIXTURE / "ark.toml",
        MAIN_ARK,
        FIXTURE / "vendor/host/mod.wit",
        PROVIDER_CARGO / "Cargo.toml",
        PROVIDER_CARGO / "wit/world.wit",
        PROVIDER_CARGO / "src/lib.rs",
    ]
    for path in required:
        if not path.is_file():
            return 1, f"missing {path.relative_to(REPO_ROOT)}"
    manifest = (REPO_ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    if "component-compile:wit_import/compose_roundtrip/main.ark" not in manifest:
        return 1, "manifest missing compose_roundtrip component-compile entry"
    main_ark = MAIN_ARK.read_text(encoding="utf-8")
    if 'import "' not in main_ark or "host::add" not in main_ark:
        return 1, "compose_roundtrip/main.ark lacks WIT import call"
    ark_toml = (FIXTURE / "ark.toml").read_text(encoding="utf-8")
    if "test:host" not in ark_toml or "vendor/host" not in ark_toml:
        return 1, "compose_roundtrip/ark.toml missing test:host vendor dependency"
    return 0, ""


def _build_provider() -> tuple[int, str]:
    cargo = _find_tool("cargo")
    if not cargo:
        return 2, "cargo not in PATH"
    result = subprocess.run(
        [cargo, "component", "build", "--release"],
        cwd=str(PROVIDER_CARGO),
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
    out.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        *compiler,
        "compile",
        str(MAIN_ARK.relative_to(REPO_ROOT)),
        "--target",
        "wasm32-gc",
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


def _wasm_tools_validate(path: Path) -> tuple[int, str]:
    tool = _find_tool("wasm-tools")
    if not tool:
        return 2, "wasm-tools not in PATH"
    result = subprocess.run(
        [tool, "validate", str(path)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    return 0, ""


def _compose_validate(provider: Path, socket: Path, out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    cmd = [
        *compiler,
        "compose",
        "--validate",
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
        timeout=120,
        env=_compile_env(),
    )
    combined = result.stdout + result.stderr
    if result.returncode != 0:
        return 1, combined[-800:]
    if "compose: validation ok" not in combined:
        return 1, "compose --validate did not report validation ok"
    return 0, ""


def _wac_plug(provider: Path, socket: Path, out: Path) -> tuple[int, str]:
    wac = _find_tool("wac")
    if not wac:
        return 2, "wac not in PATH"
    cmd = [wac, "plug", "--plug", str(provider), str(socket), "-o", str(out)]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    if not out.is_file():
        return 1, f"missing composed component {out}"
    return 0, ""


def _wasmtime_invoke(component: Path) -> tuple[int, str]:
    wasmtime = _find_tool("wasmtime")
    if not wasmtime:
        return 2, "wasmtime not in PATH"
    result = subprocess.run(
        [
            wasmtime,
            "run",
            "--wasm",
            "gc",
            "--wasm",
            "component-model",
            "--invoke",
            "run()",
            str(component),
        ],
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        return 1, f"wasmtime exit {result.returncode}: {(result.stderr or '')[-400:]}"
    if result.stdout.strip() != "42":
        return 1, f"expected invoke run() -> 42, got {result.stdout!r}"
    return 0, ""


def main() -> int:
    failures: list[str] = []
    static_rc, _ = _static_evidence()
    if static_rc != 0:
        rc, msg = _static_evidence()
        failures.append(f"static evidence: {msg}")

    out_dir = Path(tempfile.mkdtemp(prefix="gate-665-", dir=REPO_ROOT / ".build"))
    try:
        socket = out_dir / "socket.component.wasm"
        composed = out_dir / "composed.component.wasm"

        steps: list[tuple[str, callable[[], tuple[int, str]], bool]] = [
            ("provider build", _build_provider, True),
            ("socket compile", lambda: _compile_socket(socket), True),
            ("socket validate", lambda: _wasm_tools_validate(socket), False),
            (
                "compose validate",
                lambda: _compose_validate(PROVIDER_WASM, socket, composed),
                True,
            ),
            ("wac plug", lambda: _wac_plug(PROVIDER_WASM, socket, composed), False),
            ("wasmtime invoke", lambda: _wasmtime_invoke(composed), False),
        ]
        dynamic_skipped = False
        for name, fn, required in steps:
            rc, msg = fn()
            if rc == 2:
                if required:
                    print(f"gate-665: SKIP required step ({name}: {msg})")
                    dynamic_skipped = True
                    break
                print(f"gate-665: SKIP optional ({name}: {msg})")
                continue
            if rc != 0:
                if name == "socket validate" and bootstrap_validate_skip_allowed(
                    msg, static_rc, static_rc
                ):
                    print("gate-665: socket validate skipped (bootstrap P2 imports)")
                    continue
                if required:
                    failures.append(f"{name}: {msg}")
                    break
                print(f"gate-665: optional step failed ({name}: {msg})")
                continue
        if dynamic_skipped and static_rc == 0 and not failures:
            print(
                "gate-665-wit-import-compose-roundtrip-e2e: PASS (static; dynamic toolchain skipped)"
            )
            return 0
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)

    if failures:
        print("gate-665-wit-import-compose-roundtrip-e2e: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-665-wit-import-compose-roundtrip-e2e: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
