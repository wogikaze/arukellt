# セルフホスト parser に不足構文を追加する

**Status**: open
**Created**: 2026-03-31
**ID**: 290
**Depends on**: 287
**Track**: main
**Priority**: 10

## Summary

selfhost parser に Rust parser が持つ構文のうち不足しているものがある。fixture parity の前提として parser の完全性が必要。

## Current state

- selfhost parser に不足: `ArrayRepeat` (`[expr; count]`)、`Try` 演算子 (`expr?`)、struct base syntax (`..base`)
- `src/compiler/parser.ark`: NK_* 定数に上記が未定義
- Rust parser `crates/ark-parser/src/ast.rs:254-282`: ArrayRepeat, Try, Assign 等がある

## Acceptance

- [ ] `[expr; count]` 構文 (ArrayRepeat) が selfhost parser で parse できる
- [ ] `expr?` 構文 (Try) が selfhost parser で parse できる
- [ ] `Struct { field: val, ..base }` 構文が selfhost parser で parse できる
- [ ] 追加構文に対する unit test がある

## References

- `src/compiler/parser.ark`
- `crates/ark-parser/src/ast.rs`
