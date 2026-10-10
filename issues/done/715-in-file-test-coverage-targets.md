---
Status: done
Created: 2026-07-04
Updated: 2026-10-11
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

- [x] Phase 1: ≥180 in-file `test` declarations in `std/` covering pure
      functions in core, collections, text, bytes
- [x] Phase 2: ≥60 in-file `test` declarations in `src/compiler/` covering
      lexer, parser, resolver, typechecker, mir, diagnostics
- [x] No in-file tests added to side-effectful modules (host, component,
      lsp, dap, wasm emitter body) — those stay fixture-only
- [x] Coverage target documented in ADR-041 or `docs/current-state.md`
- [x] `arukellt test` discovery works on all modified files without
      typecheck errors
- [x] `python3 scripts/manager.py verify quick` passes after each phase
      (154/154 checks green on branch `docs/adr-058-language-v2-semantics`)

## Reopened blocking findings (2026-07-14 CQ-18 audit)

1. **~~Dummy/probe tests inflating count~~** (RESOLVED): All 171 `probe_N`
   tests and 4 `sanity { assert(1 == 1) }` tests removed. `std/bytes/mod.ark`
   150 probe tests and `src/compiler/main/targets.ark` 21 probe tests
   deleted. `append-issue-715-tests.py` bulk-fill logic removed.
2. **~~Trivial asserts~~** (RESOLVED): `assert(0 == 0)`, `assert(false == false)`,
   `assert(true)` patterns removed from `std/core/default.ark`.
3. **~~Removal required~~** (RESOLVED): All probe_N, sanity, and trivial-assert
   tests removed. `append-issue-715-tests.py` bulk-fill logic removed.
4. **Recount required**: After removal, coverage remeasured using
   `check-infile-test-adoption.py` (rewritten to separate test modules,
   test cases, and meaningful tests by Phase 1/2 target directories):
   - Phase 1 (std/core, std/collections, std/text, std/bytes):
     34 meaningful tests (target: 180) — BELOW THRESHOLD
   - Phase 2 (lexer, parser, resolver, typechecker, mir, diagnostics):
     10 meaningful tests (target: 60) — BELOW THRESHOLD
   - Out-of-scope: src/compiler/analysis has 2 test cases (explicit
     exception: pure IdentSpan constructors/accessors, allowed in
     ALLOWED_INFILE_EXCEPTIONS)
   Issue stays open until both Phase 1 and Phase 2 reach targets.
5. **~~Lint prevention~~** (RESOLVED): `check-trivial-tests.py` added to
   detect and reject future `probe_N`, `sanity` with trivial asserts,
   `assert(literal >= 0)`, `assert(x == x)` patterns. 7 unit tests added
   with false-positive guards.

## Final adoption recount (2026-10-11)

All added tests assert real behavior of the co-located pure functions
(edge cases, invariants, round-trips); no probe/sanity/trivial patterns
(`check-trivial-tests.py` PASS).

- Phase 1 (std/core, std/collections, std/text, std/bytes):
  250 meaningful tests (target: 180) — OK
  - std/core: 69 (math, either, clone, error, raw, hash, default)
  - std/collections: 98 (linear, linked_list, hash_set, bimap, ordered,
    trie, compiler arena/slotmap/interner, hash)
  - std/text: 51 (builder, rope, string, fmt)
  - std/bytes: 32 (byte ops, hex, endian, LEB128, buf/cursor)
- Phase 2 (lexer, parser, resolver, typechecker, mir, diagnostics):
  127 meaningful tests (target: 60) — OK
  - lexer 13, parser 22, resolver 11, typechecker 23, mir 46,
    diagnostics 12 — every required pass non-zero
- Out-of-scope side-effectful modules: 0 tests (host, component, lsp,
  dap, wasm, simd) — fixtures remain their harness
- Files that cannot standalone-typecheck under `arukellt test`
  (prelude-injected `__core_*` names, private cross-module refs) were
  excluded from new tests: `std/core/convert.ark`,
  `std/core/default.ark`, `std/core/scalar_format.ark`,
  `std/collections/sort.ark`, `std/collections/hash_map.ark`,
  `std/collections/string.ark`, `src/compiler/mir/dump_op_names_*.ark`,
  `src/compiler/typechecker/name_*.ark`,
  `src/compiler/diagnostics/{format,record,source_map_offsets}.ark`.

## Dependencies

- ADR-041 (done) — provides the `test` syntax and CLI discovery.
- No blocking dependencies; can proceed incrementally per module.

## Notes

- Adoption is incremental — each module can be tackled independently.
- In-file tests should use `std::test` assertions (`assert_eq_i32`, etc.)
  where applicable, or bare `assert(...)` for boolean conditions.
- Test names should be descriptive and unique within the file scope.
- `test mod` should be used to group related tests within a single file.
