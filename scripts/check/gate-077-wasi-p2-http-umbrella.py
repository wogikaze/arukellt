#!/usr/bin/env python3
"""Close gate for umbrella #077 — WASI P2 HTTP (requires #655 + #656)."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def _run_gate(script: str) -> tuple[int, str]:
    path = REPO_ROOT / "scripts" / "check" / script
    if not path.is_file():
        return 1, f"missing {script}"
    result = subprocess.run(
        [sys.executable, str(path)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def main() -> int:
    failures: list[str] = []

    for label, script in (
        ("#655 outgoing client", "gate-655-http-outgoing.py"),
        ("#656 incoming server", "gate-656-http-incoming.py"),
    ):
        rc, msg = _run_gate(script)
        if rc != 0:
            failures.append(f"{label}: {msg}")

    if failures:
        print("gate-077-wasi-p2-http-umbrella: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1

    print("gate-077-wasi-p2-http-umbrella: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
