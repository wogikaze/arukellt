---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 336
Track: lsp-navigation
Depends on: 334
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 7
---

# LSP: hover の型情報を resolver/typechecker 連携で精密化する
- `crates/ark-lsp/src/server.rs: 121-154` — `CachedAnalysis` に resolved / checker あり
- [x] 変数上の hover が推論された型を表示する (`let x = 1 + 2` → `x: i32`)
# LSP: hover の型情報を resolver/typechecker 連携で精密化する

## Summary

hover を「AST 上の type annotation 表示」から「resolver / typechecker が推論した型 + doc + signature の表示」に精密化する。stdlib 関数の hover は #334 で manifest から signature を引くが、ユーザー定義関数の hover は typechecker の推論結果を反映すべき。

## Current state

- `crates/ark-lsp/src/server.rs:2131-2210` (hover handler): token の種類と AST context から markdown を組み立て
- `CachedAnalysis` は `resolved` (ResolveResult) と `checker` (CheckerResult) を保持しているが、hover での活用は限定的
- 変数の型推論結果を hover に反映する導線がない
- doc comment の表示なし

## Acceptance

- [x] 変数上の hover が推論された型を表示する (`let x = 1 + 2` → `x: i32`)
- [x] 関数上の hover が full signature + return type を表示する
- [x] doc comment (`///`) が hover に含まれる
- [x] stdlib 関数の hover が manifest の description を表示する (#334 前提)

## References

- `crates/ark-lsp/src/server.rs:2131-2210` — hover handler
- `crates/ark-lsp/src/server.rs:121-154` — `CachedAnalysis` に resolved / checker あり