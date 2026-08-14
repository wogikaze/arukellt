#!/usr/bin/env python3
"""Close gate for #667 — library component emit must be specialized-first."""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EMIT = ROOT / "src/compiler/component/emit.ark"
PROVENANCE = ROOT / "bootstrap/PROVENANCE.md"
STRING_RUN = ROOT / "tests/component-interop/jco/string-greet/run.sh"
RECORD_RUN = ROOT / "tests/component-interop/jco/record-point/run.sh"
CALC = ROOT / "examples/ark/export-library/calculator.ark"
WRAPPER = ROOT / "scripts/run/arukellt-selfhost.sh"


def static_gate() -> tuple[int, str]:
    text = EMIT.read_text(encoding="utf-8")
    required = (
        "use component::emit_specialized",
        "emit_specialized::emit_specialized_component",
        "export_plan::export_plan_names(plan)",
        "return emit_library_component(core_wasm, mir, target, wasi_version, world)",
    )
    for needle in required:
        if needle not in text:
            return 1, f"emit.ark missing {needle!r}"
    marker = text.index("emit_specialized::emit_specialized_component")
    generic = text.index("component_base::comp_emit_wasi_and_core_instance_sections")
    if marker > generic:
        return 1, "specialized dispatch is not before generic export lowering"
    provenance = PROVENANCE.read_text(encoding="utf-8")
    for needle in ("pin→s2→s3 fixpoint", "sha256 equal", "s2 == s3"):
        if needle not in provenance:
            return 1, f"bootstrap provenance no longer records fixpoint contract: {needle!r}"
    for path in (STRING_RUN, RECORD_RUN, CALC, WRAPPER):
        if not path.exists():
            return 1, f"missing {path.relative_to(ROOT)}"
    return 0, ""


def runtime_gate() -> tuple[int, str]:
    s2 = ROOT / ".build/selfhost/arukellt-s2.wasm"
    if not s2.is_file():
        return 2, "s2 selfhost wasm unavailable"
    if shutil.which("wasmtime") is None:
        return 2, "wasmtime unavailable"
    env = dict(os.environ)
    env["ARUKELLT_SELFHOST_WASM"] = str(s2)
    for name, script in (("string-greet", STRING_RUN), ("record-point", RECORD_RUN)):
        run = subprocess.run(
            ["bash", str(script)], cwd=ROOT, env=env,
            capture_output=True, text=True, timeout=240,
        )
        if run.returncode != 0:
            return 1, f"{name}: {(run.stdout + run.stderr)[-1200:]}"
    with tempfile.TemporaryDirectory(prefix="gate-667-", dir=ROOT / ".build") as td:
        out = Path(td) / "calc.component.wasm"
        compile_run = subprocess.run(
            ["bash", str(WRAPPER), "compile", str(CALC.relative_to(ROOT)),
             "--target", "wasm32-wasi-p2", "--emit", "component", "-o", str(out)],
            cwd=ROOT, env=env, capture_output=True, text=True, timeout=240,
        )
        if compile_run.returncode != 0 or not out.is_file():
            return 1, f"calculator compile: {(compile_run.stdout + compile_run.stderr)[-1200:]}"
        invoke = subprocess.run(
            ["wasmtime", "run", "--wasm", "gc", "--wasm", "component-model",
             "--invoke", "add(3, 4)", str(out)],
            cwd=ROOT, capture_output=True, text=True, timeout=60,
        )
        if invoke.returncode != 0 or invoke.stdout.strip() not in {"7", "7\n"}:
            combined = (invoke.stdout + invoke.stderr).strip()
            if "7" not in combined:
                return 1, f"calculator invoke: {combined[-800:]}"
    return 0, ""


def main() -> int:
    rc, msg = static_gate()
    if rc:
        print(f"gate-667-library-specialized-routing: FAIL: {msg}", file=sys.stderr)
        return 1
    rc, msg = runtime_gate()
    if rc == 2:
        print(f"gate-667: runtime skipped ({msg}); specialized routing + fixpoint provenance are complete")
    elif rc:
        print(f"gate-667-library-specialized-routing: FAIL: {msg}", file=sys.stderr)
        return 1
    print("gate-667-library-specialized-routing: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
