# 504 — Selfhost: trait/interface syntax and impl-block infrastructure

**Track:** selfhost
**Status:** open
**Created:** 2026-04-15
**Updated:** 2026-04-15
**Source:** STOP_IF blocker detected while working #495-selfhost-trait-bounds

## Summary

Issue #495 (selfhost typechecker: trait bounds and constraint solving) hit a
hard STOP_IF condition: the selfhost language toolchain does not yet support
the full trait-bound / `impl Foo for Bar` surface needed by #495.

Repo evidence on 2026-04-15 shows **partial groundwork already exists**:
- `lexer.ark` already recognizes `trait` and `impl`
- `parser.ark` already parses `trait Foo { ... }` and a simple `impl Foo { ... }`
  block form
- What is still missing for #495 is the richer surface:
  - `impl Trait for Type { ... }`
  - generic type-parameter bounds such as `fn f<T: Foo>(x: T)`
  - HIR nodes / metadata for bound-bearing type parameters and impl targets
  - typechecker-side impl registry and bound satisfaction checks

Until all four layers are present, #495 cannot proceed.

## Depends on

- None (foundational)

## Blocks

- #495 (selfhost typechecker: trait bounds and constraint solving)

## Primary paths

- `src/compiler/lexer.ark` — add `trait`, `where` keywords (and `impl` if not
  already present as a keyword for other purposes)
- `src/compiler/parser.ark` — grammar for `trait Foo { … }`, `impl Foo for Bar
  { … }`, and bounded generic parameters `fn f<T: Foo>(x: T)`
- `src/compiler/hir.ark` — `HirTraitDecl`, `HirImplBlock`, `TraitBound` on
  `TypeParam`
- `src/compiler/typechecker.ark` — trait registry, impl lookup, and bound
  satisfaction predicate

## Non-goals

- Higher-kinded types
- Trait coherence / orphan rules
- Associated types (deferred to a follow-up)

## Acceptance

- [ ] `trait Foo { fn bar(self) -> i64 }` parses without error
- [ ] `impl Foo for MyStruct { fn bar(self) -> i64 { 0 } }` parses without error
- [ ] `fn f<T: Foo>(x: T)` parses and the bound is reachable from the HIR type
  parameter node
- [ ] Typechecker can register an impl and answer "does type T satisfy bound B?"
- [ ] At least one parse-level fixture and one typecheck-level fixture
- [ ] `cargo test` passes

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes
- `bash scripts/run/verify-harness.sh --cargo` passes

## Close gate

Acceptance items checked; #495 can be re-opened and assigned.
