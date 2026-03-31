# Formatter: parse error 時の動作契約を定義する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 344
**Depends on**: —
**Track**: formatter
**Blocks v1 exit**: no
**Priority**: 12

## Summary

`format_source()` が parse error を silent に無視し、壊れた AST をそのまま再印字する問題を解決する。formatter は parse error 時に何をすべきか (fail-fast / partial format / error return) を定義し、CLI と LSP の両方でその契約に従わせる。

## Current state

- `crates/ark-parser/src/fmt.rs:11-16`: `format_source()` が lex errors を `_lex_errors` として捨て、parse diagnostics を無視
- parse error があっても Some(formatted) を返し、壊れた出力をファイルに書き戻す可能性がある
- CLI `cmd_fmt()` (`commands.rs:67-127`) も error check なしで write back
- LSP `formatting()` も同様に全文置換

## Acceptance

- [ ] `format_source()` が parse error 時に `None` を返す (fail-fast)
- [ ] CLI `arukellt fmt` が parse error 時に diagnostics を stderr に出力し、ファイルを変更しない
- [ ] LSP `formatting()` が parse error 時に edit を返さない
- [ ] テストが parse error 入力で formatter が入力を変更しないことを検証する

## References

- `crates/ark-parser/src/fmt.rs:11-16` — `format_source()` error 無視
- `crates/arukellt/src/commands.rs:67-127` — `cmd_fmt()` error check なし
- `crates/ark-lsp/src/server.rs:2731-2761` — LSP formatting
