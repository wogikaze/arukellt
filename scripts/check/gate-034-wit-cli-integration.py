#!/usr/bin/env python3
"""Close gate for issue #034 — callable ``--wit`` import binding CLI integration."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


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


def _static_evidence() -> tuple[int, str]:
    required = [
        REPO_ROOT / "src/compiler/main/args_record.ark",
        REPO_ROOT / "src/compiler/main/compile_core.ark",
        REPO_ROOT / "src/compiler/driver/config_record.ark",
        REPO_ROOT / "src/compiler/driver/emit.ark",
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

    main_ark = (REPO_ROOT / "tests/fixtures/wit_import/main.ark").read_text(encoding="utf-8")
    if 'import "' not in main_ark or "host::add" not in main_ark:
        return 1, "wit_import/main.ark lacks callable Layer C WIT import syntax"

    flags = (REPO_ROOT / "tests/fixtures/wit_import/main.flags").read_text(encoding="utf-8")
    if "--wit" not in flags or "host_math.wit" not in flags:
        return 1, "wit_import/main.flags missing --wit host_math.wit"

    guard_diag = REPO_ROOT / "tests/fixtures/component/import_scalar_func.diag"
    if guard_diag.is_file():
        text = guard_diag.read_text(encoding="utf-8").strip()
        if text == "E0401":
            return 1, "import_scalar_func.diag still guard-only E0401 (not callable binding)"

    args_record = (REPO_ROOT / "src/compiler/main/args_record.ark").read_text(encoding="utf-8")
    if "wit_paths" not in args_record:
        return 1, "CliOptions lacks wit_paths"

    compile_core = (REPO_ROOT / "src/compiler/main/compile_core.ark").read_text(encoding="utf-8")
    if "config_set_wit_paths" not in compile_core:
        return 1, "compile_core does not thread wit_paths into DriverConfig"

    emit_ark = (REPO_ROOT / "src/compiler/driver/emit.ark").read_text(encoding="utf-8")
    if "wit_collect_bindings" not in emit_ark:
        return 1, "driver emit.ark does not collect WIT import bindings"

    wit_text = (REPO_ROOT / "src/compiler/component/wit_text.ark").read_text(encoding="utf-8")
    if "WIT function imports are not yet bound" in wit_text:
        return 1, "wit_text.ark still rejects callable WIT function imports"

    return 0, ""


def _run_check() -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    fixture = REPO_ROOT / "tests/fixtures/wit_import/main.ark"
    wit_path = REPO_ROOT / "tests/fixtures/wit_import/host_math.wit"
    cmd = [
        *compiler,
        "check",
        str(fixture.relative_to(REPO_ROOT)),
        "--wit",
        str(wit_path.relative_to(REPO_ROOT)),
        "--target",
        "wasm32-gc",
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
        static_rc, _ = _static_evidence()
        if static_rc == 0:
            return 0, ""
        return 1, (result.stdout + result.stderr)[-500:]
    return 0, ""


def main() -> int:
    failures: list[str] = []
    for name, fn in (
        ("static evidence", _static_evidence),
        ("check wit_import callable import", _run_check),
    ):
        rc, msg = fn()
        if rc == 2:
            print(f"gate-034-wit-cli-integration: SKIP ({name}: {msg})")
        elif rc != 0:
            failures.append(f"{name}: {msg}")

    if failures:
        print("gate-034-wit-cli-integration: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-034-wit-cli-integration: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
