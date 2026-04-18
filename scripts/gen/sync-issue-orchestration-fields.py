#!/usr/bin/env python3
"""Write **Orchestration class** / **Orchestration upstream** into open issue headers.

Reads a TSV with columns: id, bucket, track, upstream (track column ignored; issue file is source of truth for Track).

Usage:
  python3 scripts/gen/sync-issue-orchestration-fields.py --tsv PATH [--open-dir PATH]

Orchestration values are then exported in `issues/open/index-meta.json` when you run
`bash scripts/gen/generate-issue-index.sh`.
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


def format_upstream(raw: str) -> str:
    s = (raw or "").strip()
    if not s:
        return "—"
    return s.replace(";", ", ")


def ensure_orchestration_block(body: str, orch_class: str, upstream_raw: str) -> str:
    up = format_upstream(upstream_raw)
    block = f"**Orchestration class**: {orch_class}\n**Orchestration upstream**: {up}\n"
    if "**Orchestration class**:" in body:
        return re.sub(
            r"\*\*Orchestration class\*\*: *[^\n]*\n\*\*Orchestration upstream\*\*: *[^\n]*\n?",
            block,
            body,
            count=1,
        )
    m = re.search(r"^(\*\*Track\*\*: [^\n]*\n)", body, re.M)
    if not m:
        print(f"skip: no **Track** line", file=sys.stderr)
        return body
    insert_at = m.end()
    return body[:insert_at] + block + body[insert_at:]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tsv", type=Path, required=True)
    ap.add_argument(
        "--open-dir",
        type=Path,
        default=Path(__file__).resolve().parents[2] / "issues" / "open",
    )
    args = ap.parse_args()
    if not args.tsv.exists():
        print(f"TSV not found: {args.tsv}", file=sys.stderr)
        return 1
    rows = []
    for line in args.tsv.read_text().splitlines()[1:]:
        if not line.strip():
            continue
        parts = line.split("\t")
        if len(parts) < 2:
            continue
        iid = parts[0].strip().zfill(3)
        bucket = parts[1].strip()
        upstream = parts[3].strip() if len(parts) > 3 else ""
        rows.append((iid, bucket, upstream))
    updated = 0
    for iid, bucket, upstream in rows:
        paths = sorted(args.open_dir.glob(f"{iid}-*.md"))
        if not paths:
            print(f"warning: no file for id {iid}", file=sys.stderr)
            continue
        path = paths[0]
        text = path.read_text()
        new_text = ensure_orchestration_block(text, bucket, upstream)
        if new_text != text:
            path.write_text(new_text)
            updated += 1
    print(f"Updated {updated} issue files under {args.open_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
