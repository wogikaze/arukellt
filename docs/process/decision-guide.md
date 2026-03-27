# 意思決定ガイド

この文書は、現行プロジェクトで判断に迷ったときの**設計寄り参照先**です。
実装の現況確認には [../current-state.md](../current-state.md) を使ってください。

## 使い方

- **今何が動くか**を知りたい → `current-state.md`
- **なぜその設計なのか**を知りたい → ADR とこの文書
- **古い v0 制約をそのまま信じてよいか**迷った → まず current-state を見る

## 現在も有効な判断軸

1. Wasm-first の前提を崩さない
2. 現行 production path は T1 (`wasm32-wasi-p1`) と見る
3. 設計資料と現行実装を混同しない
4. 迷ったら current-state / 実コード / fixtures を優先する

## 主な参照先

- [../current-state.md](../current-state.md)
- [../adr/](../adr/)
- [../platform/wasm-features.md](../platform/wasm-features.md)
- [v0-scope.md](v0-scope.md)
