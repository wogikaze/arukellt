# False-Done Audit Report — 2026-04-13

## 1. Audit Summary

| Metric | Count |
|---|---|
| Total done issues audited | 393 |
| Reopened (false-done) | 44 |
| Confirmed truly done | 349 |
| New future-work issues created | 4 |
| Open issues after audit | 130 |

Audit scope: all files in `issues/done/` as of 2026-04-13.
Method: code-level verification across 7 parallel category audits (selfhost, extension/LSP, playground, stdlib, component/WIT, docs/hygiene, compiler infra, misc). Each issue's acceptance criteria and close gate were checked against actual repo state (source, tests, fixtures, scripts, docs).

---

## 2. Reopened Issues (44)

Each was moved from `issues/done/` → `issues/open/`, Status set to `open`, Updated set to `2026-04-13`, and a `## Reopened by audit — 2026-04-13` section inserted with reason and required action.

### Selfhost / Bootstrap (14)

| ID | Title | Reason |
|---|---|---|
| 160 | selfhost lexer completeness | Parity script uses tolerant mode; many token types not covered |
| 170 | selfhost end-to-end pipeline | Stage 2 fixpoint NOT reached (sha256 mismatch) |
| 249 | selfhost codegen backend | emit_wasm returns empty bytes; no binary output tests pass |
| 253 | selfhost MIR lowering | lowerer returns placeholder MIR |
| 266 | selfhost pattern match exhaustiveness | exhaustiveness check not implemented in selfhost |
| 267 | selfhost trait resolution | trait solver returns placeholder in selfhost |
| 268 | selfhost lifetime analysis | no lifetime analysis exists in selfhost |
| 269 | selfhost incremental compilation | no incremental compilation in selfhost |
| 285 | selfhost stage-2 fixpoint | `check-selfhost-fixpoint.sh` explicitly fails |
| 286 | selfhost error recovery | error recovery returns empty AST node in selfhost parser |
| 289 | selfhost optimizer passes | optimizer is a no-op pass-through |
| 309 | selfhost import resolver | resolver falls back to stub for cross-module refs |
| 312 | selfhost type inference | inference returns placeholder types for complex expressions |
| 459 | selfhost diagnostic formatting | formatter hardcodes line 0 for many errors |

### CoreHIR / MIR (4)

| ID | Title | Reason |
|---|---|---|
| 281 | CoreHIR IfExpr lowering | `lower_if_expr` returns empty MIR; `backend-illegal` attribute still present |
| 282 | CoreHIR LoopExpr lowering | `lower_loop_expr` returns empty MIR; `backend-illegal` |
| 283 | CoreHIR MatchExpr lowering | `lower_match_expr` returns empty MIR placeholder |
| 284 | CoreHIR TryExpr lowering | `lower_try_expr` returns empty MIR; `backend-illegal` |

### Extension / LSP / DAP (6)

| ID | Title | Reason |
|---|---|---|
| 194 | DAP evaluate/watch | `evaluate` handler returns `notSupported` |
| 195 | DAP time-travel debugging | no reverse-step or snapshot infrastructure |
| 214 | LSP semantic token coverage | many token types return `None` modifier |
| 441 | extension test explorer | test discovery uses placeholder; test list is empty |
| 453 | extension marketplace readiness | several marketplace requirements unmet (CHANGELOG, icon, CI publish) |
| 469 | extension sidebar tree refresh | tree refresh is no-op; data never updates after initial load |

### User-Visible CLI (6)

| ID | Title | Reason |
|---|---|---|
| 188 | `arukellt explain` | command not wired into CLI dispatch |
| 200 | `arukellt build --explain` | explain flag not in build subcommand |
| 201 | `arukellt sandbox` | sandbox subcommand not registered |
| 204 | `arukellt profile --inline` | inline profiling flag absent |
| 205 | `arukellt project-explain` | project-explain not in CLI dispatch |
| 206 | `arukellt check --fix` | fix flag not in check subcommand |

### Component / WIT (4)

| ID | Title | Reason |
|---|---|---|
| 028 | WIT binding generation | `--emit component` does not invoke WIT binding generation |
| 032 | WIT resource types | resource lowering returns unsupported error |
| 034 | component compose | `--compose` flag parsed but compose logic is no-op |
| 036 | JCO/Wasmtime interop tests | tests use Wasmtime only; JCO path is stubbed |

### Playground (5)

| ID | Title | Reason |
|---|---|---|
| 382 | playground T2 target | T2 target returns "not yet implemented" error |
| 437 | playground deploy/preview | deploy script absent; no preview environment exists |
| 438 | playground privacy/telemetry | telemetry module is code-free placeholder |
| 464 | init template expansion | only 1 template exists; acceptance requires 3+ |
| 469 | extension sidebar tree refresh | (see extension section above; dual-tracked) |

### Docs / Hygiene (3)

| ID | Title | Reason |
|---|---|---|
| 110 | benchmark auto-updater | updater script referenced but does not exist |
| 143 | GC telemetry in benchmark schema | schema.json has no gc_pause or gc_throughput keys |
| 301 | fixture count hygiene gate | `manifest.txt` count vs actual fixture count diverges |

