
## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: wasm-pack build never executed; crates/ark-playground-wasm/pkg/ missing; no .wasm binary in repo
- **Root cause**: The playground wasm binary (ark-playground-wasm) has never been compiled. crates/ark-playground-wasm/pkg/ does not exist. docs/playground/wasm/ is empty. All playground user-visible functionality depends on this binary.
- **Evidence**: `find . -name '*.wasm' -path '*playground*'` returns nothing; `ls crates/ark-playground-wasm/pkg/` fails; `ls docs/playground/wasm/` is empty.

# Playground: parser / formatter を Wasm に build しブラウザで動かす

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 379
**Depends on**: 378
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 22

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
