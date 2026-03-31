# Tooling Contract: LSP protocol E2E テストを追加する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 355
**Depends on**: 353
**Track**: tooling-contract
**Blocks v1 exit**: no
**Priority**: 23

## Summary

LSP server に対する protocol-level E2E テストを追加する。`initialize` → `didOpen` → `textDocument/completion` → `textDocument/definition` → `shutdown` のような LSP message sequence を自動テストし、protocol compliance を CI で保証する。

## Current state

- `crates/ark-lsp/src/server.rs`: 3 件の unit test (completion のみ)
- LSP protocol level の E2E テストなし
- `arukellt lsp --stdio` の起動・応答を自動テストする仕組みなし
- protocol compliance (capability negotiation, error handling, shutdown) の検証なし

## Acceptance

- [ ] LSP server を subprocess で起動し JSON-RPC message を送受する test harness が存在する
- [ ] initialize / didOpen / completion / definition / hover / shutdown の E2E テストが pass する
- [ ] protocol error handling (不正リクエスト、存在しないファイル) のテストが pass する
- [ ] CI で E2E テストが実行される

## References

- `crates/ark-lsp/src/server.rs` — LSP server 実装
- LSP 仕様: JSON-RPC over stdio protocol
