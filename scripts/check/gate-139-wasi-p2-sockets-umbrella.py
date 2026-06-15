#!/usr/bin/env python3
"""Close gate for umbrella #139 — WASI P2 sockets (requires #657 + #658)."""

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
        ("#657 connect/read/write", "gate-657-sockets-connect-read-write.py"),
        ("#658 listen/accept", "gate-658-sockets-listen-accept.py"),
    ):
        rc, msg = _run_gate(script)
        if rc != 0:
            failures.append(f"{label}: {msg}")

    if failures:
        print("gate-139-wasi-p2-sockets-umbrella: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1

    print("gate-139-wasi-p2-sockets-umbrella: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
