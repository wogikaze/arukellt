# 基盤: benchmark variance 制御と再現性プロファイル

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 146
**Depends on**: 149
**Track**: benchmark
**Blocks v1 exit**: no

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
- `scripts/perf-gate.sh`
