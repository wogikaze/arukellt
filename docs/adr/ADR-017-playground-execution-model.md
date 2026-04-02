# ADR-017: Playground Execution Model and v1 Product Contract

**Status**: DECIDED
**Created**: 2026-03-31
**Scope**: Playground (web), target roadmap, docs contract

---

## Context

The Arukellt web playground needs a concrete product contract before any
implementation work begins.  Two constraints drive the decision:

1. **T2 (`wasm32-freestanding`) is not implemented.**
   `crates/ark-target/src/lib.rs` registers the target with
   `implemented: false` and `run_supported: false`.
   `docs/target-contract.md` states: "identifier is registered but nothing
   downstream handles it."  Building a playground that runs user code in the
   browser requires either T2 or an alternative approach.

2. **T3 (`wasm32-wasi-p2`) requires wasmtime.**
   The canonical, CI-verified target cannot execute in a browser context
   directly.  Shipping a server-side executor for v1 introduces operational
   cost, abuse-surface risk, and latency — and buys little value when the
   primary user-facing benefit (instant feedback) can be achieved with
   lighter-weight client-side tooling.

The parser, formatter, and diagnostics engine are pure Rust with no WASI
dependency.  They compile to `wasm32-unknown-unknown` today and can be
packaged as a browser-safe Wasm bundle.

Issue 378 was opened to force this decision before any downstream work
(issues 379, 382, 428) begins.

---

## Decision

### Execution model: **client-side hybrid** (no server-side executor in v1)

| Surface | Execution location | Wasm target | v1? |
|---------|-------------------|-------------|-----|
| Edit (Monaco/CodeMirror shell) | browser | n/a | ✅ yes |
| Format | browser (Wasm) | `wasm32-unknown-unknown` | ✅ yes |
| Parse | browser (Wasm) | `wasm32-unknown-unknown` | ✅ yes |
| Check / typecheck | browser (Wasm) | `wasm32-unknown-unknown` | ✅ yes |
| Diagnostics (structured) | browser (Wasm) | `wasm32-unknown-unknown` | ✅ yes |
| Examples (curated set) | static / browser | n/a | ✅ yes |
| Share / permalink | browser + static host | n/a | ✅ yes |
| Full compile (emit Wasm binary) | **not in v1** | — | ❌ v2+ |
| Run (execute user program) | **not in v1** | — | ❌ v2+ |

**Rationale for no server-side executor in v1:**

- Instant feedback (parse errors, type errors, formatting) is achievable
  entirely client-side and covers the most common "try the language" use case.
- A server-side executor would require sandboxing, rate-limiting, abuse
  mitigation, and operational maintenance — all orthogonal to the v1 goal of
  "make the language explorable."
- Full execution is blocked by a real engineering dependency (T2 or wasmtime
  in-browser); shipping a server workaround in v1 would create a maintenance
  burden that disappears when T2 lands.
- Deferring execution to v2 keeps the v1 surface small, auditable, and
  shippable quickly.

### v1 scope (explicit)

> **v1 = edit + format + parse + check + diagnostics + examples + share**

All six surfaces must be present for playground v1 to be considered complete.
None of them require T2, a server executor, or wasmtime.

### v1 non-goals (explicit)

The following are explicitly **out of scope for v1**:

- Full compilation to Wasm binary (`--emit core-wasm`)
- Running user programs (any target)
- Server-side execution sandbox
- T2 (`wasm32-freestanding`) implementation
- Native (T4/LLVM) execution
- WASI P3 / async runtime
- LSP integration in the browser editor (may come with editor shell work, not
  a v1 gate)
- Authenticated sessions, saved programs, or user accounts

### T2 timeline split from playground roadmap

T2 (`wasm32-freestanding`) implementation is **tracked separately** from the
playground roadmap.  Playground v1 does not require T2 and must not be blocked
on it.  Playground v2 (full compile + run in browser) **may** use T2 once it
is implemented, but that dependency is a v2 concern.

The playground Wasm bundle for v1 targets `wasm32-unknown-unknown` (no WASI),
which is already supported by existing pure-Rust crates in the compiler
frontend.  T2 would only become relevant if v2 needs to _run_ output inside
the browser, not merely compile it.

---

## Client-side surface detail

The following compiler components run **entirely in the browser** via a
`wasm32-unknown-unknown` bundle:

| Component | Crate(s) | Notes |
|-----------|----------|-------|
| Lexer | `ark-lexer` (or equivalent frontend crate) | No WASI dependency |
| Parser | `ark-parser` (or equivalent frontend crate) | No WASI dependency |
| Type checker (check-only path) | `ark-typecheck` / `ark-driver` check gate | No codegen needed |
| Formatter | formatter surface | Pure transformation, no WASI |
| Diagnostics renderer | `ark-diagnostics` | Structured output, no WASI |

The backend (codegen, Wasm emit, wasmtime runner) is **not included** in the
v1 browser bundle.

---

## Consequences

1. **Issue 379** (Wasm packaging for browser) may proceed targeting
   `wasm32-unknown-unknown` for the frontend-only bundle.

