# ベンチスイート: workload taxonomy と機能マトリクス整備

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 147
**Depends on**: 149
**Track**: benchmark
**Blocks v1 exit**: no

## Summary

fib / vec / string だけだと言語処理系の得手不得手を取りこぼす。
CPU-bound / allocation-heavy / recursion / dispatch / parsing / IO / host call / higher-order / error path などの workload taxonomy を定義し、
各 benchmark が何を測っているのかをマトリクス化する。

## 受け入れ条件

1. benchmark ごとに primary/secondary metric と workload tag が付与される
2. 未カバー領域を一覧化できる
3. `benchmarks/README.md` に taxonomy 表がある
4. compare/gate 実行時に tag 単位の集計を出せる下地を作る

## 実装タスク

1. benchmark taxonomy と tag set を決める
2. 既存 benchmark をタグ付けする
3. 不足 workload に対応する追加 fixture 候補を issue 化する

## 参照

- `issues/done/109-bench-suite.md`
- `benchmarks/README.md`
- `docs/process/benchmark-plan.md`
