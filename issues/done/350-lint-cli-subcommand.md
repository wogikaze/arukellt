---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 350
Track: linter
Depends on: 348
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 17
---

# Linter: `arukellt lint` CLI サブコマンドを追加する
- `crates/arukellt/src/main.rs`: `lint` サブコマンドなし
- `crates/ark-manifest/src/lib.rs`: default script `lint = "arukellt check ."`
# Linter: `arukellt lint` CLI サブコマンドを追加する

## Summary

`arukellt lint` サブコマンドを追加し、`check` とは独立に lint rule のみを走らせるエントリーポイントを作る。`check` は semantic error + lint、`lint` は lint のみ。`ark.toml` の default script も `lint = "arukellt lint ."` に変更する。

## Current state

- `crates/arukellt/src/main.rs`: `lint` サブコマンドなし
- `crates/ark-manifest/src/lib.rs`: default script `lint = "arukellt check ."`
- `check` と `lint` の責務が分離されていない

## Acceptance

- [x] `arukellt lint <path>` が lint rule のみを走らせ、結果を報告する
- [x] `arukellt lint --list` が利用可能な lint rule を一覧表示する
- [x] `arukellt lint --allow <rule>` / `--deny <rule>` で CLI から severity を override できる
- [x] `ark.toml` の default lint script が `arukellt lint .` に更新される
- [x] `arukellt check` は引き続き semantic error + lint の両方を報告する

## References

- `crates/arukellt/src/main.rs` — CLI entry point
- `crates/arukellt/src/commands.rs` — subcommand 実装
- `crates/ark-manifest/src/lib.rs` — default scripts