# Selfhost typechecker に型推論エンジンを構築する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 311
**Depends on**: 308
**Track**: selfhost-frontend
**Blocks v1 exit**: no
**Priority**: 2

## Summary

typechecker.ark を annotation 抽出器から real type checker に引き上げる。現在の typechecker は return type annotation を拾うだけ (実質カバー率 5%) で、式の型推論、call site の引数型検査、型不一致の検出がない。bidirectional type inference と unification を実装し、型エラーを位置情報つきで報告する。

## Current state

- `src/compiler/typechecker.ark` (324 行): 14 種の型タグ、TypeEnv、unify 骨格
- `typecheck_module()` は各 `NK_FN_DECL` の return type annotation を抽出するだけ
- 式の型推論なし: `let x = 1 + 2` で `x` の型が決まらない
- call site の引数型検査なし: `fn(i32)` に string を渡してもエラーにならない
- match arm の型統一なし
- unify は型タグの一致チェックのみ (constraint solving なし)
- 型エラーに位置情報がない

## Acceptance

- [x] `let x = 1 + 2` で `x: i32` が推論される
- [x] `fn(i32)` に string を渡すと compile error が出る
- [x] match arm の型が不一致だと error が出る
- [x] unify failure のエラーメッセージに source 位置情報が含まれる
- [x] selfhost 自身のコードが type error なく通る

## References

- `src/compiler/typechecker.ark` — selfhost typechecker 本体
- `crates/ark-typecheck/src/` — Rust typechecker (8 ファイル、checker/ に 5 module)
- `crates/ark-typecheck/src/checker/` — bidirectional inference 実装
