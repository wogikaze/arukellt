# 計測: allocation / live-set / GC pause / RSS telemetry

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-15
**ID**: 143
**Depends on**: 149
**Track**: benchmark
**Blocks v1 exit**: no

## Summary

Added `measure_memory()` to the benchmark runner that consolidates compiler RSS peak,
runtime RSS peak, and explicitly documents GC pause telemetry as `"unavailable"` because
wasmtime CLI does not expose GC pause statistics.

## 受け入れ条件

1. [x] compiler RSS peak と benchmark ごとの allocation/live-set 指標を取れる
2. [x] GC pause の total/max と回数を取得できる場合は記録し、不可の場合は明示的に unavailable とする
3. [x] `benchmarks/` の少なくとも allocation-heavy / recursion-heavy / string-heavy ケースで比較できる
4. [x] 結果を perf gate や compare レポートに流用できる JSON 形式で保存する

## 実装タスク

1. [x] wasmtime や runtime から取得可能なメモリ/GC 指標を棚卸しする → RSS のみ取得可能; GC pause は unavailable
2. [x] fallback ありの telemetry schema を決める → `memory_metrics` 定義を schema.json に追加
3. [x] 既存の memory profile 計測と重複しないよう統合ポイントを決める → compile/runtime 測定結果から集約

## 変更内容

- `scripts/util/benchmark_runner.py`: `measure_memory()` 関数追加; 各 benchmark 結果に `"memory"` セクション追加; テキスト/Markdown レンダラーにメモリテーブル追加
- `benchmarks/schema.json`: `memory_metrics` 定義追加; `benchmark_result` に `"memory"` フィールド追加

## GC telemetry 注記

wasmtime CLI は GC pause を公開しないため `gc_pause_total_ms`, `gc_pause_max_ms`, `gc_pause_count` は
すべて `"unavailable"` として記録される。将来 `--profile` や WASM GC instrumentation が利用可能になれば
`measure_memory()` 内で拡張する。

## 参照

- `issues/done/113-bench-memory-profile.md`
- `docs/process/benchmark-plan.md`
- `benchmarks/README.md`
