# Linter: lint 結果を LSP diagnostics / code action として配信する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 352
**Depends on**: 349
**Track**: linter
**Blocks v1 exit**: no
**Priority**: 19

## Summary

lint rule の結果を LSP の diagnostics として publish し、fix-it を持つ rule は code action として提供する。unused import の削除、unused binding の warning など、editor 上で即座にフィードバックを返す。

## Current state

- LSP は `publish_diagnostics()` で compiler error / warning を返すが、lint rule は走らせていない
- code action は formatting / import 追加 / unresolved name 修正のみ
- lint rule の fix-it → code action 変換の導線がない

## Acceptance

- [ ] lint rule の結果が `textDocument/publishDiagnostics` に含まれる
- [ ] lint diagnostic の source が `"arukellt-lint"` に設定される
- [ ] fix-it を持つ rule が `textDocument/codeAction` で `quickfix` として提供される
- [ ] `did_change` 時に lint が再実行され、diagnostics が更新される

## References

- `crates/ark-lsp/src/server.rs` — publish_diagnostics / code action
- `crates/ark-diagnostics/src/codes.rs` — diagnostic コード体系
