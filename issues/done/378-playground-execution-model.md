---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 378
Track: playground
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 21
---

# Playground: execution model を選定し v1 scope を確定する
- `crates/ark-target/src/lib.rs`: "`wasm32-freestanding` は `implemented: false`, `run_supported: false`"
- `docs/target-contract.md`: T2 は "identifier is registered but nothing downstream handles it"
- `docs/current-state.md`: T2 は "not-started / unimplemented / No"
# Playground: execution model を選定し v1 scope を確定する

## Summary

web playground の実行モデル (client-side / server-side / hybrid) を選定し、v1 で提供する機能 scope を確定する。現在 `wasm32-freestanding` (T2) は未実装であり、T3 は wasmtime 依存のため、ブラウザ内でのフル実行は現時点で困難。v1 は edit + format + parse/check/diagnose + examples + share を中心とし、フル実行は v2 以降に切る判断が妥当。

## Current state

- `crates/ark-target/src/lib.rs`: `wasm32-freestanding` は `implemented: false`, `run_supported: false`
- `docs/target-contract.md`: T2 は "identifier is registered but nothing downstream handles it"
- `docs/current-state.md`: T2 は "not-started / unimplemented / No"
- T3 (wasm32-wasi-p2) は wasmtime 依存、ブラウザ直接実行不可
- parser / formatter は pure Rust で Wasm 化可能

## Acceptance

- [x] execution model が ADR として決定・記録される
- [x] v1 scope が「edit + format + parse + check + diagnostics + examples + share」に確定される
- [x] client-side で動かす surface (parser / formatter / diagnostics) が特定される
- [x] server-side に逃がす surface (full compile / run) の要否が決定される
- [x] T2 target 実装の要否と timeline が playground roadmap から分離される

## References

- `crates/ark-target/src/lib.rs` — target registry
- `docs/target-contract.md` — target 契約
- `docs/current-state.md` — implementation status
- `docs/adr/` — ADR 置き場