# 計測: Wasm サイズ内訳 diff と top contributors 追跡

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-04-03
**ID**: 145
**Depends on**: 149
**Track**: benchmark
**Blocks v1 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/145-bench-size-attribution-and-diff.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

総バイナリサイズだけでは、どの section / function / symbol が増えたか分からない。
差分計測を導入し、前回 baseline 比でどこが膨らんだかを一発で特定できるようにする。

## 受け入れ条件

1. section/type/code/data/import/export ごとの差分を表示できる
2. top 増加関数または symbol を列挙できる
3. `hello` のような極小 benchmark と `binary_tree` のような中規模 benchmark の両方に適用できる
4. compare レポートからサイズ悪化の主因へ辿れる

## 実装タスク

1. 既存の wasm size analysis を差分比較対応に拡張する
2. baseline/current の attribution schema を決める
3. perf gate failure 時に diff サマリを出せるようにする

## 参照

- `issues/done/111-bench-wasm-size-analysis.md`
- `docs/process/wasm-size-reduction.md`
- `docs/process/benchmark-results.md`
