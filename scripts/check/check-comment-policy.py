#!/usr/bin/env python3
"""Enforce structured TODO/FIXME metadata and report public Ark doc coverage."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCAN_ROOTS = (ROOT / "src/compiler", ROOT / "std", ROOT / "scripts")
TODO_RE = re.compile(r"\b(?:TODO|FIXME)\b")
STRUCTURED_RE = re.compile(
    r"\b(?:TODO|FIXME)\(#\d+ owner=[^ )]+ removal=[^ )]+ recheck=\d{4}-\d{2}-\d{2}\)"
)
PUB_FN_RE = re.compile(r"^\s*pub\s+fn\s+")


def main() -> int:
    failures: list[str] = []
    public_functions = 0
    documented_public_functions = 0
    for scan_root in SCAN_ROOTS:
        for path in sorted(scan_root.rglob("*")):
            if not path.is_file() or path.suffix not in {".ark", ".py", ".sh"}:
                continue
            if path.name == "checks_broken.py":
                continue
            try:
                lines = path.read_text(encoding="utf-8").splitlines()
            except UnicodeDecodeError:
                continue
            rel = path.relative_to(ROOT)
            for line_no, line in enumerate(lines, 1):
                if path.suffix == ".ark":
                    comment_at = line.find("//")
                    comment = line[comment_at:] if comment_at >= 0 else ""
                else:
                    comment = line if line.lstrip().startswith("#") else ""
                if TODO_RE.search(comment) and not STRUCTURED_RE.search(comment):
                    failures.append(f"{rel}:{line_no}: unstructured TODO/FIXME")
                if path.suffix == ".ark" and PUB_FN_RE.match(line):
                    public_functions += 1
                    previous = lines[line_no - 2].lstrip() if line_no >= 2 else ""
                    if previous.startswith("///"):
                        documented_public_functions += 1

    for failure in failures:
        print(f"CQ-DOC-001: {failure}")
    print(
        "CQ-DOC-002 advisory: public Ark API docs "
        f"{documented_public_functions}/{public_functions}"
    )
    if failures:
        return 1
    print("comment policy: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
