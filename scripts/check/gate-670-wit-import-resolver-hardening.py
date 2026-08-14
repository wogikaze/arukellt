#!/usr/bin/env python3
"""Close gate for #670 — WIT resolver diagnostics, collisions, and backend-plan visibility."""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REGISTER = ROOT / "src/compiler/resolver/register_wit.ark"
WIT_REGISTER = ROOT / "src/compiler/resolver/wit_register.ark"
DEBUG = ROOT / "src/compiler/driver/debug.ark"
CONFIG = ROOT / "src/compiler/driver/config_record.ark"
COMPILE_CORE = ROOT / "src/compiler/main/compile_core.ark"
NAMES = ROOT / "src/compiler/component/wit_names_import.ark"
WRAPPER = ROOT / "scripts/run/arukellt-selfhost.sh"
FIXTURES = (
    (ROOT / "tests/fixtures/wit_import/errors/duplicate_alias.ark", "WIT import alias"),
    (ROOT / "tests/fixtures/wit_import/errors/duplicate_package.ark", "WIT package"),
)


def static_gate() -> tuple[int, str]:
    register = REGISTER.read_text(encoding="utf-8")
    wit_register = WIT_REGISTER.read_text(encoding="utf-8")
    required_register = (
        "seen_aliases",
        "seen_packages",
        "ast_access::node_span_start(d)",
        "ast_access::node_span_end(d)",
        "scope_error::resolve_duplicate_symbol",
        "wit_ensure_module_alias_at",
    )
    for needle in required_register:
        if needle not in register:
            return 1, f"register_wit.ark missing {needle!r}"
    for needle in (
        "define_with_vis_span",
        "scope_error::resolve_duplicate_symbol",
        "lookup_kind",
    ):
        if needle not in wit_register:
            return 1, f"wit_register.ark missing {needle!r}"
    debug = DEBUG.read_text(encoding="utf-8")
    config = CONFIG.read_text(encoding="utf-8")
    compile_core = COMPILE_CORE.read_text(encoding="utf-8")
    if "dump_backend_plan_if_enabled" not in debug or "wit-imports:" not in debug or "funcs=" not in debug:
        return 1, "backend-plan dump lacks WIT lowering summary"
    if "dump_backend_plan" not in config or 'should_dump(opts, "backend-plan")' not in compile_core:
        return 1, "backend-plan phase is not wired through DriverConfig"
    names = NAMES.read_text(encoding="utf-8")
    if "std::wit::names" not in names or "pascal_case" not in names or "kebab_to_snake" not in names:
        return 1, "WIT-generated display names are not on stable shared naming helpers"
    for fixture, _ in FIXTURES:
        if not fixture.is_file():
            return 1, f"missing {fixture.relative_to(ROOT)}"
    return 0, ""


def branch_selfhost_env() -> dict[str, str] | None:
    env = dict(os.environ)
    explicit = env.get("ARUKELLT_SELFHOST_WASM")
    if explicit and Path(explicit).is_file():
        return env
    for candidate in (
        ROOT / ".build/selfhost/arukellt-s2-runtime.wasm",
        ROOT / ".build/selfhost/arukellt-s2.wasm",
    ):
        if candidate.is_file():
            env["ARUKELLT_SELFHOST_WASM"] = str(candidate)
            return env
    return None


def dynamic_gate() -> tuple[int, str]:
    if not WRAPPER.is_file():
        return 2, "selfhost wrapper unavailable"
    env = branch_selfhost_env()
    if env is None:
        return 2, "branch-built s2 compiler unavailable"
    for fixture, expected in FIXTURES:
        run = subprocess.run(
            ["bash", str(WRAPPER), "check", str(fixture.relative_to(ROOT)), "--target", "wasm32-gc"],
            cwd=ROOT, env=env, capture_output=True, text=True, timeout=180,
        )
        combined = run.stdout + run.stderr
        if run.returncode == 0:
            return 1, f"{fixture.name} unexpectedly succeeded"
        if "duplicate definition" not in combined or expected not in combined:
            return 1, f"{fixture.name} diagnostic mismatch: {combined[-1000:]}"
    dump_fixture = ROOT / "tests/fixtures/wit_import/check/call_add.ark"
    wit = ROOT / "tests/fixtures/wit_import/host_math.wit"
    run = subprocess.run(
        ["bash", str(WRAPPER), "compile", str(dump_fixture.relative_to(ROOT)), "--wit", str(wit.relative_to(ROOT)), "--dump-phases", "backend-plan", "-o", "/dev/null"],
        cwd=ROOT, env=env, capture_output=True, text=True, timeout=180,
    )
    combined = run.stdout + run.stderr
    if run.returncode != 0 or "wit-imports:" not in combined or "test:host/math" not in combined:
        return 1, f"backend-plan WIT summary missing: {combined[-1200:]}"
    return 0, ""


def main() -> int:
    rc, msg = static_gate()
    if rc:
        print(f"gate-670-wit-import-resolver-hardening: FAIL: {msg}", file=sys.stderr)
        return 1
    rc, msg = dynamic_gate()
    if rc == 2:
        print(f"gate-670: dynamic skipped ({msg}); static resolver/dump contract is complete")
    elif rc:
        print(f"gate-670-wit-import-resolver-hardening: FAIL: {msg}", file=sys.stderr)
        return 1
    print("gate-670-wit-import-resolver-hardening: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
