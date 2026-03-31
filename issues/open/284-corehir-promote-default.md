# CoreHIR path をデフォルトに昇格する

**Status**: open
**Created**: 2026-03-31
**ID**: 284
**Depends on**: 281, 282, 283
**Track**: main
**Priority**: 4

## Summary

IfExpr / LoopExpr / TryExpr の desugar が完了した後、`--mir-select` のデフォルトを `corehir` に切り替え、全 fixture で検証する。

## Current state

- `crates/arukellt/src/main.rs`: compile/build のデフォルトは `"legacy"`、run は `"corehir"`
- `crates/arukellt/src/commands.rs:1033-1045`: `parse_mir_select()` でパース
- `crates/ark-driver/src/session.rs:445-510`: 選択に基づき MIR を使い分け

## Acceptance

- [ ] compile / build / run すべてで `--mir-select` のデフォルトが `corehir` になる
- [ ] 全 588+ harness fixture が CoreHIR path で pass する
- [ ] `INTERFACE-COREHIR.md` の compile path status が「default」に更新される
- [ ] `docs/current-state.md` の Pipeline 節が一本化を反映する

## References

- `crates/arukellt/src/main.rs`
- `crates/ark-driver/src/session.rs`
- `INTERFACE-COREHIR.md`
