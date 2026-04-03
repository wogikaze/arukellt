# T3: 同一文字列リテラルのデータセグメント共有

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 091
**Depends on**: —
**Track**: backend-opt
**Blocks v4 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/091-t3-string-literal-dedup.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

同じ文字列リテラル (例: `"hello"` が複数箇所に出現) を
同一の passive data segment にまとめ、`array.new_data` でそれを参照する。
roadmap-v4.md §5.3 で明示的に要求されている最適化。

## 受け入れ条件

1. `EmitContext.string_segments: HashMap<String, u32>` (文字列→セグメントインデックス) を持つ
2. 同一文字列の2回目以降は既存セグメントを再利用
3. データセクションの総サイズ削減を確認 (同一文字列が多いプログラム)
4. `--opt-level 1` 以上で有効

## 参照

- roadmap-v4.md §5.3
