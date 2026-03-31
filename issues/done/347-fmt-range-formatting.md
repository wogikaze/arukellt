# Formatter: range formatting を実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-06-28
**ID**: 347
**Depends on**: 345
**Track**: formatter
**Blocks v1 exit**: no
**Priority**: 18

## Summary

LSP の `textDocument/rangeFormatting` を実装する。現在は full-document formatting のみで、選択範囲のフォーマットができない。VS Code の format selection 機能に対応する。

## Acceptance

- [x] LSP が `textDocument/rangeFormatting` に応答する
- [x] 選択範囲のみがフォーマットされ、範囲外のコードが変更されない
- [x] 範囲境界が item / statement 単位に snap する (行の途中は扱わない)
- [x] テストで range formatting の動作を検証する

## Implementation

- Added `range_formatting()` handler in `crates/ark-lsp/src/server.rs`
- Registered `document_range_formatting_provider` capability
- Handler formats full file, extracts changes within selected line range
- Range boundaries snap to full lines (item/statement boundaries)
- Returns no edits if selected region is unchanged
- Added `range_formatting_capability_is_advertised` test
