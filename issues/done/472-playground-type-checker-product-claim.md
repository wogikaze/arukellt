---
Status: done
Created: 2026-04-03
Updated: 2026-06-12
ID: 472
Track: playground
Depends on: 466
Orchestration class: implementation-ready
---

# Playground: type-checker product claim

## Summary

The reopened gap was that `playground/src/engine.ts` implemented
`typecheckSource()` by returning `parseSource()` diagnostics. The playground now
has a callable type-checker surface backed by the selfhost compiler wasm
`check --json` path, without recreating the deleted `crates/ark-playground-wasm`
crate.

## Acceptance criteria

- [x] Callable checker surface exists:
  `playground/src/compiler-host.ts` exports `checkWithCompilerWasm()` and
  `checkWithCompilerWasmSync()`.
- [x] Playground engine invokes the checker:
  `playground/src/engine.ts` exposes compiler-backed typecheck helpers and a
  no-compiler diagnostic instead of parse-only success.
- [x] Browser-facing entrypoints invoke the checker:
  `playground/src/playground.ts` routes `typecheck()` requests to the checker
  and uses checker diagnostics from `parse()` when compiler wasm is loaded.
- [x] Worker entrypoint invokes the checker:
  `playground/src/worker.ts` loads compiler wasm from `wasmUrl` and handles
  `typecheck` through the compiler-backed engine helper.
- [x] Worker client exposes the request:
  `playground/src/worker-client.ts` already sends `typecheck` commands.
- [x] Checker behavior is mechanically verified:
  `playground/src/tests/typecheck-close-gate.test.ts` checks a parse-clean
  type error and requires `phase === "typecheck"` or an `E02*` code.

## Close evidence (2026-06-12)

- Checker source: `playground/src/compiler-host.ts`.
- Engine surface: `playground/src/engine.ts`.
- Entrypoints: `playground/src/playground.ts`,
  `playground/src/worker.ts`, `playground/src/worker-client.ts`.
- Types: `playground/src/types.ts`, `playground/src/compiler-types.ts`,
  `playground/src/index.ts`.
- Test: `playground/src/tests/typecheck-close-gate.test.ts`.

## Verification

- `npm run build && node --test dist/tests/typecheck-close-gate.test.js`:
  passed.
- `npm test`: close-gate passed; full suite still has the pre-existing missing
  fixture `/workspace/.build/t2-test/t2_stdio_s3.wasm`.
- `python3 scripts/manager.py verify quick`: run; 147/150 passed, with
  unrelated #487 status mismatch and concurrent LSP/DAP lifecycle failures.
