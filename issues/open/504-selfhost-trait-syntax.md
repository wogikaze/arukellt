# 504 — Selfhost: trait/interface syntax and impl-block infrastructure

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-15
**ID**: 504
**Depends on**: none
**Track**: selfhost
**Blocks v1 exit**: no
**Source**: STOP_IF blocker detected while working #495-selfhost-trait-bounds

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

## Partial slice note — 2026-04-15

Wave 1 added parser-side work in the worktree for `impl Trait for Type` and trait-bound syntax,
but the slice did **not** reach done state:
- required verification was blocked by unrelated Rust workspace compile failures
- the slice was not committed

Keep #504 open. Resume with a new slice only after the blocking compile state is cleared and the
parser/HIR changes can be verified and committed.

## Partial slice note — 2026-04-15 (Wave 2)

Wave 2 landed commit `a0279a1a93a2589264461ab873637b0cc1c19e22`, which added:

- parser support for `impl Trait for Type { ... }`
- HIR support for bounded generic parameters such as `fn f<T: Foo>(x: T)`
- a dedicated parse/HIR smoke fixture for the new syntax

This issue remains open because the parser/HIR slice did not include typechecker
impl registration / bound satisfaction, and the issue-level `--cargo` verification
is still blocked by unrelated pre-existing failures outside this slice.

## Partial slice note — 2026-04-15 (Wave 3)

Wave 3 landed commit `060db12c887b10bb73ed6b2d1441e526c56c5368`, which added:

- minimal typechecker-side impl registration for simple `impl Trait for Type`
- bound extraction from generic type parameters
- focused bound-satisfaction enforcement for generic calls
- a dedicated typecheck-level smoke fixture for trait impl satisfaction

This issue remains open because the product surface still needs issue-level cargo
green verification, and the broader workspace cargo failures are outside this
issue's slice scope.

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
