# CoreHIR path をデフォルトに昇格する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2025-07-15
**ID**: 284
**Depends on**: 281, 282, 283, 306
**Track**: corehir
**Blocks v1 exit**: no
**Priority**: 4

## Summary

IfExpr / LoopExpr / TryExpr の desugar が完了した後、`--mir-select` のデフォルトを `corehir` に切り替え、全 fixture で検証する。

## Current state

- `crates/arukellt/src/main.rs`: compile/build のデフォルトは `"legacy"`、run は `"corehir"`
- `crates/arukellt/src/commands.rs:1033-1045`: `parse_mir_select()` でパース
- `crates/ark-driver/src/session.rs:445-510`: 選択に基づき MIR を使い分け

## Acceptance

- [x] compile / build / run すべてで `--mir-select` のデフォルトが `corehir` になる
- [x] 全 588+ harness fixture が CoreHIR path で pass する
- [x] `INTERFACE-COREHIR.md` の compile path status が「default」に更新される
- [x] `docs/current-state.md` の Pipeline 節が一本化を反映する

## References

- `crates/arukellt/src/main.rs`
- `crates/ark-driver/src/session.rs`
- `INTERFACE-COREHIR.md`
