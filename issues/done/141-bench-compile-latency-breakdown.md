# 計測: cold/warm/incremental compile と phase 別時間分解

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 141
**Depends on**: 149
**Track**: benchmark
**Blocks v1 exit**: no

## Summary

コンパイル時間を 1 本の数字だけで見ると改善点が見えにくい。
clean build / warm build / incremental build と、parse / resolve / typecheck / lower / optimize / emit の phase 別時間を計測し、
どこが支配的かを常に把握できるようにする。

## 受け入れ条件

1. 小・中・大の fixture で cold/warm/incremental compile 時間を取れる
2. phase 別時間を JSON と Markdown の両方で出力できる
3. `--quick` は代表 fixture のみ、full は全 fixture を回す
4. phase の合計と総コンパイル時間が大きく乖離しないことを self-check する

## 実装タスク

1. compiler phase timing の採取ポイントを決める
2. repeatable な cold/warm/incremental 測定手順を定義する
3. 既存 perf gate に compile breakdown を差し込める結果形式を決める

## 参照

- `scripts/perf-gate.sh`
- `docs/process/benchmark-plan.md`
- `issues/done/110-bench-perf-gate.md`
