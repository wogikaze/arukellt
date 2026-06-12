---
Status: open
Created: 2026-03-28
Updated: 2026-06-12
ID: 083
Track: mir-opt
Depends on: 080
Orchestration class: implementation-ready
Blocks v4 exit: False
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
1. `passes/loop_unroll.rs`: ループ上限が定数でかつ ≤ 16 の場合に展開
# MIR: "ループ展開 (Loop Unrolling) パス"
---
## Reopened by audit — 2026-06-12

**Reason**: MIR loop-unrolling pass absent from selfhost compiler; FD-01 stale metadata only.

**Classification**: `must-reopen` / `acceptance-not-actually-met` (FD-01 Slice A).

**Violated acceptance**: Original acceptance cites deleted Rust paths or features with no selfhost equivalent.

**Evidence**: `src/compiler/` grep; `crates/` absent; no Audit resolution / Close note after 2026-04-03 FD-01 reopen.

# MIR: ループ展開 (Loop Unrolling) パス

---

## Reopened by audit — 2026-04-03

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/083-mir-loop-unrolling.md` — incorrect directory for an open issue.

## Summary

固定長のループ (コンパイル時定数回数) を本体のコピーに展開するパスを追加する。
展開後に DCE・const_fold が追加削減できるケースが多く、
特に小さい配列の処理 (4〜16要素) で効果大。

## 受け入れ条件

1. `passes/loop_unroll.rs`: ループ上限が定数でかつ ≤ 16 の場合に展開
2. 展開後に `const_fold` → `dce` を自動実行
3. 展開後のコードサイズが元の 8x を超える場合は展開しない (コードサイズ上限)
4. `--opt-level 2` でのみ有効

## 参照

- roadmap-v4.md §5.2

## Docs sync (docs-to-issues audit 2026-06-12)

- [ ] `docs/process/roadmap-v4.md` status updated from「未着手」to reflect MIR pass implementation progress (loop unroll tracked by #083)
