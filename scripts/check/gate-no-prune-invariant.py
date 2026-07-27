#!/usr/bin/env python3
"""No-prune invariant gate for selfhost compiler MIR reachability.

The bootstrap pinned wasm and the heap-grow patcher assume the compiler's
MIR-level reachability/prune phase does not discard functions that are
reachable from export roots (including through ``ref.func``/``MIR_REF_FUNC``
edges).  This gate compiles a small set of fixtures that contain both live
export-root chains and genuinely dead code, then asserts that:

* callees of exported roots survive pruning (``call_export_roots.ark``);
* ``ref.func``-only targets survive even when they are never directly called
  (``ref_func_only_target.ark``);
* truly dead functions are still pruned;
* the kept function set is deterministic across repeated compilations.

The check inspects the MIR dump emitted by ``--dump-phases mir`` so it does not
rely on the backend successfully emitting funcref tables.

Skips gracefully when ``wasmtime`` or the selfhost compiler wasm is missing.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import NamedTuple

REPO_ROOT = Path(__file__).resolve().parents[2]
BUILD_DIR = REPO_ROOT / ".build" / "no-prune-gate"
RECEIPT = BUILD_DIR / "no-prune-invariant-receipt.json"


class Fixture(NamedTuple):
    source: str
    output: str
    expected_kept: tuple[str, ...]
    expected_pruned: tuple[str, ...]
    validate: bool = True


FIXTURES = [
    Fixture(
        source="tests/fixtures/reachability/call_export_roots.ark",
        output=".build/no-prune-gate/call_export_roots.wasm",
        expected_kept=("exported_entry", "via_call", "main"),
        expected_pruned=("truly_dead",),
    ),
    Fixture(
        source="tests/fixtures/reachability/ref_func_only_target.ark",
        output=".build/no-prune-gate/ref_func_only_target.wasm",
        expected_kept=("exported_entry", "via_ref", "ref_only_target", "main"),
        expected_pruned=("truly_dead",),
    ),
    Fixture(
        source="tests/fixtures/hello/hello.ark",
        output=".build/no-prune-gate/hello.wasm",
        expected_kept=("main",),
        expected_pruned=(),
        validate=True,
    ),
]


def _find_compiler() -> Path | None:
    """Locate the current selfhost compiler artifact.

    Prefer the runtime (heap-grow patched) stage-2 wasm, then the plain s2,
    then bootstrap fallbacks.  ``None`` means there is nothing to test with.
    """
    env = os.environ.get("ARUKELLT_SELFHOST_WASM", "").strip()
    if env:
        path = Path(env)
        if not path.is_absolute():
            path = REPO_ROOT / path
        if path.is_file():
            return path
    for rel in (
        ".build/selfhost/arukellt-s2-runtime.wasm",
        ".build/selfhost/arukellt-s2.wasm",
        ".build/selfhost/arukellt-pinned-bootstrap.wasm",
        "bootstrap/arukellt-selfhost.wasm",
    ):
        path = REPO_ROOT / rel
        if path.is_file():
            return path
    return None


def _compiler_command() -> list[str] | None:
    wrapper = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    if wrapper.is_file():
        return ["bash", str(wrapper)]
    return None


def _mir_function_names(text: str) -> list[str]:
    """Return the names of functions present in an MIR dump."""
    return re.findall(r"^\s*fn\s+(\S+)\s", text, re.MULTILINE)


def _mir_module_count(text: str) -> int | None:
    m = re.search(r"^MIR module:\s*(\d+)\s+function\(s\)", text, re.MULTILINE)
    return int(m.group(1)) if m else None


def _compile(
    compiler: list[str],
    fixture: Fixture,
    extra: list[str] | None = None,
) -> subprocess.CompletedProcess[str]:
    BUILD_DIR.mkdir(parents=True, exist_ok=True)
    out_path = REPO_ROOT / fixture.output
    out_path.parent.mkdir(parents=True, exist_ok=True)
    if out_path.is_file():
        out_path.unlink()
    cmd = [
        *compiler,
        "compile",
        fixture.source,
        "-o",
        fixture.output,
        "--target",
        "wasm32-gc",
        "--wasi-version",
        "wasi-p2",
        "--dump-phases",
        "mir",
        *(extra or []),
    ]
    return subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )


def _validate_wasm(path: Path) -> tuple[bool, str]:
    wasm_tools = shutil.which("wasm-tools")
    if wasm_tools is None:
        return True, "wasm-tools not found; skipping wasm validate"
    result = subprocess.run(
        [wasm_tools, "validate", "--features", "gc", str(path)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode == 0:
        return True, ""
    return False, (result.stdout + result.stderr)[-500:]


def _check_fixture(
    compiler: list[str],
    fixture: Fixture,
) -> tuple[bool, dict]:
    record: dict = {
        "fixture": fixture.source,
        "output": fixture.output,
    }
    result = _compile(compiler, fixture)
    text = (result.stderr or "") + "\n" + (result.stdout or "")
    record["stderr_head"] = text[:500]

    if result.returncode != 0:
        record["error"] = "compile failed"
        record["details"] = text[-800:]
        return False, record

    if "compilation succeeded" not in text:
        record["error"] = "missing 'compilation succeeded' marker"
        record["details"] = text[-800:]
        return False, record

    names = _mir_function_names(text)
    record["mir_functions"] = names
    module_count = _mir_module_count(text)
    if module_count is not None and module_count != len(names):
        record["error"] = f"MIR module count mismatch: header={module_count}, found={len(names)}"
        return False, record

    missing_kept = [n for n in fixture.expected_kept if n not in names]
    leaked_pruned = [n for n in fixture.expected_pruned if n in names]
    if missing_kept:
        record["error"] = f"expected functions missing: {missing_kept}"
        return False, record
    if leaked_pruned:
        record["error"] = f"expected pruned functions present: {leaked_pruned}"
        return False, record

    if fixture.validate:
        ok, msg = _validate_wasm(REPO_ROOT / fixture.output)
        record["wasm_validate_ok"] = ok
        if not ok:
            record["error"] = f"wasm-tools validate failed: {msg}"
            return False, record

    record["ok"] = True
    return True, record


def _check_determinism(compiler: list[str]) -> tuple[bool, dict]:
    """Compile ``call_export_roots`` twice and compare the kept MIR function set."""
    fixture = FIXTURES[0]
    names_a: list[str] | None = None
    names_b: list[str] | None = None
    for run in (1, 2):
        result = _compile(compiler, fixture)
        text = (result.stderr or "") + "\n" + (result.stdout or "")
        if result.returncode != 0 or "compilation succeeded" not in text:
            return False, {"error": f"determinism run {run} compile failed", "details": text[-800:]}
        names = _mir_function_names(text)
        if run == 1:
            names_a = names
        else:
            names_b = names

    assert names_a is not None and names_b is not None
    if sorted(names_a) != sorted(names_b):
        return False, {
            "error": "pruned function set is not deterministic",
            "run_1": names_a,
            "run_2": names_b,
        }
    return True, {"run_1": names_a, "run_2": names_b, "ok": True}


def main() -> int:
    wasmtime = shutil.which("wasmtime")
    if wasmtime is None:
        print("gate-no-prune-invariant: SKIP (wasmtime not found)")
        return 0

    compiler = _compiler_command()
    if compiler is None:
        print("gate-no-prune-invariant: SKIP (arukellt-selfhost.sh not found)")
        return 0

    compiler_wasm = _find_compiler()
    if compiler_wasm is None:
        print("gate-no-prune-invariant: SKIP (no selfhost compiler wasm found)")
        return 0

    results: list[dict] = []
    failures: list[str] = []
    for fixture in FIXTURES:
        ok, record = _check_fixture(compiler, fixture)
        results.append(record)
        if not ok:
            failures.append(f"{fixture.source}: {record.get('error')}")

    det_ok, det_record = _check_determinism(compiler)
    results.append({"determinism": det_record})
    if not det_ok:
        failures.append(f"determinism: {det_record.get('error')}")

    BUILD_DIR.mkdir(parents=True, exist_ok=True)
    receipt = {
        "schema_version": 1,
        "compiler_wasm": str(compiler_wasm.relative_to(REPO_ROOT)),
        "results": results,
    }
    RECEIPT.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")

    if failures:
        print("gate-no-prune-invariant: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1

    print("gate-no-prune-invariant: PASS")
    print(f"  receipt: {RECEIPT.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
