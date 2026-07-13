#!/usr/bin/env python3
"""Smoke gate: arukellt lint is wired and --deny escalates prefer-else-if.

Checks:
  1. `lint --list` advertises W0011 / prefer-else-if
  2. lint on prefer_else_if.ark emits W0011 and exits 0 (warnings only)
  3. lint --deny prefer-else-if on the same fixture exits non-zero
  4. lint on a clean hello example exits 0 with no W0011
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
WRAPPER = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
PREFER_ELSE_IF = Path("tests/fixtures/diagnostics/prefer_else_if.ark")
CLEAN = Path("docs/examples/hello.ark")


def _env() -> dict[str, str]:
    env = {**os.environ}
    s2 = REPO_ROOT / ".build" / "selfhost" / "arukellt-s2.wasm"
    bootstrap = REPO_ROOT / "bootstrap" / "arukellt-selfhost.wasm"
    if s2.is_file():
        env["ARUKELLT_SELFHOST_WASM"] = str(s2)
    elif bootstrap.is_file():
        env["ARUKELLT_SELFHOST_WASM"] = str(bootstrap)
    return env


def _run(args: list[str], timeout: int = 180) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(WRAPPER), *args],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=timeout,
        env=_env(),
    )


def main() -> int:
    if not WRAPPER.is_file():
        print(f"FAIL: missing {WRAPPER}", file=sys.stderr)
        return 1

    listed = _run(["lint", "--list"])
    if listed.returncode != 0:
        print("FAIL: lint --list exited non-zero", file=sys.stderr)
        print(listed.stderr, file=sys.stderr)
        return 1
    list_out = listed.stdout + listed.stderr
    if "W0011" not in list_out or "prefer-else-if" not in list_out:
        print("FAIL: lint --list missing W0011 / prefer-else-if", file=sys.stderr)
        print(list_out, file=sys.stderr)
        return 1

    warn = _run(["lint", str(PREFER_ELSE_IF)])
    warn_out = warn.stdout + warn.stderr
    if "W0011" not in warn_out:
        print("FAIL: prefer_else_if.ark did not emit W0011", file=sys.stderr)
        print(warn_out, file=sys.stderr)
        return 1
    if warn.returncode != 0:
        print(
            f"FAIL: warning-only lint should exit 0, got {warn.returncode}",
            file=sys.stderr,
        )
        print(warn_out, file=sys.stderr)
        return 1

    denied = _run(["lint", "--deny", "prefer-else-if", str(PREFER_ELSE_IF)])
    denied_out = denied.stdout + denied.stderr
    if denied.returncode == 0:
        print("FAIL: --deny prefer-else-if should exit non-zero", file=sys.stderr)
        print(denied_out, file=sys.stderr)
        return 1
    if "W0011" not in denied_out:
        print("FAIL: denied lint missing W0011 in output", file=sys.stderr)
        print(denied_out, file=sys.stderr)
        return 1

    clean = _run(["lint", str(CLEAN)])
    clean_out = clean.stdout + clean.stderr
    if clean.returncode != 0:
        print(f"FAIL: clean lint exited {clean.returncode}", file=sys.stderr)
        print(clean_out, file=sys.stderr)
        return 1
    if "W0011" in clean_out:
        print("FAIL: clean fixture unexpectedly emitted W0011", file=sys.stderr)
        print(clean_out, file=sys.stderr)
        return 1

    print("OK: ark lint smoke (list / warn-exit-0 / deny-exit-1 / clean)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
