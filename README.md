# Arukellt

Wasm-first、LLM-friendly を目指す静的型付け言語。

> 現行実装の確認先は [docs/current-state.md](docs/current-state.md) です。

<!-- BEGIN GENERATED:README_STATUS -->
## Status

- Updated: 2026-03-31
- CLI default target: `wasm32-wasi-p1`
- Canonical target: `wasm32-wasi-p2`
- Component/WIT target: `wasm32-wasi-p2`
- Unit tests: current count is verified by `cargo test --workspace --exclude ark-llvm`
- Fixture harness: 590 passed, 5 skipped / 595 entries
- Verification: `bash scripts/verify-harness.sh (fast local gate; use --full for full local verification)` — 13/13 checks pass
- Stdlib manifest-backed public API: 266 functions
<!-- END GENERATED:README_STATUS -->

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
- `extensions/arukellt-all-in-one` — VS Code extension bootstrap
- `crates/ark-llvm` — LLVM backend scaffold（既定ビルド外）
- `std/` — source-backed stdlib wrappers
- `tests/fixtures/` — manifest-driven end-to-end fixtures
- `docs/` — 利用者向け・設計向けドキュメント

## Notes

- 仕様書や ADR には設計意図も含まれます。
- 現在動くものを判断したいときは、まず `docs/current-state.md` を見てください。
