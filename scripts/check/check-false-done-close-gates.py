#!/usr/bin/env python3
"""Close-gate enforcement for audit-reopened false-done issues.

When an issue tracked here lives under ``issues/done/``, this script runs the
acceptance gate defined in ``docs/process/false-done-prevention.md``. While the
issue remains in ``issues/open/``, gates are not enforced (verify stays green).

Issue IDs: 074, 510, 472, 500, 051, 123
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
OPEN_DIR = REPO_ROOT / "issues" / "open"
DONE_DIR = REPO_ROOT / "issues" / "done"
MANIFEST = REPO_ROOT / "tests" / "fixtures" / "manifest.txt"
PLAYGROUND = REPO_ROOT / "playground"

ISSUE_ID_RE = re.compile(r"^(\d{3})")

# issue_id -> list of human-readable gate names (for error messages)
TRACKED: dict[str, list[str]] = {
    "074": ["P2 component validate + wasmtime run (wasi_p2_native/hello.ark)"],
    "510": ["P2 component wasm-tools validate"],
    "472": ["playground typecheck distinguishes parse vs type errors"],
    "500": ["playground wasm typecheck export gate"],
    "051": ["stdlib_time monotonic run + clock intrinsic emitter handlers"],
    "123": ["Layer C import string syntax component fixture"],
}


def _issue_location(issue_id: str) -> Path | None:
    for directory in (DONE_DIR, OPEN_DIR):
        for path in directory.glob(f"{issue_id}-*.md"):
            return path
    return None


def _issue_in_done(issue_id: str) -> bool:
    path = _issue_location(issue_id)
    if path is None:
        return False
    return path.parent == DONE_DIR


def _find_tool(name: str) -> str | None:
    return shutil.which(name)


def _compiler() -> Path | None:
    release = REPO_ROOT / "target" / "release" / "arukellt"
    if release.is_file():
        return release
    debug = REPO_ROOT / "target" / "debug" / "arukellt"
    if debug.is_file():
        return debug
    return None


def _manifest_contains(entry: str) -> bool:
    if not MANIFEST.is_file():
        return False
    return entry in MANIFEST.read_text(encoding="utf-8")


def _compile_p2_component(fixture_rel: str, out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "arukellt compiler binary not found (build release/debug first)"
    fixture = REPO_ROOT / fixture_rel
    if not fixture.is_file():
        return 1, f"missing fixture {fixture_rel}"
    cmd = [
        str(compiler),
        "compile",
        str(fixture),
        "--target",
        "wasm32-wasi-p2",
        "--wasi-version",
        "p2",
        "--emit",
        "component",
        "-o",
        str(out),
    ]
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        tail = (result.stderr or result.stdout)[-800:]
        return 1, f"compile failed: {tail}"
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


def _wasmtime_run(component: Path, expect_stdout: str) -> tuple[int, str]:
    wasmtime = _find_tool("wasmtime")
    if not wasmtime:
        return 2, "wasmtime not in PATH"
    result = subprocess.run(
        [wasmtime, "run", str(component)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, f"wasmtime exit {result.returncode}: {(result.stderr or '')[-400:]}"
    if expect_stdout not in result.stdout:
        return 1, f"expected stdout containing {expect_stdout!r}, got {result.stdout!r}"
    return 0, ""


def gate_074() -> tuple[int, str]:
    entry = "component-compile:wasi_p2_native/hello.ark"
    if not _manifest_contains(entry):
        return 1, f"manifest missing {entry}"
    with tempfile.TemporaryDirectory(prefix="close-gate-074-") as tmp:
        out = Path(tmp) / "hello.component.wasm"
        rc, msg = _compile_p2_component("tests/fixtures/wasi_p2_native/hello.ark", out)
        if rc != 0:
            return rc, msg
        rc, msg = _wasm_tools_validate(out)
        if rc != 0:
            return rc, msg
        return _wasmtime_run(out, "hello p2")


def gate_510() -> tuple[int, str]:
    with tempfile.TemporaryDirectory(prefix="close-gate-510-") as tmp:
        out = Path(tmp) / "p2.component.wasm"
        rc, msg = _compile_p2_component("tests/fixtures/wasi_p2_native/hello.ark", out)
        if rc != 0:
            return rc, msg
        return _wasm_tools_validate(out)


def gate_472() -> tuple[int, str]:
    if not (PLAYGROUND / "package.json").is_file():
        return 1, "playground package missing"
    build = subprocess.run(
        ["npm", "run", "build"],
        cwd=str(PLAYGROUND),
        capture_output=True,
        text=True,
        timeout=120,
    )
    if build.returncode != 0:
        return 1, f"playground build failed: {(build.stderr or '')[-400:]}"
    test = subprocess.run(
        ["node", "--test", "dist/tests/typecheck-close-gate.test.js"],
        cwd=str(PLAYGROUND),
        capture_output=True,
        text=True,
        timeout=60,
    )
    if test.returncode != 0:
        return 1, (test.stderr or test.stdout)[-800:]
    return 0, ""


def gate_500() -> tuple[int, str]:
    return gate_472()


def gate_051() -> tuple[int, str]:
    if not _manifest_contains("t3-run:stdlib_time/monotonic.ark"):
        return 1, "manifest missing t3-run:stdlib_time/monotonic.ark"
    compiler = _compiler()
    if compiler is None:
        return 2, "arukellt compiler binary not found"
    fixture = REPO_ROOT / "tests" / "fixtures" / "stdlib_time" / "monotonic.ark"
    result = subprocess.run(
        [
            str(compiler),
            "compile",
            str(fixture),
            "--target",
            "wasm32-wasi-p1",
            "-o",
            "/dev/null",
        ],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, f"stdlib_time/monotonic.ark compile failed: {(result.stderr or '')[-400:]}"
    emitter_glob = list((REPO_ROOT / "src" / "compiler").rglob("*.ark"))
    clock_handlers = 0
    for path in emitter_glob:
        text = path.read_text(encoding="utf-8", errors="replace")
        if "__intrinsic_clock_now" in text and "handler" in text.lower():
            clock_handlers += 1
    if clock_handlers == 0:
        return 1, "no selfhost emitter handler for __intrinsic_clock_now"
    return 0, ""


def gate_123() -> tuple[int, str]:
    entry = "component-compile:wit_import/main.ark"
    if not _manifest_contains(entry):
        return 1, f"manifest missing {entry} (Layer C / WIT import close gate)"
    fixture = REPO_ROOT / "tests" / "fixtures" / "wit_import" / "main.ark"
    if not fixture.is_file():
        return 1, "missing tests/fixtures/wit_import/main.ark"
    text = fixture.read_text(encoding="utf-8")
    if 'import "' not in text:
        return 1, "wit_import fixture lacks Layer C string import syntax"
    return 0, ""


GATES: dict[str, callable[[], tuple[int, str]]] = {
    "074": gate_074,
    "510": gate_510,
    "472": gate_472,
    "500": gate_500,
    "051": gate_051,
    "123": gate_123,
}


def main() -> int:
    failures: list[str] = []
    skipped = 0
    enforced = 0

    for issue_id, gate_names in sorted(TRACKED.items()):
        if not _issue_in_done(issue_id):
            continue
        enforced += 1
        gate_fn = GATES[issue_id]
        rc, msg = gate_fn()
        if rc == 2:
            skipped += 1
            failures.append(
                f"#{issue_id} ({', '.join(gate_names)}): SKIP prereq — {msg}"
            )
            continue
        if rc != 0:
            failures.append(f"#{issue_id} ({', '.join(gate_names)}): {msg}")
            continue
        print(f"  pass close-gate #{issue_id}")

    if enforced == 0:
        print("false-done-close-gates: PASS (no tracked issues in issues/done/)")
        return 0

    if failures:
        print("false-done-close-gates: FAIL", file=sys.stderr)
        for err in failures:
            print(f"  {err}", file=sys.stderr)
        return 1

    print(f"false-done-close-gates: PASS ({enforced} enforced)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
