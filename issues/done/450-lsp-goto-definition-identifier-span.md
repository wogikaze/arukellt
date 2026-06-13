---
Status: done
Created: 2026-04-02
Updated: 2026-06-13
ID: 450
Track: vscode-ide
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 1
---

# LSP: Go to Definition を identifier span ベースに修正

## Summary

`AstNode.name_span` を parser で記録し、resolver / analysis / LSP definition が識別子トークン範囲のみを返す。shadowing は resolver `decl_index` で最内側束縛を選択。

## Delivered

- `AstNode.name_span` — `stmt_let`, `fn_sig_decl`, struct/enum/trait headers
- `analysis/symbols.ark` — `session_resolve_query` + `ref_sites` で binding span
- `lsp/feature_symbol.ark`, `symbol_index_extract.ark` — decl range を name span に統一
- Fixture: `tests/fixtures/selfhost/lsp_definition_identifier_span.lsp-script`

## Acceptance

- [x] let / fn / param の definition range が識別子のみ（`fn` キーワード起点ではない）
- [x] shadowing で内側束縛に飛ぶ
- [x] LSP E2E: local let / fn / param の range 検証（lifecycle fixture）
- [x] extension #453 definition E2E が pass
- [x] `python3 scripts/check/check-lsp-lifecycle.py` pass

## Verification

- `python3 scripts/check/check-lsp-lifecycle.py` — `lsp_definition_identifier_span`, `lsp_hover_definition`
- `python scripts/manager.py verify quick` — 150/150 pass

## Audit resolution — 2026-06-13

**Reopen reason addressed**: `lsp_hover_definition.lsp-expected` が識別子 span（例: `answer` line 0 char 3–9）を返すことを golden で固定。

**Evidence**: `parser/ast_node_record.ark` `name_span`, `lsp_definition_identifier_span.lsp-script`
