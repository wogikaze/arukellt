#!/usr/bin/env python3
"""ADR registry integrity checks (identity, status, dates, supersession).

Enforces ADR-0000 process rules for docs/adr/ADR-*.md.
"""
from __future__ import annotations

import re
import sys
from collections import defaultdict
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ADR_DIR = ROOT / "docs" / "adr"

ALLOWED_STATUSES = frozenset(
    {"PROPOSED", "ACCEPTED", "SUPERSEDED", "REJECTED", "DEFERRED"}
)
FORBIDDEN_ALIASES = frozenset({"DECIDED", "DRAFT", "SURVEY"})

FILENAME_RE = re.compile(r"^ADR-0*(\d+)(?:[A-Z]?[A-Z0-9]*)?-.+\.md$", re.IGNORECASE)
# Also allow ADR-0001 style and ADR-004-P4 style
FILENAME_RE_FLEX = re.compile(r"^ADR-0*(\d+)", re.IGNORECASE)

STATUS_LINE_RE = re.compile(
    r"(?:ステータス|\*\*Status\*\*|Status)\s*[:：]\s*\*?\*?([A-Za-z]+)",
    re.IGNORECASE,
)
DATE_RE = re.compile(r"\b(20\d{2})-(\d{2})-(\d{2})\b")
SUPERSEDES_RE = re.compile(
    r"(?i)(?:\*\*)?Supersedes(?:\*\*)?\s*:\s*(.+)"
)
SUPERSEDED_BY_RE = re.compile(
    r"(?i)(?:\*\*)?Superseded-by(?:\*\*)?\s*:\s*(.+)"
)
ADR_REF_RE = re.compile(r"ADR-0*(\d+)", re.IGNORECASE)
TOMBSTONE_RE = re.compile(r"^##\s+Tombstone\s*$", re.MULTILINE)


def is_tombstone(text: str) -> bool:
    return bool(TOMBSTONE_RE.search(text))


def extract_status(text: str) -> str | None:
    for line in text.splitlines()[:20]:
        m = STATUS_LINE_RE.search(line)
        if m:
            return m.group(1).upper()
    return None


def extract_adr_refs(fragment: str) -> list[int]:
    return [int(n) for n in ADR_REF_RE.findall(fragment)]


def main() -> int:
    today = date.today()
    errors: list[str] = []
    warnings: list[str] = []

    files = sorted(ADR_DIR.glob("ADR-*.md"))
    if not files:
        print("no ADR files found", file=sys.stderr)
        return 1

    by_id: dict[int, list[Path]] = defaultdict(list)
    bodies: dict[int, list[Path]] = defaultdict(list)
    tombstones: dict[int, list[Path]] = defaultdict(list)
    id_by_path: dict[Path, int] = {}

    for path in files:
        m = FILENAME_RE_FLEX.match(path.name)
        if not m:
            errors.append(f"{path.name}: filename does not start with ADR-<number>")
            continue
        adr_id = int(m.group(1))
        by_id[adr_id].append(path)
        id_by_path[path] = adr_id
        text = path.read_text(encoding="utf-8")
        if is_tombstone(text):
            tombstones[adr_id].append(path)
        else:
            bodies[adr_id].append(path)

        status = extract_status(text)
        if status is None:
            errors.append(f"{path.name}: missing status header")
        elif status in FORBIDDEN_ALIASES:
            errors.append(
                f"{path.name}: forbidden status alias {status} "
                f"(use {', '.join(sorted(ALLOWED_STATUSES))})"
            )
        elif status not in ALLOWED_STATUSES:
            # Allow "ACCEPTED" with trailing junk only if we captured the token
            errors.append(
                f"{path.name}: unknown status {status!r}; "
                f"allowed: {', '.join(sorted(ALLOWED_STATUSES))}"
            )

        # Future dates anywhere in the file
        for ym, mo, da in DATE_RE.findall(text):
            try:
                d = date(int(ym), int(mo), int(da))
            except ValueError:
                errors.append(f"{path.name}: invalid date {ym}-{mo}-{da}")
                continue
            if d > today:
                errors.append(f"{path.name}: future date {d.isoformat()} (today={today})")

        # Unchecked boxes in ACCEPTED ADRs (warning)
        if status == "ACCEPTED" and re.search(r"^- \[ \]", text, re.MULTILINE):
            warnings.append(f"{path.name}: ACCEPTED ADR contains unchecked checkbox")

        # Supersession targets
        for label, cre in (("Supersedes", SUPERSEDES_RE), ("Superseded-by", SUPERSEDED_BY_RE)):
            for line in text.splitlines()[:30]:
                m = cre.match(line.strip())
                if not m:
                    continue
                refs = extract_adr_refs(m.group(1))
                if not refs and "none" not in m.group(1).lower():
                    # Allow prose without number only if explicit none
                    if re.search(r"ADR-\d+", m.group(1), re.I):
                        pass
                    else:
                        warnings.append(
                            f"{path.name}: {label} line has no ADR number: {m.group(1)[:80]}"
                        )
                for ref in refs:
                    if ref not in by_id and not list(ADR_DIR.glob(f"ADR-*{ref}-*.md")) and not list(
                        ADR_DIR.glob(f"ADR-{ref:03d}-*.md")
                    ) and not list(ADR_DIR.glob(f"ADR-{ref}-*.md")):
                        # deferred until by_id fully built — check later
                        pass

    # ID uniqueness: at most one non-tombstone body per number
    for adr_id, paths in sorted(bodies.items()):
        if len(paths) > 1:
            names = ", ".join(p.name for p in paths)
            errors.append(f"ADR-{adr_id:03d}: multiple non-tombstone bodies: {names}")

    # Resolve supersession against known IDs
    known_ids = set(by_id)
    for path in files:
        text = path.read_text(encoding="utf-8")
        for line in text.splitlines()[:40]:
            for cre in (SUPERSEDES_RE, SUPERSEDED_BY_RE):
                m = cre.match(line.strip())
                if not m:
                    continue
                for ref in extract_adr_refs(m.group(1)):
                    if ref not in known_ids:
                        errors.append(
                            f"{path.name}: supersession target ADR-{ref:03d} does not exist"
                        )

        if extract_status(text) == "SUPERSEDED":
            # Prefer explicit Superseded-by; tombstones should have it
            head = "\n".join(text.splitlines()[:40])
            if "Superseded-by" not in head and "Superseded-by" not in text[:800]:
                # Japanese tombstones may only link in status line
                if not ADR_REF_RE.search(head):
                    warnings.append(
                        f"{path.name}: SUPERSEDED without Superseded-by / successor link in header"
                    )

    for w in warnings:
        print(f"warning: {w}")

    if errors:
        for e in errors:
            print(f"error: {e}", file=sys.stderr)
        print(f"{len(errors)} ADR registry error(s)", file=sys.stderr)
        return 1

    print(
        f"ADR registry OK ({len(files)} files, {len(known_ids)} IDs, "
        f"{sum(len(v) for v in tombstones.values())} tombstones)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
