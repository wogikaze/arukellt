---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 216
Track: parallel
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: no
---
# Formatter surface

## Summary

Arukellt の stable formatter を実装し、VS Code の format on save / format selection / document range formatting との統合、import 並び替えルール、コメント整形、canonical surface の文書化を整備する。

現状は formatter が存在せず、#185 の「先に潰す順番 #1」に挙げられている最優先ギャップ。

## Acceptance

- [x] `arukellt fmt` または LSP `textDocument/formatting` で安定した整形が動作する
- [x] VS Code で format on save / format selection が使える
- [x] formatter と compiler の surface 整合性（canonical 表現）が文書化されている

## Scope

### Core formatter

- `arukellt fmt` CLI エントリポイント
- LSP `textDocument/formatting` / `textDocument/rangeFormatting` の実装
- idempotent かつ compiler-valid な出力保証

### VS Code 統合

- format on save（`editor.formatOnSave`）
- `editor.defaultFormatter` contribution
- format selection（`textDocument/rangeFormatting`）

### Import 整形

- import 並び替えルール（stdlib → external → local）
- `source.organizeImports` code action との協調

### 文書化

- canonical surface 仕様（スペース、改行、コメント位置）
- formatter と parser の往復一致テスト

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `issues/done/192-intent-completion-and-auto-import-intelligence.md`
- `crates/ark-lsp/src/lib.rs`