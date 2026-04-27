---
Status: done
Created: 2026-03-29
Updated: 2026-04-15
ID: 148
Track: benchmark
Depends on: 140, 141, 142, 143, 145, 146
Orchestration class: implementation-ready
---
# 基盤: benchmark 結果保存・履歴比較・トレンドレポート
**Blocks v1 exit**: no

---

## Implementation — 2026-04-15

Changes to `scripts/util/benchmark_runner.py`:

1. **`"trend"` entry added to `MODE_PRESETS`** — registers `--mode trend` in
   argparse without requiring benchmark-run iteration settings.

2. **`render_trend_report()`** — renders a human-readable time-series table for
   each benchmark (newest-first rows) plus per-metric trend labels
   (`improving` / `stable` / `degrading`) computed oldest-first using the
   existing `_trend_label()` helper.  Includes a moving median summary at the end.

3. **`_run_trend()`** — loads history from `benchmarks/results/` in preference
   order (`full → compare → quick`), calls `render_trend_report()`, and writes
   `benchmarks/results/trend-<target>-latest.json` with `schema_version`,
   `moving_median`, and `bench_trends` fields.

4. **`main()` routing** — `if args.mode == "trend": _run_trend(args); return`
   routes the new mode before `collect_results()` so no benchmarks are run.

History saving (`save_to_history()`), trend computation (`compute_trend_context()`),
and moving-median helpers were already present from prior slices.  This slice adds
the standalone trend inspection surface (`--mode trend`).

`bash scripts/run/verify-harness.sh --quick` exits 0.

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

1. benchmark 結果を JSON で保存し、比較対象を指定できる — **done**: `save_to_history()` saves `bench-{mode}-{target}-{ts}.json` to `benchmarks/results/` on every run; `--no-save-history` skips it.
2. 前回比・baseline 比・移動中央値をレポート出力できる — **done**: `compute_moving_median()` over last N prior runs, printed in Trend Context section of `render_text`.
3. perf gate failure 時に直近の改善/悪化傾向を表示できる — **done**: `compute_trend_context()` labels each benchmark compile/run as improving/stable/degrading; appended to text output when history ≥ 2 runs.
4. `--mode trend` で履歴のみから時系列レポートを表示する — **done**: `render_trend_report()` + `_run_trend()` in `benchmark_runner.py`; `benchmarks/results/trend-<target>-latest.json` written as summary.
5. docs に baseline 更新ルールと trend の読み方を記載する — **done**: existing "Result Storage and History" section in `benchmarks/README.md`.

## 実装タスク

1. result schema と保存場所を決める
2. compare current vs baseline vs previous のレポート生成を実装する
3. quick/full/ci すべてが同じ schema を出力するよう揃える

## 参照

- `docs/process/benchmark-results.md`
- `scripts/compare-benchmarks.sh`
- `scripts/update-perf-baselines.sh`

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/148-bench-result-storage-and-trend-report.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

単発計測だと「先週より良くなったか」が追えない。
ローカル baseline と CI 履歴を共通 schema で保存し、前回比・移動中央値・長期トレンドを Markdown で確認できるようにする。

## 受け入れ条件

1. benchmark 結果を JSON で保存し、比較対象を指定できる — **done**: `save_to_history()` saves `bench-{mode}-{target}-{ts}.json` to `benchmarks/results/` on every run; `--no-save-history` skips it.
2. 前回比・baseline 比・移動中央値をレポート出力できる — **done**: `compute_moving_median()` over last N prior runs, printed in Trend Context section of `render_text`.
3. perf gate failure 時に直近の改善/悪化傾向を表示できる — **done**: `compute_trend_context()` labels each benchmark compile/run as improving/stable/degrading; appended to text output when history ≥ 2 runs.
4. docs に baseline 更新ルールと trend の読み方を記載する — **done**: "Result Storage and History" section added to `benchmarks/README.md`.

## 実装タスク

1. result schema と保存場所を決める
2. compare current vs baseline vs previous のレポート生成を実装する
3. quick/full/ci すべてが同じ schema を出力するよう揃える

## 参照

- `docs/process/benchmark-results.md`
- `scripts/compare-benchmarks.sh`
- `scripts/update-perf-baselines.sh`