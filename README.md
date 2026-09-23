# Arukellt

Wasm-first、LLM-friendly を目指す静的型付け言語。

> 現行実装の確認先は [docs/current-state.md](docs/current-state.md) です。

<!-- BEGIN GENERATED:README_STATUS -->
## Status

- Updated: 2026-09-22
- CLI default target: `wasm32-gc`
- Canonical target: `wasm32-gc`
- Component/WIT target: `wasm32-gc`
- Unit tests: selfhost verification is tracked by `python3 scripts/manager.py verify`
- Current observation (2026-09-17): base `edc010bb9fd57c73bc9ae003b3c9392dc94d6279`; uncommitted fixture gate fixes and documentation updates; not a clean-HEAD receipt.
- ADR-057 current-only fixture gate: `python3 scripts/manager.py verify fixtures` — 1691 compile/validate passed, 0 failed; 100 runtime executed; 2 runtime contracts delegated to native-cpp.
- Observed quick gate: `python3 scripts/manager.py verify quick` — 151/151 checks pass.
- Coverage: not exhaustive runtime or pinned/reference parity; not a release-readiness receipt. Historical failures are not individually resolved by this observation.
- Historical fixture harness (2026-07-15): 57 passed, 1089 failed, 442 skipped (observed harness: 1588); registry: 2991 manifest entries
- Historical verification: `python3 scripts/manager.py verify quick` — 166/166 checks pass
- Stdlib manifest-backed public API: 792 functions
<!-- END GENERATED:README_STATUS -->

## Start here

- [Current state](docs/current-state.md) — 実装の現在地
- [Quickstart](docs/quickstart.md) — まず動かすための最短ガイド
- [Stdlib overview](docs/stdlib/README.md) — 現在の標準ライブラリ公開面
- [Docs index](docs/README.md) — ドキュメント入口（ルート参照一覧含む）
- [見取り図 HTML（アーカイブ）](docs/overview.html) — 旧ターゲット体系の視覚マップ。正本ではない
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

## License

MIT License

Copyright (c) 2026 wogikaze

本リポジトリのコードは [MIT License](LICENSE) のもとで公開されています。
