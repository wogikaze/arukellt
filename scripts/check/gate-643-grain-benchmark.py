#!/usr/bin/env python3
"""Close gate for issue #643 — Grain benchmark hook."""
from pathlib import Path
import subprocess
import sys

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "perf" / "compare-benchmarks.sh"
FIB_GRAIN = REPO_ROOT / "benchmarks" / "fib.grain"


def main() -> int:
    if not SCRIPT.is_file():
        print("FAIL: missing compare-benchmarks.sh", file=sys.stderr)
        return 1
    text = SCRIPT.read_text(encoding="utf-8")
    if "--help" not in text or "grain" not in text.lower():
        print("FAIL: compare-benchmarks.sh lacks --help grain hook", file=sys.stderr)
        return 1
    if not FIB_GRAIN.is_file():
        print("FAIL: missing benchmarks/fib.grain", file=sys.stderr)
        return 1
    proc = subprocess.run(
        ["bash", str(SCRIPT), "--help"],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
    )
    out = (proc.stdout + proc.stderr).lower()
    if proc.returncode != 0 or "grain" not in out:
        print("FAIL: --help missing grain documentation", file=sys.stderr)
        return 1
    print("gate-643-grain-benchmark: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
