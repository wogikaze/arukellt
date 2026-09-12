#!/usr/bin/env python3
"""Issue #666 gate: package a library through the official WASI P2 toolchain."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

_SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from lib.selfhost_s2 import gate_env, is_current_selfhost_wasm  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = REPO_ROOT / "examples" / "ark" / "export-library" / "calculator.ark"
GOLDEN = REPO_ROOT / "examples" / "ark" / "export-library" / "calculator.expected.wit"
OUT_WIT = REPO_ROOT / ".build" / "gate-666-calculator.wit"
OUT_COMPONENT = REPO_ROOT / ".build" / "gate-666-calculator.component.wasm"
COMPILER = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"


def _normalize_wit(text: str) -> str:
    lines = [line.rstrip() for line in text.strip().splitlines()]
    return "\n".join(line for line in lines if line) + "\n"


def main() -> int:
    if not FIXTURE.is_file():
        print(f"error: missing {FIXTURE}", file=sys.stderr)
        return 1
    if not GOLDEN.is_file():
        print(f"error: missing {GOLDEN}", file=sys.stderr)
        return 1
    if not COMPILER.is_file():
        print("error: arukellt-selfhost.sh missing", file=sys.stderr)
        return 1

    env_wasm_tools = os.environ.get("ARUKELLT_WASM_TOOLS_BIN", "")
    wasm_tools = env_wasm_tools or shutil.which("wasm-tools")
    if not wasm_tools:
        print("FAIL: gate-666 requires wasm-tools for official component packaging", file=sys.stderr)
        return 1

    try:
        env = gate_env(REPO_ROOT, build=True)
    except (FileNotFoundError, RuntimeError) as exc:
        print(f"FAIL: gate-666 requires s2 selfhost wasm: {exc}", file=sys.stderr)
        return 1

    wasm = env.get("ARUKELLT_SELFHOST_WASM", "")
    if not is_current_selfhost_wasm(wasm):
        print(f"FAIL: gate-666 requires s2/s3 selfhost wasm, got {wasm!r}", file=sys.stderr)
        return 1

    OUT_WIT.parent.mkdir(parents=True, exist_ok=True)
    wit_cmd = [
        "bash",
        str(COMPILER),
        "compile",
        str(FIXTURE.relative_to(REPO_ROOT)),
        "--target",
        "wasm32-gc",
        "--emit",
        "wit",
        "-o",
        str(OUT_WIT.relative_to(REPO_ROOT)),
    ]
    result = subprocess.run(
        wit_cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
        env=env,
    )
    if result.returncode != 0:
        tail = (result.stderr or result.stdout)[-800:]
        print(f"FAIL: calculator --emit wit compile failed: {tail}", file=sys.stderr)
        return 1
    if not OUT_WIT.is_file() or OUT_WIT.stat().st_size == 0:
        print("FAIL: calculator --emit wit returned empty output", file=sys.stderr)
        return 1

    emitted = _normalize_wit(OUT_WIT.read_text(encoding="utf-8"))
    golden = _normalize_wit(GOLDEN.read_text(encoding="utf-8"))
    if emitted != golden:
        print("FAIL: calculator --emit wit diverges from golden", file=sys.stderr)
        print("--- golden", file=sys.stderr)
        print(golden, file=sys.stderr)
        print("+++ emitted", file=sys.stderr)
        print(emitted, file=sys.stderr)
        return 1

    if "export add:" not in emitted or "export mul:" not in emitted:
        print("FAIL: calculator WIT missing add/mul exports", file=sys.stderr)
        return 1

    component_cmd = [
        "bash",
        str(COMPILER),
        "compile",
        str(FIXTURE.relative_to(REPO_ROOT)),
        "--target",
        "wasm32-gc",
        "--wasi-version",
        "wasi-p2",
        "--emit",
        "component",
        "-o",
        str(OUT_COMPONENT.relative_to(REPO_ROOT)),
    ]
    result = subprocess.run(
        component_cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
        env=env,
    )
    if result.returncode != 0:
        tail = (result.stderr or result.stdout)[-1200:]
        print(f"FAIL: calculator official component packaging failed: {tail}", file=sys.stderr)
        return 1
    if not OUT_COMPONENT.is_file() or OUT_COMPONENT.stat().st_size == 0:
        print("FAIL: official component packaging returned empty output", file=sys.stderr)
        return 1

    validate = subprocess.run(
        [wasm_tools, "validate", str(OUT_COMPONENT)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=30,
        env=env,
    )
    if validate.returncode != 0:
        print(f"FAIL: official component is invalid: {validate.stderr[-800:]}", file=sys.stderr)
        return 1

    wit = subprocess.run(
        [wasm_tools, "component", "wit", str(OUT_COMPONENT)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=30,
        env=env,
    )
    if wit.returncode != 0:
        print(f"FAIL: packaged component WIT inspection failed: {wit.stderr[-800:]}", file=sys.stderr)
        return 1
    for marker in ("wasi:cli/environment", "add", "mul"):
        if marker not in wit.stdout:
            print(f"FAIL: packaged component WIT missing {marker!r}", file=sys.stderr)
            return 1

    print("pass: calculator WIT golden and official WASI P2 component packaging")
    return 0


if __name__ == "__main__":
    sys.exit(main())
