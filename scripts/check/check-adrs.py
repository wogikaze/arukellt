#!/usr/bin/env python3
"""ADR 台帳の整合性検査（識別子・ステータス・日付・後継関係）。

docs/adr/ADR-*.md に対して ADR-0000 の規則を強制する。
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
    r"(?i)(?:\*\*)?(?:Supersedes|廃止)\s*(?:\*\*)?\s*[:：]\s*(.+)"
)
SUPERSEDED_BY_RE = re.compile(
    r"(?i)(?:\*\*)?(?:Superseded-by|後継)\s*(?:\*\*)?\s*[:：]\s*(.+)"
)
ADR_REF_RE = re.compile(r"ADR-0*(\d+)", re.IGNORECASE)
TOMBSTONE_RE = re.compile(r"^##\s+(?:Tombstone|廃止記録)\s*$", re.MULTILINE)


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
        print("ADR ファイルが見つかりません", file=sys.stderr)
        return 1

    by_id: dict[int, list[Path]] = defaultdict(list)
    bodies: dict[int, list[Path]] = defaultdict(list)
    tombstones: dict[int, list[Path]] = defaultdict(list)
    id_by_path: dict[Path, int] = {}

    for path in files:
        m = FILENAME_RE_FLEX.match(path.name)
        if not m:
            errors.append(f"{path.name}: ファイル名が ADR-<番号> で始まっていません")
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
            errors.append(f"{path.name}: ステータスヘッダがありません")
        elif status in FORBIDDEN_ALIASES:
            errors.append(
                f"{path.name}: 禁止されたステータス別名 {status} "
                f"（使うべき値: {', '.join(sorted(ALLOWED_STATUSES))}）"
            )
        elif status not in ALLOWED_STATUSES:
            errors.append(
                f"{path.name}: 未知のステータス {status!r}; "
                f"許可: {', '.join(sorted(ALLOWED_STATUSES))}"
            )

        for ym, mo, da in DATE_RE.findall(text):
            try:
                d = date(int(ym), int(mo), int(da))
            except ValueError:
                errors.append(f"{path.name}: 不正な日付 {ym}-{mo}-{da}")
                continue
            if d > today:
                errors.append(
                    f"{path.name}: 未来日付 {d.isoformat()}（今日={today}）"
                )

        if status == "ACCEPTED" and re.search(r"^- \[ \]", text, re.MULTILINE):
            warnings.append(
                f"{path.name}: ACCEPTED なのに未完了チェックボックスがあります"
            )

        for label, cre in (("廃止", SUPERSEDES_RE), ("後継", SUPERSEDED_BY_RE)):
            for line in text.splitlines()[:30]:
                m = cre.match(line.strip())
                if not m:
                    continue
                refs = extract_adr_refs(m.group(1))
                if not refs and "none" not in m.group(1).lower() and "なし" not in m.group(1):
                    if not re.search(r"ADR-\d+", m.group(1), re.I):
                        warnings.append(
                            f"{path.name}: {label} 行に ADR 番号がありません: {m.group(1)[:80]}"
                        )

    for adr_id, paths in sorted(bodies.items()):
        if len(paths) > 1:
            names = ", ".join(p.name for p in paths)
            errors.append(
                f"ADR-{adr_id:03d}: 廃止記録以外の本文が複数あります: {names}"
            )

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
                            f"{path.name}: 後継/廃止先 ADR-{ref:03d} が存在しません"
                        )

        if extract_status(text) == "SUPERSEDED":
            head = "\n".join(text.splitlines()[:40])
            if (
                "Superseded-by" not in head
                and "後継" not in head
                and not ADR_REF_RE.search(head)
            ):
                warnings.append(
                    f"{path.name}: SUPERSEDED なのにヘッダに後継リンクがありません"
                )

    for w in warnings:
        print(f"警告: {w}")

    if errors:
        for e in errors:
            print(f"エラー: {e}", file=sys.stderr)
        print(f"ADR 台帳エラー {len(errors)} 件", file=sys.stderr)
        return 1

    print(
        f"ADR 台帳 OK（{len(files)} ファイル、{len(known_ids)} ID、"
        f"廃止記録 {sum(len(v) for v in tombstones.values())}）"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
