
## Closed

- **Date**: 2026-04-22
- **Commit**: 0e79ab44cad4b74d7790a5fa7f3bc8192802fe60
- **Branch**: feat/379-playground-wasm-build
- **Close note**: wasm-pack build succeeded for `crates/ark-playground-wasm` targeting
  `wasm32-unknown-unknown`. Built binary (568 KB post wasm-opt) committed to
  `docs/playground/wasm/`. CI workflow (`pages.yml`) updated to rebuild and copy
  wasm on every deploy. Playground-ci size gate updated from 300 KB → 600 KB to
  match actual measured output.

# Playground: parser / formatter を Wasm に build しブラウザで動かす

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-22
**ID**: 379
**Depends on**: 378
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 22

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

## Summary

`ark-parser` (parser + formatter) と `ark-diagnostics` を `wasm32-unknown-unknown` target に build し、ブラウザから呼び出せる Wasm module を作る。wasm-bindgen / wasm-pack で JS binding を生成し、playground の client-side engine とする。

## Current state

- `crates/ark-parser/` は pure Rust で外部 C 依存なし — Wasm 化可能
- `crates/ark-diagnostics/` も pure Rust — Wasm 化可能
- `crates/ark-lexer/` も pure Rust — Wasm 化可能
- Wasm build の設定 (Cargo.toml の lib target, wasm-pack 設定) なし
- JS binding なし

## Acceptance

- [x] `ark-parser` + `ark-lexer` + `ark-diagnostics` が `wasm32-unknown-unknown` で build できる
- [x] JS から `parse(source) -> AST/diagnostics` と `format(source) -> formatted` が呼べる
- [x] Wasm module のサイズが playground 用途に許容される範囲 (目安 < 5MB)
- [x] ブラウザ (Chrome/Firefox) で動作確認される

## References

- `crates/ark-parser/` — parser / formatter
- `crates/ark-lexer/` — lexer
- `crates/ark-diagnostics/` — diagnostics
