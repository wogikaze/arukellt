#!/usr/bin/env python3
"""Generate docs/process/issue-dispatch-flows.md from open issue index."""

from __future__ import annotations

import json
import re
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).parent.parent.parent
OPEN_DIR = ROOT / "issues/open"
META = OPEN_DIR / "index-meta.json"
OUT = ROOT / "docs/process/issue-dispatch-flows.md"

MULTI = [
    ("1", "Playground", "playground", [(645, "T2 実行 + DOM I/O"), (491, "CI パフォーマンス予算")]),
    ("2", "LSP テスト硬化", "lsp/testing", [(454, "regression fixtures"), (463, "performance smoke"), (355, "protocol E2E")]),
    ("3", "VSCode プロジェクト", "vscode-ide", [(441, "ark.toml"), (502, "multi-root")]),
    ("4", "VSCode セマンティック UX", "vscode-ide", [(439, "stdlib nav"), (440, "fix-all")]),
    ("5", "VSCode 基盤→Epic", "vscode-ide / runtime", [(191, "setup doctor"), (184, "extension foundation"), (183, "all-in-one epic"), (638, "Wasm debug")]),
    ("6", "MIR 最適化", "mir-opt", [(80, "LICM"), (83, "loop unrolling"), (650, "T3 gated O2/O3 passes")]),
    ("7", "std::host 実装", "runtime/stdlib", [(445, "process"), (446, "http"), (447, "sockets"), (633, "capability honesty")]),
    ("8", "Selfhost フロント", "selfhost-frontend", [(566, "partial AST"), (636, "structured diag")]),
    ("9", "Wasm 品質", "wasm-quality", [(117, "WIT quality"), (118, "multi-export world")]),
    ("10", "WASI P2 ハブ", "wasi-feature", [(510, "P2 import switch"), (74, "native component (#074)"), (76, "filesystem (#076)"), (637, "fs metadata")]),
    ("11", "Extension docs", "docs", [(480, "extension README / settings")]),
    (
        "12",
        "Component compose",
        "wasm-feature",
        [(74, "native component (#074)"), (476, "wasm-tools compose"), (443, "composition linking"), (618, "WIT bindings round-trip"), (648, "general canonical ABI (#648)"), (659, "f32 general adapters"), (660, "Tier2 general adapters")],
    ),
    ("13", "WIT / CLI", "component-model", [(74, "native component (#074)"), (124, "WIT import syntax (#124)"), (652, "WIT import parser"), (653, "WIT import resolver+MIR"), (654, "WIT import component emit"), (473, "WIT resource handles"), (651, "WIT flags type"), (30, "jco interop"), (28, "wit-cli (#034)")]),
    (
        "14",
        "std::host P2 rollout",
        "wasi-feature",
        [(74, "native component (#074)"), (139, "sockets P2 facade (#139)"), (657, "sockets connect+read"), (658, "sockets listen+accept"), (63, "http P2 facade (#077)"), (655, "HTTP outgoing client"), (656, "HTTP incoming server"), (138, "shared capabilities T1/T3"), (136, "host layer rollout")],
    ),
    ("15", "Async / P3", "wasm-feature", [(74, "native component (#074)"), (474, "async component (v5)"), (646, "T5 wasi-p3 scaffold"), (649, "T4 native full lowering")]),
]

BLOCKED = [
    ("16", "stdlib blocked", "stdlib", 41, "std-time-random (#051 umbrella)"),
    ("16", "stdlib blocked", "stdlib", 661, "clock/random intrinsics emitter"),
    ("16", "stdlib blocked", "stdlib", 662, "time duration typecheck"),
    ("16", "stdlib blocked", "stdlib", 44, "std-wit-component"),
    ("16", "stdlib blocked", "stdlib", 45, "std-json-toml-csv"),
]

HYGIENE: list[tuple[str, str, str, int, str]] = [
    ("17", "Selfhost hygiene", "selfhost-retirement", 647, "remove mir-select legacy docs"),
]
LANE_ISSUE_KEYS: set[str] = set()


def norm_key(token: str) -> str:
    token = token.strip().lstrip("#")
    m = re.match(r"^(\d+)", token)
    return str(int(m.group(1))) if m else token


def load_from_meta() -> tuple[dict, dict[str, dict]]:
    meta = json.loads(META.read_text(encoding="utf-8"))
    open_by_key: dict[str, dict] = {}
    for row in meta["open_issues"]:
        iid = row["id"]
        prefix = row["path"].split("-", 1)[0]
        for key in {iid, prefix, norm_key(iid), norm_key(prefix)}:
            open_by_key[key] = row
    return meta, open_by_key


def is_open(key: str, open_by_key: dict[str, dict]) -> bool:
    return key in open_by_key


def unresolved_deps(row: dict, open_by_key: dict[str, dict]) -> list[str]:
    waiting = []
    for dep in row.get("depends_on") or []:
        key = norm_key(dep)
        if key in open_by_key:
            waiting.append(open_by_key[key]["id"])
    return waiting


def load_priority() -> dict[str, int]:
    prio: dict[str, int] = {}
    for line in (OPEN_DIR / "priority-table.md").read_text(encoding="utf-8").splitlines():
        m = re.match(r"^\| (\d+) \| (\d+) \|", line)
        if m:
            prio[norm_key(m.group(2))] = int(m.group(1))
    return prio


def resolve_lane_id(iid: int) -> str:
    return norm_key(str(iid))


def register_lane_id(iid: int) -> None:
    LANE_ISSUE_KEYS.add(resolve_lane_id(iid))


