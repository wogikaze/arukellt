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

---

## Wave 3 — 2026-06-12 (prevention + close-gates)

### Deliverables

| Artifact | Purpose |
|----------|---------|
| `docs/process/false-done-prevention.md` | Root-cause catalog FD-01–FD-10 + close checklist |
| `docs/process/false-done-audit-orchestration-plan.md` | Full-audit slice plan + cloud kickoff blocker |
| `scripts/check/check-false-done-hygiene.py` | Mechanical hygiene in verify quick |
| `scripts/check/check-false-done-close-gates.py` | Re-close gate when tracked issues land in `done/` |
| `playground/src/tests/typecheck-close-gate.test.ts` | #472 / #500 behavioral contract |

### Hygiene fixes (wave 3)

| Issue | Action |
|-------|--------|
| #473 | Removed stale duplicate under `issues/open/` (authoritative copy in `issues/done/`) |
| #057 | Added audit resolution; `stdlib_migration/*` fixtures satisfy close gate |

### Cloud orchestrate status

Cloud kickoff succeeded (`bc-5380e3ed-…`); worker fan-out blocked without
`CURSOR_API_KEY` in the cloud VM. Planner performed serial audit per
`unsupported-in-this-run` (see Wave 3b below).

### Remaining audit surface

~600 `issues/done/` files not yet spot-checked in this run. Slice A (~150 with historical reopen metadata) is next priority.

---

## Wave 3b — 2026-06-11 (host capability surface, cloud agent)

Orchestration note: the `/orchestrate` substrate (`scripts/cli.ts`) is present
as a Cursor plugin skill, but `bun cli.ts run --root` exits immediately with
`CURSOR_API_KEY required` (`agent-manager.ts`), and no such user key is injected
into the cloud VM. Per `prompts/research.md` → `unsupported-in-this-run`,
the absence of spawnable workers is **not** a stop reason; the audit/reopen/
issue-creation work was performed directly by the planner, matching the wave 1–2
methodology.

### 1. Audit theme

The selfhost-first migration (#559 / #583, ADR-029) **deleted the entire Rust
workspace** (`crates/` is absent). The Rust `arukellt_host` runtime — which
provided the Wasmtime linker host functions for HTTP, sockets, UDP, clock, and
random — is gone, and the selfhost compiler never reimplemented those host
intrinsics. The only execution path is now `wasmtime run <selfhost.wasm>`
(`scripts/run/arukellt-selfhost.sh`). The selfhost host-call dispatch
`src/compiler/wasm/call_host_io.ark` handles **only** `env`, `fs`
(exists/read/write), `process::exit`, and `stdio`. Yet `std/manifest.toml` and
the stdlib docs still advertise the deleted-runtime host families as available.

### 2. Reopened issues

#### #446 — std::host::http implementation

| Field | Value |
|-------|-------|
| Old path | `issues/done/446-std-host-http-implementation.md` |
| New path | `issues/open/446-std-host-http-implementation.md` |
| Classification | `must-reopen` / `wired-but-not-user-reachable` + `docs-ahead-of-reality` |
| Reopen reason | Acceptance basis (`crates/arukellt/src/runtime.rs`, `register_http_host_fns`, `HOST_STUB_BUILTINS`, `verify-harness.sh`) all deleted; selfhost compiler has no `__intrinsic_http_*` dispatch; manifest still claims `t1=true,t3=true` |
| Violated acceptance | All 6 completion conditions (T1/T3 `http::get` runnable, fixtures CI-pass, capability-surface enforcement, target matrix, `verify-harness.sh` 13/13) |
| Evidence | `src/compiler/wasm/call_host_io.ark`, `std/manifest.toml` (`std::host::http`), `scripts/run/arukellt-selfhost.sh`, absent `crates/`, absent `scripts/run/verify-harness.sh` |
| Follow-up split | #633 (manifest/docs honesty) |

#### #447 — std::host::sockets implementation

| Field | Value |
|-------|-------|
| Old path | `issues/done/447-std-host-sockets-implementation.md` |
| New path | `issues/open/447-std-host-sockets-implementation.md` |
| Classification | `must-reopen` / `wired-but-not-user-reachable` |
| Reopen reason | Same root cause as #446; `tcp_connect_impl` / `sockets_connect` linker lived in deleted `runtime.rs`; no `__intrinsic_sockets_connect` dispatch in selfhost compiler |
| Violated acceptance | T3 fixture CI-pass, T3 compile pass, T1 diagnostic, capability-surface status, `verify-harness.sh` 13/13 |
| Evidence | `src/compiler/wasm/call_host_io.ark`, `std/manifest.toml` (`std::host::sockets`), absent `crates/`, absent `verify-harness.sh` |
| Follow-up split | #633; real P2 backing tracked by #139 / #074 |

### 3. Newly-created future issues

#### #633 — Reconcile host capability claims with the selfhost execution path

| Field | Value |
|-------|-------|
| Track | stdlib |
| Why it must exist | `std/manifest.toml` + docs advertise `std::host::http` (`t1/t3=true`) and `sockets`/`udp` (`t3=true`) via the deleted `arukellt_host` runtime; user-visible over-claim (drives generated stdlib reference & capability badges) |
| Evidence source | `std/manifest.toml`, `src/compiler/wasm/call_host_io.ark`, `docs/capability-surface.md`, `docs/stdlib/modules/{http,sockets}.md`, `docs/current-state.md` |
| Primary paths | `std/manifest.toml`, `docs/capability-surface.md`, `docs/stdlib/modules/*`, `scripts/gen/generate-docs.py` |
| Acceptance | Manifest/docs no longer advertise unbacked host availability; no active reference to `arukellt_host` as current backing; cross-links to #446/#447/#077/#139 |
| Close gate | `check-docs-consistency.py` rc=0, `manager.py verify quick` rc=0 |
| Checklist item source | n/a (manifest over-claim) |

### 4. Still-truly-done (spot checks this wave)

| ID | Area | Evidence |
|----|------|----------|
| 561 | Delete `crates/ark-mir` | `crates/` absent; `rg ark_mir` outside issues/historical returns nothing. The top-of-file "blocked-by-upstream" line is stale pre-deletion scratch; the Resolution section is correct. Keep done. |
| 445 | std::host::process | `process::exit` dispatched in `call_host_io.ark` and backed by WASI `proc_exit` via wasmtime. (See high-risk note re: `process::abort`.) |
| 506/507/509/584 | Placeholder issues | Legitimate administrative numbering no-ops; no implementation claim. Keep done. |

### 5. Docs / manifest mismatches

| Claim location | Reality | Action |
|----------------|---------|--------|
| `std/manifest.toml` `std::host::http` `availability={t1=true,t3=true}` | No HTTP host fn on selfhost path | Reopen #446 + fix via #633 |
| `std/manifest.toml` `std::host::sockets`/`udp` `t3=true` | No sockets/udp host fn on selfhost path | Reopen #447 + fix via #633 |
| `docs/capability-surface.md`, `docs/stdlib/modules/{http,sockets}.md`, `docs/current-state.md` reference `arukellt_host` / `register_http_host_fns` | Rust runtime deleted | Fix via #633 |

### 6. Evidence table (reopen decisions)

| Issue | Repo proof checked | Result |
|-------|-------------------|--------|
| 446 | `call_host_io.ark` (no http dispatch), absent `crates/`, absent `verify-harness.sh`, manifest over-claim | FAIL — reopen |
| 447 | `call_host_io.ark` (no sockets dispatch), absent `crates/`, manifest `t3=true` | FAIL — reopen |
| 561 | `crates/` absent, no active `ark_mir` refs | PASS — keep done |
| 445 | `process::exit` dispatched + WASI-backed | PASS — keep done (abort gap noted) |

### 7. Dependency updates

- `#633` depends on `#446`, `#447` (index + dependency-graph regenerated).
- `#077` (native P2 http) and `#139` (P2 sockets) remain the native-path tracks.

### 8. Remaining high-risk false-done items (monitor / next wave)

- **#445 process::abort** — manifest declares `abort` (`__intrinsic_process_abort`,
  `t1/t3=true`) but `call_host_io.ark` has no `process::abort` dispatch and no
  `process_abort` exists in `src/`. `process::exit` works; `abort` appears
  unbacked. Smaller sub-surface — flagged, not reopened this wave.
