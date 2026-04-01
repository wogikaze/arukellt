# Selfhost parser の構文 surface を拡張する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2025-07-15
**ID**: 308
**Depends on**: —
**Track**: selfhost-frontend
**Blocks v1 exit**: no
**Priority**: 1

## Summary

selfhost parser (`src/compiler/parser.ark`) に不足している構文 surface を追加する。現在 parser は Rust parser の約 60% をカバーしているが、generic type parameter (`<T>`)、match guard (`if cond`)、struct base syntax (`..base`)、array repeat (`[v; n]`)、doc comment (`///`)、postfix `?` operator の expression-level handling が欠落している。これらがないと resolver / typechecker / MIR lowering に進めない。

## Current state

- `src/compiler/parser.ark` (1411 行): 23 種の式、8 種の文、8 種の宣言を parse
- Pratt parser で 18 種の二項演算子を処理
- generic parameter は一切 parse されない (fn/struct/enum 全て non-generic 前提)
- match arm に guard condition がない
- struct literal で `..base` を受ける構文がない
- `[value; count]` array repeat は未実装
- postfix `?` は token として存在するが式側での処理がない
- attribute (`#[...]`) は未実装
- doc comment (`///`) は未実装

## Acceptance

- [x] `fn foo<T>(x: T) -> T` が parse される (generic type parameter on fn)
- [x] `struct Foo<T> { x: T }` が parse される (generic type parameter on struct)
- [x] `match x { v if v > 0 => ... }` が parse される (match guard)
- [x] `Struct { x: 1, ..base }` が parse される (struct base syntax)
- [x] `[0; 10]` が parse される (array repeat)
- [x] postfix `?` が式として parse される (try operator)
- [x] selfhost parser で selfhost 自身の全 `.ark` ファイルが parse error なく通る

## References

- `src/compiler/parser.ark` — selfhost parser 本体
- `crates/ark-parser/src/parser/expr.rs` — Rust parser の式解析 (804 行)
- `crates/ark-parser/src/parser/decl.rs` — Rust parser の宣言解析 (472 行)
- `crates/ark-parser/src/ast.rs` — canonical AST node 一覧
