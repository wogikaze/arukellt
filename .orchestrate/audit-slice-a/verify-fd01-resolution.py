#!/usr/bin/env python3
"""Verifier repro: FD-01 candidates and unresolved stale metadata in issues/done/."""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
DONE = REPO / "issues" / "done"
OPEN = REPO / "issues" / "open"

FRONTMATTER_RE = re.compile(r"^---\s*$(.*?)^---\s*$", re.MULTILINE | re.DOTALL)
MOVE_RE = re.compile(
    r"Moved from.*issues/done.*(?:to|→).*issues/open",
    re.IGNORECASE,
)
RESOLUTION_RES = [
    re.compile(r"^##\s+(Audit resolution|Closed by audit|Close note|Completed\s+—|Slice complete)", re.M | re.I),
    re.compile(r"^##\s+Closed by wave\d+", re.M | re.I),
    re.compile(r"^##\s+Closed by orchestrator", re.M | re.I),
    re.compile(r"^##\s+Closed\s+—", re.M | re.I),
    re.compile(r"^##\s+Completion(?: note)?\s+—", re.M | re.I),
    re.compile(r"^##\s+Queue closure verification", re.M | re.I),
    re.compile(r"^##\s+Slice \d+ complete", re.M | re.I),
    re.compile(r"Audit resolution — 2026-06-12", re.I),
]
ISSUE_ID_RE = re.compile(r"^(\d{3})")


def parse_fm(text: str) -> dict[str, str]:
    match = FRONTMATTER_RE.match(text)
    if not match:
        return {}
    fields: dict[str, str] = {}
    for line in match.group(1).splitlines():
        if ":" in line:
            key, _, val = line.partition(":")
            fields[key.strip()] = val.strip()
    return fields


def has_move_metadata(text: str, fm: dict[str, str]) -> bool:
    hay = text + fm.get("Action", "") + fm.get("Reason", "")
    return bool(MOVE_RE.search(hay))


def has_resolution(text: str) -> bool:
    return any(r.search(text) for r in RESOLUTION_RES)


def main() -> int:
    candidates: list[tuple[str, str]] = []
    stale: list[str] = []

    for directory in (DONE, OPEN):
        for path in sorted(directory.glob("*.md")):
            text = path.read_text(encoding="utf-8")
            fm = parse_fm(text)
            if not has_move_metadata(text, fm):
                continue
            issue_id = ISSUE_ID_RE.match(path.name)
            iid = issue_id.group(1) if issue_id else "???"
            candidates.append((iid, directory.name))
            if directory == DONE and not has_resolution(text):
                stale.append(f"#{iid} {path.name}")

    done_count = sum(1 for _, loc in candidates if loc == "done")
    open_count = sum(1 for _, loc in candidates if loc == "open")
    audit_res = sum(
        1 for p in DONE.glob("*.md") if "Audit resolution — 2026-06-12" in p.read_text(encoding="utf-8")
    )

    print(f"FD-01 candidates total: {len(candidates)} (done={done_count}, open={open_count})")
    print(f"Audit resolution — 2026-06-12 in done/: {audit_res}")
    print(f"done/ stale (move metadata, no resolution): {len(stale)}")
    for item in stale:
        print(f"  {item}")

    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main())
