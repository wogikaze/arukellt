#!/usr/bin/env python3
"""Close gate for issue #679 — docs-to-runtime contract audit."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT = REPO_ROOT / "docs" / "process" / "docs-runtime-contract-audit-2026-06-17.md"
CHECK_SCRIPT = REPO_ROOT / "scripts" / "check" / "check-docs-consistency.py"

CHECKLIST_ROWS = (
    ("Wasm-first", r"Wasm-first"),
    ("Component/WIT target", r"Component/WIT target"),
    ("README status block", r"README status block"),
    ("target-contract", r"target-contract"),
    ("capability-surface", r"capability-surface"),
    ("stdlib docs", r"stdlib docs"),
    ("legacy", r"legacy|archived"),
    ("false-done", r"false-done"),
)


def _find_checklist_row(table_section: str, pattern: str) -> str | None:
    for line in table_section.splitlines():
        if not re.match(r"^\| \d+ \|", line):
            continue
        if re.search(pattern, line, re.IGNORECASE):
            return line
    return None

VERDICT_RE = re.compile(r"\b(OK|gap|deferred)\b", re.IGNORECASE)
ISSUE_LINK_RE = re.compile(r"\[#\d+\]\([^)]*issues/(?:open|done)/\d+[^)]*\)")


def main() -> int:
    failures: list[str] = []

    if not REPORT.is_file():
        failures.append(f"missing audit report: {REPORT.relative_to(REPO_ROOT)}")
        return _finish(failures)

    text = REPORT.read_text(encoding="utf-8")

    for heading in ("Executive summary", "Gap → issue"):
        if heading not in text:
            failures.append(f"audit report missing required section: {heading!r}")

    table_section = text
    if "## 1." in text:
        table_section = text.split("## 6.", 1)[0]

    for label, pattern in CHECKLIST_ROWS:
        row = _find_checklist_row(table_section, pattern)
        if row is None:
            failures.append(f"checklist row not found for: {label}")
            continue
        if not VERDICT_RE.search(row):
            failures.append(f"checklist row missing OK/gap/deferred verdict: {label}")
        if "**OK**" not in row and not ISSUE_LINK_RE.search(row):
            failures.append(f"gap/deferred row missing issue link: {label}")

    if CHECK_SCRIPT.is_file():
        result = subprocess.run(
            [sys.executable, str(CHECK_SCRIPT)],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
            timeout=180,
        )
        if result.returncode != 0:
            tail = (result.stdout + result.stderr)[-1200:]
            failures.append(f"check-docs-consistency.py failed:\n{tail}")
    else:
        failures.append("missing scripts/check/check-docs-consistency.py")

    return _finish(failures)


def _finish(failures: list[str]) -> int:
    if failures:
        print("gate-679-docs-runtime-contract-audit: FAIL", file=sys.stderr)
        for item in failures:
            print(f"  - {item}", file=sys.stderr)
        return 1
    print("gate-679-docs-runtime-contract-audit: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
