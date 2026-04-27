---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 338
Track: lsp-semantic
Depends on: 333
Orchestration class: implementation-ready
---
# LSP: references を semantic symbol ID ベースに置き換える
**Blocks v1 exit**: no
**Priority**: 2

## Summary

`references()` を token text 一致から、resolver の symbol ID ベースに置き換える。現在は同一ファイル内で同じ identifier 文字列を持つ全 token を返すため、同名別 symbol (shadowing、別 scope の同名変数) を区別できない。cross-file references も #333 の index に基づいて実装する。

## Current state

- `crates/ark-lsp/src/server.rs:2271-2314`: `references()` が current file の token 列を走査し、同一 identifier text を返す
- semantic symbol ID なし — `find_ident_at_offset()` が名前文字列のみ返す
- shadowing を区別しない: 内側 scope の `x` と外側の `x` が同一 reference 扱い
- cross-file references なし

## Acceptance

- [x] 同名別 symbol が区別される (shadowing の内外で別 reference set)
- [x] resolver の binding 情報に基づいて reference を返す
- [x] project-wide の cross-file references が動作する (#333 前提)
- [x] `document_highlight` も同様に semantic 化する

## References

- `crates/ark-lsp/src/server.rs:2271-2314` — `references()` text-based 実装
- `crates/ark-lsp/src/server.rs:2316-2358` — `document_highlight()` text-based 実装