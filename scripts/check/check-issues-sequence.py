#!/usr/bin/env python3
from pathlib import Path
import re
import sys

REPO_ROOT = Path(__file__).resolve().parents[2]

ISSUE_DIRS = [
    REPO_ROOT / "issues" / "open",
    REPO_ROOT / "issues" / "done",
    REPO_ROOT / "issues" / "blocked",
    REPO_ROOT / "issues" / "reject",
]

PATTERN = re.compile(r"^(\d{3})([a-z]?)-.*\.md$")

ids = set()

for d in ISSUE_DIRS:
    if not d.exists():
        continue
    for p in d.glob("*.md"):
        m = PATTERN.match(p.name)
        if not m:
            continue
        ids.add(int(m.group(1)))

if not ids:
    print("no issue files found")
    sys.exit(1)

start = min(ids)
end = max(ids)
missing = [i for i in range(start, end + 1) if i not in ids]

print(f"range: {start:03d}..{end:03d}")

if missing:
    print("missing:")
    for i in missing:
        print(f"  {i:03d}")
    sys.exit(1)

print("OK: no gaps")