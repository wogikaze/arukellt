# 504 — Selfhost: trait/interface syntax and impl-block infrastructure

**Track:** selfhost
**Status:** open
**Created:** 2026-04-15
**Updated:** 2026-04-15
**Source:** STOP_IF blocker detected while working #495-selfhost-trait-bounds

## Summary

Issue #495 (selfhost typechecker: trait bounds and constraint solving) hit a
hard STOP_IF condition: the selfhost language toolchain has no trait or
interface syntax at any layer.

Audit across the selfhost compiler (`src/compiler/`):
- `lexer.ark` — no `trait`, `impl`, or `where` keywords
- `parser.ark` — no grammar rules for trait definitions, impl blocks, or
  generic type-parameter bounds (e.g. `<T: Foo>`)
- `hir.ark` — no HIR node or variant for trait declarations, impl-for blocks,
  or trait-bound annotations on type parameters
- `typechecker.ark` — no infrastructure to register trait implementations,
  look up impls, or check `T: Bound` constraints

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
