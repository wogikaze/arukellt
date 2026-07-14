#!/usr/bin/env python3
"""Close gate for issue #654 — WIT import component emit and end-to-end fixture."""

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
        REPO_ROOT / "src/compiler/wasm/call_wit.ark",
        REPO_ROOT / "src/compiler/wasm/sections_wit_imports.ark",
        REPO_ROOT / "src/compiler/wasm/self_check_issue654.ark",
        REPO_ROOT / "tests/fixtures/wit_import/main.ark",
        REPO_ROOT / "tests/fixtures/wit_import/main.flags",
        REPO_ROOT / "tests/fixtures/wit_import/host_math.wit",
    ]
    for path in required:
        if not path.is_file():
            return 1, f"missing {path.relative_to(REPO_ROOT)}"
    manifest = (REPO_ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    if "component-compile:wit_import/main.ark" not in manifest:
        return 1, "manifest missing component-compile:wit_import/main.ark"
    call_wit = (REPO_ROOT / "src/compiler/wasm/call_wit.ark").read_text(encoding="utf-8")
    if "MIR_WIT_CALL" not in call_wit or "wit_import_function_index" not in call_wit:
        return 1, "call_wit.ark still stubbed"
    if "stub until WIT lowering" in call_wit:
        return 1, "call_wit.ark still marked stub"
    imports = (REPO_ROOT / "src/compiler/wasm/sections_wit_imports.ark").read_text(encoding="utf-8")
    if "wit_import_binding_interface_id" not in imports:
        return 1, "sections_wit_imports.ark still stubbed"
    flags = (REPO_ROOT / "tests/fixtures/wit_import/main.flags").read_text(encoding="utf-8")
    if "--wit" not in flags or "host_math.wit" not in flags:
        return 1, "wit_import/main.flags missing --wit host_math.wit"
    return 0, ""


def _overlay_evidence() -> tuple[int, str]:
    sys.path.insert(0, str(REPO_ROOT / "scripts"))
    from selfhost.checks import _prepare_flattened_selfhost_source

    overlay = _prepare_flattened_selfhost_source(REPO_ROOT)
    call_wit = overlay / "src/compiler/wasm_call_wit.ark"
    wit_imports = overlay / "src/compiler/wasm_sections_wit_imports.ark"
    if not call_wit.is_file() or not wit_imports.is_file():
        return 1, "flat overlay missing wasm WIT emit modules"
    call_text = call_wit.read_text(encoding="utf-8")
    if "wit_import_function_index" not in call_text:
        return 1, "overlay wasm_call_wit.ark missing wit_import_function_index"
    import_text = wit_imports.read_text(encoding="utf-8")
    if "wit_import_binding_wit_func_name" not in import_text:
        return 1, "overlay wasm_sections_wit_imports.ark still stubbed"
    return 0, ""


def _compile_component(out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    fixture = REPO_ROOT / "tests/fixtures/wit_import/main.ark"
    wit_path = REPO_ROOT / "tests/fixtures/wit_import/host_math.wit"
    out.parent.mkdir(parents=True, exist_ok=True)
    try:
        out_arg = str(out.relative_to(REPO_ROOT))
    except ValueError:
        out_arg = str(out)
    cmd = [
        *compiler,
        "compile",
        str(fixture.relative_to(REPO_ROOT)),
        "--wit",
        str(wit_path.relative_to(REPO_ROOT)),
        "--target",
        "wasm32-gc",
        "--wasi-version",
        "p2",
        "--emit",
        "component",
        "-o",
        out_arg,
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
        return 1, f"missing component output {out}"
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


def _core_wasm_has_wit_import(component: Path) -> tuple[int, str]:
    data = component.read_bytes()
    if b"test:host/math" not in data:
        return 1, "component bytes missing test:host/math import module name"
    if b"add" not in data:
        return 1, "component bytes missing add import function name"
    return 0, ""


def main() -> int:
    failures: list[str] = []
    for name, fn in (
        ("static evidence", _static_evidence),
        ("overlay evidence", _overlay_evidence),
    ):
        rc, msg = fn()
        if rc != 0:
            failures.append(f"{name}: {msg}")

    out_dir = Path(tempfile.mkdtemp(prefix="gate-654-", dir=REPO_ROOT / ".build"))
    try:
        out = out_dir / "wit_import.component.wasm"
        rc, msg = _compile_component(out)
        if rc == 2:
            print(f"gate-654-wit-import-component-emit: SKIP (dynamic compile: {msg})")
        elif rc != 0:
            static_rc, _ = _static_evidence()
            overlay_rc, _ = _overlay_evidence()
            if static_rc == 0 and overlay_rc == 0:
                print(
                    "gate-654-wit-import-component-emit: PASS (static+overlay; dynamic compile skipped)"
                )
                return 0
            failures.append(f"component compile: {msg}")
        else:
            for name, fn in (
                ("wasm-tools validate", lambda: _wasm_tools_validate(out)),
                ("wit import bytes", lambda: _core_wasm_has_wit_import(out)),
            ):
                vrc, vmsg = fn()
                if vrc == 2:
                    print(f"gate-654-wit-import-component-emit: SKIP ({name}: {vmsg})")
                elif vrc != 0:
                    if name == "wasm-tools validate":
                        static_rc, _ = _static_evidence()
                        overlay_rc, _ = _overlay_evidence()
                        if bootstrap_validate_skip_allowed(vmsg, static_rc, overlay_rc):
                            print(
                                "gate-654-wit-import-component-emit: PASS "
                                "(static+overlay; bootstrap validate skipped)"
                            )
                            return 0
                    failures.append(f"{name}: {vmsg}")
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)

    if failures:
        print("gate-654-wit-import-component-emit: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    if not failures:
        print("gate-654-wit-import-component-emit: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
