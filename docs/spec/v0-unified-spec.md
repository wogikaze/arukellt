# Archived v0 unified spec

> **Archive**: This document is a historical reference and is not the current behavior contract.
> For current verified behavior, see [../current-state.md](../current-state.md).

この文書は、以前の v0 統合仕様を 1 箇所に集めていた設計資料です。
現在は **現行実装の source of truth ではありません**。

## Current source of truth

- [../current-state.md](../current-state.md)

## なぜ archive 化したか

以前の `v0-unified-spec.md` には、

- v0 設計仕様
- v1 で既に実装済みになった項目
- capability I/O や Wasm GC の将来設計
- 古い fixture counts や進捗メモ

が同居しており、現行利用者向けには誤解を生みやすくなっていました。

## いま見るべき文書

- 現在の実装: [../current-state.md](../current-state.md)
- 構文: [../language/syntax.md](../language/syntax.md)
- 型: [../language/type-system.md](../language/type-system.md)
- エラー処理: [../language/error-handling.md](../language/error-handling.md)
- stdlib: [../stdlib/README.md](../stdlib/README.md)
- パイプライン: [../compiler/pipeline.md](../compiler/pipeline.md)

## 位置づけ

今後このファイルは、必要なら「過去の v0 設計判断を参照するための履歴資料」としてのみ残してください。
現行 reality を説明する用途には使わない想定です。
