# セルフホスト parser に不足構文を追加する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2025-07-15
**ID**: 290
**Depends on**: —
**Track**: selfhost
**Blocks v1 exit**: no
**Priority**: 10

## Summary

selfhost parser に Rust parser が持つ構文のうち不足しているものがある。fixture parity の前提として parser の完全性が必要。

## Current state

- selfhost parser に不足: `ArrayRepeat` (`[expr; count]`)、`Try` 演算子 (`expr?`)、struct base syntax (`..base`)
- `src/compiler/parser.ark`: NK_* 定数に上記が未定義
- Rust parser `crates/ark-parser/src/ast.rs:254-282`: ArrayRepeat, Try, Assign 等がある

## Acceptance

- [x] `[expr; count]` 構文 (ArrayRepeat) が selfhost parser で parse できる
- [x] `expr?` 構文 (Try) が selfhost parser で parse できる
- [x] `Struct { field: val, ..base }` 構文が selfhost parser で parse できる
- [x] 追加構文に対する unit test がある
- [x] Match guards (`pattern if cond =>`) も追加実装

## Verification

- `arukellt check src/compiler/parser.ark` → OK
- `verify-bootstrap.sh --stage1-only` → PASS (9/9 compiled, 96075 bytes)
- `arukellt check tests/fixtures/selfhost/parser_new_syntax.ark` → OK
- New NK_ARRAY_REPEAT (24), NK_TRY (25), NK_MATCH_ARM (26) node kinds added

## References

- `src/compiler/parser.ark`
- `crates/ark-parser/src/ast.rs`
