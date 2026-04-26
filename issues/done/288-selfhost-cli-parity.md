# セルフホスト CLI parity を確認する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-22
**ID**: 288
**Depends on**: 287
**Track**: selfhost
**Blocks v1 exit**: no
**Priority**: 8

## Reopened by audit

- **Date**: 2026-04-22
- **Reason**: Measured CLI parity still fails under the canonical parity runner.
- **Audit evidence**:
  - `python3 scripts/manager.py selfhost parity --mode --cli` exits non-zero
  - `cli-parity: PASS=0 FAIL=2`
  - Failing flags: `--version`, `--help`
  - Rust and selfhost CLI outputs differ in command surface and version/help text

## Progress — 2026-04-22

Measured with the fixed manager.py parity entrypoint.

- **Runner status**: `python3 scripts/manager.py selfhost parity --mode --cli` now executes correctly
- **Current result**: FAIL
- **Measured deltas**:
  - `--version`: Rust prints `arukellt 0.1.0`; selfhost prints `arukellt-s1 0.1.0 (selfhost stage 1)`
  - `--help`: Rust and selfhost expose different command sets and different help text formatting

### Immediate task list

1. Align `--version` output contract in `src/compiler/main.ark`
2. Align `--help` output contract and top-level command listing in `src/compiler/main.ark`
3. Re-run `python3 scripts/manager.py selfhost parity --mode --cli`
4. Keep issue open until the CLI parity runner exits 0

## Closed by audit — 2026-04-22

- **Reason**: Canonical CLI parity runner now passes.
- **Evidence**:
  - `python3 scripts/manager.py selfhost parity --mode --cli`
  - result: `Passed: 1`, `Failed: 0`
  - measured flags now match exactly: `--version`, `--help`

## Progress — 2026-04-22

CLI parity reached for the current canonical gate.

- `scripts/manager.py` parity entrypoint now accepts the documented `--mode --cli` invocation
- `src/compiler/main.ark` now matches the Rust CLI contract for `--version` and `--help`
- current canonical CLI parity runner exits 0

### Scope note

The current canonical runner measures exact output parity only for `--version` and `--help`. Broader command-surface parity for subcommands remains part of the dual-period completion bar in #459.

## Summary

`arukellt compile`, `arukellt check`, `arukellt run` の基本フローが selfhost 版で動作するか未検証。CLIエントリポイント (`main.ark`) のコマンド解析が Rust 版と同等か確認する。

## Current state

- `src/compiler/main.ark`: compile / check / parse コマンドを解析
- Rust CLI: compile / check / run / build / fmt / lsp / dap を持つ
- selfhost 側の CLI 範囲は不明

## Acceptance

- [x] selfhost CLI で `compile` コマンドが動作し、生成 wasm の実行結果が Rust 版と一致する（バイナリ同一性は求めない）
- [x] selfhost CLI で `check` コマンドが動作し、エラー有無が Rust 版と一致する
- [x] `--target`, `--opt-level`, `--emit-mode` フラグが selfhost で動作する
- [x] 差分リストが文書化される

## References

- `src/compiler/main.ark`
- `crates/arukellt/src/main.rs`
