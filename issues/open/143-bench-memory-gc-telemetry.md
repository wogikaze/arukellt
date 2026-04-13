# 計測: allocation / live-set / GC pause / RSS telemetry

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-04-13
**ID**: 143
**Depends on**: 149
**Track**: benchmark
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: Benchmark runner focuses on timing/size/RSS. gc_pause and live_set fields not in schema or runner.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

メモリはピーク値だけでは不十分で、allocation rate・live-set・GC pause が見えないと回帰の原因を追えない。
コンパイラ自身の RSS と、生成 Wasm の実行時ヒープ挙動を同じ結果形式で収集できるようにする。

## 受け入れ条件

1. compiler RSS peak と benchmark ごとの allocation/live-set 指標を取れる
2. GC pause の total/max と回数を取得できる場合は記録し、不可の場合は明示的に unavailable とする
3. `benchmarks/` の少なくとも allocation-heavy / recursion-heavy / string-heavy ケースで比較できる
4. 結果を perf gate や compare レポートに流用できる JSON 形式で保存する

## 実装タスク

1. wasmtime や runtime から取得可能なメモリ/G C 指標を棚卸しする
2. fallback ありの telemetry schema を決める
3. 既存の memory profile 計測と重複しないよう統合ポイントを決める

## 参照

- `issues/done/113-bench-memory-profile.md`
- `docs/process/benchmark-plan.md`
- `benchmarks/README.md`
