# Selfhost CLI のフラグ semantics を driver に接続する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 319
**Depends on**: —
**Track**: selfhost-cli
**Blocks v1 exit**: no
**Priority**: 7

## Summary

main.ark で parse している `--emit`, `--target`, `--opt-level` を driver.ark の DriverConfig に実際に接続し、backend の挙動を変える。現在フラグは parse されるが DriverConfig に `emit_mode` フィールドが存在せず、backend に伝搬しない。

## Current state

- `src/compiler/main.ark`: `--emit` を parse するが、driver::compile() に渡していない
- `src/compiler/driver.ark`: DriverConfig に `target` / `opt_level` / dump flags はあるが `emit_mode` がない
- `--emit wat` を指定しても常に `.wasm` バイナリが出力される
- `--opt-level 1` を指定しても最適化が行われない
- `--target` は config に渡されるが emitter 側で意味を持たない

## Acceptance

- [ ] DriverConfig に `emit_mode` フィールドが追加される
- [ ] `--emit wat` が WAT テキスト出力を生成する (最低限 wasm2wat 相当)
- [ ] `--opt-level 1` が最適化 pass を有効にする (pass が存在する範囲で)
- [ ] `--target wasm32-wasi-p2` が正しい target config を driver / emitter に伝搬する

## References

- `src/compiler/main.ark` — CLI flag parsing
- `src/compiler/driver.ark` — DriverConfig 定義
- `crates/arukellt/src/commands.rs` — Rust 版 flag → config 接続
