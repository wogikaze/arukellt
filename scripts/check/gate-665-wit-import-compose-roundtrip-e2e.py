#!/usr/bin/env python3
"""Validate an Ark WIT-import consumer against an official Component provider."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = REPO_ROOT / "tests/fixtures/wit_import/compose_roundtrip"
MAIN_ARK = FIXTURE / "main.ark"
PROVIDER_WIT = FIXTURE / "provider.wit"
WASI_P2_WIT_DIR = REPO_ROOT / "scripts/selfhost/wit/deps/wasi-cli-0.2.0/wit"


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
    return shutil.which(name)


def _copy_official_wit_dependencies(tree: Path) -> None:
    cli_root = tree / "deps/wasi-cli"
    cli_root.mkdir(parents=True, exist_ok=True)
    for wit_file in WASI_P2_WIT_DIR.glob("*.wit"):
        shutil.copy2(wit_file, cli_root / wit_file.name)
    for dependency in ("clocks", "filesystem", "io", "random", "sockets"):
        dependency_root = tree / "deps" / dependency
        dependency_root.mkdir(parents=True, exist_ok=True)
        for wit_file in (WASI_P2_WIT_DIR / "deps" / dependency).glob("*.wit"):
            shutil.copy2(wit_file, dependency_root / wit_file.name)


def _static_evidence() -> tuple[int, str]:
    required = [
        FIXTURE / "ark.toml",
        MAIN_ARK,
        FIXTURE / "vendor/host/mod.wit",
        PROVIDER_WIT,
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
    provider = PROVIDER_WIT.read_text(encoding="utf-8")
    required_provider_parts = (
        "world provider",
        "export math",
        "world consumer",
        "import math",
        "export run",
    )
    if any(part not in provider for part in required_provider_parts):
        return 1, "provider.wit lacks the official WIT provider world"
    imports = (REPO_ROOT / "src/compiler/wasm/sections_wit_imports.ark").read_text(encoding="utf-8")
    if "cm32p2|" not in imports or "component_interface_id" not in imports:
        return 1, "P2 WIT imports are not emitted with the standard32 module prefix"
    return 0, ""


def _build_provider(out_dir: Path) -> tuple[int, str, Path | None]:
    wasm_tools = _find_tool("wasm-tools")
    if wasm_tools is None:
        return 2, "wasm-tools not in PATH", None
    core = out_dir / "provider.core.wasm"
    component = out_dir / "provider.component.wasm"
    wit_tree = out_dir / "provider-wit"
    wit_tree.mkdir(parents=True, exist_ok=True)
    shutil.copy2(PROVIDER_WIT, wit_tree / "provider.wit")
    _copy_official_wit_dependencies(wit_tree)
    result = subprocess.run(
        [
            wasm_tools,
            "component",
            "embed",
            str(wit_tree),
            "--world",
            "provider",
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
    if not component.is_file():
        return 1, f"missing provider component {component}", None
    return 0, "", component


def _compile_consumer(out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    out.parent.mkdir(parents=True, exist_ok=True)
    result = subprocess.run(
        [
            *compiler,
            "compile",
            str(MAIN_ARK.relative_to(REPO_ROOT)),
            "--target",
            "wasm32-gc",
            "--wasi-version",
            "wasi-p2",
            "--emit",
            "component",
            "-o",
            str(out.relative_to(REPO_ROOT)),
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=300,
        env=_compile_env(),
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    if not out.is_file():
        return 1, f"missing consumer component {out}"
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


def _compose_validate(provider: Path, consumer: Path, out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    # The selfhost compiler runs with the repository root as its WASI preopen.
    # Pass repository-relative paths so its filesystem API can resolve the
    # provider and consumer components consistently with other selfhost gates.
    provider_arg = str(provider.relative_to(REPO_ROOT))
    consumer_arg = str(consumer.relative_to(REPO_ROOT))
    output_arg = str(out.relative_to(REPO_ROOT))
    result = subprocess.run(
        [
            *compiler,
            "compose",
            "--validate",
            "--plug",
            provider_arg,
            consumer_arg,
            "-o",
            output_arg,
        ],
        cwd=REPO_ROOT,
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


def _wac_plug(provider: Path, consumer: Path, out: Path) -> tuple[int, str]:
    wac = _find_tool("wac")
    if not wac:
        return 2, "wac not in PATH"
    result = subprocess.run(
        [wac, "plug", "--plug", str(provider), str(consumer), "-o", str(out)],
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    if not out.is_file():
        return 1, f"missing composed component {out}"
    return 0, ""


def _consumer_has_standard_wit_import(path: Path) -> tuple[int, str]:
    wasm_tools = _find_tool("wasm-tools")
    if wasm_tools is None:
        return 2, "wasm-tools not in PATH"
    result = subprocess.run(
        [wasm_tools, "print", str(path)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    printed = result.stdout
    if '(import "cm32p2|test:host/math@0.1" "add"' not in printed:
        return 1, "consumer lacks the standard32 cm32p2 test:host/math@0.1 import"
    if '(import "test:host/math" "add"' in printed:
        return 1, "consumer still contains the retired unqualified WIT import"
    return 0, ""


def main() -> int:
    static_rc, static_msg = _static_evidence()
    if static_rc != 0:
        print(f"gate-665: FAIL: static evidence: {static_msg}", file=sys.stderr)
        return 1

    if _find_tool("wasm-tools") is None:
        print("gate-665: PASS (static; wasm-tools unavailable)")
        return 0

    build_dir = REPO_ROOT / ".build"
    build_dir.mkdir(parents=True, exist_ok=True)
    out_dir = Path(tempfile.mkdtemp(prefix="gate-665-", dir=build_dir))
    try:
        rc, msg, provider = _build_provider(out_dir)
        if rc != 0 or provider is None:
            print(f"gate-665: FAIL: provider packaging: {msg}", file=sys.stderr)
            return 1

        consumer = out_dir / "consumer.component.wasm"
        composed = out_dir / "composed.component.wasm"
        rc, msg = _compile_consumer(consumer)
        if rc == 2:
            print(f"gate-665: PASS (static; consumer compiler unavailable: {msg})")
            return 0
        if rc != 0:
            print(f"gate-665: FAIL: consumer compile: {msg}", file=sys.stderr)
            return 1

        rc, msg = _wasm_tools_validate(consumer)
        if rc != 0:
            print(f"gate-665: FAIL: consumer validate: {msg}", file=sys.stderr)
            return 1

        rc, msg = _consumer_has_standard_wit_import(consumer)
        if rc == 2:
            print(f"gate-665: PASS (static; standard WIT inspection unavailable: {msg})")
            return 0
        if rc != 0:
            print(f"gate-665: FAIL: consumer WIT import: {msg}", file=sys.stderr)
            return 1

        rc, msg = _compose_validate(provider, consumer, composed)
        if rc == 2:
            print(f"gate-665: PASS (static; compose compiler unavailable: {msg})")
            return 0
        if rc != 0:
            print(f"gate-665: FAIL: compose validation: {msg}", file=sys.stderr)
            return 1

        rc, msg = _wac_plug(provider, consumer, composed)
        if rc == 0:
            rc, msg = _wasm_tools_validate(composed)
            if rc != 0:
                print(f"gate-665: FAIL: composed component validate: {msg}", file=sys.stderr)
                return 1
        elif rc != 2:
            print(f"gate-665: FAIL: wac plug: {msg}", file=sys.stderr)
            return 1

        print("gate-665-wit-import-compose-roundtrip-e2e: PASS")
        return 0
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
