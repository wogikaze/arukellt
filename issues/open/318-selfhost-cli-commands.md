# Selfhost CLI にコマンド surface を追加する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 318
**Depends on**: 319
**Track**: selfhost-cli
**Blocks v1 exit**: no
**Priority**: 12

## Summary

selfhost CLI に `run` / `build` / `test` コマンドを追加する。現在 main.ark は `parse` / `compile` / `check` / `help` / `version` のみ。ユーザーが日常的に使うコマンドが欠落しており、Rust CLI の代替にはなれない。

## Current state

- `src/compiler/main.ark` (272 行): 5 コマンドのみ
- Rust CLI (`crates/arukellt/src/main.rs`): 12+ コマンド (compile, run, build, init, fmt, test, check, targets, script, lsp, debug-adapter, analyze)
- selfhost に `run` がないため、compile + 手動 wasmtime 実行が必要
- `build` がないため、ark.toml ベースのプロジェクト build ができない
- `test` がないため、fixture 実行ができない

## Acceptance

- [ ] `arukellt run file.ark` が compile → wasmtime 実行のワンステップで動作する
- [ ] `arukellt build` が ark.toml を読んでプロジェクト build する
- [ ] `arukellt test` がテスト fixture を発見・実行する
- [ ] 各コマンドが `--help` で usage を出力する

## References

- `src/compiler/main.ark` — selfhost CLI entry point
- `crates/arukellt/src/main.rs` — Rust CLI (329 行)
- `crates/arukellt/src/commands.rs` — Rust subcommand 定義
