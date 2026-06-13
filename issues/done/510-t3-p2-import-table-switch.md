---
Status: done
Created: 2026-04-15
Updated: 2026-06-13
ID: 510
Track: wasi-feature
Depends on: —
Blocks: 074, 076, 121
Orchestration class: implementation-ready
Implementation target: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.
Blocks v4 exit: no
---

# T3 emitter: WASI P2 import-table switch (full P2-native component)

## Summary

The `--wasi-version p2` / `--p2-native` flags skip the WASI P1 adapter at the
component-wrapping stage, but a fully working P2-native component requires the
T3 emitter to import P2 interface names directly in the core Wasm module.

## Unblocks chain

`#510` (P2 import table) → `#121` (component import wiring can assume P2
interface names) → `#074` (parent P2 native component gate).

## Implementation

- `src/compiler/wasm/sections_imports.ark` — `emit_p2_import_entries` when
  `wasi_version == "p2"`.
- `src/compiler/wasm/emit_target.ark` — P2 import/type counts.
- `src/compiler/driver/emit.ark` — `normalize_wasi_version`.
- `src/compiler/component/emit.ark`, `component_base.ark`, `wasi_p2_stub.ark`.

Fixture: `tests/fixtures/wasi_p2_native/hello.ark`.

## Acceptance

- [x] T3 emitter import-table generation has an explicit P2-native branch while keeping P1 behavior unchanged.
- [x] `WasiVersion` is propagated through backend planning into import-table selection logic.
- [x] `--target wasm32-wasi-p2 --wasi-version p2 --emit component` output validates with `wasm-tools validate`.
- [x] The issue body explicitly documents how this issue unblocks `#121` and the `#510 -> #121 -> #074` chain.
- [x] Regression checks pass for both P1 and P2 paths under existing verification gates.
- [x] `docs/target-contract.md` component output tier table synced with P2 import-table switch status.

## Close note (2026-06-13)

Gate `#510` in `scripts/check/check-false-done-close-gates.py` passes
`wasm-tools validate` on `tests/fixtures/wasi_p2_native/hello.ark`.
