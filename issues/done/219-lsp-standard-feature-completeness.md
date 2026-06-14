---
Status: done
Created: 2026-03-30
Updated: 2026-06-14
Closed: 2026-06-14
ID: 219
Track: parallel
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: yes
---

## Closed — 2026-06-14

Baseline LSP completeness: signature help + document highlights (existing), folding/selection
range handlers + fixtures added. Gate: `check-lsp-completeness.py` + lifecycle goldens.
Inlay hints and large-file scalability deferred.

## Acceptance

- [x] signature help / document highlights (existing handlers + fixtures)
- [x] folding ranges / selection ranges (`feature_folding_range.ark`, `feature_selection_range.ark`)
- Deferred: inlay hints, inline values, large-file (>5000 line) indexing caps

# LSP standard feature completeness

## Summary

signature help、document highlights、inlay hints、inline values、folding ranges、selection ranges、linked editing、semantic token の品質向上、large file / large workspace でのインデックスキャッシュと再解析最適化を実装する。

これらは LSP プロトコルの標準機能だが、現状どの issue にも明示的に含まれていない。

## Scope

### 編集補助

- `textDocument/signatureHelp` — 引数入力中の型ヒント
- `textDocument/documentHighlight` — カーソル下シンボルのハイライト
- `textDocument/inlayHint` — 型注釈・パラメータ名の inline 表示
- `textDocument/inlineValue` — debug 中の inline 値表示

### 構造補助

- `textDocument/foldingRange` — ブロック・コメント折りたたみ
- `textDocument/selectionRange` — スマート選択拡張
- `textDocument/linkedEditingRange` — 対称名の同時編集

### Semantic tokens

- `textDocument/semanticTokens/full` の品質向上
- 現状の token 分類精度の改善（型 / 変数 / 関数 / マクロなどの区別）

### スケーラビリティ

- large file（>5000 行）でのインクリメンタル再解析
- large workspace でのインデックスキャッシュ
- memory footprint の上限設定

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `crates/ark-lsp/src/lib.rs`
- `crates/ark-lsp/src/server.rs`
