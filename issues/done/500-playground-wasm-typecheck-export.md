---
Status: done
Created: 2026-04-14
Updated: 2026-06-12
ID: 500
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Source: audit - issues/done/472-playground-type-checker-product-claim.md
Track: main
Orchestration class: implementation-ready
Depends on: none
---

## Summary

The stale acceptance text for this issue pointed at deleted
`crates/ark-playground-wasm`. That crate was not recreated. The playground
now uses the selfhost compiler wasm path for type checking.

## Acceptance

- [x] Replacement checker surface exists in `playground/src/compiler-host.ts`.
- [x] `checkWithCompilerWasm()` and `checkWithCompilerWasmSync()` run
  `arukellt check --json` in the selfhost compiler wasm.
- [x] `playground/src/engine.ts` exposes compiler-backed typecheck helpers and
  no longer returns parse-only diagnostics from `typecheckSource()`.
- [x] `playground/src/playground.ts` and `playground/src/worker.ts` invoke the
  selfhost checker path and surface `TypecheckResponse`.
- [x] `createPlayground().parse()` surfaces checker diagnostics for the
  playground app when compiler wasm is loaded.
- [x] `playground/src/tests/typecheck-close-gate.test.ts` verifies a parse-clean
  type error through the selfhost compiler wasm path.

## Required verification

- `npm run build && node --test dist/tests/typecheck-close-gate.test.js`:
  passed.
- `npm test`: close-gate passed; full suite still has the pre-existing missing
  fixture `/workspace/.build/t2-test/t2_stdio_s3.wasm`.
- `python3 scripts/manager.py verify quick`: run; 147/150 passed, with
  unrelated #487 status mismatch and concurrent LSP/DAP lifecycle failures.

## Close evidence (2026-06-12)

- Checker source: `playground/src/compiler-host.ts`.
- Playground wiring: `playground/src/engine.ts`,
  `playground/src/playground.ts`, `playground/src/worker.ts`.
- Type exports: `playground/src/compiler-types.ts`, `playground/src/index.ts`.
- Test: `playground/src/tests/typecheck-close-gate.test.ts`.