- **std::host::udp** — same deleted-runtime root cause as http/sockets; no
  dedicated done issue, covered by #633 docs honesty + #139 native track.
- **std::host::clock / random** — already reopened as #051 (wave 1); consistent
  with the deleted-`arukellt_host` root cause confirmed this wave.

### 9. Checklist items not tracked as issues

None newly identified this wave (`docs/release-checklist.md` items remain
covered by existing CI/issue work, unchanged from wave 1).

### 10. Newly-created checklist tracking issues

None.

### Verification at wave close

`python3 scripts/manager.py verify quick` → 138 passed / 9 failed. All 9
failures are pre-existing systemic gates (fixture manifest sync, doc-example,
selfhost analysis/LSP/DAP gates #568/#569/#571, docs consistency, compiler
boundary/import-cycle, broken links) untouched by this wave; this wave modified
only `issues/**` + regenerated `issues/open/` indexes. No new failure introduced.

---

## Slice F — 2026-06-12 (release / benchmark / hygiene)

Orchestrator: subplanner `audit-slice-f` (57 done issues in batch; cloud worker
spawn unavailable — planner executed audit directly per `unsupported-in-this-run`).

Cross-check anchors: `scripts/manager.py verify quick`, `docs/release-criteria.md`,
`benchmarks/` (read-only), `mise.toml` bench tasks, hygiene scripts under
`scripts/check/`.

### 1. Audit summary

