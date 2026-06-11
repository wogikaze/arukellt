# False-Done Audit Report — 2026-06-12

Audit orchestrator: `prompts/research.md`  
Audit scope: HIGH-RISK `issues/done/` slice (616 total; this run prioritized user-visible claims, explicit reopen notes, component/WIT/stdlib gates, playground typecheck, and status/directory mismatches).  
Verification gate at audit time: `python3 scripts/manager.py verify quick` — 147/147 checks pass.

## 1. Audit summary

| Metric | Count |
|--------|------:|
| Done issues reviewed (targeted) | ~45 |
| **Reopened (must-reopen)** | **6** (wave 1: 5, wave 2: 1) |
| Truly-done spot checks | 14 |
| Stale reopen notes (kept done; evidence now satisfies claim) | 2 (#034, #028b) |
| High-risk remaining in done (monitor) | 1 (#051 random-only partial was reopened) |

Confirmed false-done items were moved from `issues/done/` → `issues/open/` with `Status: open`, audit evidence, and acceptance rollback where applicable. Issue index regenerated via `python3 scripts/gen/generate-issue-index.py`.

## 2. Reopened issues

### #074 — WASI P2 native component (parent gate)

| Field | Value |
|-------|-------|
| Old path | `issues/done/074-wasi-p2-native-component.md` |
| New path | `issues/open/074-wasi-p2-native-component.md` |
| Classification | `must-reopen` / `acceptance-not-actually-met` |
| Reopen reason | Close gate requires P2 stdio, `wasi:cli/run` export shape, validate+run proof; issue body lists ❌ for stdio and wasmtime execution |
| Violated acceptance | Close gate items 3–6; acceptance 3–4 (P1 adapter-free run, size reduction proof) |
| Evidence | `scripts/selfhost/checks.py` (`BOOTSTRAP_COMPONENT_STUB` passthrough), issue close-gate section, `tests/fixtures/wasi_p2_native/hello.ark` (compile-only) |
| Follow-up split | none |

### #510 — T3 P2 import-table switch

| Field | Value |
|-------|-------|
| Old path | `issues/done/510-t3-p2-import-table-switch.md` |
| New path | `issues/open/510-t3-p2-import-table-switch.md` |
| Classification | `must-reopen` (status/directory mismatch + incomplete validation) |
| Reopen reason | Frontmatter `Status: open` while filed under `issues/done/`; close note skips `wasm-tools validate` |
| Violated acceptance | Acceptance #3 (`wasm-tools validate` on P2 component output) |
| Evidence | Issue body `**Status**: open`, close note 2026-04-26 |
| Follow-up split | none |

### #472 — Playground type-checker product claim

| Field | Value |
|-------|-------|
| Old path | `issues/done/472-playground-type-checker-product-claim.md` |
| New path | `issues/open/472-playground-type-checker-product-claim.md` |
| Classification | `must-reopen` / `docs-ahead-of-reality` |
| Reopen reason | `typecheckSource()` delegates to `parseSource()` only; no real typechecker |
| Violated acceptance | Callable checker surface; entrypoint invocation; checker-specific tests |
| Evidence | `playground/src/engine.ts`, prior audit note "CHECKER SURFACE ABSENT", #631 crate removal |
| Follow-up split | none (use #500 for wasm export wiring when re-dispatched) |

### #500 — Playground wasm typecheck export

| Field | Value |
|-------|-------|
| Old path | `issues/done/500-playground-wasm-typecheck-export.md` |
| New path | `issues/open/500-playground-wasm-typecheck-export.md` |
| Classification | `must-reopen` |
| Reopen reason | Acceptance cites deleted `crates/ark-playground-wasm`; prerequisite for #472 not met |
| Violated acceptance | All four acceptance checkboxes |
| Evidence | Missing `crates/ark-playground-wasm/`, `playground/src/engine.ts` |
| Follow-up split | none |

### #051 — std::time + std::random

| Field | Value |
|-------|-------|
| Old path | `issues/done/051-std-time-random.md` |
| New path | `issues/open/051-std-time-random.md` |
| Classification | `must-reopen` / `implementation-parts-only` |
| Reopen reason | Only seeded `std::random` sub-surface is complete; `std::time` and host clock/random intrinsics remain blocked |
| Violated acceptance | `now_unix_ms`, `monotonic_now_ns`, `sleep_ms`; fixture `stdlib_time/monotonic.ark` fails typecheck |
| Evidence | `tests/fixtures/stdlib_time/monotonic.ark` (E0200 i64 vs i32), no `__intrinsic_clock_now` in `src/compiler/` |
| Follow-up split | Consider splitting random (done) vs time/clock (open) in a future wave |

## 3. Newly-created future issues

None required this run. Deferred work already tracked:

- Playground typecheck → #472 / #500 (reopened)
- P2 native component → #074 / #510 (reopened)
- Host clock intrinsics → #051 (reopened); also referenced from #606 docs split

## 4. Still-truly-done (spot checks)

| ID | Area | Evidence |
|----|------|----------|
| 034 | WIT CLI + import binding | `component-compile:wit_import/main.ark` in manifest; compile succeeds |
| 028b | WIT pipeline wiring | `src/compiler/resolver/wit_import_bind.ark`, component emit path |
| 466–471 | Playground entrypoint/deploy/docs | `docs/playground/index.html`, pages workflow, build-path-proof |
| 475, 557, 575 | CLI surface | `src/compiler/main/*.ark` |
| 559, 631 | Phase 5 selfhost-first / delete playground wasm | wrapper scripts, crate absent |
| 628 | LSP MVP | `cmd_lsp`, selfhost lsp-script fixtures |
| 622 | Extension E2E | extension test suite |
| 632 | Playground compiler-wasm loop | `docs/playground/assets/`, `npm test` 173/173, `.build/t2-test/t2_stdio_s3.wasm` |
| 045 | std::collections Deque/PQ | Completion 2026-06-09 + fixtures |
| 121 | Canonical ABI hardening | prior close evidence + component fixtures |

## 5. Docs / extension / CLI / workflow mismatches

| Claim location | Reality | Action |
|----------------|---------|--------|
| `docs/playground/README.md` may imply typecheck | `engine.ts` parse shim only | Fix when #472 closes |
| `docs/current-state.md` P2 native component paragraph | Parent gate #074 not closed | Keep provisional label; reopen #074 |
| Issue #034 stale "moved to open" notes | WIT import fixture now passes | Metadata cleanup optional |

## 6. Evidence table (reopen decisions)

| Issue | Repo proof checked | Result |
|-------|-------------------|--------|
| 074 | bootstrap stub, issue close gate | FAIL — reopen |
| 510 | status in done/, validate skipped | FAIL — reopen |
| 472 | `engine.ts` typecheckSource | FAIL — reopen |
| 500 | ark-playground-wasm absent | FAIL — reopen |
| 051 | monotonic.ark compile | FAIL — reopen |
| 034 | wit_import compile | PASS — keep done |
| 632 | playground npm test | PASS — keep done |

## 7. Dependency updates

- `#510` blocks `#074` (unchanged)
- `#074` blocks `#124` dispatch note remains valid
- `#500` blocks `#472` (restated)
- Index regenerated: `issues/open/index.md`, `issues/open/dependency-graph.md`

## 8. Remaining high-risk false-done items

Monitor on next full audit (not reopened this run due to insufficient new contradicting evidence or already re-closed with fixtures):

- **#124** — blocked note references #074; Phase 1 import syntax exists (`wit_import/main.ark`) but parent P2 gate open
- Historical "moved to open (2026-04-03)" notes in many done issues — most were subsequently re-closed with completion evidence; do not bulk-reopen without per-issue proof

## 9. Checklist items not tracked as issues

`docs/release-criteria.md` pre-release steps (determinism sha256, full verify) are covered by existing CI/issue work (#242 determinism layer, `manager.py verify`). No new checklist tracking issues created.

## 10. Newly-created checklist tracking issues

None.

---

## Wave 2 — 2026-06-12 (continued)

### Additional reopen

#### #123 — import syntax unification

| Field | Value |
|-------|-------|
| Old path | `issues/done/123-import-syntax-unification.md` |
| New path | `issues/open/123-import-syntax-unification.md` |
| Classification | `must-reopen` / `acceptance-not-actually-met` |
| Reopen reason | Close review explicitly says full issue not ready for `done`; only docs/ADR slice was completed |
| Violated acceptance | Implementation work for unified import syntax policy (beyond documentation) |
| Evidence | Issue body lines 425–432 (`Full issue ready to move to done: no`) |

### Audit resolution (kept in done)

#### #034 — WIT CLI integration

`truly-done` after 2026-06-12 recheck: `wit_import/main.ark` and
`component/import_scalar_func.ark` prove callable WIT imports through
`--wit` + `--emit component`. Audit resolution note added to issue file.

### Commit policy

Wave completion commits are mandatory per updated `prompts/research.md`
(`autonomous commit policy` — orchestration-state only, per wave).
