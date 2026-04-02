# Playground: wasm32-freestanding (T2) target の downstream 実装を開始する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 382
**Depends on**: 378
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 25

## Summary

playground v2 (ブラウザ内フル実行) に向けて、`wasm32-freestanding` target の downstream 実装を開始する。T2 は WASI 非依存の GC Wasm output であり、ブラウザの Wasm runtime で直接実行可能なバイナリを生成する。playground v1 の scope 外だが、playground のロードマップ上の次ステップとして issue 化しておく。

## Current state

- `crates/ark-target/src/lib.rs`: `wasm32-freestanding` registered, `implemented: false`
- `docs/target-contract.md`: "identifier is registered but nothing downstream handles it"
- `docs/current-state.md`: T2 = "not-started / unimplemented / No"
- T3 emitter (`crates/ark-wasm/src/emit/t3/`) は WASI import を前提 — T2 には fd_write 等が使えない
- T2 用の I/O surface (console.log binding 等) の設計なし

## Acceptance

- [x] T2 emitter が WASI import なしで最小限の Wasm module を生成する
- [x] T2 output がブラウザの Wasm runtime (Chrome/Firefox) でインスタンス化できる
- [x] T2 の I/O surface (console / DOM bridge) の設計が ADR として記録される
- [x] 最低 1 つの fixture が T2 target で compile + browser 実行される
- [x] `docs/target-contract.md` の T2 状態が更新される

## References

- `crates/ark-target/src/lib.rs` — target registry
- `crates/ark-wasm/src/emit/t3/` — T3 emitter (参考)
- `docs/target-contract.md` — target 契約
- `docs/current-state.md` — implementation status