def dispatch_cell(
    lane_iid: int, open_by_key: dict[str, dict], prio: dict[str, int]
) -> tuple[str, str, str]:
    key = resolve_lane_id(lane_iid)
    register_lane_id(lane_iid)
    if not is_open(key, open_by_key):
        return "done", "—", "closed"
    row = open_by_key[key]
    orch = (row.get("orchestration") or {}).get("class") or ""
    pr = f"P{prio[key]}" if key in prio else "—"
    if orch == "done":
        return "hygiene", pr, "orchestration done"
    waiting = unresolved_deps(row, open_by_key)
    if waiting:
        return "blocked", pr, f"wait {', '.join('#' + d for d in waiting)}"
    if orch in ("implementation-ready", "design-ready"):
        return "ready", pr, orch.replace("-ready", "")
    if orch in ("blocked-by-upstream", "partially-blocked"):
        return "blocked-upstream", pr, orch
    return orch or "—", pr, orch or "—"


def lane_headline(
    steps: list[tuple[int, str]], open_by_key: dict[str, dict], prio: dict[str, int]
) -> str:
    if all(dispatch_cell(iid, open_by_key, prio)[0] == "done" for iid, _ in steps):
        return "完了"
    for i, (iid, _) in enumerate(steps):
        st, _, _ = dispatch_cell(iid, open_by_key, prio)
        if st == "ready":
            return f"{chr(65 + i)} 着手可"
    return "blocked"


def summary_counts() -> tuple[str, str]:
    summary = (OPEN_DIR / "index.md").read_text(encoding="utf-8").split("## Summary", 1)[1].split("##", 1)[0]
    open_m = re.search(r"Total open issues: (\d+)", summary)
    done_m = re.search(r"Done issues: (\d+)", summary)
    return open_m.group(1) if open_m else "?", done_m.group(1) if done_m else "?"


def format_issue_ref(iid: int) -> str:
    return f"#{iid:03d}"


def assert_full_coverage(meta: dict, open_by_key: dict[str, dict]) -> None:
    open_ids = {row["id"] for row in meta["open_issues"]}
    covered = {open_by_key[k]["id"] for k in LANE_ISSUE_KEYS if k in open_by_key}
    missing = open_ids - covered
    if missing:
        raise SystemExit(f"dispatch flow coverage gap: {sorted(missing)}")


def main() -> None:
    if not META.exists():
        raise SystemExit(f"Missing {META}; run generate-issue-index.py first")

    meta, open_by_key = load_from_meta()
    prio = load_priority()
    open_n, done_n = summary_counts()
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    lines = [
        "# Issue dispatch flows (independent lanes)",
        "",
        f"> Generated by `scripts/gen/generate-dispatch-flows.py` at **{ts}**.",
        f"> Queue: **{open_n}** open / **{done_n}** done.",
        "",
        "各レーンは **他レーンと依存しない**。レーン内は A→B→C の順で進める。",
        "",
        "## Multi-step lanes",
        "",
        "| Lane | Track | 現在地 | Step | ID | Priority | Dispatch | 概要 |",
        "|------|-------|--------|------|-----|----------|----------|------|",
    ]

    for lane_id, name, track, steps in MULTI:
        headline = lane_headline(steps, open_by_key, prio)
        for i, (iid, label) in enumerate(steps):
            st, pr, _ = dispatch_cell(iid, open_by_key, prio)
            lines.append(
                f"| {lane_id} {name} | {track} | {headline} | {chr(65 + i)} | {format_issue_ref(iid)} | {pr} | {st} | {label} |"
            )

    lines.extend(
        [
            "",
            "## Blocked lanes (upstream 待ち)",
            "",
            "| Lane | Track | ID | Priority | Dispatch | 概要 |",
            "|------|-------|-----|----------|----------|------|",
        ]
    )
    for lane_id, name, track, iid, summary in BLOCKED:
        key = resolve_lane_id(iid)
        if not is_open(key, open_by_key):
            continue
        st, pr, _ = dispatch_cell(iid, open_by_key, prio)
        lines.append(f"| {lane_id} {name} | {track} | {format_issue_ref(iid)} | {pr} | {st} | {summary} |")

    for lane_id, name, track, iid, summary in HYGIENE:
        key = resolve_lane_id(iid)
        register_lane_id(iid)
        if not is_open(key, open_by_key):
            continue
        st, pr, _ = dispatch_cell(iid, open_by_key, prio)
        lines.append(f"| {lane_id} {name} | {track} | {format_issue_ref(iid)} | {pr} | {st} | {summary} |")

    lines.extend(
        [
            "",
            "## 並行ディスパッチ例",
            "",
            "| Agent | Lane | 次の step |",
            "|-------|------|-----------|",
            "| 1 | 12 Component | D #618 (ready) |",
            "| 2 | 14 std::host P2 | B #139 / C #063 (ready) |",
            "| 3 | 12 Component | C #443 (orch 要確認) |",
            "| 4 | 15 Async / P3 | B #474 |",
            "",
            "## Regenerate",
            "",
            "```bash",
            "python3 scripts/gen/generate-issue-index.py",
            "python3 scripts/gen/generate-dispatch-flows.py",
            "```",
            "",
        ]
    )

    assert_full_coverage(meta, open_by_key)
    OUT.write_text("\n".join(lines), encoding="utf-8")
    print(f"Wrote {OUT.relative_to(ROOT)} ({open_n} open / {done_n} done)")


if __name__ == "__main__":
    main()