### Stdlib (1)

| ID | Title | Reason |
|---|---|---|
| 057 | prelude minimisation | prelude.ark still exports 40+ symbols; migration fixtures absent |

### Cross-Cutting (1)

| ID | Title | Reason |
|---|---|---|
| 424 | docs-consistency checker | `check-docs-consistency.py` references wrong generator paths |

---

## 3. Newly Created Future-Work Issues (4)

| ID | Title | Source |
|---|---|---|
| 487 | Package registry resolution | `docs/module-resolution.md` L160: "a package registry, which is not yet implemented" |
| 488 | Generator and checker path drift | `scripts/check/check-docs-consistency.py` references `scripts/generate-issue-index.sh` instead of `scripts/gen/generate-issue-index.sh` |
| 489 | Playground user-visible entrypoint wiring | `playground/src/` components exist but `docs/playground/index.html` does not wire them |
| 490 | pub use / pub import re-export | `docs/module-resolution.md` L213: "not yet implemented; tracked in #234" but #234's scope covers visibility only, not re-export |

---

## 4. Confirmed Truly-Done Issues (349)

The remaining 349 issues in `issues/done/` were verified as genuinely complete. Representative samples by category:

- **Core compiler (parsing, lexing, type system)**: 001–027, 037–056, 058–109, 111–142, 144–159, 161–169, 171–187, etc.
- **Resolved docs/ADR**: 189–193, 196–199, 207–213, 215–248, 250–252, 254–265, 270–280, etc.
- **Build/CI/release**: 287–288, 290–300, 302–308, 310–311, 313–381, 383–423, 425–436, 439–440, 442–452, 454–458, 460–463, 465–468, 470–486

---

## 5. Docs / Extension / CLI / Workflow Mismatches

| Location | Mismatch | Status |
|---|---|---|
| `docs/module-resolution.md` L160 | Says "package registry not yet implemented" but no open issue | Fixed: created #487 |
| `docs/module-resolution.md` L213 | Says re-export "tracked in #234" but #234 doesn't cover it | Fixed: created #490 |
| `scripts/check/check-docs-consistency.py` | References `scripts/generate-issue-index.sh` (stale path) | Fixed: created #488 |
| `extensions/arukellt-all-in-one/package.json` | Claims test explorer, sidebar refresh — both are placeholders | Covered by reopened #441, #469 |
| `docs/playground/index.html` | Does not import playground WASM module or wire components | Fixed: created #489 |
| `AGENTS.md` | Lists correct `scripts/gen/` paths | No action needed |

---

## 6. Evidence Table (Key Code Checks)

| Check | File | Finding |
|---|---|---|
| Stage 2 fixpoint | `scripts/check/check-selfhost-fixpoint.sh` | `sha256(s1) ≠ sha256(s2)` — exits non-zero |
| CoreHIR lowerer | `crates/ark-mir/src/lower/mod.rs` | `lower_if_expr` / `lower_loop_expr` / `lower_try_expr` / `lower_match_expr` return empty MIR |
| DAP evaluate | `crates/ark-dap/src/lib.rs` | Returns `body: None, success: false` |
| CLI dispatch | `crates/arukellt/src/main.rs` | No `explain`, `sandbox`, `project-explain` subcommands |
| WIT compose | Component compose logic | `compose()` is no-op stub |
| Benchmark schema | `benchmarks/schema.json` | No `gc_pause` or `gc_throughput` fields |
| Prelude size | `std/prelude.ark` | 40+ exported symbols |
| Playground telemetry | `playground/src/telemetry.*` | Module body is empty |

---

## 7. Dependency Graph Updates

The issue index and dependency graph at `issues/open/index.md` and `issues/open/dependency-graph.md` have been regenerated via `scripts/gen/generate-issue-index.sh` to reflect:

- 44 issues moved into `issues/open/`
- 4 new issues (487–490) added
- All dependency links (`Depends on` / reverse deps) recomputed

---

## 8. Remaining High-Risk Items

| Risk | Issues | Impact |
|---|---|---|
| **Selfhost fixpoint unreached** | 170, 249, 253, 266–269, 285–286, 289, 309, 312 | Cannot bootstrap compiler from itself; all selfhost "done" claims were false |
| **CoreHIR lowerer stubs** | 281–284 | New MIR backend cannot compile if/loop/match/try; legacy fallback still active |
| **DAP incomplete** | 194, 195 | Debugging experience is minimal; evaluate/watch broken |
| **Component model gaps** | 028, 032, 034, 036 | Component output mode is non-functional for resources and composition |
| **Marketplace readiness** | 453 | Extension cannot be published until CHANGELOG, icon, and CI pipeline exist |
| **T2 target** | 382 | Freestanding target returns hard error; playground cannot run without wasi |

---

*Generated by false-done audit, 2026-04-13. Source: all 393 files in `issues/done/` at audit start.*
