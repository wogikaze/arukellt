# 計測: 入力サイズ sweep とスケーリングカーブ可視化

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-04-03
**ID**: 144
**Depends on**: 141, 142, 143, 149
**Track**: benchmark
**Blocks v1 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/144-bench-scaling-curves.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

1 点計測だけでは「あるサイズから急に遅くなる」崖を見逃す。
入力サイズを sweep して compile/runtime/memory の伸び方を取り、複雑度の崩れや GC cliff を早期に検知する。

## 受け入れ条件

1. 同一 benchmark を複数入力サイズで自動実行できる
2. compile/runtime/memory の size-to-cost テーブルを出力する
3. 直近のサイズ帯に対する増分比率を出し、異常な cliff を警告できる
4. quick は 3 点、full は 5 点以上のサイズで計測する

## 実装タスク

1. 各 benchmark に size parameter を与える方法を決める
2. 結果形式に x 軸サイズ情報を追加する
3. compare レポートで slope/ratio を見やすく表示する

## 参照

- `docs/process/benchmark-plan.md`
- `benchmarks/README.md`
