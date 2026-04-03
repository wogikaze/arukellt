# T3: enum dispatch の br_on_cast 連鎖最適化

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 094
**Depends on**: —
**Track**: backend-opt
**Blocks v4 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/094-t3-br-on-cast-chain-opt.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

Arukellt の enum パターンマッチは T3 で `br_on_cast` / `br_on_cast_fail` の連鎖として emit されるが、
バリアントの出現頻度に基づいて最も高頻度のバリアントを先に試みる順序に並び替える
（プロファイルがない場合は、タグ番号の小さい順で最適化）。

また、連続する `br_on_cast` の対象型が完全に非交差の場合、
`br_table` による O(1) ディスパッチに変換できないかを検討する。

## 受け入れ条件

1. enum ディスパッチの `br_on_cast` 連鎖が3個以上の場合に最適化対象
2. `br_table` への変換: i31 タグを使った O(1) ディスパッチ実装
3. パターンマッチを多用するベンチマークで実行時間改善を確認

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/MVP.md` §br_on_cast
- `docs/spec/spec-3.0.0/OVERVIEW.md` §GC詳細
