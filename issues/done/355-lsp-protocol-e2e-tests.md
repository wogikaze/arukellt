---
Status: done
Created: 2026-03-31
Updated: 2026-04-03
ID: 355
Track: tooling-contract
Depends on: 353
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 23
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Evidence: crates/ark-lsp/tests/lsp_e2e.rs exists
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---

# Tooling Contract: LSP protocol E2E テストを追加する
- `crates/ark-lsp/src/server.rs`: "3 件の unit test (completion のみ)"
- LSP 仕様: JSON-RPC over stdio protocol
# Tooling Contract: LSP protocol E2E テストを追加する

---

## Closed by audit — 2026-04-03




## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/355-lsp-protocol-e2e-tests.md` — incorrect directory for an open issue.


## Summary

LSP server に対する protocol-level E2E テストを追加する。`initialize` → `didOpen` → `textDocument/completion` → `textDocument/definition` → `shutdown` のような LSP message sequence を自動テストし、protocol compliance を CI で保証する。

## Current state

- `crates/ark-lsp/src/server.rs`: 3 件の unit test (completion のみ)
- LSP protocol level の E2E テストなし
- `arukellt lsp --stdio` の起動・応答を自動テストする仕組みなし
- protocol compliance (capability negotiation, error handling, shutdown) の検証なし

## Acceptance

- [x] LSP server を subprocess で起動し JSON-RPC message を送受する test harness が存在する
- [x] initialize / didOpen / completion / definition / hover / shutdown の E2E テストが pass する
- [x] protocol error handling (不正リクエスト、存在しないファイル) のテストが pass する
- [x] CI で E2E テストが実行される

## References

- `crates/ark-lsp/src/server.rs` — LSP server 実装
- LSP 仕様: JSON-RPC over stdio protocol