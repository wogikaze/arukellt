---
name: benchmark-change
description: Arukelltのbenchmark、性能回帰gate、baseline、threshold、測定方法を変更・評価する。速度やサイズの比較、benchmark追加、baseline更新、回帰判定に使う。単なる最適化実装だけには使わない。
---

# Benchmark change

測定値より先に、再現性と baseline の正当性を守る。

## 正本

- `docs/process/benchmark-plan.md`
- `docs/benchmarks/governance.md`
- `docs/benchmarks/variance-control.md`
- `docs/data/verification-commands.toml`
- benchmark runner と committed baseline

## 手順

1. 目的を smoke、比較、回帰 gate、baseline 更新、benchmark 追加のいずれかに分類する。
2. quick run の単一サンプルを性能判断に使わない。比較・回帰判定には統計的 sampling を行う正規モードを使う。
3. compiler/runtime、target、commit、OS、CPU、runner設定など結果解釈に必要な環境情報を記録する。
4. benchmark 追加時は入力、期待出力、登録、baseline を同じ変更単位で整合させる。
5. baseline 更新は、改善、妥当な中立変更、新規fixture、明示承認された一時回帰に限る。意図しない回帰や分散を隠すために更新しない。
6. threshold 変更は測定根拠と承認条件を文書化する。閾値を緩めるだけの修正をしない。
7. 現行の正規コマンドを registry から確認し、通常は次を組み合わせる。
   - `python3 scripts/manager.py perf benchmarks`
   - `python3 scripts/manager.py perf baseline`（意図した baseline 更新時のみ）
   - `python3 scripts/manager.py perf gate`
   - サイズ変更なら `python3 scripts/manager.py verify size`
8. correctness gate を性能 gate で代用しない。コード変更には `$code-change-verification` の通常検証も行う。

## 報告

測定条件、base/head、サンプル数、中央値または採用統計、差分、閾値判定、baseline変更理由、分散上の注意を記録する。
