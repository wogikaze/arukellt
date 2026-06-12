---
Status: done
Created: 2026-03-31
Updated: 2026-06-13
ID: 338
Track: lsp-semantic
Depends on: 333
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 2
---

# LSP: references を semantic symbol ID ベースに置き換える

## Summary

`textDocument/references` と `textDocument/documentHighlight` を resolver の `decl_index` ベースに置き換えた。shadowing 下の同名 symbol を区別し、ローカル解決が空のときは #333 の symbol index で cross-file fallback する。

## Delivered

- resolver が decl/use site を `RefIndex` に記録 (`ref_index.ark`, `ref_sites.ark`, `ref_record.ark`)
- `analysis/symbol_references.ark` が offset から同一 `decl_index` の site を収集
- `lsp/references_semantic.ark` が references / documentHighlight JSON を組み立て
- fixture: `tests/fixtures/selfhost/lsp_semantic_references.lsp-script`（shadowing ケース）

## Acceptance

- [x] 同名別 symbol が区別される (shadowing の内外で別 reference set)
- [x] resolver の binding 情報に基づいて reference を返す
- [x] project-wide の cross-file references が動作する (#333 index fallback)
- [x] `document_highlight` も同様に semantic 化する

## Verification

- `python3 scripts/check/check-lsp-lifecycle.py` — 5/5 scripts pass
- `python scripts/manager.py verify quick` — 150/150 pass
