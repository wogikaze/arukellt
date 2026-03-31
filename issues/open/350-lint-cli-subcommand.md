# Linter: `arukellt lint` CLI サブコマンドを追加する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 350
**Depends on**: 348
**Track**: linter
**Blocks v1 exit**: no
**Priority**: 17

## Summary

`arukellt lint` サブコマンドを追加し、`check` とは独立に lint rule のみを走らせるエントリーポイントを作る。`check` は semantic error + lint、`lint` は lint のみ。`ark.toml` の default script も `lint = "arukellt lint ."` に変更する。

## Current state

- `crates/arukellt/src/main.rs`: `lint` サブコマンドなし
- `crates/ark-manifest/src/lib.rs`: default script `lint = "arukellt check ."`
- `check` と `lint` の責務が分離されていない

## Acceptance

- [ ] `arukellt lint <path>` が lint rule のみを走らせ、結果を報告する
- [ ] `arukellt lint --list` が利用可能な lint rule を一覧表示する
- [ ] `arukellt lint --allow <rule>` / `--deny <rule>` で CLI から severity を override できる
- [ ] `ark.toml` の default lint script が `arukellt lint .` に更新される
- [ ] `arukellt check` は引き続き semantic error + lint の両方を報告する

## References

- `crates/arukellt/src/main.rs` — CLI entry point
- `crates/arukellt/src/commands.rs` — subcommand 実装
- `crates/ark-manifest/src/lib.rs` — default scripts
