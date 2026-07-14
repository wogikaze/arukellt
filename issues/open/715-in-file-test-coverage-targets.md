---
Status: open
Created: 2026-07-04
Updated: 2026-07-14
ID: 715
Track: testing
Depends on: "041 (ADR, done)"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: In-file test syntax (ADR-041) coverage planning 2026-07-04
---

# 715 — In-file test coverage targets for compiler and stdlib

## Summary

ADR-041 landed the `test` declaration syntax (NK_TEST_DECL / NK_TEST_MOD)
with parser, resolver, typechecker, and CLI `arukellt test` discovery.
The syntax is functional but no coverage targets or prioritized adoption
plan exist. This issue defines a phased adoption plan so that in-file
tests complement (not duplicate) the existing 2,670-entry fixture harness.

## Current state

- `test` declarations parse, resolve, and typecheck correctly (ADR-041 done).
- `arukellt test <file>` discovers and lists tests, then type-checks.
- No in-file `test` declarations exist in `src/compiler/` or `std/` yet.
- Existing fixture harness: 2,670 entries (run 1,578 / t3-compile 408 /
  t3-run 400 / compile-error 57 / others).
- `std::test` module (17 assertion functions) used by ~1,124 fixture files.
- Old `test_` prefix naming convention: 0 functions in `src/compiler/`.
- Compiler: ~9,700 `fn` declarations across 17 namespaces.
- Stdlib: 1,327 `pub fn` declarations across 24 modules.

## Design rationale

In-file tests are **white-box unit tests** — their value is proximity to
the implementation and access to non-pub items. They complement the
existing fixture harness (integration / black-box tests). Adding `test`
to every function is neither practical nor desirable:

- Side-effectful functions (WASI calls, wasm emission, LSP/DAP protocol)
  are better tested via fixtures (run / t3-compile / t3-run).
- Intrinsic wrappers and thin prelude wrappers have little logic to test
  in-file; fixtures already cover them.
- Pure functions with clear invariants and edge cases benefit most from
  co-located `test` blocks.

Reference: Zig stdlib uses `test` blocks extensively for pure functions;
Rust std uses `#[test]` for unit tests alongside integration tests in
`tests/`. Both languages keep side-effectful / I/O tests in separate
harnesses.

## Required work

### Phase 1: stdlib pure functions (~180–220 functions, ~14–17% of pub fn)

Priority modules where in-file tests add the most value:

- [x] `std/core/` (clone, cmp, convert, ops, iter, math) — 80 pub fn,
      target 50–60 tests (boundary values, typeclass laws)
- [x] `std/collections/` (vec, hash_map, sort, string, linked_list, trie)
      — 240 pub fn, target 80–100 tests (data structure invariants,
      edge cases: empty, single-element, overflow)
- [x] `std/text/` (builder, fmt, rope, string) — 86 pub fn, target 30–40
      tests (string edge cases, formatting)
- [x] `std/bytes/` — 53 pub fn, target 20–25 tests (boundary values)

### Phase 2: compiler transformation passes (~60–90 functions, ~1% of fn)

- [x] `lexer/` — 5–8 tests (tokenization edge cases, character classification)
- [x] `parser/` — 10–15 tests (`is_*_start` dispatch predicates, AST shape
      assertions)
- [x] `resolver/` — 8–12 tests (scope resolution invariants)
- [x] `typechecker/` — 10–15 tests (unify edge cases, type inference)
- [x] `mir/` — 15–20 tests (lowering shape verification, complementing
      t3-compile fixtures)
- [x] `diagnostics/` — 5–8 tests (error code → message mapping)

### Phase 3 (not recommended / deferred)

- `host/` (44 fn) — WASI side effects; fixtures (run) are better suited.
- `component/` (19 fn) — component model output; fixtures
  (component-compile) are better suited.
- `lsp/`, `dap/`, `analysis/` — IDE protocol; lifecycle gates exist.
- `wasm/` emitter body (2,357 fn) — binary output; t3-compile fixtures
  are better suited.
- `simd/` (262 fn) — intrinsic wrappers; fixtures already cover.

### Documentation

- [x] Add coverage target section to ADR-041 or `docs/current-state.md`.
- [x] Document the decision: in-file tests for pure functions / white-box
      invariants; fixtures for integration / side-effectful behavior.
- [x] Add a `verify` gate that counts in-file `test` declarations and
      reports adoption progress (advisory, non-blocking).

## Acceptance

- [ ] Phase 1: ≥180 in-file `test` declarations in `std/` covering pure
      functions in core, collections, text, bytes
- [ ] Phase 2: ≥60 in-file `test` declarations in `src/compiler/` covering
      lexer, parser, resolver, typechecker, mir, diagnostics
- [ ] No in-file tests added to side-effectful modules (host, component,
      lsp, dap, wasm emitter body) — those stay fixture-only
- [ ] Coverage target documented in ADR-041 or `docs/current-state.md`
- [ ] `arukellt test` discovery works on all modified files without
      typecheck errors
- [ ] `python3 scripts/manager.py verify quick` passes after each phase

## Reopened blocking findings (2026-07-14 CQ-18 audit)

1. **Dummy/probe tests inflating count**: At least 171 `probe_N { assert(N >= 0) }`
   tests and 4 `sanity { assert(1 == 1) }` tests exist. `std/bytes/mod.ark` has
   150 probe tests, `src/compiler/main/targets.ark` has 21. These are generated
   by `scripts/gen/append-issue-715-tests.py` to meet count targets, not to
   verify contracts, boundaries, or invariants. This violates the Acceptance
   intent and AGENTS.md test conventions.
2. **Trivial asserts**: `assert(0 == 0)`, `assert(false == false)`, `assert(true)`
   patterns exist in generated tests. These provide zero verification value.
3. **Removal required**: All probe_N, sanity, and trivial-assert tests must be
   removed. `append-issue-715-tests.py` bulk-fill logic must be removed.
4. **Recount required**: After removal, coverage must be remeasured counting
   only tests that verify function contracts, boundary values, or invariants.
   If count falls below Acceptance threshold, issue stays open.
5. **Lint prevention**: A quality checker or lint rule must detect and reject
   future `probe_N`, `sanity` with trivial asserts, `assert(literal >= 0)`,
   `assert(x == x)` patterns. False-positive unit tests required.

## Dependencies

- ADR-041 (done) — provides the `test` syntax and CLI discovery.
- No blocking dependencies; can proceed incrementally per module.

## Notes

- Adoption is incremental — each module can be tackled independently.
- In-file tests should use `std::test` assertions (`assert_eq_i32`, etc.)
  where applicable, or bare `assert(...)` for boolean conditions.
- Test names should be descriptive and unique within the file scope.
- `test mod` should be used to group related tests within a single file.
