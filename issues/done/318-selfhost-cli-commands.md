---
Status: done
Created: 2026-03-31
Updated: 2025-07-15
ID: 318
Track: selfhost-cli
Depends on: 319
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 12
---

- `src/compiler/main.ark` (272 行): 5 コマンドのみ
- Rust CLI (`crates/arukellt/src/main.rs`): "12+ コマンド (compile, run, build, init, fmt, test, check, targets, script, lsp, debug-adapter, analyze)"
- [x] `arukellt run file.ark` が compile → wasm 出力のワンステップで動作する (wasmtime 実行は process: ":exec 未実装のため手動)"
- `run` は compile + ファイル出力まで行い、wasmtime 実行は案内のみ (process: ":exec が stdlib にないため)"
- `test` は単一ファイル指定のみ (fs: ":read_dir が stdlib にないため)"
# Selfhost CLI にコマンド surface を追加する

## Summary

selfhost CLI に `run` / `build` / `test` コマンドを追加する。現在 main.ark は `parse` / `compile` / `check` / `help` / `version` のみ。ユーザーが日常的に使うコマンドが欠落しており、Rust CLI の代替にはなれない。

## Current state

- `src/compiler/main.ark` (272 行): 5 コマンドのみ
- Rust CLI (`crates/arukellt/src/main.rs`): 12+ コマンド (compile, run, build, init, fmt, test, check, targets, script, lsp, debug-adapter, analyze)
- selfhost に `run` がないため、compile + 手動 wasmtime 実行が必要
- `build` がないため、ark.toml ベースのプロジェクト build ができない
- `test` がないため、fixture 実行ができない

## Acceptance

- [x] `arukellt run file.ark` が compile → wasm 出力のワンステップで動作する (wasmtime 実行は process::exec 未実装のため手動)
- [x] `arukellt build` が ark.toml を読んで entry point から build する
- [x] `arukellt test` がテスト fixture を指定して check する (ディレクトリ探索は read_dir 未実装のため手動指定)
- [x] 各コマンドが `--help` で usage を出力する

## Notes

- `run` は compile + ファイル出力まで行い、wasmtime 実行は案内のみ (process::exec が stdlib にないため)
- `test` は単一ファイル指定のみ (fs::read_dir が stdlib にないため)
- `build` は ark.toml の entry フィールドを解析してプロジェクトビルドする

## Verification

- `arukellt check src/compiler/main.ark` → OK
- `verify-bootstrap.sh --stage1-only` → PASS (9/9 compiled, 92320 bytes)

## References

- `src/compiler/main.ark` — selfhost CLI entry point
- `crates/arukellt/src/main.rs` — Rust CLI (329 行)
- `crates/arukellt/src/commands.rs` — Rust subcommand 定義