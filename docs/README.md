# Arukellt Documentation

> **Source of truth**: 現在の実装状況は [current-state.md](current-state.md) を参照してください。
> current-first の説明を優先し、古い design / ADR / scope 文書は archive として読むのが安全です。

## まず読む

- [Current state](current-state.md) — 実装の現在地、fixture/baseline、制限
- [Quickstart](quickstart.md) — 現在動く書き方
- [標準ライブラリ概要](stdlib/README.md) — 現在の stdlib 公開面
- [コンパイルパイプライン](compiler/pipeline.md) — current path と refactor target path
- [診断システム](compiler/diagnostics.md) — canonical diagnostics registry / phase-aware rendering

## 利用者向け

- [構文リファレンス](language/syntax.md)
- [型システム](language/type-system.md)
- [エラーハンドリング](language/error-handling.md)
- [コア API](stdlib/core.md)
- [I/O API](stdlib/io.md)
- [Cookbook](stdlib/cookbook.md)

## 実装 / 運用の current-first 文書

- [Operational policy](process/policy.md)
- [T1 → T3 migration](migration/t1-to-t3.md)
- [ABI ポリシー](platform/abi.md)
- [Wasm 機能レイヤー](platform/wasm-features.md)
- [Contributing](contributing.md)

## 履歴・設計・アーカイブ

以下は current behavior の source of truth ではない。
制約、設計意図、履歴として読む。

- [v1 非ゴール](process/v1-non-goals.md)
- [trait なし環境での抽象化戦略](design/trait-less-abstraction.md)
- [Archived WASI resource model](platform/wasi-resource-model.md)
- [Archived v0 unified spec](spec/v0-unified-spec.md)
- [Archived v0 status](process/v0-status.md)
- [Archived v1 status](process/v1-status.md)
- [Archived v0 freeze note](FREEZE-v0-READY.md)
