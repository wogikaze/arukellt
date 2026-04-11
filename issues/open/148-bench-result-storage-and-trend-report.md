# 基盤: benchmark 結果保存・履歴比較・トレンドレポート

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-04-03
**ID**: 148
**Depends on**: 140, 141, 142, 143, 145, 146
**Track**: benchmark
**Blocks v1 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/148-bench-result-storage-and-trend-report.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

単発計測だと「先週より良くなったか」が追えない。
ローカル baseline と CI 履歴を共通 schema で保存し、前回比・移動中央値・長期トレンドを Markdown で確認できるようにする。

## 受け入れ条件

1. benchmark 結果を JSON で保存し、比較対象を指定できる
2. 前回比・baseline 比・移動中央値をレポート出力できる
3. perf gate failure 時に直近の改善/悪化傾向を表示できる
4. docs に baseline 更新ルールと trend の読み方を記載する

## 実装タスク

1. result schema と保存場所を決める
2. compare current vs baseline vs previous のレポート生成を実装する
3. quick/full/ci すべてが同じ schema を出力するよう揃える

## 参照

- `docs/process/benchmark-results.md`
- `scripts/compare-benchmarks.sh`
- `scripts/update-perf-baselines.sh`
