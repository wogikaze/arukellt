---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Linter: lint 結果を LSP diagnostics / code action として配信する
**Closed**: 2026-04-01
**ID**: 352
**Depends on**: 349
**Track**: linter
**Blocks v1 exit**: no
**Priority**: 19

## Summary

lint rule の結果を LSP の diagnostics として publish し、fix-it を持つ rule は code action として提供する。

## Acceptance

- [x] lint rule の結果が `textDocument/publishDiagnostics` に含まれる
- [x] lint diagnostic の source が `"arukellt-lint"` に設定される
- [x] fix-it を持つ rule が `textDocument/codeAction` で `quickfix` として提供される
- [x] `did_change` 時に lint が再実行され、diagnostics が更新される

## Implementation

- `crates/ark-lsp/src/server.rs`:
  - `analyze_source()` に `check_unused_imports()` と `check_unused_bindings()` の呼び出しを追加
  - `collect_lsp_diagnostics()` で W-code diagnostics に `"arukellt-lint"` source を設定
  - W0006 (unused import) に対する quickfix code action を追加（import 行の削除）
  - `did_change` → `refresh_diagnostics` → `analyze_source` により lint が自動再実行
- 2つの unit test を追加:
  - `lint_diagnostics_have_arukellt_lint_source`: W0006/W0007 が arukellt-lint source で発行されることを検証
  - `compiler_diagnostics_have_arukellt_source`: compiler error が arukellt source のままであることを検証

## References

- `crates/ark-lsp/src/server.rs`
- `crates/ark-resolve/src/unused.rs`