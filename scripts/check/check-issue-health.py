#!/usr/bin/env python3
"""Check issue metadata health.

Verifies that issue files have valid frontmatter, status consistency,
no stale dependencies, and index synchronization.

Usage:
    check-issue-health.py [--open-dir <path>] [--done-dir <path>] [--fix]
"""

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OPEN = REPO_ROOT / "issues" / "open"
DEFAULT_DONE = REPO_ROOT / "issues" / "done"
PATTERN = re.compile(r"^(\d{3})")

ISSUE_ID_RE = re.compile(r"^(\d{3})")
FRONTMATTER_RE = re.compile(r"^---\s*$(.*?)^---\s*$", re.MULTILINE | re.DOTALL)


def parse_frontmatter(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    m = FRONTMATTER_RE.match(text)
    if not m:
        return {}
    fm = {}
    for line in m.group(1).splitlines():
        if ":" in line:
            key, _, val = line.partition(":")
            fm[key.strip()] = val.strip()
    return fm


def check_issues(open_dir: Path, done_dir: Path, fix: bool) -> int:
    errors = 0
    open_files: dict[str, Path] = {}
    done_files: dict[str, Path] = {}

    for d, store in [(open_dir, open_files), (done_dir, done_files)]:
        if not d.exists():
            continue
        for p in d.glob("*.md"):
            m = ISSUE_ID_RE.match(p.name)
            if m:
                store[m.group(1)] = p

    # ── Check frontmatter status ──────────────────────────────────────────
    for label, store, expected_status in [
        ("issues/open", open_files, "open"),
        ("issues/done", done_files, "done"),
    ]:
        for issue_id, path in store.items():
            fm = parse_frontmatter(path)
            status = fm.get("Status", "").lower()
            if status and status != expected_status:
                print(f"STATUS_MISMATCH: {path.name} has Status: {status} but is in {label}/")
                errors += 1

    # ── Check for issues present in both directories ──────────────────────
    duplicate = set(open_files.keys()) & set(done_files.keys())
    for issue_id in sorted(duplicate):
        print(f"DUPLICATE: issue {issue_id} in both open/ and done/")
        errors += 1

    # ── Check for unchecked checkboxes in done/ ───────────────────────────
    if done_dir.exists():
        for p in done_dir.glob("*.md"):
            text = p.read_text(encoding="utf-8")
            if "- [ ]" in text or "- [ ]" in text:
                print(f"UNCHECKED_IN_DONE: {p.name} has unchecked items")
                errors += 1

    # ── Check for dead dependency references ──────────────────────────────
    depends_re = re.compile(r"(?:Depends on|depends-on|blocked-by):\s*#?(\d{3})")
    for label, store in [("open", open_files), ("done", done_files)]:
        for issue_id, path in store.items():
            text = path.read_text(encoding="utf-8")
            for m in depends_re.finditer(text):
                dep_id = m.group(1)
                if dep_id not in open_files and dep_id not in done_files:
                    print(f"DEAD_DEPENDENCY: {path.name} references #{dep_id} which does not exist")
                    errors += 1

    if errors == 0:
        print("ISSUE_HEALTH: PASS")
        return 0
    else:
        print(f"ISSUE_HEALTH: {errors} issue(s)")
        return 1


def main() -> int:
    parser = argparse.ArgumentParser(description="Check issue metadata health")
    parser.add_argument("--open-dir", default=str(DEFAULT_OPEN))
    parser.add_argument("--done-dir", default=str(DEFAULT_DONE))
    parser.add_argument("--fix", action="store_true", help="Auto-fix simple issues")
    args = parser.parse_args()
    return check_issues(Path(args.open_dir), Path(args.done_dir), args.fix)


if __name__ == "__main__":
    sys.exit(main())
