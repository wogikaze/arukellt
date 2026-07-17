#!/usr/bin/env python3
"""Fail on new semantic-debt workarounds unless allowlisted with an issue."""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from datetime import date
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 in the bootstrap environment
    import tomli as tomllib


ROOT = Path(__file__).resolve().parents[2]
ALLOWLIST_PATH = ROOT / "docs/data/semantic-debt-allowlist.toml"
SCAN_ROOTS = (ROOT / "src/compiler", ROOT / "std")
SKIP_DIR_NAMES = {".git", "target", "node_modules", "test_tmp"}

# Packed i32 pair into i64 — not nanosecond→millisecond time math (those use 1000000i64).
RULES: dict[str, re.Pattern[str]] = {
    # Assignment of the pack radix only (unpack lines also contain i32_to_i64(1000000)).
    "i32-pair-pack-million": re.compile(r"=\s*i32_to_i64\(\s*1000000\s*\)"),
    "i32-pair-unpack-million": re.compile(
        r"(?:/|%)\s*i32_to_i64\(\s*1000000\s*\)"
    ),
}

WORKAROUND_COMMENT_RE = re.compile(
    r"(?i)\b(?:workaround|encoding\s*hack|packed\s+span|"
    r"とりあえず|応急|暫定hack)\b"
)
STRUCTURED_TODO_RE = re.compile(
    r"\b(?:TODO|FIXME)\(#\d+ owner=[^\s)]+ "
    r"removal=(?:\"[^\"]+\"|'[^']+'|[^\s)]+) recheck=\d{4}-\d{2}-\d{2}\)"
)


@dataclass(frozen=True)
class Finding:
    rule_id: str
    severity: str
    path: str
    line: int
    message: str


@dataclass(frozen=True)
class AllowEntry:
    id: str
    rule: str
    path: str
    issue: int
    removal: str
    recheck: str


def _load_allowlist(path: Path) -> list[AllowEntry]:
    if not path.is_file():
        return []
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    entries: list[AllowEntry] = []
    for raw in data.get("debt") or []:
        if not isinstance(raw, dict):
            raise ValueError(f"{path}: each [[debt]] must be a table")
        entries.append(
            AllowEntry(
                id=str(raw["id"]),
                rule=str(raw["rule"]),
                path=str(raw["path"]),
                issue=int(raw["issue"]),
                removal=str(raw["removal"]),
                recheck=str(raw["recheck"]),
            )
        )
    return entries


def _issue_exists(issue: int) -> bool:
    open_path = ROOT / "issues" / "open"
    done_path = ROOT / "issues" / "done"
    prefix = f"{issue}-"
    for folder in (open_path, done_path):
        if not folder.is_dir():
            continue
        if any(p.name.startswith(prefix) for p in folder.iterdir() if p.is_file()):
            return True
    return False


def _iter_ark_files() -> list[Path]:
    files: list[Path] = []
    for scan_root in SCAN_ROOTS:
        if not scan_root.is_dir():
            continue
        for path in sorted(scan_root.rglob("*.ark")):
            if any(part in SKIP_DIR_NAMES for part in path.parts):
                continue
            files.append(path)
    return files


def _allow_covers(entries: list[AllowEntry], rule: str, rel: str) -> AllowEntry | None:
    for entry in entries:
        if entry.rule == rule and entry.path == rel:
            return entry
    return None


