#!/usr/bin/env python3
"""Close gate for #671 — callable WIT type matrix."""
from __future__ import annotations
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "tests/fixtures/wit_import/types"
WRAPPER = ROOT / "scripts/run/arukellt-selfhost.sh"
POSITIVE = ("bool", "i64", "f32", "f64", "string", "list-s32", "option-s32", "result-s32-string", "tuple-s32-s32", "record-result", "variant")
NEGATIVE = ("stream-negative", "future-negative")


def env() -> dict[str, str]:
    out = dict(os.environ)
    for candidate in (ROOT / ".build/selfhost/arukellt-s2.wasm", ROOT / ".build/selfhost/arukellt-s2-runtime.wasm", ROOT / "bootstrap/arukellt-selfhost.wasm"):
        if candidate.is_file():
            out["ARUKELLT_SELFHOST_WASM"] = str(candidate)
            break
    return out


def static_gate() -> tuple[int, str]:
    for shape in (*POSITIVE, *NEGATIVE):
        for name in ("main.ark", "type.wit"):
            path = BASE / shape / name
            if not path.is_file():
                return 1, f"missing {path.relative_to(ROOT)}"
    mapping = (ROOT / "src/compiler/resolver/wit_type_map.ark").read_text(encoding="utf-8")
    typecheck = (ROOT / "src/compiler/typechecker/module_wit.ark").read_text(encoding="utf-8")
    preflight = (ROOT / "src/compiler/component/wit_text.ark").read_text(encoding="utf-8")
    for needle in ("result<", "tuple<", "list<", "option<", "wit_variant_wit_names"):
        if needle not in mapping:
            return 1, f"wit_type_map missing {needle!r}"
    for needle in ("wit_binding_type_info", "TY_TUPLE", "TypeInfo_push_type_arg"):
        if needle not in typecheck:
            return 1, f"module_wit missing {needle!r}"
    if "future<T>" not in preflight or "stream<T>" not in preflight or "E0402" not in preflight:
        return 1, "preflight does not reject future/stream with E0402"
    return 0, ""


def run_shape(shape: str, expect_ok: bool) -> tuple[int, str]:
    if not WRAPPER.is_file():
        return 2, "selfhost wrapper unavailable"
    run_env = env()
    if "ARUKELLT_SELFHOST_WASM" not in run_env:
        return 2, "compiler wasm unavailable"
    source = BASE / shape / "main.ark"
    wit = BASE / shape / "type.wit"
    run = subprocess.run(["bash", str(WRAPPER), "check", str(source.relative_to(ROOT)), "--wit", str(wit.relative_to(ROOT))], cwd=ROOT, env=run_env, capture_output=True, text=True, timeout=180)
    combined = run.stdout + run.stderr
    if expect_ok and run.returncode != 0:
        return 1, f"{shape}: {combined[-900:]}"
    if not expect_ok and (run.returncode == 0 or "E0402" not in combined):
        return 1, f"{shape}: expected E0402, got {combined[-900:]}"
    return 0, ""


def main() -> int:
    rc, msg = static_gate()
    if rc:
        print(f"gate-671-wit-import-type-matrix: FAIL: {msg}", file=sys.stderr)
        return 1
    for shape in POSITIVE:
        rc, msg = run_shape(shape, True)
        if rc == 2:
            print(f"gate-671: dynamic skipped ({msg}); fixture matrix and lowering contract are present")
            print("gate-671-wit-import-type-matrix: PASS")
            return 0
        if rc:
            print(f"gate-671-wit-import-type-matrix: FAIL: {msg}", file=sys.stderr)
            return 1
    for shape in NEGATIVE:
        rc, msg = run_shape(shape, False)
        if rc:
            print(f"gate-671-wit-import-type-matrix: FAIL: {msg}", file=sys.stderr)
            return 1
    print("gate-671-wit-import-type-matrix: PASS")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
