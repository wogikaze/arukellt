# Stdlib: prelude 露出面を監査し completion / lint / docs と揃える

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 394
**Depends on**: 361
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 12

## Summary

prelude に何が見えるかを単なる設計判断で終わらせず、実際の compiler / completion / docs / lint が同じ結果を返すように監査する。canonical path が module 側なのに prelude でも見える、という状態を減らす。

## Current state

- prelude と module path の二重露出がツールごとに見え方を変える。
- LSP completion、reference docs、resolver が同一の canonical path を見ていない。
- historical API が prelude 露出によって長く残りやすい。

## Acceptance

- [ ] prelude 露出一覧が自動生成される。
- [ ] completion / docs / resolver の表示結果が一致することを確認するチェックが追加される。
- [ ] canonical path から外れる露出が是正される。
- [ ] 監査結果が docs または current-state に記録される。

## References

- ``std/prelude.ark``
- ``crates/ark-resolve/``
- ``crates/ark-lsp/src/server.rs``
- ``docs/stdlib/reference.md``
