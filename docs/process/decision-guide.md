# 意思決定ガイド

この文書は、現行プロジェクトで判断に迷ったときの**設計寄り参照先**です。
実装の現況確認には [../current-state.md](../current-state.md) を使ってください。

## 使い方

- **今何が動くか**を知りたい → `current-state.md`
- **なぜその設計なのか**を知りたい → ADR とこの文書
- **古い v0 制約をそのまま信じてよいか**迷った → まず current-state を見る

## 現在も有効な判断軸

1. Wasm-first の前提を崩さない
2. 現行 production / primary path は `wasm32-gc`（ADR-013）。`wasm32` は supported 互換パス
3. 設計資料と現行実装を混同しない
4. 迷ったら current-state / 実コード / fixtures を優先する
5. 旧ターゲット名（`wasm32-wasi-p1` / `wasm32-wasi-p2` / T1–T5）は履歴・alias 表以外に書かない

## 主な参照先

- [../current-state.md](../current-state.md)
- [../data/project-state.toml](../data/project-state.toml)
- [../adr/](../adr/)
- [ADR-007: Targets](../adr/ADR-007-targets.md)
- [ADR-013: Primary target](../adr/ADR-013-primary-target.md)
