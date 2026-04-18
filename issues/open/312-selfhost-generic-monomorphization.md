# Selfhost に generic instantiation と monomorphization を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-04-13
**ID**: 312
**Depends on**: 311
**Track**: selfhost-frontend
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no
**Priority**: 11


## Reopened by audit — 2026-04-13

**Reason**: No monomorphization pass.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Summary

型パラメータの具体化と monomorphization pass を実装する。#308 で parse した generic 宣言を typechecker で instantiate し、backend に渡す前に具象型に展開する。Vec<i32> と Vec<String> を異なる具象関数/型として扱えるようにする。

## Current state

- `src/compiler/typechecker.ark`: TypeInfo に `type_var` タグはあるが、substitution / instantiation のロジックがない
- `src/compiler/hir.ark`: generic parameter を追跡する構造はあるが `?T` のまま解決されない
- monomorphization pass は存在しない
- Rust 版は `crates/ark-mir/src/lower/mod.rs` で monomorphization を行う

## Acceptance

- [x] `Vec<i32>` と `Vec<String>` が異なる具象型として扱われる
- [x] generic fn の呼び出しで型引数が推論される
- [x] monomorphization 後の typed function list が backend に渡される
- [x] 未使用の generic instantiation が codegen に含まれない

## References

- `src/compiler/typechecker.ark` — TypeInfo, type_var
- `src/compiler/hir.ark` — HIR generic parameter
- `crates/ark-mir/src/lower/mod.rs` — Rust monomorphization
- `crates/ark-typecheck/src/checker/` — Rust generic instantiation
