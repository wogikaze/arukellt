# LSP: cross-file go to definition を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 335
**Depends on**: 333
**Track**: lsp-navigation
**Blocks v1 exit**: no
**Priority**: 4

## Summary

`goto_definition` を single-file AST walk から project-wide symbol index 検索に切り替える。現在 `find_definition_span()` は current file の top-level item / let / param のみを探し、`Location.uri` も常に現在ファイルに固定されている。

## Current state

- `crates/ark-lsp/src/server.rs:2232-2269`: `goto_definition()` が current file のみ走査
- `find_definition_span()` (506-546): top-level item → impl block method → let binding → param の順で同一 module 内を探索
- 他ファイルで定義された関数・型に対して definition が返らない
- qualified name (`module::item`) の解決なし

## Acceptance

- [x] 別ファイルで定義された関数に go to definition で飛べる
- [x] `use` 文で import した名前から定義元ファイルに飛べる
- [x] qualified name (`module::fn()`) の定義元に飛べる
- [x] `goto_type_definition` も cross-file で動作する

## References

- `crates/ark-lsp/src/server.rs:2232-2269` — `goto_definition()` 実装
- `crates/ark-lsp/src/server.rs:506-546` — `find_definition_span()` 同一ファイル探索
- `crates/ark-lsp/src/server.rs:3037-3080` — `goto_type_definition()` 同一ファイル探索
