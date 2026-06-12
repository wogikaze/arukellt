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

## Wave 4 — Slice D (stdlib / host intrinsics) — 2026-06-12

Orchestration note: subplanner workspace `.orchestrate/audit-slice-d/`;
`bun cli.ts run` blocked without `CURSOR_API_KEY` (same as wave 3b). Audit
performed serially per `prompts/research.md` unsupported-in-this-run policy.

### Scope

43-issue wave-1 batch (`.orchestrate/audit-slice-d/batches/wave1-host-stdlib.txt`):
core `std::*` modules (#041–#057), host capability issues (#137–#138, #291–#295,
#358, #445), stdlib baseline (#604–#608), JSON/TOML/fs contracts (#521–#528),
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
