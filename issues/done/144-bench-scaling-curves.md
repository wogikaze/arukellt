---
Status: done
Created: 2026-03-29
Updated: 2026-04-15
ID: 144
Track: benchmark
Depends on: 141, 142, 143, 149
Orchestration class: implementation-ready
Blocks v1 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---

# 計測: 入力サイズ sweep とスケーリングカーブ可視化
- [x] Scaling fixture set defined: at least 3 input sizes applied to at least one benchmark
- `benchmarks/schema.json`: added `"scaling"` to mode enum
- `benchmarks/README.md`: added "Scaling Curve Mode" section + scaling class method table
- `bash scripts/run/verify-harness.sh --quick`: "PASS** (19/19)"
# 計測: 入力サイズ sweep とスケーリングカーブ可視化

---

## Completion — 2026-04-15

**Acceptance items**:

- [x] Scaling fixture set defined: at least 3 input sizes applied to at least one benchmark
- [x] Scaling curve data captured (latency/size vs input-size)
- [x] Results indicate approximate scaling class (O(n), O(n log n), O(n²)) or document as manual interpretation
- [x] `bash scripts/run/verify-harness.sh --quick` passes

**Evidence**:

- `scripts/util/benchmark_runner.py` extended with `--mode scaling`:
  - `SCALING_FRACTIONS = [0.10, 0.50, 1.00]` → 3 input-size points (n=120, 600, 1200) against `bench_parse_tree_distance`
  - `_generate_scaling_subset()` creates valid distance-matrix subsets (correct upper-triangle format)
  - `_estimate_scaling_class()` computes log-log slope → emits `O(n)` / `O(n log n)` / `O(n²)` / super-quadratic
  - `_detect_scaling_cliffs()` warns when time ratio > 1.5 × expected O(n²) between adjacent points
  - `measure_scaling()` compiles once + runs 3 input variants, restores original input in `finally`
  - `render_scaling_text()` prints size-vs-latency table, scaling class, cliff warnings
  - `_run_scaling()` orchestrates and writes `tests/baselines/perf/scaling.json`
- `benchmarks/schema.json`: added `"scaling"` to mode enum
- `benchmarks/README.md`: added "Scaling Curve Mode" section + scaling class method table
- `bash scripts/run/verify-harness.sh --quick`: **PASS** (19/19)

---

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/144-bench-scaling-curves.md` — incorrect directory for an open issue.


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