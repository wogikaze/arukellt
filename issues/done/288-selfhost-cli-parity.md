# セルフホスト CLI parity を確認する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-08
**ID**: 288
**Depends on**: 287
**Track**: selfhost
**Blocks v1 exit**: no
**Priority**: 8

## Summary

`arukellt compile`, `arukellt check`, `arukellt run` の基本フローが selfhost 版で動作するか未検証。CLIエントリポイント (`main.ark`) のコマンド解析が Rust 版と同等か確認する。

## Current state

- `src/compiler/main.ark`: compile / check / parse コマンドを解析
- Rust CLI: compile / check / run / build / fmt / lsp / dap を持つ
- selfhost 側の CLI 範囲は不明

## Acceptance

- [ ] selfhost CLI で `compile` コマンドが動作し、生成 wasm の実行結果が Rust 版と一致する（バイナリ同一性は求めない）
- [ ] selfhost CLI で `check` コマンドが動作し、エラー有無が Rust 版と一致する
- [ ] `--target`, `--opt-level`, `--emit-mode` フラグが selfhost で動作する
- [ ] 差分リストが文書化される

## References

- `src/compiler/main.ark`
- `crates/arukellt/src/main.rs`
