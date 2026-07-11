#!/usr/bin/env python3
"""ADR 台帳の整合性検査（識別子・ステータス・日付・後継関係）。

docs/adr/ADR-*.md に対して ADR-000 の規則を強制する。
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

# 番号はちょうど 3 桁。接尾辞は小文字 kebab（レガシー大文字接尾辞は警告）
FILENAME_RE = re.compile(r"^ADR-(\d{3})(?:-([a-z0-9]+(?:-[a-z0-9]+)*))?-$")
FILENAME_RE_FLEX = re.compile(r"^ADR-(\d{3})(?:-([A-Za-z0-9]+(?:-[A-Za-z0-9]+)*))?-")
FILENAME_OVERPAD_RE = re.compile(r"^ADR-\d{4,}")

STATUS_LINE_RE = re.compile(
    r"(?:ステータス|\*\*Status\*\*|Status)\s*[:：]\s*\*?\*?([A-Za-z]+)\*?\*?"
    r"(?:\s*[—–-]\s*(.+))?",
    re.IGNORECASE,
)
DATE_RE = re.compile(r"\b(20\d{2})-(\d{2})-(\d{2})\b")
HTML_COMMENT_RE = re.compile(r"<!--.*?-->", re.DOTALL)
SUPERSEDES_RE = re.compile(
    r"(?i)(?:\*\*)?(?:Supersedes|廃止)\s*(?:\*\*)?\s*[:：]\s*(.+)"
)
SUPERSEDED_BY_RE = re.compile(
    r"(?i)(?:\*\*)?(?:Superseded-by|後継)\s*(?:\*\*)?\s*[:：]\s*(.+)"
)
ADR_REF_RE = re.compile(r"ADR-0*(\d+)", re.IGNORECASE)
TOMBSTONE_RE = re.compile(r"^##\s+(?:Tombstone|廃止記録)\s*$", re.MULTILINE)
DECISION_HEADING_RE = re.compile(
    r"^##\s+(決定事項|決定)\s*$", re.MULTILINE
)
PROPOSED_DECISION_HEADING_RE = re.compile(
    r"^##\s+提案する決定\s*$", re.MULTILINE
)


def is_tombstone(text: str) -> bool:
    return bool(TOMBSTONE_RE.search(text))


def extract_status_and_summary(text: str) -> tuple[str | None, str | None]:
    for line in text.splitlines()[:20]:
        m = STATUS_LINE_RE.search(line)
        if m:
            status = m.group(1).upper()
            summary = (m.group(2) or "").strip() or None
            return status, summary
    return None, None


def extract_status(text: str) -> str | None:
    return extract_status_and_summary(text)[0]


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
    status_by_id: dict[int, str] = {}
    id_by_path: dict[Path, int] = {}

    for path in files:
        if FILENAME_OVERPAD_RE.match(path.name):
            errors.append(
                f"{path.name}: 番号はちょうど 3 桁にしてください（例: ADR-001、ADR-0001 は不可）"
            )
            continue
        m = FILENAME_RE_FLEX.match(path.name)
        if not m:
            errors.append(
                f"{path.name}: ファイル名が ADR-NNN-... 形式ではありません"
            )
            continue
        adr_id = int(m.group(1))
        suffix = m.group(2)
        if suffix and not re.fullmatch(
            r"[a-z0-9]+(?:-[a-z0-9]+)*", suffix
        ):
            warnings.append(
                f"{path.name}: ファイル名接尾辞は小文字 kebab-case を推奨"
                f"（レガシー接尾辞: {suffix}）"
            )
        by_id[adr_id].append(path)
        id_by_path[path] = adr_id
        text = path.read_text(encoding="utf-8")
        if is_tombstone(text):
            tombstones[adr_id].append(path)
        else:
            bodies[adr_id].append(path)

        status, summary = extract_status_and_summary(text)
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
        else:
            status_by_id[adr_id] = status
            if not is_tombstone(text) and not summary:
                errors.append(
                    f"{path.name}: ステータス行に一行要約がありません"
                    f"（形式: ステータス: **{status}** — <要約>）"
                )

        if status == "PROPOSED" and DECISION_HEADING_RE.search(text):
            if not PROPOSED_DECISION_HEADING_RE.search(text):
                errors.append(
                    f"{path.name}: PROPOSED なのに `## 決定` / `## 決定事項` がある"
                    f"（`## 提案する決定` を使う）"
                )

        if status == "ACCEPTED" and "CURRENT_STATE_TARGET_SUMMARY_SOURCE" in text:
            errors.append(
                f"{path.name}: ACCEPTED ADR に current-state 生成ソースを置かない"
                f"（docs/data/target-contract-summary.md へ）"
            )

        date_text = HTML_COMMENT_RE.sub("", text)
        for ym, mo, da in DATE_RE.findall(date_text):
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
        status = extract_status(text)
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

        if status == "SUPERSEDED":
            head = "\n".join(text.splitlines()[:40])
            succ_refs: list[int] = []
            for line in text.splitlines()[:40]:
                m = SUPERSEDED_BY_RE.match(line.strip())
                if m:
                    succ_refs.extend(extract_adr_refs(m.group(1)))
            if not succ_refs and not ADR_REF_RE.search(head):
                warnings.append(
                    f"{path.name}: SUPERSEDED なのにヘッダに後継リンクがありません"
                )
            for ref in succ_refs:
                succ_status = status_by_id.get(ref)
                if succ_status and succ_status != "ACCEPTED":
                    errors.append(
                        f"{path.name}: SUPERSEDED の後継 ADR-{ref:03d} は"
                        f" ACCEPTED である必要がある（現在: {succ_status}）"
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
