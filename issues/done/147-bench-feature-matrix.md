# ベンチスイート: workload taxonomy と機能マトリクス整備

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-15
**ID**: 147
**Depends on**: 149
**Track**: benchmark
**Blocks v1 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/147-bench-feature-matrix.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

fib / vec / string だけだと言語処理系の得手不得手を取りこぼす。
CPU-bound / allocation-heavy / recursion / dispatch / parsing / IO / host call / higher-order / error path などの workload taxonomy を定義し、
各 benchmark が何を測っているのかをマトリクス化する。

## 受け入れ条件

- [x] benchmark ごとに primary/secondary metric と workload tag が付与される
- [x] 未カバー領域を一覧化できる
- [x] `benchmarks/workload-taxonomy.md` に feature coverage matrix 表がある (STOP_IF: README より taxonomy file を拡張)
- [x] `bash scripts/run/verify-harness.sh --quick` passes

## 実装タスク

1. benchmark taxonomy と tag set を決める
2. 既存 benchmark をタグ付けする
3. 不足 workload に対応する追加 fixture 候補を issue 化する

## 参照

- `issues/done/109-bench-suite.md`
- `benchmarks/README.md`
- `docs/process/benchmark-plan.md`
