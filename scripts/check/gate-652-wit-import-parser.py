#!/usr/bin/env python3
"""Close gate for issue #652 — WIT import parser grammar."""

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
        REPO_ROOT / "bootstrap/arukellt-selfhost.wasm",
    ):
        if candidate.is_file():
            env["ARUKELLT_SELFHOST_WASM"] = str(candidate)
            break
    return env


def _static_evidence() -> tuple[int, str]:
    required = [
        REPO_ROOT / "src/compiler/parser/imports_wit.ark",
        REPO_ROOT / "src/compiler/parser/imports_local.ark",
        REPO_ROOT / "src/compiler/parser/imports_wit_validate.ark",
        REPO_ROOT / "src/compiler/parser/self_check_issue652.ark",
        REPO_ROOT / "tests/fixtures/wit_import/main.ark",
        REPO_ROOT / "tests/fixtures/wit_import/parse/with_alias.ark",
        REPO_ROOT / "tests/fixtures/wit_import/parse/no_alias.ark",
        REPO_ROOT / "tests/fixtures/wit_import/parse/malformed.ark",
        REPO_ROOT / "tests/fixtures/wit_import/parse/local_ident.ark",
    ]
    for path in required:
        if not path.is_file():
            return 1, f"missing {path.relative_to(REPO_ROOT)}"
    kinds = (REPO_ROOT / "src/compiler/parser/kinds.ark").read_text(encoding="utf-8")
    if "IMPORT_KIND_WIT" not in kinds or "IMPORT_KIND_LOCAL" not in kinds:
        return 1, "parser/kinds.ark lacks IMPORT_KIND_WIT / IMPORT_KIND_LOCAL"
    manifest = (REPO_ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    for entry in (
        "parse-only:wit_import/parse/with_alias.ark",
        "parse-only:wit_import/parse/no_alias.ark",
        "diag:wit_import/parse/malformed.ark",
    ):
        if entry not in manifest:
            return 1, f"manifest missing {entry}"
    self_check = (REPO_ROOT / "src/compiler/parser/self_check_issue652.ark").read_text(
        encoding="utf-8"
    )
    if "IMPORT_KIND_WIT" not in self_check or "IMPORT_KIND_LOCAL" not in self_check:
        return 1, "self_check_issue652.ark lacks import kind assertions"
    return 0, ""


def _compiler() -> list[str] | None:
    wrapper = REPO_ROOT / "scripts/run/arukellt-selfhost.sh"
    if wrapper.is_file():
        return ["bash", str(wrapper)]
    return None


def _parse_fixture(fixture_rel: str) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    fixture = REPO_ROOT / "tests/fixtures" / fixture_rel
    if not fixture.is_file():
        return 1, f"missing fixture {fixture_rel}"
    result = subprocess.run(
        [*compiler, "parse", str(fixture.relative_to(REPO_ROOT))],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
        env=_compile_env(),
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-400:]
    return 0, ""


def _parse_malformed_fixture() -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    fixture = REPO_ROOT / "tests/fixtures/wit_import/parse/malformed.ark"
    result = subprocess.run(
        [*compiler, "parse", str(fixture.relative_to(REPO_ROOT))],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
        env=_compile_env(),
    )
    if result.returncode == 0:
        return 1, "malformed WIT import parse succeeded unexpectedly"
    out = result.stdout + result.stderr
    if "E0001" not in out and "namespace:package/interface" not in out:
        return 1, f"malformed WIT import missing parse diagnostic: {out[-200:]}"
    return 0, ""


def main() -> int:
    failures: list[str] = []
    for name, fn in (
        ("static evidence", _static_evidence),
        ("parse with_alias", lambda: _parse_fixture("wit_import/parse/with_alias.ark")),
        ("parse no_alias", lambda: _parse_fixture("wit_import/parse/no_alias.ark")),
        ("parse local_ident", lambda: _parse_fixture("wit_import/parse/local_ident.ark")),
        ("parse malformed", _parse_malformed_fixture),
    ):
        rc, msg = fn()
        if rc == 2:
            print(f"gate-652-wit-import-parser: SKIP ({name}: {msg})")
            continue
        if rc != 0:
            failures.append(f"{name}: {msg}")
    if failures:
        print("gate-652-wit-import-parser: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-652-wit-import-parser: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
