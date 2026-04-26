# Selfhost CLI のフラグ semantics を driver に接続する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2025-07-15
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

- [x] DriverConfig に `emit_mode` フィールドが追加される
- [x] `--emit wat` が WAT テキスト出力を生成する (最低限 wasm2wat 相当)
- [x] `--opt-level 1` が最適化 pass を有効にする (pass が存在する範囲で) — フラグは driver に正しく伝搬される。selfhost に最適化 pass はまだ存在しないため挙動変化なし。
- [x] `--target wasm32-wasi-p2` が正しい target config を driver / emitter に伝搬する

## Verification

- `arukellt check src/compiler/main.ark` → OK
- `arukellt check src/compiler/driver.ark` → OK
- `arukellt check src/compiler/emitter.ark` → OK
- `verify-bootstrap.sh --stage1-only` → PASS (9/9 compiled, 82709 bytes)

## References

- `src/compiler/main.ark` — CLI flag parsing
- `src/compiler/driver.ark` — DriverConfig 定義
- `crates/arukellt/src/commands.rs` — Rust 版 flag → config 接続
