# T3: 定数条件 if の emit 時除去

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 093
**Depends on**: —
**Track**: backend-opt
**Blocks v4 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/093-t3-const-if-elimination.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

MIR レベルで `BranchFold` パスが定数条件を除去するが、
T3 emit 時点でも「条件が `i32.const 1` または `i32.const 0` に固定されている `if`」を
対応するブランチの本体のみに置き換えるバックエンド最適化を追加する。
roadmap-v4.md §5.3 で明示的に要求されている最適化。

## 受け入れ条件

1. `emit_if` で条件が定数の場合に `if`/`else`/`end` を省略して直接 emit
2. `--opt-level 1` 以上で有効
3. `wc -c` で対象 fixture のバイナリサイズが削減されることを確認

## 参照

- roadmap-v4.md §5.3
