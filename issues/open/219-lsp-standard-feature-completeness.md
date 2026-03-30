# LSP standard feature completeness

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 219
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: yes

## Summary

signature help、document highlights、inlay hints、inline values、folding ranges、selection ranges、linked editing、semantic token の品質向上、large file / large workspace でのインデックスキャッシュと再解析最適化を実装する。

これらは LSP プロトコルの標準機能だが、現状どの issue にも明示的に含まれていない。

## Acceptance

- [ ] signature help / document highlights / inlay hints が動作する
- [ ] folding ranges / selection ranges が動作する
- [ ] large file（>5000 行）で LSP が応答不能にならない

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
