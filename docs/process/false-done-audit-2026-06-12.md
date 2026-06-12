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
