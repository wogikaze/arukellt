#!/usr/bin/env python3
"""Smoke gate: arukellt lint tiers, exit contract, and --local package modules.

Checks:
  1. `lint --list` advertises W0011 / prefer-else-if
  2. full lint on prefer_else_if.ark emits W0011 and exits 0
  3. full lint --deny prefer-else-if exits non-zero
  4. full lint on a clean hello example exits 0 with no W0011
  5. `lint --local` works on a src/compiler package module (no module-load fail)
  6. `lint --local` on prefer_else_if.ark still emits W0011 / deny works
  7. `lint --help` / usage mentions --local
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
PACKAGE_MODULE = Path("src/compiler/driver/backend_typecheck.ark")


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

    help_out = _run(["--help"]).stdout + _run(["--help"]).stderr
    if "--local" not in help_out:
        print("FAIL: usage missing --local", file=sys.stderr)
        print(help_out, file=sys.stderr)
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

    pkg = _run(["lint", "--local", str(PACKAGE_MODULE)])
    pkg_out = pkg.stdout + pkg.stderr
    if "failed to load module" in pkg_out or "module loading error" in pkg_out:
        print("FAIL: lint --local still tries to load package modules", file=sys.stderr)
        print(pkg_out, file=sys.stderr)
        return 1
    if pkg.returncode != 0 and "W0011" not in pkg_out:
        print(f"FAIL: lint --local on package module exited {pkg.returncode}", file=sys.stderr)
        print(pkg_out, file=sys.stderr)
        return 1

    local_warn = _run(["lint", "--local", str(PREFER_ELSE_IF)])
    local_warn_out = local_warn.stdout + local_warn.stderr
    if "W0011" not in local_warn_out:
        print("FAIL: lint --local prefer_else_if.ark missing W0011", file=sys.stderr)
        print(local_warn_out, file=sys.stderr)
        return 1
    if local_warn.returncode != 0:
        print(
            f"FAIL: lint --local warning-only should exit 0, got {local_warn.returncode}",
            file=sys.stderr,
        )
        print(local_warn_out, file=sys.stderr)
        return 1

    local_deny = _run(["lint", "--local", "--deny", "prefer-else-if", str(PREFER_ELSE_IF)])
    if local_deny.returncode == 0:
        print("FAIL: lint --local --deny prefer-else-if should exit non-zero", file=sys.stderr)
        print(local_deny.stdout + local_deny.stderr, file=sys.stderr)
        return 1

    print("OK: ark lint smoke (tiers / list / warn / deny / local package)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
