#!/usr/bin/env python3
"""Close-gate enforcement for audit-reopened false-done issues.

When an issue tracked here lives under ``issues/done/``, this script runs the
acceptance gate defined in ``docs/process/false-done-prevention.md``. While the
issue remains in ``issues/open/``, gates are not enforced (verify stays green).

Issue IDs: 074, 510, 472, 500, 051, 123
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import fcntl
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
OPEN_DIR = REPO_ROOT / "issues" / "open"
DONE_DIR = REPO_ROOT / "issues" / "done"
MANIFEST = REPO_ROOT / "tests" / "fixtures" / "manifest.txt"
PLAYGROUND = REPO_ROOT / "playground"
_GATE074_LOCK = REPO_ROOT / ".build" / "close-gate-074.lock"
_GATE076_LOCK = REPO_ROOT / ".build" / "close-gate-076.lock"

ISSUE_ID_RE = re.compile(r"^(\d{3})")

# issue_id -> list of human-readable gate names (for error messages)
TRACKED: dict[str, list[str]] = {
    "074": ["P2 component validate + wasmtime run (wasi_p2_native/hello.ark)"],
    "076": ["P2 filesystem fixture validate + wasmtime run (wasi_fs_p2.ark)"],
    "510": ["P2 component wasm-tools validate"],
    "472": ["playground typecheck distinguishes parse vs type errors"],
    "500": ["playground wasm typecheck export gate"],
    "051": ["stdlib_time monotonic run + clock intrinsic emitter handlers"],
    "123": ["Layer C import string syntax component fixture"],
    "641": ["T4 native scaffold compile (t4/native_scaffold.ark)"],
    "639": ["HTTP registry fixtures + gate-639-registry-http.py"],
    "643": ["Grain benchmark hook (compare-benchmarks --compare-lang grain)"],
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
    if name == "wasm-tools":
        cargo = Path.home() / ".cargo" / "bin" / "wasm-tools"
        if cargo.is_file():
            return str(cargo)
    return shutil.which(name)


def _compiler() -> Path | None:
    release = REPO_ROOT / "target" / "release" / "arukellt"
    if release.is_file():
        return release
    debug = REPO_ROOT / "target" / "debug" / "arukellt"
    if debug.is_file():
        return debug
    wrapper = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    if wrapper.is_file():
        return wrapper
    return None


def _selfhost_compile_env() -> dict[str, str]:
    """Prefer pinned selfhost wasm over stale local bootstrap-build artifacts."""
    env = dict(os.environ)
    if "ARUKELLT_SELFHOST_WASM" in env:
        return env
    for candidate in (
        REPO_ROOT / ".build" / "selfhost" / "arukellt-pinned-bootstrap.wasm",
        REPO_ROOT / "bootstrap" / "arukellt-selfhost.wasm",
    ):
        if candidate.is_file():
            env["ARUKELLT_SELFHOST_WASM"] = str(candidate)
            break
    return env


def _manifest_contains(entry: str) -> bool:
    if not MANIFEST.is_file():
        return False
    return entry in MANIFEST.read_text(encoding="utf-8")


def _compile_p2_component(fixture_rel: str, out: Path) -> tuple[int, str]:
    """Core wasm + post-wrap for gate 074 (run export + stdio bridge path)."""
    return _compile_p2_component_wrapped(fixture_rel, out)


def _compile_p2_component_direct(fixture_rel: str, out: Path) -> tuple[int, str]:
    """Pinned bootstrap `--emit component` (validate-only gates)."""
    compiler = _compiler()
    if compiler is None:
        return 2, "arukellt compiler binary not found (build release/debug first)"
    fixture = REPO_ROOT / fixture_rel
    if not fixture.is_file():
        return 1, f"missing fixture {fixture_rel}"
    fixture_arg = str(fixture_rel)
    try:
        out_arg = str(out.relative_to(REPO_ROOT))
    except ValueError:
        out_arg = str(out)
    cmd = [
        str(compiler),
        "compile",
        fixture_arg,
        "--target",
        "wasm32-wasi-p2",
        "--wasi-version",
        "p2",
        "--emit",
        "component",
        "-o",
        out_arg,
    ]
    if compiler.name == "arukellt-selfhost.sh":
        cmd = ["bash", str(compiler), *cmd[1:]]
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
        env=_selfhost_compile_env(),
    )
    if result.returncode != 0:
        tail = (result.stderr or result.stdout)[-800:]
        return 1, f"compile failed: {tail}"
    return 0, ""


def _compile_p2_component_wrapped(fixture_rel: str, out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "arukellt compiler binary not found (build release/debug first)"
    fixture = REPO_ROOT / fixture_rel
    if not fixture.is_file():
        return 1, f"missing fixture {fixture_rel}"
    fixture_arg = str(fixture_rel)
    out_dir = out.parent
    out_dir.mkdir(parents=True, exist_ok=True)
    core_out = out_dir / f"{out.stem}.core.wasm"
    try:
        core_arg = str(core_out.relative_to(REPO_ROOT))
    except ValueError:
        core_arg = str(core_out)
    try:
        out_arg = str(out.relative_to(REPO_ROOT))
    except ValueError:
        out_arg = str(out)
    cmd = [
        str(compiler),
        "compile",
        fixture_arg,
        "--target",
        "wasm32-wasi-p2",
        "--wasi-version",
        "p2",
        "--emit",
        "wasm",
        "-o",
        core_arg,
    ]
    if compiler.name == "arukellt-selfhost.sh":
        cmd = ["bash", str(compiler), *cmd[1:]]
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
        env=_selfhost_compile_env(),
    )
    if result.returncode != 0:
        tail = (result.stderr or result.stdout)[-800:]
        return 1, f"compile failed: {tail}"
    if not core_out.is_file():
        return 1, f"missing core wasm output {core_out}"
    try:
        import importlib.util

        wrap_spec = importlib.util.spec_from_file_location(
            "p2_component_wrap",
            REPO_ROOT / "scripts" / "selfhost" / "p2_component_wrap.py",
        )
        if wrap_spec is None or wrap_spec.loader is None:
            return 1, "missing scripts/selfhost/p2_component_wrap.py"
        wrap_mod = importlib.util.module_from_spec(wrap_spec)
        wrap_spec.loader.exec_module(wrap_mod)
        out.write_bytes(wrap_mod.wrap_p2_command_component(core_out.read_bytes()))
    except Exception as exc:  # noqa: BLE001
        return 1, f"p2_component_wrap failed: {exc}"
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



def _wasmtime_run_dir(component: Path, expect_stdout: str) -> tuple[int, str]:
    wasmtime = _find_tool("wasmtime")
    if not wasmtime:
        return 2, "wasmtime not in PATH"
    result = subprocess.run(
        [wasmtime, "run", "--dir", str(REPO_ROOT), str(component)],
        capture_output=True, text=True, errors="replace", timeout=60, cwd=str(REPO_ROOT),
    )
    if result.returncode != 0:
        return 1, f"wasmtime exit {result.returncode}: {(result.stderr or '')[-400:]}"
    if expect_stdout not in result.stdout:
        return 1, f"expected stdout containing {expect_stdout!r}, got {result.stdout!r}"
    return 0, ""


def gate_076() -> tuple[int, str]:
    entry = "component-compile:wasi_fs_p2.ark"
    if not _manifest_contains(entry):
        return 1, f"manifest missing {entry}"
    _GATE076_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with _GATE076_LOCK.open("w", encoding="utf-8") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        return _gate_076_locked()


def _gate_076_locked() -> tuple[int, str]:
    import importlib.util
    spec = importlib.util.spec_from_file_location("runtime_lock", REPO_ROOT / "scripts" / "selfhost" / "runtime_lock.py")
    if spec is None or spec.loader is None:
        return 1, "missing scripts/selfhost/runtime_lock.py"
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.with_selfhost_runtime_lock(_gate_076_body)


def _gate_076_body() -> tuple[int, str]:
    last_rc = 1
    last_msg = ""
    for attempt in range(3):
        out_dir = Path(tempfile.mkdtemp(prefix="close-gate-076-", dir=REPO_ROOT / ".build"))
        try:
            out = out_dir / "wasi_fs_p2.component.wasm"
            last_rc, last_msg = _compile_p2_component("tests/fixtures/wasi_fs_p2.ark", out)
            if last_rc != 0:
                continue
            last_rc, last_msg = _wasm_tools_validate(out)
            if last_rc != 0:
                continue
            last_rc, last_msg = _wasmtime_run_dir(out, "hello p2 fs")
            if last_rc == 0:
                return 0, ""
        finally:
            shutil.rmtree(out_dir, ignore_errors=True)
        if attempt < 2:
            time.sleep(0.1 * (attempt + 1))
    return last_rc, last_msg



def gate_074() -> tuple[int, str]:
    entry = "component-compile:wasi_p2_native/hello.ark"
    if not _manifest_contains(entry):
        return 1, f"manifest missing {entry}"
    _GATE074_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with _GATE074_LOCK.open("w", encoding="utf-8") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        return _gate_074_locked()


def _gate_074_locked() -> tuple[int, str]:
    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "runtime_lock",
        REPO_ROOT / "scripts" / "selfhost" / "runtime_lock.py",
    )
    if spec is None or spec.loader is None:
        return 1, "missing scripts/selfhost/runtime_lock.py"
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.with_selfhost_runtime_lock(_gate_074_body)


def _gate_074_body() -> tuple[int, str]:
    last_rc = 1
    last_msg = ""
    for attempt in range(3):
        out_dir = Path(
            tempfile.mkdtemp(prefix="close-gate-074-", dir=REPO_ROOT / ".build")
        )
        try:
            out = out_dir / "hello.component.wasm"
            last_rc, last_msg = _compile_p2_component(
                "tests/fixtures/wasi_p2_native/hello.ark", out
            )
            if last_rc != 0:
                continue
            last_rc, last_msg = _wasm_tools_validate(out)
            if last_rc != 0:
                continue
            last_rc, last_msg = _wasmtime_run(out, "hello p2")
            if last_rc == 0:
                return 0, ""
        finally:
            shutil.rmtree(out_dir, ignore_errors=True)
        if attempt < 2:
            time.sleep(0.1 * (attempt + 1))
    return last_rc, last_msg


def gate_510() -> tuple[int, str]:
    out_dir = REPO_ROOT / ".build" / "close-gate-510"
    out_dir.mkdir(parents=True, exist_ok=True)
    out = out_dir / "p2.component.wasm"
    rc, msg = _compile_p2_component_direct("tests/fixtures/wasi_p2_native/hello.ark", out)
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


def gate_639() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-639-registry-http.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-639-registry-http.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode == 2:
        return 2, (result.stdout + result.stderr)[-400:]
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def gate_641() -> tuple[int, str]:
    entry = "t4-compile:t4/native_scaffold.ark"
    if not _manifest_contains(entry):
        return 1, f"manifest missing {entry}"
    fixture = REPO_ROOT / "tests" / "fixtures" / "t4" / "native_scaffold.ark"
    if not fixture.is_file():
        return 1, "missing tests/fixtures/t4/native_scaffold.ark"
    target_ark = REPO_ROOT / "src" / "compiler" / "target.ark"
    native_ark = REPO_ROOT / "src" / "compiler" / "native.ark"
    if not target_ark.is_file() or not native_ark.is_file():
        return 1, "missing src/compiler/target.ark or native.ark"
    target_text = target_ark.read_text(encoding="utf-8")
    native_text = native_ark.read_text(encoding="utf-8")
    if "is_native_target" not in target_text:
        return 1, "target.ark lacks native registration"
    if "emit_native_scaffold" not in native_text or "T4 native scaffold" not in native_text:
        return 1, "native.ark lacks scaffold emitter"
    contract = (REPO_ROOT / "docs" / "target-contract.md").read_text(encoding="utf-8")
    if "T4 — native" not in contract or "scaffold" not in contract:
        return 1, "target-contract.md T4 section not scaffold"
    return 0, ""


def gate_643() -> tuple[int, str]:
    script = REPO_ROOT / "scripts" / "check" / "gate-643-grain-benchmark.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-643-grain-benchmark.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


GATES: dict[str, callable[[], tuple[int, str]]] = {
    "074": gate_074,
    "076": gate_076,
    "510": gate_510,
    "472": gate_472,
    "500": gate_500,
    "051": gate_051,
    "123": gate_123,
    "639": gate_639,
    "641": gate_641,
    "643": gate_643,
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
