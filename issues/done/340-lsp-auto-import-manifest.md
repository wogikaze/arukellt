---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 340
Track: lsp-semantic
Depends on: 334
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 5
---

# LSP: auto-import 候補を manifest / module graph 駆動にする
- `crates/ark-lsp/src/server.rs: "200-220` — `already_imported_modules()` / `imported_aliases()`"
# LSP: auto-import 候補を manifest / module graph 駆動にする

## Summary

auto-import の候補を `server.rs` 内の固定配列 (6 module) から、`std/manifest.toml` + project module graph 駆動に切り替える。import 追加の code action も、固定 template ではなく実際の module / export 状態に基づく提案にする。

## Current state

- `crates/ark-lsp/src/server.rs:344-413`: 6 module (stdio, fs, env, math, string, collections) を hardcoded
- completion の auto-import candidate は `already_imported_modules()` (200-206) でチェックするが、候補自体は固定
- code action の import 追加は unresolved name に対する固定 template
- 新規 stdlib module が追加されても LSP の hardcoded list が更新されないと候補に出ない

## Acceptance

- [x] auto-import 候補が `std/manifest.toml` の全公開 module を含む
- [x] project 内の他 `.ark` ファイルの公開 symbol も import 候補に出る
- [x] hardcoded module list が廃止され、manifest / index 駆動になる
- [x] import 追加の code action が正しい `use` 文を生成する

## References

- `crates/ark-lsp/src/server.rs:344-413` — hardcoded module 一覧
- `crates/ark-lsp/src/server.rs:200-220` — `already_imported_modules()` / `imported_aliases()`
- `std/manifest.toml` — canonical module / function 定義