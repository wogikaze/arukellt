# 計測: 入力サイズ sweep とスケーリングカーブ可視化

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 144
**Depends on**: 141, 142, 143, 149
**Track**: benchmark
**Blocks v1 exit**: no

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
