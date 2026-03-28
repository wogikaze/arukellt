# コンパイル速度: 未使用 stdlib 関数の遅延解決 (lazy-resolve)

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 096
**Depends on**: —
**Track**: compile-speed
**Blocks v4 exit**: yes

## Summary

現在の `ark-resolve` は stdlib 全体を常に解決するが、
`hello.ark` のような小さなプログラムは std の10%以下しか使わない。
未使用の stdlib 関数を resolve・typecheck・MIR lower しないよう遅延評価を導入し、
`hello.ark` の 50ms コンパイル目標達成に貢献する。

## 受け入れ条件

1. `ark-resolve` に「未使用関数スキップ」モードを追加
2. エントリポイントから呼び出しグラフを辿り、到達可能な関数のみを処理
3. `hello.ark` のコンパイル時間が lazy-resolve なし比 30% 以上削減
4. `--no-lazy-resolve` フラグで従来動作を復元可能

## 実装タスク

1. `ark-resolve/src/resolve.rs`: 呼び出しグラフ構築 + 到達可能集合計算
2. `ark-typecheck`: 未到達関数の型チェックをスキップ
3. `ark-mir/src/lower.rs`: 未到達関数の MIR lowering をスキップ

## 参照

- roadmap-v4.md §2 (hello.ark 50ms 目標)