def collect_findings(today: date | None = None) -> list[Finding]:
    today = today or date.today()
    allow = _load_allowlist(ALLOWLIST_PATH)
    findings: list[Finding] = []
    seen_ids: set[str] = set()

    for entry in allow:
        if entry.id in seen_ids:
            findings.append(
                Finding(
                    "allowlist-duplicate-id",
                    "error",
                    ALLOWLIST_PATH.relative_to(ROOT).as_posix(),
                    0,
                    f"duplicate debt id {entry.id!r}",
                )
            )
        seen_ids.add(entry.id)
        if entry.rule not in RULES:
            findings.append(
                Finding(
                    "allowlist-unknown-rule",
                    "error",
                    ALLOWLIST_PATH.relative_to(ROOT).as_posix(),
                    0,
                    f"debt {entry.id!r} references unknown rule {entry.rule!r}",
                )
            )
        if not entry.removal.strip():
            findings.append(
                Finding(
                    "allowlist-missing-removal",
                    "error",
                    ALLOWLIST_PATH.relative_to(ROOT).as_posix(),
                    0,
                    f"debt {entry.id!r} missing removal condition",
                )
            )
        try:
            recheck = date.fromisoformat(entry.recheck)
        except ValueError:
            findings.append(
                Finding(
                    "allowlist-bad-recheck",
                    "error",
                    ALLOWLIST_PATH.relative_to(ROOT).as_posix(),
                    0,
                    f"debt {entry.id!r} has invalid recheck date {entry.recheck!r}",
                )
            )
            recheck = today
        if recheck < today:
            findings.append(
                Finding(
                    "allowlist-recheck-overdue",
                    "error",
                    entry.path,
                    0,
                    f"debt {entry.id!r} recheck={entry.recheck} is overdue; "
                    f"fix root cause (#{entry.issue}) or extend recheck with justification",
                )
            )
        if not _issue_exists(entry.issue):
            findings.append(
                Finding(
                    "allowlist-missing-issue",
                    "error",
                    ALLOWLIST_PATH.relative_to(ROOT).as_posix(),
                    0,
                    f"debt {entry.id!r} references missing issue #{entry.issue}",
                )
            )
        target = ROOT / entry.path
        if not target.is_file():
            findings.append(
                Finding(
                    "allowlist-stale-path",
                    "error",
                    entry.path,
                    0,
                    f"debt {entry.id!r}: path missing; remove allowlist entry after cleanup",
                )
            )
            continue
        text = target.read_text(encoding="utf-8")
        if entry.rule in RULES and not RULES[entry.rule].search(text):
            findings.append(
                Finding(
                    "allowlist-stale-pattern",
                    "error",
                    entry.path,
                    0,
                    f"debt {entry.id!r}: pattern for {entry.rule!r} no longer matches; "
                    "remove the allowlist entry",
                )
            )

    used_allow: set[str] = set()
    for path in _iter_ark_files():
        rel = path.relative_to(ROOT).as_posix()
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue
        for index, line in enumerate(lines, start=1):
            for rule_id, pattern in RULES.items():
                if not pattern.search(line):
                    continue
                cover = _allow_covers(allow, rule_id, rel)
                if cover is None:
                    findings.append(
                        Finding(
                            rule_id,
                            "error",
                            rel,
                            index,
                            "semantic-debt pattern without allowlist entry. "
                            "Fix the owning layer, or add docs/data/semantic-debt-allowlist.toml "
                            "with issue + removal + recheck (see "
                            ".cursor/rules/no-semantic-debt-workarounds.mdc).",
                        )
                    )
                else:
                    used_allow.add(cover.id)

            comment_at = line.find("//")
            if comment_at < 0:
                continue
            comment = line[comment_at:]
            if WORKAROUND_COMMENT_RE.search(comment) and not STRUCTURED_TODO_RE.search(
                comment
            ):
                # Structured marker may be on a nearby line; accept same-line or previous.
                prev = lines[index - 2] if index >= 2 else ""
                if STRUCTURED_TODO_RE.search(prev):
                    continue
                findings.append(
                    Finding(
                        "untracked-workaround-comment",
                        "error",
                        rel,
                        index,
                        "workaround/hack comment requires a structured "
                        "todo with issue, owner, removal, and recheck date "
                        "on this or the previous line",
                    )
                )

    for entry in allow:
        if entry.id not in used_allow and entry.rule in RULES:
            # Path-level allow still "uses" if pattern matched somewhere in file.
            # used_allow is filled per match; stale-pattern already covers no match.
            pass

    return findings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    findings = collect_findings()
    errors = [f for f in findings if f.severity == "error"]
    if args.json:
        print(json.dumps([asdict(f) for f in findings], ensure_ascii=False, indent=2))
    else:
        if not errors:
            print("semantic-debt: ok")
        for finding in errors:
            loc = f"{finding.path}:{finding.line}" if finding.line else finding.path
            print(f"{loc}: {finding.rule_id}: {finding.message}", file=sys.stderr)
        if errors:
            print(
                f"semantic-debt: {len(errors)} error(s)",
                file=sys.stderr,
            )
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
