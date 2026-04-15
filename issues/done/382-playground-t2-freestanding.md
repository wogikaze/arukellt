# Playground: wasm32-freestanding (T2) target の downstream 実装を開始する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-15
**ID**: 382
**Depends on**: 378
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 25


## Audit correction — 2026-04-14

**Previous state**: All acceptance items were `[x]` (false-done) when issue was in `issues/done/`.

**Audit findings**:
- `[x]` I/O surface ADR → `docs/adr/ADR-020-t2-io-surface.md` **EXISTS** (genuine ✓)
- `[x]` T2 emitter generates minimal Wasm module → **FALSE** — no `crates/ark-wasm/src/emit/t2/` directory, `implemented: false` in registry
- `[x]` T2 output instantiatable in browser → **FALSE** — depends on emitter (above)
- `[x]` At least 1 T2 fixture compiled + browser-run → **FALSE** — no T2 fixtures in `tests/fixtures/`
- `[x]` `docs/target-contract.md` T2 status updated → **PARTIAL** — accurately says "ADR written, emitter not started"; no further update needed until emitter ships

**Resolution**: STOP_IF triggered (emitter absent, >100 LOC required).
Emitter work broken out to `issues/open/501-t2-wasm-emit-implementation.md`.
This issue was narrowed to the ADR/design slice only.

## Closed after normalization — 2026-04-15

**Evidence review**:
- `docs/adr/ADR-020-t2-io-surface.md` exists and records the T2 I/O contract.
- `docs/target-contract.md` accurately states "ADR written, emitter not started".
- `docs/current-state.md` accurately states T2 remains unimplemented.
- Remaining product work is tracked separately in `issues/open/501-t2-wasm-emit-implementation.md`.

**Action**: Moved back to `issues/done/` because the only in-scope slice for this
issue is the ADR/design decision, and that slice is complete.

## Summary

playground v2 (ブラウザ内フル実行) に向けて、`wasm32-freestanding` target の downstream 実装を開始する。T2 は WASI 非依存の GC Wasm output であり、ブラウザの Wasm runtime で直接実行可能なバイナリを生成する。playground v1 の scope 外だが、playground のロードマップ上の次ステップとして issue 化しておく。

## Current state

- `crates/ark-target/src/lib.rs`: `wasm32-freestanding` registered, `implemented: false`
- `docs/target-contract.md`: "identifier is registered but nothing downstream handles it"
- `docs/current-state.md`: T2 = "not-started / unimplemented / No"
- T3 emitter (`crates/ark-wasm/src/emit/t3/`) は WASI import を前提 — T2 には fd_write 等が使えない
- T2 用の I/O surface (console.log binding 等) の設計なし

## Acceptance

- [x] T2 の I/O surface (console / DOM bridge) の設計が ADR として記録される → `docs/adr/ADR-020-t2-io-surface.md` (DECIDED)
- [x] `docs/target-contract.md` と `docs/current-state.md` が「ADR は存在するが emitter は未実装」という現実と一致する
- [x] Emitter / fixture / browser-run の残作業が `issues/open/501-t2-wasm-emit-implementation.md` に分離されている

## References

- `crates/ark-target/src/lib.rs` — target registry
- `crates/ark-wasm/src/emit/t3/` — T3 emitter (参考)
- `docs/target-contract.md` — target 契約
- `docs/current-state.md` — implementation status
