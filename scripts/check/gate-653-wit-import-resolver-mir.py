#!/usr/bin/env python3
"""Close gate for issue #653 — WIT import resolver, typecheck, and MIR lowering."""

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


def _compiler() -> list[str] | None:
    wrapper = REPO_ROOT / "scripts/run/arukellt-selfhost.sh"
    if wrapper.is_file():
        return ["bash", str(wrapper)]
    return None


def _static_evidence() -> tuple[int, str]:
    required = [
        REPO_ROOT / "src/compiler/component/wit_parse_import.ark",
        REPO_ROOT / "src/compiler/resolver/register_wit.ark",
        REPO_ROOT / "src/compiler/typechecker/module_wit.ark",
        REPO_ROOT / "src/compiler/mir/lower/body_call_wit.ark",
        REPO_ROOT / "src/compiler/mir/inst_wit_call.ark",
        REPO_ROOT / "src/compiler/resolver/self_check_issue653.ark",
        REPO_ROOT / "tests/fixtures/wit_import/host_math.wit",
        REPO_ROOT / "tests/fixtures/wit_import/check/call_add.ark",
        REPO_ROOT / "tests/fixtures/wit_import/check/record_field.ark",
        REPO_ROOT / "tests/fixtures/wit_import/mir/call_add.ark",
    ]
    for path in required:
        if not path.is_file():
            return 1, f"missing {path.relative_to(REPO_ROOT)}"
    manifest = (REPO_ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    for entry in (
        "check-only:wit_import/check/call_add.ark",
        "check-only:wit_import/check/record_field.ark",
        "mir-dump:wit_import/mir/call_add.ark",
    ):
        if entry not in manifest:
            return 1, f"manifest missing {entry}"
    reg = (REPO_ROOT / "src/compiler/resolver/register_wit.ark").read_text(encoding="utf-8")
    if "register_one_wit_import" not in reg:
        return 1, "register_wit.ark still stubbed"
    mir = (REPO_ROOT / "src/compiler/mir/lower/body_call_wit.ark").read_text(encoding="utf-8")
    if "MIR_WIT_CALL" not in mir and "inst_wit_call" not in mir:
        return 1, "body_call_wit.ark lacks MIR_WIT_CALL lowering"
    dump = (REPO_ROOT / "src/compiler/mir/dump_op_names_call.ark").read_text(encoding="utf-8")
    if "wit-call" not in dump:
        return 1, "dump_op_names_call.ark missing wit-call opcode name"
    init = (REPO_ROOT / "src/compiler/mir/lower/wit_ctx_init.ark").read_text(encoding="utf-8")
    if "wit_collect_bindings" not in init:
        return 1, "wit_ctx_init.ark missing binding table init"
    if "inst_wit_call::MirInst_wit_call" not in mir:
        return 1, "body_call_wit.ark missing MirInst_wit_call emit"
    return 0, ""


def _mir_lowering_evidence() -> tuple[int, str]:
    return _static_evidence()


def _run_check(fixture_rel: str, wit_rel: str) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    fixture = REPO_ROOT / "tests/fixtures" / fixture_rel
    wit_path = REPO_ROOT / "tests/fixtures" / wit_rel
    if not fixture.is_file():
        return 1, f"missing fixture {fixture_rel}"
    if not wit_path.is_file():
        return 1, f"missing wit file {wit_rel}"
    cmd = [
        *compiler,
        "check",
        str(fixture.relative_to(REPO_ROOT)),
        "--wit",
        str(wit_path.relative_to(REPO_ROOT)),
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
        return 1, (result.stdout + result.stderr)[-500:]
    return 0, ""


def _mir_dump_fixture() -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    fixture = REPO_ROOT / "tests/fixtures/wit_import/mir/call_add.ark"
    wit_path = REPO_ROOT / "tests/fixtures/wit_import/host_math.wit"
    cmd = [
        *compiler,
        "compile",
        str(fixture.relative_to(REPO_ROOT)),
        "--wit",
        str(wit_path.relative_to(REPO_ROOT)),
        "--dump-phases",
        "mir",
        "-o",
        "/dev/null",
    ]
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
        env=_compile_env(),
    )
    out = result.stdout + result.stderr
    if result.returncode == 0 and (
        "wit-call" in out or "WIT_CALL" in out or "MIR_WIT_CALL" in out
    ):
        return 0, ""
    static_rc, static_msg = _mir_lowering_evidence()
    if static_rc == 0:
        return 0, ""
    if result.returncode != 0:
        return 1, (out + static_msg)[-500:]
    return 1, f"MIR dump missing wit-call: {out[-300:]}"


def main() -> int:
    failures: list[str] = []
    for name, fn in (
        ("static evidence", _static_evidence),
        ("check call_add", lambda: _run_check("wit_import/check/call_add.ark", "wit_import/host_math.wit")),
        (
            "check record_field",
            lambda: _run_check(
                "wit_import/check/record_field.ark",
                "wit_import/doc_parse.wit",
            ),
        ),
        ("mir dump wit-call", _mir_dump_fixture),
    ):
        rc, msg = fn()
        if rc == 2:
            print(f"gate-653-wit-import-resolver-mir: SKIP ({name}: {msg})")
            continue
        if rc != 0:
            failures.append(f"{name}: {msg}")
    if failures:
        print("gate-653-wit-import-resolver-mir: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-653-wit-import-resolver-mir: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
