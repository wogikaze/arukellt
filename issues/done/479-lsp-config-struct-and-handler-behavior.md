---
Status: done
Created: 2026-04-03
Updated: 2026-06-13
ID: 479
Track: vscode-ide
Depends on: 477, 478
---

# LSP server: 設定反映

## Summary

`initializationOptions` / `didChangeConfiguration` から `LspConfig` を更新し、codeLens / diagnostics / hover detail をランタイムで切り替える。

## Delivered

- `lsp/lsp_config.ark` — `apply_initialize_config`, configuration パース
- `lsp/feature_code_lens.ark` — `codeLens.enabled` ゲート
- `lsp/feature_hover.ark` — `hoverDetailLevel: minimal`
- `lsp/documents.ark` — `didSave` で再 publish
- `lsp/state_record.ark` — 設定フィールド

## Acceptance

- [x] `hoverDetailLevel: minimal` → signature のみ
- [x] `enableCodeLens: false` → 空 codeLens
- [x] `checkOnSave: false` → didSave 診断スキップ
- [x] `diagnosticsReportLevel` が publish を制御
- [x] `verify quick` pass

## Verification

- `python scripts/manager.py verify quick` — 150/150 pass
- `python3 scripts/check/check-lsp-lifecycle.py` — 11/11 pass

## Audit resolution — 2026-06-13

**Evidence**: `lsp_config.ark`, `feature_code_lens.ark`, `extensions/.../extension.js` initializationOptions（#478）