| Metric | Count |
|--------|------:|
| Done issues reviewed (Slice F batch) | 57 |
| **Reopened (must-reopen)** | **2** |
| Audit-resolved stale reopen metadata (kept done) | 6 (#109, #140, #146, #149, #531, #547) |
| Truly-done spot checks (release #546–556, CI #016/#242/#530–532, hygiene #373–377/#417/#419–421/#424–427) | 49 |

### 2. Reopened issues

#### #418 — orphan/stale file inventory script

| Field | Value |
|-------|-------|
| Old path | `issues/done/418-hygiene-orphan-stale-file-inventory.md` |
| New path | `issues/open/418-hygiene-orphan-stale-file-inventory.md` |
| Classification | `must-reopen` / `acceptance-not-actually-met` |
| Reopen reason | Close evidence cites `scripts/check/check-orphan-inventory.sh`; path absent; no manager.py gate |
| Violated acceptance | Inventory script added; CI/hook callable |
| Evidence | Missing script; `scripts/manager.py` has no orphan inventory gate |
| Follow-up split | none |

#### #422 — large artifact size budget / pruning

| Field | Value |
|-------|-------|
| Old path | `issues/done/422-hygiene-large-artifact-budget-pruning.md` |
| New path | `issues/open/422-hygiene-large-artifact-budget-pruning.md` |
| Classification | `must-reopen` / `acceptance-not-actually-met` |
| Reopen reason | Resolution cites missing `check-orphan-inventory.sh` and `check-admission-gate.sh`; policy docs exist but measurement check absent |
| Violated acceptance | Size measurement script/check; CI/hook visibility |
| Evidence | `docs/retention-policy.md` present; both scripts missing |
| Follow-up split | blocked by #418 |

### 3. Newly-created future issues

None. Missing orphan inventory reimplementation tracked by reopened #418/#422.

### 4. Still-truly-done (representative)

| ID | Area | Evidence |
|----|------|----------|
| 225 | Release criteria doc | `docs/release-criteria.md` + checklist cross-links |
| 531 | Scripts consolidation epic | #532–537 done; manager.py domains; status corrected |
| 546–556 | Release verification slices | 2026-05-17 recheck notes + CI integration job |
| 140 | mise bench workflow | `mise.toml` bench tasks + `benchmark_runner.py` |
| 109 | Benchmark suite | `benchmarks/legacy/*` + runner supersession (#537) |
| 373, 417, 421, 427 | Hygiene gates | `check-generated-files.sh`, `check-asset-naming.sh`, `check-links.sh` in verify quick |
| 465 | Playground false-done audit table | Meta governance issue; no product claim |

### 5. Docs / manager / benchmark mismatches

| Claim location | Reality | Action |
|----------------|---------|--------|
| #418/#422 close notes | `check-orphan-inventory.sh`, `check-admission-gate.sh` missing | Reopened #418, #422 |
| #547 Required Verification | cites `cargo build -p arukellt` | Audit resolution; selfhost wrapper is current path |
| #544 goals | `benchmarks/cpu/` subdirs | Flat `bench_*` layout + `benchmarks/legacy/`; monitor only |
| `docs/release-criteria.md` step 4 | needs wasmtime for determinism smoke | Covered by #547/#555; env prerequisite only |

### 6. Evidence table (reopen decisions)

| Issue | Repo proof checked | Result |
|-------|-------------------|--------|
| 418 | `scripts/check/check-orphan-inventory.sh` | FAIL — reopen |
| 422 | orphan script + admission gate | FAIL — reopen |
| 531 | #533–537 done + manager.py | PASS — keep done (status fix) |
| 555 | release recheck 2026-05-17 | PASS — keep done |
| 140 | `mise.toml` bench tasks | PASS — keep done |

### 7. Dependency updates

- `#422` depends on `#418` (size scan blocked on inventory script)
- Index regenerated after reopen

### 8. Remaining high-risk false-done items (monitor)

- **#426** — cites retired `verify-harness.sh`; hygiene checks now live in `manager.py verify quick` (keep done; doc drift)
- **#544** — directory reorg goals partially met (flat bench layout); no contradicting user-visible claim

### 9. Checklist items not tracked as issues

`docs/release-criteria.md` pre-release steps remain covered by #242/#555/#547 and
`manager.py verify` gates. No new checklist tracking issues.

### 10. Newly-created checklist tracking issues

None.

### Verification at slice close

`python3 scripts/manager.py verify quick` — orchestration-state diff only (`issues/**`);
pre-existing failures unchanged from prior waves.

---

## Slice A — FD-01 historical reopen metadata (2026-06-12)

Workspace: `.orchestrate/audit-slice-a/`  
Contract: `prompts/research.md`  
Scope: all `issues/done/*.md` whose body or frontmatter records `Moved from issues/done/ → issues/open/` (FD-01 pattern), including `→` arrow notation missed by the original hygiene regex.

### 1. Audit summary

| Metric | Count |
|--------|------:|
| FD-01 candidates (historical move metadata) | 156 |
| FD-01 frontmatter-stale (move note, no resolution section) | 57 |
| Classified `truly-done` (kept in done) | 149 |
| **Reopened (`must-reopen` with repo proof)** | **7** |
| Audit resolution notes added (stale metadata cleanup) | 50 |

Orchestration note: `bun cli.ts run` unavailable in cloud VM (`bun` not on PATH, `CURSOR_API_KEY` unset). Subplanner executed Slice A serially per `prompts/research.md` unsupported-in-this-run policy.

### 2. Reopened issues

| ID | Old path | New path | Reopen reason | Evidence |
|----|----------|----------|---------------|----------|
| #064 | `issues/done/064-wasm-branch-hinting.md` | `issues/open/064-wasm-branch-hinting.md` | Branch-hint emission not in selfhost | No `branch_hint` in `src/compiler/` |
| #067 | `issues/done/067-wasm-sign-extension-ops.md` | `issues/open/067-wasm-sign-extension-ops.md` | Sign-extension Wasm ops absent | No `extend8_s` / `SignExtend` in emitter |
| #070 | `issues/done/070-wasm-i31ref-scalar.md` | `issues/open/070-wasm-i31ref-scalar.md` | i31ref/scalar support absent | No `i31` refs in `src/compiler/` |
| #080 | `issues/done/080-mir-licm.md` | `issues/open/080-mir-licm.md` | MIR LICM never landed | `crates/ark-mir` deleted; no `licm` in selfhost |
| #082 | `issues/done/082-mir-gc-hint.md` | `issues/open/082-mir-gc-hint.md` | GC-hint pass absent | No `gc_hint` in selfhost |
| #083 | `issues/done/083-mir-loop-unrolling.md` | `issues/open/083-mir-loop-unrolling.md` | Loop-unroll pass absent | No unroll pass in selfhost |
| #115 | `issues/done/115-wasm-name-section.md` | `issues/open/115-wasm-name-section.md` | Custom name section absent | No name-section emitter in `src/compiler/wasm/` |

All seven had FD-01 stale frontmatter (`Action: Moved … → issues/open/`) with only a 2026-04 `Reopened by audit` note and no subsequent Close note / Audit resolution / fixture proof.

### 3. Truly-done (stale metadata only)

149 candidates remain in `issues/done/`. Breakdown:

| Sub-class | Count | Handling |
|-----------|------:|----------|
| Pre-existing resolution / Close / Slice-complete sections | 99 | No change |
| Audit resolution added 2026-06-12 (fixture or selfhost proof) | 50 | `## Audit resolution — 2026-06-12` appended |
| Already had audit resolution (#034, #057, …) | included above | — |

Representative selfhost-backed resolutions: #100 (`--time`), #101 (`--opt-level`), #132/#134/#135/#128/#129 (modularized `src/compiler/`), #281–#284 (CoreHIR lowering), #109/#140 (benchmark workflow).

### 4. Monitor (not reopened — insufficient contradicting proof this slice)

~43 FD-01-stale issues cite deleted `crates/ark-*` paths for Rust-era MIR/opt/bench/docs work without a standardized `## Close note`, but acceptance is either (a) satisfied by selfhost successors, (b) backed by `tests/fixtures/manifest.txt`, or (c) non-user-visible roadmap items deferred to a future selfhost opt track. These stay done with new audit-resolution notes where proof was found; otherwise flagged for Slice G mechanical spot-check.

Notable monitors (stay done): #046 `stdlib_collections_ordered/*` fixtures; #073 WASI P1 imports in `src/compiler/wasm/sections_imports.ark`; #125–#127 CoreHIR path items with partial selfhost coverage.

### 5. Hygiene finding (out of Slice A reopen scope)

`check-false-done-hygiene.py` still reports **FD-02** on #487 (`Status: fixed` under `issues/done/`). Pre-existing; not introduced by Slice A.

### 6. Verification at slice close

`python3 scripts/manager.py verify quick` → **143 passed / 6 failed**. Failures unchanged from pre-existing systemic gates (#568/#569/#571, doc-example, docs consistency drift, #487 FD-02). No new failure from Slice A issue moves. `issues/open/index.md` and `issues/open/dependency-graph.md` regenerated.

## Wave 4 — 2026-06-12 (Slice B: user-visible surfaces)

Orchestration note: `/orchestrate` substrate (`bun cli.ts run`) unavailable in cloud VM
(no `CURSOR_API_KEY`). Subplanner performed serial audit per `prompts/research.md`
`unsupported-in-this-run` policy, matching waves 1–3b methodology.

### Scope

Tracks audited: `playground`, `playground-deploy`, `playground-audit`, `docs-audit`,
`cli`, `docs`, `docs/ops`, `vscode-ide`, `parallel` (user-visible subset).
~100 done issues in scope; spot-checked all user-visible acceptance claims against
`playground/`, `docs/playground/`, `extensions/`, `src/compiler/main/`, `src/compiler/lsp/`,
`.github/workflows/pages.yml`, `.github/workflows/playground-ci.yml`.

### Reopened issues (7)

| ID | Area | Classification | Root cause |
|----|------|----------------|------------|
| 464 | CLI init templates | `acceptance-not-actually-met` | `cmd_init` emits single minimal scaffold only |
| 456 | CLI doc command | `implementation-parts-only` | Minimal markdown reader; no `--json`/`--target`/fuzzy |
| 491 | Playground CI gates | `acceptance-not-actually-met` | `playground-wasm-size` job cited but absent from workflow |
| 216 | Formatter surface | `acceptance-not-actually-met` | No LSP formatting or VS Code formatter contribution |
| 217 | Code actions | `acceptance-not-actually-met` | No `textDocument/codeAction` in selfhost LSP |
| 219 | LSP completeness | `acceptance-not-actually-met` | No signatureHelp/inlayHint/foldingRange handlers |
| 440 | VS Code fix pipeline | `implementation-parts-only` | Deleted Rust fix pipeline; no LSP fix-all integration |

### Still-truly-done (Slice B spot checks)

| ID | Area | Evidence |
|----|------|----------|
| 466–471, 489 | Playground entrypoint/docs/deploy | `docs/playground/index.html`, `pages.yml`, build-path-proof |
| 498 | Playground Lighthouse CI | `.github/workflows/playground-ci.yml` job `playground-lighthouse` |
| 632 | Playground compiler-wasm loop | `docs/playground/assets/compiler-asset.json`, playground tests |
| 453, 622 | Extension E2E | `extensions/arukellt-all-in-one/src/test/extension.test.js` |
| 461 | Doc example CI | `scripts/check/check-doc-examples.py` in verify quick |
| 475, 575 | CLI surface wiring | `src/compiler/main/dispatch_aux.ark` |
| 480 | Extension README settings | `extensions/arukellt-all-in-one/README.md` Extension Settings table |
| 301–304 | Docs ops hygiene | verify quick docs-consistency gates (pre-existing drift aside) |
| 194, 203 | Parallel planning slices | Acceptance is tracking-only; close notes document planning scope |

### Newly-created future issues

None. Gaps covered by reopened issues above.

### Verification at wave close

`python3 scripts/manager.py verify quick` → 143 passed / 6 failed. Failures are
pre-existing (#568 analysis, #569 LSP, #571 DAP, doc-example, docs consistency,
issue #487 hygiene FD-02 outside Slice B scope). This wave modified only `issues/**` +
regenerated indexes. No new failure introduced by reopen moves.

## Slice C — 2026-06-12 (component / WIT / WASI, #074 dependency graph)

Orchestration note: `/orchestrate` worker fan-out unavailable (`CURSOR_API_KEY`
unset). Subplanner performed audit directly per `prompts/research.md`
`unsupported-in-this-run` policy.

### Scope

~32 `issues/done/` files in component / WIT / WASI / CABI cluster, cross-checked
against `tests/fixtures/manifest.txt` (102 `component-compile:` entries),
`src/compiler/component/` (115 modules), and open #074 dependency graph
(#074, #510, #124, #476).

### 1. Audit summary

| Metric | Count |
|--------|------:|
| Done issues reviewed | 32 |
| **Reopened (must-reopen)** | **7** |
| Truly-done (spot checks) | 22 |
| Prior resolution retracted | 1 (#034 wave-2 truly-done claim) |
| New future issues | 0 (gaps tracked by #124, #476, #051) |

### 2. Reopened issues

#### #618 — WIT bindings round-trip

| Field | Value |
|-------|-------|
| Old path | `issues/done/618-wit-bindings-round-trip.md` |
| New path | `issues/open/618-wit-bindings-round-trip.md` |
| Classification | `must-reopen` / `acceptance-not-actually-met` |
| Reopen reason | Acceptance all `[x]` but design spec §6 phases unchecked; only skeleton `roundtrip.ark`; no `run.sh` or bindings-generation step |
| Violated acceptance | Items 2–4 (bindings gen, round-trip smoke, stable runner) |
| Evidence | `tests/component-interop/native/roundtrip/roundtrip.ark` only; issue design spec |
| Follow-up split | none |

#### #443 — component composition / linking

| Field | Value |
|-------|-------|
| Old path | `issues/done/443-component-composition-linking-model.md` |
| New path | `issues/open/443-component-composition-linking-model.md` |
| Classification | `must-reopen` / `implementation-parts-only` |
| Reopen reason | `arukellt compose` returns `CMD_NOT_YET()`; no linking model on selfhost path |
| Violated acceptance | All five items (compose, dependency graph, conflict detection) |
| Evidence | `src/compiler/main/commands.ark`; `issues/open/476-wasm-tools-compose-integration.md` |
| Follow-up split | none (#476 active) |

#### #118 — multi-export WASI world

| Field | Value |
|-------|-------|
| Old path | `issues/done/118-wasm-multi-export-world.md` |
| New path | `issues/open/118-wasm-multi-export-world.md` |
| Classification | `must-reopen` |
| Reopen reason | `--world wasi:cli/command` CLI flag absent; never re-closed after 2026-04-03 reopen |
| Violated acceptance | All four items |
| Evidence | No `--world` in `src/compiler/main/`; blocked by #074 |
| Follow-up split | none |

#### #117 — WIT generation quality

| Field | Value |
|-------|-------|
| Old path | `issues/done/117-wasm-component-model-wit-gen-quality.md` |
| New path | `issues/open/117-wasm-component-model-wit-gen-quality.md` |
| Classification | `must-reopen` / `acceptance-not-actually-met` |
| Reopen reason | References deleted `crates/ark-wasm`; no `wasm-tools component wit` CI gate |
| Violated acceptance | All five items |
| Evidence | Absent `crates/`; no dedicated WIT quality fixture in manifest |
| Follow-up split | none |

#### #073 — WASI P1 full syscalls

| Field | Value |
|-------|-------|
| Old path | `issues/done/073-wasi-p1-full-syscalls.md` |
| New path | `issues/open/073-wasi-p1-full-syscalls.md` |
| Classification | `must-reopen` / `acceptance-not-actually-met` |
| Reopen reason | Claims 46 syscalls; only 3 `stdlib_host` smoke fixtures |
| Violated acceptance | Items 1–3 |
| Evidence | `tests/fixtures/manifest.txt`; `src/compiler/wasm/sections_imports.ark` |
| Follow-up split | Consider clock/random/args vs remainder |

#### #138 — std::host shared capabilities T1/T3

| Field | Value |
|-------|-------|
| Old path | `issues/done/138-std-wasi-shared-capabilities-t1-t3.md` |
| New path | `issues/open/138-std-wasi-shared-capabilities-t1-t3.md` |
| Classification | `must-reopen` / `implementation-parts-only` |
| Reopen reason | Six-module T1/T3 matrix incomplete; `verify-harness.sh` deleted; clock/random gap (#051) |
| Violated acceptance | Items 3–5 |
| Evidence | Partial `stdlib_host/*` fixtures; `issues/open/051-std-time-random.md` |
| Follow-up split | none |

#### #034 — WIT CLI integration (resolution retracted)

| Field | Value |
|-------|-------|
| Old path | `issues/done/034-wit-cli-integration.md` |
| New path | `issues/open/034-wit-cli-integration.md` |
| Classification | `must-reopen` / `wired-but-not-user-reachable` |
| Reopen reason | Wave-2 truly-done claim retracted: `wit_import/` absent; `import_scalar_func.diag` = E0401 |
| Violated acceptance | Callable WIT import binding (blocked on #124) |
| Evidence | `import_scalar_func.diag`; absent `tests/fixtures/wit_import/` |
| Follow-up split | none |

### 3. Newly-created future issues

None. Gaps already tracked: #124 (import syntax), #476 (compose), #051 (clock/random), #074 (P2 native parent gate).

### 4. Still-truly-done (spot checks)

| ID | Area | Evidence |
|----|------|----------|
| 028, 028b | WIT import parse/wiring | `src/compiler/resolver/`, `src/compiler/component/wit_*.ark` |
| 029–032 | CABI / export / resources | Component fixtures + #121 close evidence |
| 030, 033, 038 | Emit component + fixtures | 102 `component-compile:` manifest entries |
| 121 | Canonical ABI hardening | Issue close evidence; JCO interop surface |
| 137 | Host namespace gating | `diag:target_gating/t1_import_{sockets,udp}.ark` |
| 258 | Core vs component guarantee split | `docs/target-contract.md` §Component output |
| 262, 296–300 | Interop / CABI / multi-export | `tests/component-interop/jco/*` |
| 391 | stdlib WIT helpers | `run:stdlib_wit/*`, `run:stdlib_component/*` |
| 442 | Interop readiness | JCO 101-scenario surface (wasmtime skipped in VM) |
| 475, 485 | `arukellt component` CLI + docs | `src/compiler/main/component_cmd.ark`, `docs/cli-reference.md` |
| 616 | Selfhost component emit infra | `src/compiler/component/` module tree |
| 073-adjacent | Partial WASI smoke | `module-run:stdlib_host/wasi_{clock,random,args}.ark` (partial only; parent #073 reopened) |

### 5. Docs / manifest mismatches

| Claim location | Reality | Action |
|----------------|---------|--------|
| Wave-2 #034 truly-done note | `wit_import/` absent; import_scalar_func = E0401 guard | Reopen #034 |
| `docs/target-contract.md` fixture counts | Says 16 `component-compile:`; manifest has 102 | Docs drift (non-blocking); #258 acceptance still met on tier split |
| `import_scalar_func` manifest kind | Registered `component-compile:` but `.diag` expects E0401 | Monitor; may need `compile-error:` reclassification when #124 closes |

### 6. Evidence table

| Issue | Repo proof | Result |
|-------|-----------|--------|
| 618 | roundtrip skeleton only | FAIL — reopen |
| 443 | compose = CMD_NOT_YET | FAIL — reopen |
| 118 | no --world flag | FAIL — reopen |
| 117 | deleted crate refs, no CI gate | FAIL — reopen |
| 073 | 3/46 syscall fixtures | FAIL — reopen |
| 138 | partial host fixtures | FAIL — reopen |
| 034 | E0401 diag, no wit_import | FAIL — reopen |
| 121 | component interop evidence | PASS — keep done |
| 258 | target-contract tier split | PASS — keep done |
| 033 | 102 component-compile entries | PASS — keep done |

### 7. Dependency updates

- #443 now depends on #476 (compose track).
- #118 blocked by #074 (P2 native world).
- #138 blocked by #051 (clock/random).
- #034 remains blocked by #124.
- Index regenerated: `issues/open/index.md`, `issues/open/dependency-graph.md`.

### 8. Remaining high-risk false-done items (monitor)

- **#032 resource types** — closed with handle-table evidence but resources rejected at compile (E0402); tracked separately from export surface.
- **#442** — broad interop claim references deleted `crates/`; JCO surface provides partial proof; monitor on full audit.
- **#300** — cites deleted `verify-harness.sh`; JCO `multi-type-exports` provides replacement evidence.

### 9–10. Checklist / new tracking issues

None.

### Verification at slice close

`python3 scripts/manager.py verify quick` → 143 passed / 6 failed (pre-existing:
doc-example, docs consistency drift, selfhost analysis/LSP/DAP gates, false-done
hygiene #487 unrelated). Slice modified only `issues/**` + audit report + regenerated indexes.

## Wave 4 — Slice D (stdlib / host intrinsics) — 2026-06-12

Orchestration note: subplanner workspace `.orchestrate/audit-slice-d/`;
`bun cli.ts run` blocked without `CURSOR_API_KEY` (same as wave 3b). Audit
performed serially per `prompts/research.md` unsupported-in-this-run policy.

### Scope

43-issue wave-1 batch (`.orchestrate/audit-slice-d/batches/wave1-host-stdlib.txt`):
core `std::*` modules (#041–#057), host capability issues (#137–#138, #291–#295,
issues #358 and #445), stdlib baseline (#604–#608), JSON/TOML/fs contracts (#521–#528),
manifest/docs metadata (#383, #397, #455, #457).

Cross-check axes: `std/manifest.toml` availability claims vs
`src/compiler/wasm/call_host_io.ark` + `intrinsic_*.ark` dispatch;
`tests/fixtures/stdlib_*` registration in `tests/fixtures/manifest.txt`.

### Reopened issues (must-reopen)

| ID | Old path | Classification | Root cause |
|----|----------|----------------|------------|
| #358 | `issues/done/358-stdlib-host-family-completion.md` | `acceptance-not-actually-met` | Umbrella claims http/sockets/env::var executable; selfhost path lacks dispatch |
| #293 | `issues/done/293-env-var-implementation.md` | `implementation-parts-only` | `emit_env_var` stubs to `None`; close cites deleted `crates/ark-wasm` |
| #295 | `issues/done/295-host-api-runtime-tests.md` | `acceptance-not-actually-met` | Clock/random fixture claims; no `__intrinsic_clock_*` / `__intrinsic_random_*` handlers |
| #445 | `issues/done/445-std-host-process-implementation.md` | `implementation-parts-only` | `process::abort` / `__intrinsic_process_abort` unbacked on selfhost path |
| #292 | `issues/done/292-stub-host-compile-error.md` | `acceptance-not-actually-met` | `host_stub` kind removed from manifest; no compile-time stub gate |
| #137 | `issues/done/137-std-wasi-namespace-and-target-gating.md` | `acceptance-not-actually-met` | T3-only import gating cited deleted `crates/ark-resolve`; no selfhost gating |

Previously reopened (not re-touched): #051, #446, #447. Manifest honesty: #633 open.

### Still-truly-done (spot checks)

| ID | Area | Evidence |
|----|------|----------|
| #045 | std::collections Deque/PQ | Prior wave close + `tests/fixtures/stdlib_*` parity fixtures |
| #057 | prelude migration | Audit resolution 2026-06-12; `stdlib_migration/*` fixtures |
| #524–#528 | JSON/TOML/fs/hash contracts | Fixture dirs present + registered in `manifest.txt` |
| #604–#608 | stdlib baseline plan children | Gap-ledger / contract-honesty artifacts; #605 host_module_contract fixture exists (clock sub-surface blocked on #051) |
| #041–#050, #052–#053 | Core std modules | Source under `std/` + extensive `tests/fixtures/stdlib_*` trees |

### Evidence table (slice D reopen decisions)

| Issue | Repo proof checked | Result |
|-------|-------------------|--------|
| 358 | call_host_io + manifest http/sockets/env | FAIL — reopen |
| 293 | intrinsic_env_args emit_env_var | FAIL — reopen |
| 295 | clock/random emitter handlers | FAIL — reopen |
| 445 | process_abort dispatch | FAIL — reopen |
| 292 | host_stub kind + compile gate | FAIL — reopen |
| 137 | T3_ONLY gating in selfhost resolver | FAIL — reopen |
| 045 | stdlib fixtures | PASS — keep done |
| 524 | stdlib_fs fixtures | PASS — keep done |

### Newly-created future issues

None (gaps covered by reopened issues + existing #633).

### Verification at wave close

`python3 scripts/manager.py verify quick` — orchestration-state diff only
(issues/** + regenerated indexes). Pre-existing systemic failures unchanged.

## Wave 4 — Slice E (LSP / IDE / vscode) — 2026-06-12

Orchestration note: `/orchestrate` worker spawn unavailable (`CURSOR_API_KEY` unset in cloud VM). Slice E audit performed directly by subplanner per `prompts/research.md` autonomous policy.

### 1. Audit theme

Cross-checked **~90 candidate** done issues tagged LSP/IDE/vscode/DAP/extension against:

- `scripts/check/check-lsp-lifecycle.py` + `check-dap-lifecycle.py`
- `tests/fixtures/selfhost/lsp_*.lsp-script`, `dap_*.dap-script`
- `src/compiler/{analysis,lsp,dap}.ark` and CLI wiring (`src/compiler/main/editor.ark`)
- `extensions/arukellt-all-in-one/` package + `src/test/extension.test.js`

Headline: **#572 deleted `crates/ark-lsp`**, but many done LSP navigation/semantic issues still claimed Rust-server features. Selfhost MVP (#569/#570) covers lifecycle + hover/definition via **script replay only**; **stdio user entrypoint is missing** (#628 reopened, #634 created).

### 2. Reopened issues (29)

| ID | Area | Classification | Primary evidence |
|----|------|----------------|------------------|
| 183 | extension epic | `must-reopen` | Child/rollup gaps (#191, #479, LSP nav) |
| 184 | extension foundation | `must-reopen` | Depends on false-done children |
| 191 | setup doctor / command graph | `implementation-parts-only` | `showCommandGraph` is text dump only |
| 236 | CLI LSP contract | `wired-but-not-user-reachable` | `cmd_lsp` requires script file, not stdio |
| 271 | extension CI | `acceptance-not-actually-met` | No extension job in `.github/workflows/ci.yml` |
| 273 | extension task E2E | `acceptance-not-actually-met` | `build` task names ≠ repo tests |
| 333–342 | LSP navigation/semantic | `must-reopen` | Features absent in `src/compiler/lsp/*` |
| 355 | LSP protocol E2E | `acceptance-not-actually-met` | Only 2 script fixtures vs broad acceptance |
| 450–452 | LSP quality | `acceptance-not-actually-met` | Span/E0100 gaps; extension `test.skip` |
| 454, 463 | LSP tests/perf | `must-reopen` | Rust tests deleted, not ported |
| 462, 479, 480 | settings → LSP behavior | `must-reopen` / `docs-ahead-of-reality` | `LspConfig` / `crates/ark-lsp` absent |
| 502 | multi-root LSP | `must-reopen` | No multi-root state in selfhost LSP |
| 566, 626 | Phase 6 parser recovery | `acceptance-not-actually-met` | NK_ERROR/MISSING contract missing |
| 628 | LSP MVP parent | `wired-but-not-user-reachable` | Handlers exist; stdio transport missing |

All moved `issues/done/` → `issues/open/` with `Status: open`, `Reopened by audit — 2026-06-12`, and acceptance rollback notes where applicable.

### 3. Newly-created future issues

#### #634 — Selfhost LSP/DAP stdio transport entrypoint

| Field | Value |
|-------|-------|
| Path | `issues/open/634-selfhost-lsp-dap-stdio-transport-entrypoint.md` |
| Track | selfhost-frontend |
| Why | #628 reopen: extension/docs claim stdio; CLI reads script files only |
| Depends on | #628 (reopened) |
| Close gate | stdio loop + lifecycle gates + extension smoke |

### 4. Still-truly-done (Slice E spot checks)

| ID | Area | Evidence |
|----|------|----------|
| 565, 567 | Phase 6 lexer/diagnostics | Fixtures + modular `lexer/` / resolver/typechecker accumulation |
| 568, 627 | Analysis API | `src/compiler/analysis.ark`, `check-analysis-api.py`, 3 fixtures |
| 569, 570 | LSP handlers (script MVP) | `src/compiler/lsp/*`, 2 lifecycle fixtures |
| 571 | DAP scaffold | `src/compiler/dap/*`, `dap_lifecycle.dap-script` |
| 572, 573 | Rust IDE retirement | `crates/` absent; selfhost replacements exist |
| 189, 190 | Extension bootstrap | `package.json` + `extension.js` wiring |
| 254, 272, 274, 275, 278 | Extension E2E slices | `extension.test.js` suites (scope-limited claims documented) |
| 453 | Editor behavior E2E | Def/hover suites active; E0100 explicitly skipped |
| 477, 478 | Settings manifest/wiring | `package.json` + initializationOptions forwarding |
| 622 | Task execution + test discovery | `#622` suite closes #254 deferred gaps |
| 469 | Playground route guard | Narrow URL proof met (`docs/playground/index.html`) |

Added `Audit resolution — 2026-06-12 (Slice E)` notes to these done files.

### 5. Docs / extension / CLI mismatches

| Claim location | Reality | Action |
|----------------|---------|--------|
| `docs/current-state.md`, CLI help | `arukellt lsp` is live stdio server | Reopened #236, #628; new #634 |
| Extension README (#480) | Settings control LSP behavior | Reopened #480; blocked on #479 |
| `issues/done/` LSP nav (#333–342) | Checked acceptance vs deleted `ark-lsp` | Reopened all |
| VS Code LanguageClient stdio spawn | Selfhost `cmd_lsp` needs script path | #634 |

### 6. Evidence table (lifecycle gates)

| Gate | Repo artifact | Audit env note |
|------|---------------|----------------|
| LSP lifecycle | `check-lsp-lifecycle.py`, 2 fixtures | wasmtime absent in VM; file artifacts + registration verified |
| DAP lifecycle | `check-dap-lifecycle.py`, 1 fixture | Same |
| Analysis API | `check-analysis-api.py`, 3 fixtures | Registered in `manager.py verify quick` |

### 7. Dependency updates

- `#634` depends on `#628` (reopened)
- `#480` blocked by `#479` (reopened)
- `#462` rollup blocked by `#479`
- `#183`/`#184` rollups blocked by reopened children
- Index + dependency graph regenerated

### 8. Remaining high-risk (monitor)

- **#548, #550, #551, #554, #555** — release/extension release gates not fully spot-checked this slice
- **#185–#216, #219–#220, #439–#444** — IDE workflow epics with broad claims; partial extension wiring only
- **#453 E0100 skip** — kept done with documented gap; revisit with #452 reopen

### 9–10. Checklist tracking issues

None newly required.

### Verification at slice close

Orchestration-state only (`issues/**`, audit report). `python3 scripts/manager.py verify quick` not re-run in wasmtime-less VM; pre-existing gate failures unchanged from Wave 3b baseline.

## Slice G — 2026-06-12 (remainder mechanical spot-check)

Orchestrator: subplanner `audit-slice-g` (278 done issues uncovered by slices A–F;
wave 1 deep spot-check on 50 high-risk IDs; full mechanical classification on all 278).
Cloud worker spawn unavailable — planner executed audit directly per
`unsupported-in-this-run`.

Coverage method: subtract slice A–F batch manifests and branch diffs from current
`issues/done/` inventory (`598` on master at audit time → **278 uncovered**).

### 1. Audit summary

| Metric | Count |
|--------|------:|
| Uncovered done issues (Slice G scope) | 278 |
| Wave 1 deep spot-check | 50 |
| **Reopened (must-reopen, repo proof)** | **2** |
| Mechanical `truly-done` | 171 |
| `implementation-parts-only` (monitor) | 50 |
| `monitor` (Rust-era refs, close note or successor path) | 21 |
| `false-done-risk-high` (no reopen without further proof) | 34 |

Full per-issue classification table:
`.orchestrate/audit-slice-g/classification-table.md` (278 rows).

### 2. Reopened issues

#### #439 — VSCode LSP semantic stdlib navigation

| Field | Value |
|-------|-------|
| Old path | `issues/done/439-vscode-lsp-semantic-stdlib-navigation.md` |
| New path | `issues/open/439-vscode-lsp-semantic-stdlib-navigation.md` |
| Classification | `must-reopen` / `acceptance-not-actually-met` |
| Reopen reason | Selfhost LSP uses single-buffer `symbol_at` only; no std/manifest index, references, rename, or multi-file workspace |
| Violated acceptance | All five acceptance checkboxes |
| Evidence | `src/compiler/lsp/feature_definition.ark`, `src/compiler/analysis/symbols.ark`, absent `crates/ark-lsp` |
| Follow-up split | none |

#### #441 — VSCode project-aware workspace / ark.toml

| Field | Value |
|-------|-------|
| Old path | `issues/done/441-vscode-project-aware-workspace-package-ark-toml.md` |
| New path | `issues/open/441-vscode-project-aware-workspace-package-ark-toml.md` |
| Classification | `must-reopen` / `wired-but-not-user-reachable` |
| Reopen reason | Close note delegated to deleted `ark-lsp` / #502; selfhost LSP lacks multi-package graph, cross-package index, package-aware imports |
| Violated acceptance | All five acceptance checkboxes |
| Evidence | `src/compiler/lsp/feature_symbol.ark`, `tests/fixtures/selfhost/lsp_hover_definition.lsp-script`, absent `crates/` |
| Follow-up split | none |

### 3. Newly-created future issues

None. IDE nav/workspace gaps remain covered by reopened #439/#441 and slice E
branch reopens (#333–#342, #502) pending merge.

### 4. Still-truly-done (wave 1 spot checks)

| ID | Area | Evidence |
|----|------|----------|
| 444 | VSCode component build | `extensions/arukellt-all-in-one/src/extension.js` `buildComponent` commands |
| 501 | T2 emitter | `run:t2/t2_scaffold.ark`, `run:t2/t2_stdio.ark` in manifest |
| 560 | Delete ark-driver | `crates/` absent |
| 614 | Structured diagnostics | Close gate 2026-05-16 + selfhost JSON diag fixtures |
| 428 | Playground ADR contract | `docs/adr/ADR-017-playground-execution-model.md` |
| 436 | Docs ↔ playground nav | `docs/playground/index.html`, `docs/index.html` link |
| 490 | pub use re-export | `modules/pub_use_*` manifest fixtures |
| 209 | CLI ↔ driver | `src/compiler/main.ark` wired to driver |
| 185 | IDE workflow parent | Tracking-only child separation scope |

### 5. Docs / extension / CLI / workflow mismatches

| Claim location | Reality | Action |
|----------------|---------|--------|
| #439 acceptance (stdlib nav) | Selfhost LSP single-file only | Reopened #439 |
| #441 acceptance (multi-package) | No ark.toml graph in selfhost LSP | Reopened #441 |
| 34 `false-done-risk-high` rows | Rust-era close prose; may need follow-up after A–F branch merges | Monitor; no bulk reopen without proof |

### 6. Evidence table (reopen decisions)

| Issue | Repo proof checked | Result |
|-------|-------------------|--------|
| 439 | selfhost LSP symbol resolution | FAIL — reopen |
| 441 | selfhost LSP workspace/package | FAIL — reopen |
| 444 | extension buildComponent | PASS — keep done |
| 501 | T2 manifest fixtures | PASS — keep done |
| 560 | crates/ absent | PASS — keep done |

### 7. Dependency updates

- `#441` depends on #333, #335, #340 (unchanged)
- `#439` depends on #333–#339 (unchanged)
- Index regenerated after reopen

### 8. Remaining high-risk false-done items (monitor)

- **34 issues** classified `false-done-risk-high` — user-visible or harness claims citing
  deleted `crates/` without close notes; await targeted proof pass or slice E/B branch merge
  (#216/#217/#219/#502 on sibling branches).
- **228 uncovered issues** beyond wave 1 received mechanical classification only; no
  contradicting repo proof surfaced in automated pass.

### 9. Checklist items not tracked as issues

None newly identified. Release checklist items remain covered by existing CI/issue work.

### 10. Newly-created checklist tracking issues

None.

### Verification at slice close

`python3 scripts/manager.py verify quick` — orchestration-state diff only (`issues/**`);
pre-existing failures unchanged from prior waves.
