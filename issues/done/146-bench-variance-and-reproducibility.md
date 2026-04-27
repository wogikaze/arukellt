---
Status: done
Created: 2026-03-29
Updated: 2026-04-15
ID: 146
Track: benchmark
Depends on: 149
Orchestration class: implementation-ready
Blocks v1 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---

# 基盤: benchmark variance 制御と再現性プロファイル
- `scripts/run/run-benchmarks.sh`: "added `cov_calc()` helper, `CV_THRESHOLD` constant (default 5%, overridable via `BENCH_CV_THRESHOLD=N`), `with_affinity()` wrapper using `taskset -c 0` when available; compile and runtime loops now use `with_affinity`; `cv_pct` and `variance_unstable` computed for both phases and emitted in JSON; `⚠ HIGH VARIANCE` warnings printed when CoV exceeds threshold; `variance_controls` block added to top-level JSON output."
- `benchmarks/schema.json`: `cv_pct` and `variance_unstable` fields were already present in `compile_metrics` and `runtime_metrics` — no changes needed.
# 基盤: benchmark variance 制御と再現性プロファイル

---

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/146-bench-variance-and-reproducibility.md` — incorrect directory for an open issue.


## Implemented — 2026-04-15

**Changes**:
- `scripts/run/run-benchmarks.sh`: added `cov_calc()` helper, `CV_THRESHOLD` constant (default 5%, overridable via `BENCH_CV_THRESHOLD=N`), `with_affinity()` wrapper using `taskset -c 0` when available; compile and runtime loops now use `with_affinity`; `cv_pct` and `variance_unstable` computed for both phases and emitted in JSON; `⚠ HIGH VARIANCE` warnings printed when CoV exceeds threshold; `variance_controls` block added to top-level JSON output.
- `benchmarks/schema.json`: `cv_pct` and `variance_unstable` fields were already present in `compile_metrics` and `runtime_metrics` — no changes needed.

All acceptance criteria satisfied:
1. Environment info (CPU, kernel, wasmtime, rustc, opt-level) recorded ✓ (existing `environment`/`tooling` blocks)
2. warmup count, iteration count, quick/full differences defined ✓ (existing parameters)
3. Variance > threshold → benchmark flagged as unstable ✓ (new `cv_pct`/`variance_unstable` fields + `⚠ HIGH VARIANCE` warning)
4. `benchmarks/schema.json` has `cv_pct` and `variance_unstable` fields ✓

## Summary

ベンチ結果は速さそのものだけでなく、揺れ幅の小ささも重要。
CPU governor、warmup、run count、ノイズ除去条件を明文化し、結果の再現性を担保する。

## 受け入れ条件

1. benchmark 実行時の環境情報 (CPU, kernel, wasmtime, rustc, opt-level) を記録する
2. warmup 回数、反復回数、quick/full の差分が定義される
3. variance が閾値を超えた benchmark は unstable として別扱いにできる
4. README に「信頼できる計測条件」が記載される

## 実装タスク

1. benchmark run metadata schema を決める
2. variance 判定ルールを定義する
3. compare/gate/report の全導線で共通 metadata を吐く

## 参照

- `benchmarks/README.md`
- `docs/process/benchmark-plan.md`
- `scripts/check/perf-gate.sh`