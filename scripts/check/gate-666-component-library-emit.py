#!/usr/bin/env python3
"""Issue #666 gate: library export --emit wit golden for calculator.ark."""

from __future__ import annotations

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
OUT = REPO_ROOT / ".build" / "gate-666-calculator.wit"
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

    try:
        env = gate_env(REPO_ROOT, build=True)
    except (FileNotFoundError, RuntimeError) as exc:
        print(f"FAIL: gate-666 requires s2 selfhost wasm: {exc}", file=sys.stderr)
        return 1

    wasm = env.get("ARUKELLT_SELFHOST_WASM", "")
    if not is_current_selfhost_wasm(wasm):
        print(f"FAIL: gate-666 requires s2/s3 selfhost wasm, got {wasm!r}", file=sys.stderr)
        return 1

    OUT.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        "bash",
        str(COMPILER),
        "compile",
        str(FIXTURE.relative_to(REPO_ROOT)),
        "--target",
        "wasm32-gc",
        "--emit",
        "wit",
        "-o",
        str(OUT.relative_to(REPO_ROOT)),
    ]
    result = subprocess.run(
        cmd,
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
    if not OUT.is_file() or OUT.stat().st_size == 0:
        print("FAIL: calculator --emit wit returned empty output", file=sys.stderr)
        return 1

    emitted = _normalize_wit(OUT.read_text(encoding="utf-8"))
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

    print("pass: gate-666 calculator library WIT golden")
    return 0


if __name__ == "__main__":
    sys.exit(main())
