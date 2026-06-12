---
Status: done
Created: 2026-04-14
Updated: 2026-06-12
ID: 500
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Source: audit — issues/done/472-playground-type-checker-product-claim.md
Track: main
Orchestration class: implementation-ready
Depends on: none
---

## Reopened by audit — 2026-06-12

**Reopen reason:** Acceptance cites deleted `crates/ark-playground-wasm` typecheck export. Playground uses TypeScript parse shim, not ark-typecheck.

**Violated acceptance:** All acceptance items (typecheck export, playground wiring, native tests)

**Evidence files:**
- `issues/done/500-playground-wasm-typecheck-export.md`
- `playground/src/engine.ts`
- `glob: crates/ark-playground-wasm/ (absent)`

**Follow-up split issue:** none (scope unchanged)

- [x] Replacement accepted: the deleted `crates/ark-playground-wasm` crate was not recreated.
- [x] `playground/src/compiler-host.ts` exports `checkWithCompilerWasm()` / `checkWithCompilerWasmSync()` backed by the selfhost compiler wasm `check --json` command.
- [x] `playground/src/engine.ts` exposes compiler-backed `typecheckSourceWithCompilerBytes*()` helpers and no longer returns parse-only diagnostics from `typecheckSource()`.
- [x] `playground/src/worker.ts` loads the selfhost compiler wasm from `wasmUrl` and invokes the real typecheck helper for `typecheck` requests.
- [x] `playground/src/tests/typecheck-close-gate.test.ts` exercises a parse-clean type error through the selfhost compiler wasm path.

## Reopened by audit

## Summary

Issue #472 audited the playground type-checker product claim and found that
`crates/ark-playground-wasm/src/lib.rs` exports only `parse`, `format`,
`tokenize`, and `version`. There is no `typecheck` export backed by
`ark-typecheck`. The playground browser entrypoint and TypeScript source
(`playground/src/`) have no typecheck invocation.

This issue tracks adding the `typecheck` wasm binding and wiring it into the
playground frontend, which is the prerequisite for closing #472.

## Primary paths

- `playground/src/`

## Non-goals

- Changing compiler architecture
- Full IDE-quality diagnostics in playground (incremental work)

## Acceptance

- [x] Selfhost replacement for the deleted Rust wasm export exists in `playground/src/compiler-host.ts`
- [x] `playground/src/engine.ts`, `playground/src/playground.ts`, and `playground/src/worker.ts` call the selfhost checker path and surface `TypecheckResponse`; `createPlayground().parse()` also surfaces checker diagnostics for the app when compiler wasm is loaded
- [x] `playground/src/tests/typecheck-close-gate.test.ts` verifies a type-phase diagnostic for a parse-clean type error
- [x] `node --test dist/tests/typecheck-close-gate.test.js` passes
- [x] `python3 scripts/manager.py verify quick` was run; this slice's done-issue hygiene passes, with unrelated #487/LSP/DAP failures remaining in the shared workspace

## Required verification

- `npm run build && node --test dist/tests/typecheck-close-gate.test.js` passes
- `python3 scripts/manager.py verify quick` was run; remaining failures are unrelated #487 status mismatch and LSP/DAP lifecycle work in concurrent files

## Close gate

Acceptance items checked; deleted `crates/ark-playground-wasm` was not recreated. The callable checker surface is the selfhost compiler wasm host in `playground/src/compiler-host.ts`, invoked from `playground/src/engine.ts`, `playground/src/playground.ts`, and `playground/src/worker.ts`. The close-gate test proves a parse-clean type error returns `phase === "typecheck"` / `E02*` diagnostics.

## Close evidence (2026-06-12)

- Checker source: `playground/src/compiler-host.ts` — `checkWithCompilerWasm()` / `checkWithCompilerWasmSync()` run `arukellt check --json` in the selfhost compiler wasm.
- Playground wiring: `playground/src/engine.ts`, `playground/src/playground.ts`, `playground/src/worker.ts`.
- Test: `playground/src/tests/typecheck-close-gate.test.ts`.
- Verification: `npm run build && node --test dist/tests/typecheck-close-gate.test.js`; `python3 scripts/manager.py verify quick` (147/150, unrelated #487/LSP/DAP failures).

## Note

Closing this issue is a prerequisite for closing #472 (playground type-checker product claim).
