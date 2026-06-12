---
Status: done
Created: 2026-04-03
Updated: 2026-06-12
ID: 472
Track: playground
Depends on: 466
Orchestration class: implementation-ready
---

## Reopened by audit — 2026-06-12

**Reopen reason:** Callable type-checker product claim is false: `playground/src/engine.ts` `typecheckSource()` wraps parse diagnostics only. `crates/ark-playground-wasm` was removed in #631.

**Violated acceptance:** Acceptance: callable checker surface; entrypoint invokes real checker; tests verify checker behavior (not parser diagnostics)

**Evidence files:**
- `playground/src/engine.ts (typecheckSource → parseSource)`
- `issues/done/472-playground-type-checker-product-claim.md (audit: CHECKER SURFACE ABSENT)`
- `issues/done/631-phase7-delete-ark-playground-wasm.md`

**Follow-up split issue:** none (scope unchanged)

# Playground: type-checker product claim を独立 issue に分離する

Blocks v1 exit: no
Priority: 8
Audit result: checker surface restored via selfhost compiler wasm — issue closed
The module doc comment explicitly states: "Provides JS-callable APIs for **parsing and formatting**".
1. Do not recreate deleted `crates/ark-playground-wasm`; use the selfhost compiler wasm path.
- Checker source: `playground/src/compiler-host.ts` — `checkWithCompilerWasm()` / `checkWithCompilerWasmSync()` run `arukellt check --json` in the selfhost compiler wasm.
- Frontend wiring: `playground/src/engine.ts`, `playground/src/playground.ts`, `playground/src/worker.ts`, and `playground/src/worker-client.ts` expose/invoke `typecheck`; `createPlayground().parse()` surfaces checker diagnostics for the playground app when compiler wasm is loaded.
- Type definitions: `playground/src/types.ts` already exposes `TypecheckResponse`; `playground/src/index.ts` exports it and the selfhost check host result.
- Tests: `playground/src/tests/typecheck-close-gate.test.ts` passes for a parse-clean type error.
- Verification: `npm run build && node --test dist/tests/typecheck-close-gate.test.js`; `python3 scripts/manager.py verify quick` (147/150, unrelated #487/LSP/DAP failures).

# Playground: type-checker product claim を独立 issue に分離する

## Progress Note — 2026-04-14 (impl-playground audit)

**Audit result: CHECKER SURFACE ABSENT — issue remains open**

Verification commands run:

```
grep -rn "typecheck|type_check|TypeCheck|checker|Checker" crates/ark-playground-wasm/src/
# → no output (FAIL)

grep -rn "typecheck|type_check|TypeCheck|checker|Checker" playground/src/
# → no output (FAIL)
```

Findings:

- `crates/ark-playground-wasm/src/lib.rs` exports exactly four functions:
  `parse`, `format`, `tokenize`, `version`.
  The module doc comment explicitly states: "Provides JS-callable APIs for **parsing and formatting**".
  No `typecheck` export exists. No `ark-typecheck` crate is referenced anywhere in this file.

- `playground/src/` contains no invocation of any checker surface
  (no import, no call, no binding to a typecheck function).

- Issue 466 (browser entrypoint) is done and confirmed:
  `docs/playground/index.html` calls `createPlaygroundApp()`, which is backed
  only by the four wasm exports above. No checker is wired.

Acceptance criteria status:

- Historical audit: callable checker surface exists in repo — **NO** (`typecheckSource` in `playground/src/engine.ts` wraps parse only; `crates/ark-playground-wasm` removed in #631)
- Historical audit: entrypoint invokes checker surface — **NO** (no real typechecker invocation in browser entrypoint)
- Historical audit: command/test/fixture verifies checker behavior — **NO** (no test proves type errors distinct from parse errors)

**This issue must NOT be closed until `ark-typecheck` (or equivalent) is exported
from `crates/ark-playground-wasm/src/lib.rs` and invoked from `playground/src/`.**

Gap to close:
1. Add `#[wasm_bindgen] pub fn typecheck(source: &str) -> String` to `crates/ark-playground-wasm/src/lib.rs`,
   backed by a real invocation of `ark-typecheck`.
2. Add a corresponding invocation in `playground/src/` (e.g., in `playground-app.ts` or `worker-client.ts`).
3. Add a native-target test in `lib.rs` that exercises `typecheck` on known-valid and known-invalid input.

## Summary

`type checking available` を parser / diagnostics wording に紛れ込ませず、独立した product claim として追跡する。callable checker surface と entrypoint-level proof がない限り、この claim は done にしない。

## Visibility

user-visible

## Why this is a separate issue

type-checker claim は parser / format / diagnostics とは別の product claim である。混ぜると parser diagnostics だけで false-done になる。

## Primary paths

- actual checker invocation surface to be determined
- browser entrypoint path from issue 466
- `docs/playground/README.md`

## Allowed adjacent paths

- `crates/ark-playground-wasm/**`
- `playground/src/**`
- checker implementation / invocation files in compiler crates if they become the actual source of proof

## Non-goals

- docs-only wording tweak で claim を成立させること
- deploy / extension exposure
- parser diagnostics を checker proof とみなすこと

## Acceptance criteria

- [x] current repo に callable checker surface が存在し、その source path が issue 本文に明記されている。
- [x] issue 466 の browser entrypoint から、その checker surface が実際に invoke されることを repo files で確認できる。
- [x] checker result を機械的に検証する command / test / fixture が repo に存在する。

## Close evidence (2026-06-12)

- Checker source: `playground/src/compiler-host.ts` — selfhost compiler wasm `check --json` host entry points.
- Engine surface: `playground/src/engine.ts` — compiler-backed `typecheckSourceWithCompilerBytes()` / `typecheckSourceWithCompilerBytesSync()` and no parse-only `typecheckSource()` fallback.
- Entrypoints: `playground/src/playground.ts` invokes the checker path for `typecheck` requests and for `parse()` diagnostics when compiler wasm is loaded; `playground/src/worker.ts` invokes the checker path for worker `typecheck` requests; `playground/src/worker-client.ts` exposes the worker request.
- Types: `playground/src/types.ts`, `playground/src/compiler-types.ts`, `playground/src/index.ts`.
- Tests: `playground/src/tests/typecheck-close-gate.test.ts`.
- Verification: `npm run build && node --test dist/tests/typecheck-close-gate.test.js`; `python3 scripts/manager.py verify quick` (147/150, unrelated #487/LSP/DAP failures).

## Required verification

- checker surface の source path を grep する
- entrypoint から checker invocation を grep する
- command / test / fixture を実行して checker behavior を確認する

## Close gate

- repo 内の現物ファイルが列挙されている
- user-visible entrypoint から checker invocation が確認できる
- parser diagnostics だけを evidence にしない
- docs claim だけで close しない

## Evidence to cite when closing

- checker source file(s)
- entrypoint file(s)
- verification command / test / fixture
- any docs file updated after implementation proof exists

## False-done prevention checks

- Can this be closed with only parts existing? **No**
- Can docs get ahead and still allow close? **No**
- Can extension expose a link and still allow close without route proof? **No**
- Can deploy be claimed without workflow/output proof? **No**
- Does this rely on a repo-external URL as proof? **No**
- Can it be closed without concrete evidence files? **No**
- Does this contain a user-visible claim without entrypoint acceptance? **No**

## False-done risk if merged incorrectly

very high — parse errors や lexer diagnostics を type checking と誤認すると即 false-done になる。
