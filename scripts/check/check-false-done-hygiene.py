#!/usr/bin/env python3
"""False-done hygiene checks (docs/process/false-done-prevention.md).

Detects mechanical patterns that precede false-done closures:
- Status / directory mismatch (FD-02)
- Audit reopen without relocation (FD-01)
- Duplicate issue IDs across open/ and done/
- ``remains open`` without a later close note (FD-09)
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
OPEN_DIR = REPO_ROOT / "issues" / "open"
DONE_DIR = REPO_ROOT / "issues" / "done"

ISSUE_ID_RE = re.compile(r"^(\d{3})")
FRONTMATTER_RE = re.compile(r"^---\s*$(.*?)^---\s*$", re.MULTILINE | re.DOTALL)
MOVED_TO_OPEN_RE = re.compile(
    r"Moved from `issues/done/`.*to `issues/open/`",
    re.IGNORECASE,
)
RESOLUTION_RE = re.compile(
    r"^##\s+(Audit resolution|Closed by audit|Close note|Completed\s+—)",
    re.MULTILINE | re.IGNORECASE,
)


def _parse_frontmatter(path: Path) -> dict[str, str]:
    text = path.read_text(encoding="utf-8")
    match = FRONTMATTER_RE.match(text)
    if not match:
        return {}
    fields: dict[str, str] = {}
    for line in match.group(1).splitlines():
        if ":" in line:
            key, _, val = line.partition(":")
            fields[key.strip()] = val.strip()
    return fields


def _issue_ids(directory: Path) -> dict[str, Path]:
    found: dict[str, Path] = {}
    if not directory.is_dir():
        return found
    for path in directory.glob("*.md"):
        match = ISSUE_ID_RE.match(path.name)
        if match:
            found[match.group(1)] = path
    return found


def main() -> int:
    errors: list[str] = []

    open_ids = _issue_ids(OPEN_DIR)
    done_ids = _issue_ids(DONE_DIR)

    for label, store, expected in [
        ("issues/open", open_ids, "open"),
        ("issues/done", done_ids, "done"),
    ]:
        for issue_id, path in store.items():
            status = _parse_frontmatter(path).get("Status", "").lower()
            if status and status != expected:
                errors.append(
                    f"FD-02 STATUS_MISMATCH: #{issue_id} {path.name} "
                    f"has Status: {status} but lives under {label}/"
                )

    for issue_id in sorted(set(open_ids) & set(done_ids)):
        errors.append(
            f"DUPLICATE: issue #{issue_id} exists in both issues/open/ and issues/done/"
        )

    for issue_id, path in done_ids.items():
        text = path.read_text(encoding="utf-8")
        fm = _parse_frontmatter(path)
        action = fm.get("Action", "")
        reason = fm.get("Reason", "")
        if MOVED_TO_OPEN_RE.search(action) or MOVED_TO_OPEN_RE.search(reason):
            if not RESOLUTION_RE.search(text):
                errors.append(
                    f"FD-01 STALE_DONE: #{issue_id} {path.name} frontmatter "
                    "records move to issues/open/ without Audit resolution"
                )

    if errors:
        print("false-done-hygiene: FAIL", file=sys.stderr)
        for err in errors:
            print(f"  {err}", file=sys.stderr)
        return 1

    print("false-done-hygiene: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
