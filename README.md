# Arukellt

Wasm-first、LLM-friendly を目指す静的型付け言語。

> 現行実装の確認先は [docs/current-state.md](docs/current-state.md) です。

## Status

- 更新基準日: 2026-03-27
- 既定ターゲット: `wasm32-wasi-p1` (T1)
- `wasm32-wasi-p2` (T3) は experimental
- Unit tests: 95 passed
- Fixture harness: 182 passed / 182 entries
- `std/` では source-backed stdlib への移行を進めています

## Start here

- [Current state](docs/current-state.md) — 実装の現在地
- [Quickstart](docs/quickstart.md) — まず動かすための最短ガイド
- [Stdlib overview](docs/stdlib/README.md) — 現在の標準ライブラリ公開面
- [Compiler pipeline](docs/compiler/pipeline.md) — 現在のコンパイラ構成

## Workspace overview

- `crates/arukellt` — CLI (`check` / `compile` / `run`)
- `crates/ark-lexer` 〜 `crates/ark-wasm` — 主要コンパイラパイプライン
- `crates/ark-driver` — 共有 analysis/session 層
- `crates/ark-target` — ターゲット定義
- `crates/ark-lsp` — LSP scaffold
- `crates/ark-llvm` — LLVM backend scaffold（既定ビルド外）
- `std/` — source-backed stdlib wrappers
- `tests/fixtures/` — manifest-driven end-to-end fixtures
- `docs/` — 利用者向け・設計向けドキュメント

## Notes

- 仕様書や ADR には設計意図も含まれます。
- 現在動くものを判断したいときは、まず `docs/current-state.md` を見てください。
