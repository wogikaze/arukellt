# Selfhost MIR lowering: 制御フローを構築する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 314
**Depends on**: 313
**Track**: selfhost-backend
**Blocks v1 exit**: no
**Priority**: 5

## Summary

if / while / loop / for / match / break / continue を MIR の block + branch 構造に lowering する。#313 で式が動いた後、制御フロー構文をブロックグラフに変換する。

## Current state

- `src/compiler/mir.ark`: MIR_BR, MIR_BR_IF, MIR_BLOCK, MIR_LOOP, MIR_IF の opcode は定義済み
- しかし lower_to_mir() がスタブなので、これらの opcode を生成するコードがない
- alloc_block() / emit_inst() のユーティリティは存在する
- Rust 版は `crates/ark-mir/src/lower/stmt.rs` で 35K 行の制御フロー lowering を行う

## Acceptance

- [ ] `if/else` が `MIR_BR_IF` + block に変換される
- [ ] `while` が `MIR_LOOP` + `MIR_BR_IF` に変換される
- [ ] `match` が分岐チェーン (tag 比較 + `MIR_BR`) に変換される
- [ ] `break` / `continue` が正しい block depth の `MIR_BR` に変換される
- [ ] nested control flow が正しくブロック化される

## References

- `src/compiler/mir.ark` — MIR opcode 定義 (BR, BR_IF, BLOCK, LOOP, IF)
- `crates/ark-mir/src/lower/stmt.rs` — Rust 制御フロー lowering (35K 行)
- `crates/ark-mir/src/lower/pattern.rs` — Rust pattern matching lowering (42K 行)
