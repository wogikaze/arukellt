---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 337
Track: lsp-navigation
Depends on: 334
Orchestration class: implementation-ready
---
# LSP: signature help を stdlib 連携で完成させる
**Blocks v1 exit**: no
**Priority**: 9

## Summary

`signature_help` を hardcoded builtin 一覧ではなく、project symbol index + stdlib manifest から引くようにする。現在は関数呼び出し中の `(` 直後で限定的な情報を返すのみ。active parameter tracking も不完全。

## Current state

- `crates/ark-lsp/src/server.rs`: signature_help handler 実装あり
- builtin 関数の signature は hardcoded で、stdlib manifest や resolver から引いていない
- ユーザー定義関数の signature はパースされた AST から取得するが、cross-file では動作しない
- active parameter (カーソル位置の引数) のトラッキングが不正確

## Acceptance

- [x] stdlib 関数の呼び出しで manifest の signature が表示される
- [x] ユーザー定義関数の呼び出しで cross-file の signature が表示される
- [x] active parameter がカーソル位置に追従する
- [x] overload がある場合に複数 signature が表示される

## References

- `crates/ark-lsp/src/server.rs` — signature_help handler
- `std/manifest.toml` — stdlib function signature