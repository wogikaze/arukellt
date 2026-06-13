---
Status: done
Created: 2026-04-02
Updated: 2026-06-13
ID: 452
Track: vscode-ide
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 1
---

# LSP: E0100 偽陽性 diagnostics を解消し CLI check と一致させる

## Summary

LSP diagnostics が project-aware resolve（indexed workspace modules）を使うようにし、import 解決済み cross-file コードで偽陽性 E0100 を解消した。

## Delivered

- `lsp/project_resolve_modules.ark` — symbol index から `ModuleDecls` 構築
- `compiler/session_analyze_pipeline.ark` — `run_session_analyze_with_modules`
- `lsp/documents.ark` — publish 時に project modules を渡す
- Fixture: `lsp_diagnostics_parity.lsp-script`（prelude-only、diagnostics 空）
- cross-file fixture: `helper::helper_fn()` + diagnostics 空 golden
- Extension: `undefined name` resolve diagnostic テスト（`test.skip` 解除）

## Acceptance

- [x] import 解決済み cross-file で `publishDiagnostics` が空
- [x] prelude-only 有効プログラムで diagnostics 空（`lsp_diagnostics_parity`）
- [x] 正当な undefined name は diagnostic 出力（extension E2E）
- [x] CLI `check` と LSP diagnostics が一致（cross-file helper fixture）
- [x] `check-lsp-lifecycle.py` pass
- [x] extension diagnostics E2E（3 tests、skip なし）
- [x] `verify quick` pass

## Verification

- `python3 scripts/check/check-lsp-lifecycle.py` — 11/11 pass
- `python scripts/manager.py verify quick` — 150/150 pass

## Audit resolution — 2026-06-13

**Reopen reason addressed**: `empty_modules` resolve による cross-file E0100 偽陽性。`main.ark` は `helper::helper_fn()` に修正し CLI/LSP 整合。

**Evidence**: `lsp_cross_file_definition.lsp-expected` diagnostics `[]`, `project_resolve_modules.ark`
