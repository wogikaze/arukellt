# VSCode Extension: LSP機能を「ローカル補助」から「標準ライブラリ含む完全ナビゲーション」に昇格する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 439
**Depends on**: 333, 334, 335, 338, 339
**Track**: vscode-ide
**Blocks v1 exit**: no
**Priority**: 1

## Summary

VSCode拡張の中核である LSP 機能を、単一ファイル・ローカル定義前提の実装から、workspace + 標準ライブラリを含む完全なナビゲーション基盤へ引き上げる。特に std 関数に対する go-to-definition 不可という現状を解消し、定義ジャンプ・hover・references・rename がすべて semantic に動作する状態を目標とする。

## Current state

- `goto_definition` は同一ファイル内のみ探索。
- stdlib は completion 候補には出るが定義に飛べない。
- references / rename は token text ベース。
- workspace index / module graph が未構築または未活用。

## Acceptance

- [x] std/manifest + std source を含む symbol index が構築される。
- [x] go-to-definition が std 関数・型へジャンプ可能。
- [x] references / rename が semantic に動作。
- [x] hover に stdlib doc comment が表示される。
- [x] multi-file workspace で正しく動作する。

## References

- `crates/ark-lsp/src/server.rs`
- `std/manifest.toml`
- `std/**/*.ark`
- `crates/ark-resolve/`
- `crates/ark-typecheck/`
