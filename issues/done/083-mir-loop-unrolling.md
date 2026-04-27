---
Status: done
Created: 2026-03-28
Updated: 2026-04-03
ID: 083
Track: mir-opt
Depends on: 080
Orchestration class: implementation-ready
---
# MIR: ループ展開 (Loop Unrolling) パス
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/083-mir-loop-unrolling.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

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