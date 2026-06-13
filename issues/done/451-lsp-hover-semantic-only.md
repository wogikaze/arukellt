---
Status: done
Created: 2026-04-02
Updated: 2026-06-13
ID: 451
Track: vscode-ide
Depends on: none
---

# LSP: hover semantic-only

## Summary

リテラル・識別子境界外では hover を null にし、semantic binding / index のみ hover を返す。`IdentSpan` で `offset < span_end` を統一。`hoverDetailLevel: minimal` は #479 と連携。

## Delivered

- `analysis/ident_span.ark` — `IdentSpan` と `ident_span_at_offset`
- `analysis/symbols.ark` — strict ident boundary
- `lsp/feature_hover.ark` — semantic ident 必須、`hover_detail_minimal` 対応
- Fixture: `tests/fixtures/selfhost/lsp_hover_semantic_only.lsp-script`
- Extension #453: string literal null + function hover + identifier trailing null

## Acceptance

- [x] int リテラル hover → null
- [x] 識別子末尾（`offset == span_end`）hover → null
- [x] 関数呼び出し hover → signature
- [x] string literal hover → null（extension #453）
- [x] LSP lifecycle fixture pass
- [x] extension hover E2E pass
- [x] `verify quick` pass

## Verification

- `python3 scripts/check/check-lsp-lifecycle.py` — `lsp_hover_semantic_only`
- `extensions/arukellt-all-in-one` suite "Hover (#451 / #453)"

## Audit resolution — 2026-06-13

**Reopen reason addressed**: AST ident のみ hover；リテラル/境界外は null。lifecycle + extension E2E で固定。

**Evidence**: `lsp_hover_semantic_only.lsp-expected`, `extension.test.js` Hover suite
