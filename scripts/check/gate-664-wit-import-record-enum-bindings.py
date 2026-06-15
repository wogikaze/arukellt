#!/usr/bin/env python3
"""Close gate for issue #664 — general WIT record/enum import bindings."""
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
        REPO_ROOT / "bootstrap/arukellt-selfhost.wasm",
    ):
        if candidate.is_file():
            env["ARUKELLT_SELFHOST_WASM"] = str(candidate)
            break
    return env


def _compiler() -> list[str] | None:
    wrapper = REPO_ROOT / "scripts/run/arukellt-selfhost.sh"
    return ["bash", str(wrapper)] if wrapper.is_file() else None


def _static_evidence() -> tuple[int, str]:
    required = [
        REPO_ROOT / "src/compiler/component/wit_parse_text.ark",
        REPO_ROOT / "src/compiler/resolver/wit_register.ark",
        REPO_ROOT / "src/compiler/typechecker/module_wit.ark",
        REPO_ROOT / "tests/fixtures/wit_import/geo_types.wit",
        REPO_ROOT / "tests/fixtures/wit_import/check/record_enum_general.ark",
    ]
    for path in required:
        if not path.is_file():
            return 1, f"missing {path.relative_to(REPO_ROOT)}"
    manifest = (REPO_ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    if "check-only:wit_import/check/record_enum_general.ark" not in manifest:
        return 1, "manifest missing record_enum_general fixture"
    parse_text = (REPO_ROOT / "src/compiler/component/wit_parse_text.ark").read_text(encoding="utf-8")
    if "WitParsedEnum" not in parse_text:
        return 1, "wit_parse_text.ark lacks enum parsing"
    module_wit = (REPO_ROOT / "src/compiler/typechecker/module_wit.ark").read_text(encoding="utf-8")
    if "wit_push_fn_sigs_for_doc" not in module_wit:
        return 1, "module_wit.ark still uses fixture-only sig tables"
    return 0, ""


def _check_fixture() -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "no compiler"
    fixture = REPO_ROOT / "tests/fixtures/wit_import/check/record_enum_general.ark"
    wit_path = REPO_ROOT / "tests/fixtures/wit_import/geo_types.wit"
    result = subprocess.run(
        [
            *compiler,
            "check",
            str(fixture.relative_to(REPO_ROOT)),
            "--wit",
            str(wit_path.relative_to(REPO_ROOT)),
            "--target",
            "wasm32-wasi-p2",
        ],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
        env=_compile_env(),
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def main() -> int:
    failures: list[str] = []
    for name, fn in (("static", _static_evidence), ("check", _check_fixture)):
        rc, msg = fn()
        if rc == 2:
            print(f"gate-664: SKIP {name}: {msg}")
            continue
        if rc != 0:
            failures.append(f"{name}: {msg}")
    if failures:
        print("gate-664-wit-import-record-enum-bindings: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-664-wit-import-record-enum-bindings: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
