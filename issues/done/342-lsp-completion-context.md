# LSP: completion をコンテキスト対応にする

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 342
**Depends on**: 338
**Track**: lsp-semantic
**Blocks v1 exit**: no
**Priority**: 8

## Summary

completion を「位置無関係の全候補フラット表示」から、カーソル位置の構文コンテキストに応じたフィルタリング・ランキングに改善する。dot completion (method / field)、pattern context での enum variant 提案、type annotation context での型名提案を実装する。

## Current state

- `crates/ark-lsp/src/server.rs:244-484` (`get_completions()`): prefix 文字列一致で全 builtin + keyword + module を返す
- dot completion なし: `x.` の後でメソッド / field を提案しない
- pattern context での enum variant 提案なし
- 型注釈位置での型名優先表示なし

## Acceptance

- [x] `.` の後でレシーバ型に応じた method / field が提案される
- [x] `match` arm の pattern 位置で enum variant が提案される
- [x] 型注釈位置 (`:` の後) で型名が優先表示される
- [x] `use` 文の後で import 可能な module / symbol が提案される

## References

- `crates/ark-lsp/src/server.rs:244-484` — `get_completions()` hardcoded 一覧
- `crates/ark-lsp/src/server.rs:185-198` — `completion_prefix()` テキスト抽出
