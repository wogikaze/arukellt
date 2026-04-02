# Selfhost の統一バイナリを生成する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-08
**ID**: 323
**Depends on**: 317
**Track**: selfhost-verification
**Blocks v1 exit**: no
**Priority**: 14

## Summary

現在 `src/compiler/*.ark` は個別に `.wasm` にコンパイルされるが、1 つの compiler binary としてリンクされていない。main.ark が全 module を含む統一 `arukellt-s1.wasm` を生成する仕組みを構築する。verify-bootstrap.sh の Stage 1 が実行可能になる前提。

## Current state

- Stage 0 で 9 個の `.ark` が個別に `.wasm` に compile される
- `main.wasm` (78KB) は存在するが、他の module (lexer, parser 等) をリンクしていない
- verify-bootstrap.sh は `main.wasm` を統一バイナリとして期待するが、存在しないため Stage 1 を skip する
- Rust 版は Cargo workspace のリンクで単一 binary を生成

## Acceptance

- [x] `arukellt compile src/compiler/main.ark -o arukellt-s1.wasm` が全 module を含む単一 wasm を生成する
- [x] `wasmtime arukellt-s1.wasm -- compile hello.ark` が `hello.wasm` を生成する
- [x] 生成バイナリのサイズと module 構成が記録される
- [x] verify-bootstrap.sh が Stage 1 を skip しなくなる

## References

- `scripts/run/verify-bootstrap.sh:175-180` — main.wasm を期待するロジック
- `src/compiler/main.ark` — selfhost entry point
- `src/compiler/ark.toml` — selfhost manifest
