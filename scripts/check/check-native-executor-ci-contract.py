#!/usr/bin/env python3
"""Enforce native-executor CI contract for Experimental promotion (#834).

- CI workflows must never pass --allow-high-rss.
- Manager rejects --allow-high-rss when CI/GITHUB_ACTIONS is set.
- verify quick must include native capability / safepoint / CI-contract gates.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CI_YML = ROOT / ".github" / "workflows" / "ci.yml"
MANAGER = ROOT / "scripts" / "manager.py"


def _fail(message: str) -> int:
    print(f"FAIL: {message}", file=sys.stderr)
    return 1


def _check_ci_yml() -> int:
    text = CI_YML.read_text(encoding="utf-8")
    # Mentions inside the forbid-negative-test are allowed; actual lane success
    # invocations (native-executor --build --allow-high-rss without expecting
    # failure) are not.
    for line_number, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if "--allow-high-rss" not in stripped:
            continue
        if stripped.startswith("#"):
            continue
        if "expected" in stripped.lower() and "rejected" in stripped.lower():
            continue
        if "forbid" in stripped.lower():
            continue
        # Negative test: command must be inside `if ...; then` failure branch.
        if "if " in stripped and "--allow-high-rss" in stripped:
            continue
        return _fail(
            f"ci.yml:{line_number}: disallowed --allow-high-rss usage: {stripped}"
        )
    required_markers = (
        "native-executor-gates",
        "check-native-cpp-capabilities.py",
        "check-native-executor-ci-contract.py",
    )
    for marker in required_markers:
        if marker not in text:
            return _fail(f"ci.yml missing native executor gate marker: {marker}")
    return 0


def _check_manager_ci_reject() -> int:
    env = os.environ.copy()
    env["CI"] = "true"
    env["GITHUB_ACTIONS"] = "true"
    result = subprocess.run(
        [
            sys.executable,
            str(MANAGER),
            "selfhost",
            "native-executor",
            "--build",
            "--allow-high-rss",
            "--dry-run",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    if result.returncode == 0:
        return _fail("manager accepted --allow-high-rss under CI (expected reject)")
    combined = (result.stdout or "") + (result.stderr or "")
    if "forbidden" not in combined.lower() and "escape hatch" not in combined.lower():
        return _fail(
            "manager CI reject message missing; stdout/stderr:\n" + combined[-1500:]
        )
    return 0


def _check_verify_quick_wiring() -> int:
    text = MANAGER.read_text(encoding="utf-8")
    required = (
        "check-native-cpp-capabilities.py",
        "check-native-cpp-safepoint-audit.py",
        "check-native-executor-ci-contract.py",
        "check-native-runtime-c99-werror.py",
        "test_native_cpp_executor.py",
    )
    for marker in required:
        if marker not in text:
            return _fail(f"verify quick wiring missing {marker} in manager.py")
    return 0


def main() -> int:
    if not CI_YML.is_file():
        return _fail(f"missing {CI_YML}")
    for checker in (_check_ci_yml, _check_manager_ci_reject, _check_verify_quick_wiring):
        rc = checker()
        if rc != 0:
            return rc
    print("native-executor-ci-contract: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
