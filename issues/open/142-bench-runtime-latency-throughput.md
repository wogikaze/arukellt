# 計測: startup / throughput / tail latency ベンチ

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-04-03
**ID**: 142
**Depends on**: 149
**Track**: benchmark
**Blocks v1 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/142-bench-runtime-latency-throughput.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

実行時間は平均値だけでは足りない。
Wasm の instantiate/startup コスト、steady-state throughput、p50/p95/p99 の tail latency を分けて取り、
CLI 的な短命 workload と長く回る workload を別々に評価できるようにする。

## 受け入れ条件

1. startup latency と guest 実行時間を別々に測定する
2. throughput 系 benchmark は 1 回実行ではなく一定反復で計測する
3. p50/p95/p99 と標準偏差または MAD を出力する
4. quick/full で実行回数と fixture 数を切り替えられる

## 実装タスク

1. 短命 workload と定常 workload の fixture 分類を作る
2. hyperfine または同等手段で percentile を取れる runner を整える
3. perf gate で平均値回帰だけでなく tail latency 回帰も扱えるようにする

## 参照

- `benchmarks/run_benchmarks.sh`
- `docs/process/benchmark-plan.md`
- `issues/done/109-bench-suite.md`
