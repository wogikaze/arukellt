# Selfhost Wasm emitter: 構造化ブロック出力を実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 316
**Depends on**: 314, 315
**Track**: selfhost-backend
**Blocks v1 exit**: no
**Priority**: 8

## Summary

MIR のブロックグラフを Wasm の構造化制御フロー (block / loop / if / br / br_if) に変換する emit pass を実装する。現在 emitter.ark は opcode 定数と section header は持つが、関数本体のコード生成が欠落している。

## Current state

- `src/compiler/emitter.ark` (513 行): 70+ Wasm opcode 定義、LEB128 encoding、section header 出力
- `emit_mir_inst()` は 43 MIR opcodes 中約 25 のみ処理 (定数、算術、比較)
- 構造化ブロック (block / loop / if) の emit がない
- local.get / local.set の index 割り当てが未完
- nested block depth の管理がない
- Rust 版 `crates/ark-wasm/src/emit/t1/` は 12K 行

## Acceptance

- [x] MIR block graph が valid な Wasm 命令列に変換される
- [x] nested block / loop が正しい depth で br を出す
- [x] `local.get` / `local.set` が正しい index を使う
- [x] 生成 Wasm が `wasm-tools validate` を通る
- [x] 生成 Wasm が wasmtime で実行可能

## References

- `src/compiler/emitter.ark` — selfhost Wasm emitter
- `crates/ark-wasm/src/emit/t1/operands.rs` — Rust T1 operand emit
- `crates/ark-wasm/src/emit/t1/stmts.rs` — Rust T1 statement emit
