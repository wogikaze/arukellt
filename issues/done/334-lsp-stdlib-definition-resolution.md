---
Status: done
Created: 2026-03-31
Updated: 2026-06-13
ID: 334
Track: lsp-navigation
Depends on: none
---

# LSP: stdlib definition resolution

## Summary

`std/manifest.toml` から signature / doc を読み、stdlib ソースの識別子 span で goto/hover を返す。

## Delivered

- `lsp/symbol_index_stdlib.ark` — manifest doc/params/returns パース
- `lsp/symbol_index_stdlib_spans.ark` — 遅延ソース span 解決
- `lsp/source_text.ark` — cross-file position 変換
- Fixture: `tests/fixtures/selfhost/lsp_stdlib_definition.lsp-script`

## Acceptance

- [x] `println` goto → `std/host/stdio.ark` 識別子 span（line 18 char 7–14）
- [x] hover に manifest doc + signature
- [x] manifest 駆動（hardcoded placeholder span 廃止）
- [x] `check-lsp-lifecycle.py` pass

## Verification

- `python3 scripts/check/check-lsp-lifecycle.py` — `lsp_stdlib_definition`
- `python scripts/manager.py verify quick` — 150/150 pass

## Audit resolution — 2026-06-13

**Reopen reason addressed**: 6/12「manifest 未駆動・decl_start:0」は selfhost `symbol_index_stdlib*.ark` で反証。golden で span + doc を固定。

**Evidence**: `lsp_stdlib_definition.lsp-expected`, `std/manifest.toml` println entry
