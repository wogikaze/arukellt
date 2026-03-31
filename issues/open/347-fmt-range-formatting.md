# Formatter: range formatting を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 347
**Depends on**: 345
**Track**: formatter
**Blocks v1 exit**: no
**Priority**: 18

## Summary

LSP の `textDocument/rangeFormatting` を実装する。現在は full-document formatting のみで、選択範囲のフォーマットができない。VS Code の format selection 機能に対応する。

## Current state

- `crates/ark-lsp/src/server.rs:2731-2761`: `formatting()` は full document のみ
- `range_formatting()` / `on_type_formatting()` の handler なし
- `TextDocumentSyncCapability` に range formatting の capability 宣言なし

## Acceptance

- [ ] LSP が `textDocument/rangeFormatting` に応答する
- [ ] 選択範囲のみがフォーマットされ、範囲外のコードが変更されない
- [ ] 範囲境界が item / statement 単位に snap する (行の途中は扱わない)
- [ ] テストで range formatting の動作を検証する

## References

- `crates/ark-lsp/src/server.rs:2731-2761` — full document formatting のみ
- LSP 仕様: `textDocument/rangeFormatting`
