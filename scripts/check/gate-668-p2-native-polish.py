#!/usr/bin/env python3
"""Close gate for #668 — P2 native polish (guest-native stdio + hygiene).

Proves:
1. Proof path uses current s2/s3 (not bootstrap stub overlay alone)
2. All wasi_p2_native/*.ark fixtures compile + validate + wasmtime
3. Guest core imports get-stdout / bwaf / get-stderr (no pseudo stdout::write)
4. Component artifact has wasi:cli/run@0.2.0 and no @0.2.6
5. Size / no P1 adapter markers (via gate-668-p2-size)
6. Platform / current-state docs are not deferred-only for P2 native
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

_SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from lib.selfhost_s2 import gate_env, is_current_selfhost_wasm  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE_DIR = REPO_ROOT / "tests" / "fixtures" / "wasi_p2_native"

FIXTURE_EXPECT = {
    "hello.ark": {"stdout": "hello p2", "exit": 0},
    "eprintln_stderr.ark": {"stderr": "hello stderr", "exit": 0},
    "args.ark": {"stdout": "alpha\nbeta\n", "exit": 0, "args": ["alpha", "beta"]},
    "env_var.ark": {
        "stdout": "env-ok\n",
        "exit": 0,
        "env": {"ARUKELLT_P2_ENV_PROOF": "env-ok"},
    },
    "exit_code.ark": {"stdout": "exit-marker", "exit": "nonzero"},
}

DOC_PATHS = (
    REPO_ROOT / "docs" / "platform" / "target-runtime-and-surfaces.md",
    REPO_ROOT / "docs" / "current-state.md",
)


def _compile(fixture_rel: str, out: Path, *, emit_component: bool) -> tuple[int, str]:
    compiler = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    if not compiler.is_file():
        return 2, "missing scripts/run/arukellt-selfhost.sh"
    try:
        out_arg = str(out.relative_to(REPO_ROOT))
    except ValueError:
        out_arg = str(out)
    cmd = [
        "bash",
        str(compiler),
        "compile",
        fixture_rel,
        "--target",
        "wasm32-gc",
        "--wasi-version",
        "wasi-p2",
    ]
    if emit_component:
        cmd.extend(["--emit", "component"])
    cmd.extend(["-o", out_arg])
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    if not out.is_file():
        return 1, f"missing output {out}"
    return 0, ""


def _validate(path: Path) -> tuple[int, str]:
    result = subprocess.run(
        ["wasm-tools", "validate", str(path)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    return 0, ""


def _wasmtime_run(
    path: Path,
    *,
    expect_stdout: str | None = None,
    expect_stderr: str | None = None,
    expect_exit: int | str = 0,
    args: list[str] | None = None,
    env: dict[str, str] | None = None,
) -> tuple[int, str]:
    cmd = [
        "wasmtime",
        "run",
        "--wasm",
        "gc",
        "--wasm",
        "function-references",
    ]
    if env:
        for key, value in env.items():
            cmd.extend(["--env", f"{key}={value}"])
    cmd.append(str(path))
    if args:
        cmd.extend(args)
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
    if expect_exit == "nonzero":
        if result.returncode == 0:
            return 1, "expected non-zero exit, got 0"
    elif result.returncode != expect_exit:
        return 1, f"wasmtime exit {result.returncode}: {(result.stderr or '')[-400:]}"
    if expect_stdout is not None and expect_stdout not in (result.stdout or ""):
        return 1, f"expected stdout {expect_stdout!r}, got {result.stdout!r}"
    if expect_stderr is not None and expect_stderr not in (result.stderr or ""):
        return 1, f"expected stderr {expect_stderr!r}, got {result.stderr!r}"
    return 0, ""


def _assert_guest_native_core(path: Path) -> tuple[int, str]:
    printed = subprocess.run(
        ["wasm-tools", "print", str(path)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if printed.returncode != 0:
        return 1, (printed.stderr or printed.stdout)[-400:]
    text = printed.stdout
    if 'import "wasi:cli/stdout@0.2.0" "write"' in text:
        return 1, "guest still imports pseudo stdout write"
    if 'import "wasi:cli/stderr@0.2.0" "write"' in text:
        return 1, "guest still imports pseudo stderr write"
    if 'import "wasi:cli/stdout@0.2.0" "get-stdout"' not in text:
        return 1, "guest missing get-stdout import"
    if 'import "wasi:io/streams@0.2.0" "blocking-write-and-flush"' not in text:
        return 1, "guest missing blocking-write-and-flush import"
    if 'import "wasi:cli/stderr@0.2.0" "get-stderr"' not in text:
        return 1, "guest missing get-stderr import"
    return 0, ""


def _assert_component_hygiene(path: Path) -> tuple[int, str]:
    data = path.read_bytes()
    if b"@0.2.6" in data:
        return 1, "artifact contains @0.2.6"
    if b"wasi:cli/run@0.2.0" not in data:
        return 1, "artifact missing wasi:cli/run@0.2.0"
    if b"wasi_snapshot_preview1" in data:
        return 1, "artifact contains wasi_snapshot_preview1"
    if b"wasi:cli/stdout@0.2.0::write" in data:
        return 1, "artifact contains pseudo stdout::write literal"
    return 0, ""


def _assert_docs_not_deferred_only() -> tuple[int, str]:
    for path in DOC_PATHS:
        if not path.is_file():
            return 1, f"missing {path.relative_to(REPO_ROOT)}"
        text = path.read_text(encoding="utf-8")
        lowered = text.lower()
        if re.search(r"p2 native.*deferred.?only|deferred.?only.*p2 native", lowered):
            return 1, f"{path.name} still claims P2 native deferred-only"
        if "guest-native" in lowered and "残件" in text and "get-stdout" in text:
            # Allow historical notes only when they no longer assert remaining work
            # for guest-native as open. current-state must describe landed path.
            if path.name == "current-state.md" and "guest-native 直呼びは #668" in text:
                return 1, "current-state.md still marks guest-native as #668 remaining"
    return 0, ""


def main() -> int:
    failures: list[str] = []
    try:
        env = gate_env(REPO_ROOT, build=False)
    except Exception as exc:  # noqa: BLE001
        print(f"gate-668-p2-native-polish: FAIL selfhost env: {exc}", file=sys.stderr)
        return 1
    wasm = Path(env.get("ARUKELLT_SELFHOST_WASM", ""))
    if not is_current_selfhost_wasm(wasm):
        print(
            "gate-668-p2-native-polish: FAIL proof must use current s2/s3 "
            f"(got {wasm})",
            file=sys.stderr,
        )
        return 1

    size_gate = subprocess.run(
        [sys.executable, str(REPO_ROOT / "scripts" / "check" / "gate-668-p2-size.py")],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=300,
    )
    if size_gate.returncode != 0:
        failures.append(f"size gate: {(size_gate.stderr or size_gate.stdout)[-400:]}")

    rc, msg = _assert_docs_not_deferred_only()
    if rc != 0:
        failures.append(f"docs: {msg}")

    out_dir = Path(tempfile.mkdtemp(prefix="gate-668-polish-", dir=REPO_ROOT / ".build"))
    try:
        core = out_dir / "hello-core.wasm"
        rc, msg = _compile("tests/fixtures/wasi_p2_native/hello.ark", core, emit_component=False)
        if rc != 0:
            failures.append(f"hello core compile: {msg}")
        else:
            rc, msg = _assert_guest_native_core(core)
            if rc != 0:
                failures.append(f"guest-native core: {msg}")

        for name, expect in FIXTURE_EXPECT.items():
            fixture_rel = f"tests/fixtures/wasi_p2_native/{name}"
            if not (REPO_ROOT / fixture_rel).is_file():
                failures.append(f"missing fixture {fixture_rel}")
                continue
            out = out_dir / f"{Path(name).stem}.component.wasm"
            rc, msg = _compile(fixture_rel, out, emit_component=True)
            if rc != 0:
                failures.append(f"{name} compile: {msg}")
                continue
            rc, msg = _validate(out)
            if rc != 0:
                failures.append(f"{name} validate: {msg}")
                continue
            rc, msg = _assert_component_hygiene(out)
            if rc != 0:
                failures.append(f"{name} hygiene: {msg}")
                continue
            rc, msg = _wasmtime_run(
                out,
                expect_stdout=expect.get("stdout"),
                expect_stderr=expect.get("stderr"),
                expect_exit=expect.get("exit", 0),
                args=expect.get("args"),
                env=expect.get("env"),
            )
            if rc != 0:
                failures.append(f"{name} wasmtime: {msg}")
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)

    if failures:
        print("gate-668-p2-native-polish: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-668-p2-native-polish: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
