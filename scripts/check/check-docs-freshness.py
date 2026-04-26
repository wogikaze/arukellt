#!/usr/bin/env python3
"""Check that project-state.toml fixture counts and updated date are fresh."""
from __future__ import annotations

import re
import sys
from datetime import date, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
STATE_FILE = ROOT / "docs" / "data" / "project-state.toml"
MANIFEST_FILE = ROOT / "tests" / "fixtures" / "manifest.txt"
MAX_STALENESS_DAYS = 60


def read_toml_value(text: str, key: str) -> str | None:
    for line in text.splitlines():
        m = re.match(rf'^{re.escape(key)}\s*=\s*"?([^"]*)"?', line)
        if m:
            return m.group(1).strip()
    return None


def count_manifest_entries(path: Path) -> int:
    count = 0
    for line in path.read_text().splitlines():
        stripped = line.strip()
        if stripped and not stripped.startswith("#"):
            count += 1
    return count


def main() -> int:
    errors: list[str] = []

    if not STATE_FILE.exists():
        print(f"SKIP: {STATE_FILE} not found")
        return 0
    state = STATE_FILE.read_text()

    # Check updated date freshness
    updated = read_toml_value(state, "updated")
    if updated:
        try:
            updated_date = date.fromisoformat(updated)
            staleness = (date.today() - updated_date).days
            if staleness > MAX_STALENESS_DAYS:
                errors.append(
                    f"project-state.toml updated date ({updated}) is "
                    f"{staleness} days old (max {MAX_STALENESS_DAYS})"
                )
        except ValueError:
            errors.append(f"project-state.toml updated date is not ISO format: {updated}")

    # Check fixture count consistency
    if MANIFEST_FILE.exists():
        actual_count = count_manifest_entries(MANIFEST_FILE)
        toml_count_str = read_toml_value(state, "fixture_count")
        if toml_count_str:
            try:
                toml_count = int(toml_count_str)
                if toml_count != actual_count:
                    errors.append(
                        f"project-state.toml fixture_count={toml_count} "
                        f"but manifest.txt has {actual_count} entries"
                    )
            except ValueError:
                errors.append(f"project-state.toml fixture_count is not an integer: {toml_count_str}")

    if errors:
        for e in errors:
            print(f"FAIL: {e}", file=sys.stderr)
        return 1

    print("docs freshness OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
