---
Status: open
Created: 2026-03-31
Updated: 2026-06-12
ID: 439
Track: vscode-ide
Depends on: 333, 334, 335, 338, 339
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 1
# VSCode Extension: LSP機能を「ローカル補助」から「標準ライブラリ含む完全ナビゲーション」に昇格する
---
# VSCode Extension: LSP機能を「ローカル補助」から「標準ライブラリ含む完全ナビゲーション」に昇格する

## Reopened by audit — 2026-06-12 (Slice G)

**Classification:** `must-reopen` / `acceptance-not-actually-met`

**Reopen reason:** Acceptance claims std/manifest-backed symbol index, stdlib go-to-definition, semantic references/rename, and multi-file workspace navigation. Selfhost LSP resolves symbols via single-buffer `analysis::symbols::symbol_at` only; no `std/manifest.toml` integration, no references/rename handlers, and no workspace index.

**Violated acceptance:** All five checkboxes (stdlib index, std go-to-definition, references/rename, stdlib hover docs, multi-file workspace).

**Evidence files:**
- `src/compiler/lsp/feature_definition.ark` — single-file `symbol_at` only
- `src/compiler/analysis/symbols.ark` — no stdlib/manifest lookup
- `src/compiler/lsp/` — no `references` / `rename` handlers (grep empty)
- `crates/` absent — cited `crates/ark-lsp/src/server.rs` deleted (#559 migration)

**Follow-up split:** none (blocked on selfhost LSP nav port tracked by reopened #333–#342 cluster on slice E branch)

## Summary

VSCode拡張の中核である LSP 機能を、単一ファイル・ローカル定義前提の実装から、workspace + 標準ライブラリを含む完全なナビゲーション基盤へ引き上げる。特に std 関数に対する go-to-definition 不可という現状を解消し、定義ジャンプ・hover・references・rename がすべて semantic に動作する状態を目標とする。

## Current state

- `goto_definition` は同一ファイル内のみ探索。
- stdlib は completion 候補には出るが定義に飛べない。
- references / rename は token text ベース。
- workspace index / module graph が未構築または未活用。

## Acceptance

- [ ] std/manifest + std source を含む symbol index が構築される。
- [ ] go-to-definition が std 関数・型へジャンプ可能。
- [ ] references / rename が semantic に動作。
- [ ] hover に stdlib doc comment が表示される。
- [ ] multi-file workspace で正しく動作する。

## References

- `crates/ark-lsp/src/server.rs`
- `std/manifest.toml`
- `std/**/*.ark`
- `crates/ark-resolve/`
- `crates/ark-typecheck/`
