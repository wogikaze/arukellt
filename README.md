# Arukellt

Wasm-first、LLM-friendly を目指す静的型付け言語。

> 現行実装の確認先は [docs/current-state.md](docs/current-state.md) です。

<!-- BEGIN GENERATED:README_STATUS -->
## Status

- Updated: 2026-05-14
- CLI default target: `wasm32-wasi-p1`
- Canonical target: `wasm32-wasi-p2`
- Component/WIT target: `wasm32-wasi-p2`
- Unit tests: selfhost verification is tracked by `python3 scripts/manager.py verify`
- Fixture harness: 641 passed, 28 skipped / 1072 entries
- Verification: `python3 scripts/manager.py verify quick` — 22/22 checks pass
- Stdlib manifest-backed public API: 595 functions
<!-- END GENERATED:README_STATUS -->

## Start here

- [Current state](docs/current-state.md) — 実装の現在地
- [Quickstart](docs/quickstart.md) — まず動かすための最短ガイド
- [Stdlib overview](docs/stdlib/README.md) — 現在の標準ライブラリ公開面
- [Compiler pipeline](docs/compiler/pipeline.md) — 現在のコンパイラ構成

## Workspace overview

- `src/compiler/` — selfhost compiler pipeline (`lexer.ark`, `parser.ark`, `resolver.ark`, `typechecker.ark`, MIR, emitters, diagnostics, LSP)
- `scripts/run/arukellt-selfhost.sh` — CLI wrapper (`check` / `compile` / `run` / `lsp`)
- `std/` — source-backed stdlib wrappers
- `tests/fixtures/` — manifest-driven end-to-end fixtures
- `playground/` — browser playground source
- `extensions/arukellt-all-in-one` — VS Code extension bootstrap
- `docs/` — 利用者向け・設計向けドキュメント

## Notes

- 仕様書や ADR には設計意図も含まれます。
- 現在動くものを判断したいときは、まず `docs/current-state.md` を見てください。
