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
