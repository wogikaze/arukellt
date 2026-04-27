---
Status: Implementation-ready
Created: 2026-04-03
Updated: 2026-04-03
ID: 474
Track: wasm-feature
Depends on: "035 (v2-verification-cleanup, done), 074 (wasi-p2-native-component)"
Orchestration class: blocked-by-upstream
Orchestration upstream: #74
---

# Async Component Support (v5/T5)
**Blocks v4 exit**: no
**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

---

## Reopened by audit — 2026-04-03

**Source**: Future-work gap extracted from `issues/done/035-v2-verification-cleanup.md`.

**Quote**: "Async component support (deferred to v5/T5)"

**Action**: New open issue created per audit rule — deferred items in done issues must
have a corresponding open issue.

---

## Summary

Arukellt v2 components are synchronous (no async WIT functions). This issue tracks adding
async component support as defined by the Component Model async proposal and WASI Preview 2
async interfaces (deferred to v5/T5 tier).

Scope:
- Async WIT function signatures (`async func foo(...)`)
- Suspendable execution (stackful coroutines or async/await lowering in MIR)
- WASI P2 async I/O binding generation
- Interop with `wasmtime`'s async runtime

## Non-goals

- Synchronous WASI P2 capabilities (already done in 447, 073)
- Basic resource types without async drops (done in 032)
- T1/T3 optimizations unrelated to async

## Primary paths

- `crates/ark-mir/src/lib.rs` — MIR async function representation
- `crates/ark-wasm/src/emit/` — component async lowering
- `std/host/` — WASI P2 async host bindings
- `tests/fixtures/component/` — async component fixtures

## Acceptance

- [ ] Arukellt source can declare and call async functions
- [ ] Async WIT function signatures are parsed and type-checked
- [ ] An async WASI P2 function (e.g., `wasi:io/streams@0.2.0#write`) can be called
- [ ] `python scripts/manager.py verify` passes

## Required verification

- New fixtures in `tests/fixtures/component/async_component/`
- `cargo test -p ark-wasm` passes async component tests

## Close gate

All acceptance items checked; async component round-trip smoke test passes in CI.