2. **Issue 382** (T2 freestanding implementation) is **decoupled** from the
   playground roadmap.  It may proceed independently on its own schedule
   without blocking playground v1 or v2 scoping.

3. **Issue 428** (v1 contract ADR, follow-on) may reference this document as
   the authoritative execution model decision.

4. The share/permalink feature requires a static hosting solution (or a
   minimal read-only permalink service); it does not require a code-execution
   backend.

5. `docs/target-contract.md` is **not changed** by this ADR — T2 status
   remains "not-started."  That document is updated only when T2 gains
   codegen or test infrastructure.

6. `docs/current-state.md` active work note is updated to reflect that
   playground v1 ADR has been decided (see below).

---

## Alternatives considered

### A: Server-side executor for v1

Ship a sandbox server that compiles and runs user code.

**Rejected**: Operational complexity, abuse surface, and latency outweigh the
benefit for v1.  The primary value prop of a playground ("try the syntax,
see errors immediately") does not require execution.

### B: Block v1 on T2 landing

Wait for T2 (`wasm32-freestanding`) to be implemented, then ship a playground
that runs code in the browser.

**Rejected**: T2 has no codegen, no tests, and no timeline.  Blocking
playground on T2 would indefinitely delay a useful v1.  T2 is a compiler
backend concern; the playground's near-term value is in the editor + check
feedback loop.

### C: Compile-only v1 (emit Wasm, no run)

Ship parse/check/diagnostics/format **plus** Wasm emit to binary, but no
execution.

**Rejected**: Emitting a binary requires the full codegen backend
(`wasm32-unknown-unknown` or T2) which is not available without significant
additional work.  The marginal utility of showing users a binary blob they
cannot run is low.  This option can be revisited in v2 alongside execution.

### D: Hybrid with optional server-side run

v1 ships client-side check/format, plus an optional server-side run button.

**Rejected**: "Optional" server infrastructure still requires the full
operational setup.  Complexity is not reduced.  Deferred to v2.

---

## Connections to docs, tests, and examples

### Docs connection points

| Doc / page | Relationship to playground v1 |
|-----------|-------------------------------|
| `docs/current-state.md` | Must reflect playground v1 status once each surface ships; the "active work" block is updated per ADR consequence 6. |
| `docs/target-contract.md` | Read-only reference for T2/T3 status; playground v1 does **not** change it. |
| `docs/adr/README.md` | This ADR (ADR-017) is listed there; issue 379 / 428 ADRs (if any) must also be listed on merge. |
| Language reference / stdlib docs | The playground **editor shell** (issue 379) may link to relevant doc pages for each example snippet; no new doc pages are required for v1 launch. |

### Test connection points

The v1 browser Wasm bundle (issue 379) is composed of pure-Rust crates.
Existing Rust unit/integration tests for those crates are the primary test
signal.  Playground-specific verification layers are:

| Layer | Scope | Location |
|-------|-------|----------|
| Cargo unit tests | Each crate compiled into the bundle (lexer, parser, typecheck, diagnostics, formatter) | `crates/*/tests/` and `#[test]` blocks |
| Harness smoke tests | `scripts/run/verify-harness.sh --quick` and `--cargo` must pass; the bundle is not gated by a separate browser test in v1 | `harness/`, `scripts/` |
| Docs-consistency check | `python3 scripts/check/check-docs-consistency.py` must pass; playground additions must not break doc cross-references | `scripts/check/check-docs-consistency.py` |
| Browser smoke test (v1 gate) | A minimal JS/HTML smoke test that imports the Wasm bundle and calls `parse()` is sufficient for v1; full browser integration tests are a v2 concern | Defined in issue 379 |

No new test infrastructure is required to close issue 428; the test contract
above is the authoritative v1 requirement.

### Examples connection points

The "Examples (curated set)" surface listed in the scope table is defined as:

- A **static, version-controlled set** of `.ark` snippets, stored under
  `std/examples/` or a dedicated `playground/examples/` directory (exact path
  decided in issue 379 / editor shell work).
- Each example must **compile-check cleanly** (parse + typecheck pass) as
  verified by the harness; examples that exercise features not yet in the
  type-checker are explicitly labelled or excluded from v1.
- Examples are **not auto-generated** from stdlib tests; they are hand-curated
  to illustrate idiomatic language usage and cover the six v1 surfaces.
- The share/permalink surface (issue 379) uses the same example files as its
  seed content; no separate example corpus is maintained.

These connection rules ensure that playground examples stay in sync with the
compiler's actual capabilities without requiring a separate CI pipeline.

---

## References

- `crates/ark-target/src/lib.rs` — target registry (T2: `implemented: false`)
- `docs/target-contract.md` — T2 status: "not-started"
- `docs/current-state.md` — authoritative implementation status
- [ADR-007](ADR-007-targets.md) — target taxonomy (T1–T5)
- [ADR-013](ADR-013-primary-target.md) — T3 as primary target
- Issue 378 — this decision
- Issue 379 — Wasm packaging (follows from this ADR)
- Issue 382 — T2 freestanding (decoupled from playground)
- Issue 428 — v1 contract follow-on (references this ADR)
