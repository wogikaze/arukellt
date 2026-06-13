---
Status: done
Created: 2026-03-31
Updated: 2026-06-13
ID: 335
Track: lsp-navigation
Depends on: 333
---

# LSP: cross-file go to definition

## Summary

qualified name 解決、`textDocument/typeDefinition`、project symbol index 連携。

## Delivered

- `lsp/qualified_name.ark` — `module::item` 抽出
- `lsp/feature_type_definition.ark` + `dispatch_features.ark`
- Fixtures: `lsp_qualified_definition`, `lsp_type_definition`, `lsp_cross_file_definition`
- `lsp_symbol_index/helper.ark` に `Point` struct（typeDefinition 用）

## Acceptance

- [x] 別ファイル定義への bare name goto（`lsp_cross_file_definition`）
- [x] qualified name goto（`helper::helper_fn`）
- [x] `textDocument/typeDefinition` → struct（`helper::Point`）
- [x] project index + import graph（`symbol_index_project.ark`）
- [x] `check-lsp-lifecycle.py` pass

## Verification

- `python3 scripts/check/check-lsp-lifecycle.py` — cross-file / qualified / type definition scripts
- `python scripts/manager.py verify quick` — 150/150 pass

## Audit resolution — 2026-06-13

**Reopen reason addressed**: 6/12「single-buffer only」は symbol index fallback + qualified name + typeDefinition で解消。

**Evidence**: `lsp_qualified_definition.lsp-expected`, `lsp_type_definition.lsp-expected`

**Scope note**: マルチファイル workspace 全体は #441 スコープ外（import graph 起点の index のみ）。
