# Defer status — dispatch-20260615

Generated: 2026-06-15  
Plan: [Remaining Issues Plan](../../../../.cursor/plans/remaining_issues_plan_7c9d204b.plan.md) — Wave **Defer / 監視のみ**

These issues are intentionally **not dispatched** in the current remaining-issues round. They stay open (or blocked) with documented re-open gates.

---

## Summary

| ID | File | Queue | Why deferred | Re-open when |
|----|------|-------|--------------|--------------|
| **474** | `issues/open/474-async-component-support-v5.md` | deferred | v5/T5 async component work; `stream<T>` / `future<T>` still E0402 | Component Model async spec + wasmtime async runtime policy agreed; Phase 1 design slice filed |
| **646** | `issues/open/646-t5-wasm32-wasi-p3-target-scaffold.md` | deferred | T5 target has no backend; depends on #474 async v5 boundary | #474 Phase 1 lands; T5 scope (scaffold vs full P3) agreed in `docs/target-contract.md` |
| **649** | `issues/open/649-t4-native-full-lowering.md` | deferred | T4 is scaffold-only (#641); full lowering is design-only, large scope | ADR on selfhost-native lowering (asm scope, host ABI, GC model); compile-only vs `run_supported` boundary decided |
| **030** | `issues/open/036-jco-javascript-interop.md` | deferred | `partially-blocked`: wasmtime scalar path done; Node/jco + `greet(String)` incomplete | Scalar jco Node smoke passes in CI (`ARUKELLT_TEST_JCO=1`); string canonical ABI adapters or documented permanent exclusion |
| **037** | `issues/blocked/037-jco-gc-support.md` | blocked (external) | External dependency on jco GC transpile (see upstream check below) | Unblock condition #1 verified + #030 Node gate green, or Option C (canonical ABI adapter) chosen |

---

## #474 — Async component support (v5)

**Why deferred**

- Tracked as v5/T5 tier work per `issues/done/035-v2-verification-cleanup.md` and `docs/current-state.md` Known Limitations.
- Synchronous WASI P2 and scalar component export are done; async WIT (`async func`), suspendable MIR, and wasmtime async runtime integration are out of scope for the current P2-focused dispatch.
- `stream<T>` / `future<T>` async resource shapes remain `E0402` rejections.

**Re-open conditions**

1. Bytecode Alliance Component Model async proposal and WASI P3/async I/O direction are stable enough for an Arukellt design slice.
2. wasmtime async execution model is chosen (stackful coroutines vs async/await MIR lowering).
3. A bounded Phase 1 issue is filed (parser/typecheck for `async func` only, or WASI async binding stub).

**Blocks:** #646 (T5 scaffold).

---

## #646 — T5 `wasm32-wasi-p3` target scaffold

**Why deferred**

- `docs/current-state.md`: target id exists but **not-started** — no driver registration, backend, or runtime scaffold.
- Explicitly distinct from, and dependent on, #474 async component v5 work.
- No honest scaffold can be registered until async/P3 boundary is defined.

**Re-open conditions**

1. #474 Phase 1 completes or is split so T5 registration does not imply false P3 runtime claims.
2. `docs/target-contract.md` T5 row updated with scaffold vs full-runtime intent.
3. Driver emits a documented compile-only or clear error for `--target wasm32-wasi-p3`.

---

## #649 — T4 native full lowering

**Why deferred**

- #641 delivered **scaffold-only** T4: `native::emit_native_scaffold` compile-only GNU asm stub; `run_supported=false`.
- Full selfhost-native lowering is a multi-phase effort (#529 Phase 7 follow-up); no LLVM revival (#586 removed `ark-llvm`).
- Orchestration class is `design-ready` — needs ADR before implementation dispatch.

**Re-open conditions**

1. Design ADR: asm backend scope, host ABI, GC/runtime model for native target.
2. Decision: pursue `run_supported=true` for a minimal fixture set **or** document compile-only as permanent boundary.
3. `docs/current-state.md` and `docs/target-contract.md` T4 rows updated to match the decision.

---

## #030 — jco JavaScript interop (`036-jco-javascript-interop.md`, ID 30)

**Why deferred**

- Status: `partially-blocked` on upstream #037 and Arukellt canonical ABI gaps.
- **Done:** wasmtime CLI scalar interop (`tests/component-interop/jco/calculator/run.sh`, check 17 via `ARUKELLT_TEST_COMPONENT=1`).
- **Open:** Node.js `test.mjs` jco transpile+import path; `greet(String) -> String` blocked on canonical ABI string adapters (#029 area).
- v2 roadmap jco completion criterion cannot close until Node path is green.

**Re-open conditions**

1. #037 upstream GC transpile verified on Arukellt T3 calculator fixture (see below).
2. `npx jco transpile` + Node import smoke for scalar exports (`add`, `mul`, `negate`) wired under `ARUKELLT_TEST_JCO=1`.
3. String export: either canonical ABI adapters implemented **or** acceptance explicitly narrowed to scalar-only with docs update.

---

## #037 — jco Wasm GC support (blocked, external)

**Why deferred / blocked**

- Arukellt T3 components embed Wasm GC type definitions in the core module type section even for scalar-only exports.
- Historical blocker: jco ≤1.17.x failed with `array indexed types not supported without the gc feature`.
- Arukellt-side work for #036 is implemented; this issue tracks **external** jco readiness.

### Last verified upstream status (2026-06-15)

| Check | Result |
|-------|--------|
| **jco version** | `@bytecodealliance/jco@1.23.0` (`npx jco --version`) |
| **Fixture** | `tests/component-interop/jco/calculator/calculator.component.wasm` |
| **Transpile** | **PASS** — `npx jco transpile calculator.component.wasm -o /tmp/jco-out` exits 0; emits `calculator.component.js` + `.d.ts` |
| **Node invoke (scalar)** | **NOT VERIFIED** — no `test.mjs` gate; jco 1.23 uses `"use components"` module shape (not legacy `instantiate` export) |
| **String export `greet`** | **BLOCKED (Arukellt)** — canonical ABI string lift/lower not implemented; unrelated to jco GC |
| **Prior doc claim** | `issues/blocked/037-jco-gc-support.md` cites jco 1.13.2 GC support (2025-07-15); local transpile re-confirmed 2026-06-15 on 1.23.0 |

**Assessment:** Upstream GC transpile blocker for scalar calculator component appears **resolved** in jco ≥1.23. Issue remains in `issues/blocked/` until:

1. Arukellt adds Node/jco smoke test and confirms scalar invoke, **and**
2. #030 acceptance for jco path is updated, **or**
3. Team explicitly chooses Option C (canonical ABI adapter bypass) from #037.

**Re-open conditions** (from issue, unchanged)

1. `npx jco transpile calculator.component.wasm -o ./out` succeeds **and** Node smoke asserts scalar exports — **transpile half met 2026-06-15; invoke half pending**.
2. Option C feasibility confirmed and implementation approved.

**Tracking:** [bytecodealliance/jco](https://github.com/bytecodealliance/jco) releases; periodic re-run of transpile + planned Node gate.

---

## Orchestration linkage

- Machine queue: [`queues.json`](queues.json) → `queues.deferred`
- Open index: [`issues/open/index.md`](../../../issues/open/index.md)
- Blocked index row: issue ID 31 → `issues/blocked/037-jco-gc-support.md`
