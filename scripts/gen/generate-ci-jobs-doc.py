#!/usr/bin/env python3
"""Generate docs/data/ci-jobs.md from .github/workflows/ci.yml (#769)."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
OUT = ROOT / "docs" / "data" / "ci-jobs.md"

# Human notes for known jobs (keep short; do not invent missing jobs).
NOTES = {
    "quality-format": "Canonical Ark formatter check through manager.py.",
    "quality-lint": "Ark correctness lint and lint-contract smoke through manager.py.",
    "verify-quick": "PR-required quick verification, including quality quick.",
    "verification": "Runs `python3 scripts/manager.py verify` (and related verify lanes).",
    "selfhost": "Selfhost fixpoint + fixture/CLI/diag parity (ADR-029).",
    "docs": "Docs consistency / freshness / structure checks.",
    "extension-tests": "VS Code extension activation and live CLI E2E.",
    "release-tag": "Tag-only: project-state.toml version vs git tag.",
    "verify": "Aggregator / final required-gate summary over blocking jobs.",
    "ci-category-summary": "Always-published category-to-job result summary.",
}


def parse_jobs(text: str) -> list[str]:
    if "jobs:" not in text:
        return []
    body = text.split("jobs:", 1)[1]
    return re.findall(r"^  ([A-Za-z0-9_-]+):\s*$", body, re.M)


def render(jobs: list[str]) -> str:
    lines = [
        "# CI jobs (generated)",
        "",
        "> Generated from `.github/workflows/ci.yml` by `scripts/gen/generate-ci-jobs-doc.py`.",
        "> Do not hand-edit the table. Unknown job IDs in current docs fail gate-765 / gate-769 checks.",
        "",
        "| Job ID | Notes |",
        "|--------|-------|",
    ]
    for job in jobs:
        note = NOTES.get(job, "See workflow definition.")
        lines.append(f"| `{job}` | {note} |")
    lines.extend(
        [
            "",
            "## Category mapping (informal)",
            "",
            "| Category | Primary job ID |",
            "|----------|----------------|",
            "| verification / fixtures / docs checks | `verification`, `docs` |",
            "| formatting / lint policy | `quality-format`, `quality-lint` |",
            "| quick verification | `verify-quick` |",
            "| bootstrap / selfhost parity | `selfhost` |",
            "| editor / VS Code extension | `extension-tests` |",
            "| release tag version | `release-tag` |",
            "| merge aggregator | `verify` |",
            "| run summary | `ci-category-summary` |",
            "",
            "There is **no** `fixture-primary`, `verification-bootstrap`,",
            "`verification-harness-quick`, or `determinism` top-level job in `ci.yml`.",
            "Those historical names must not appear as current CI job IDs.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    check = "--check" in sys.argv
    if not WORKFLOW.is_file():
        print(f"missing {WORKFLOW}", file=sys.stderr)
        return 1
    jobs = parse_jobs(WORKFLOW.read_text(encoding="utf-8"))
    if not jobs:
        print("no jobs parsed from ci.yml", file=sys.stderr)
        return 1
    content = render(jobs)
    if check:
        if not OUT.is_file() or OUT.read_text(encoding="utf-8") != content:
            print("ci-jobs.md stale; run python3 scripts/gen/generate-ci-jobs-doc.py", file=sys.stderr)
            return 1
        print("ci-jobs.md up to date")
        return 0
    OUT.write_text(content, encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)} ({len(jobs)} jobs)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
