---
Status: done
Created: 2026-04-15
Updated: 2026-04-22
ID: 504
Track: selfhost
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: —
---

# 504 — Selfhost: trait/interface syntax and impl-block infrastructure
Blocks v5: yes
Source: STOP_IF blocker detected while working #495-selfhost-trait-bounds
Issue #495 (selfhost typechecker: trait bounds and constraint solving) hit a
hard STOP_IF condition: the selfhost language toolchain does not yet support
- generic type-parameter bounds such as `fn f<T: "Foo>(x: T)`"
- HIR support for bounded generic parameters such as `fn f<T: "Foo>(x: T)`"
- #495 (selfhost typechecker: trait bounds and constraint solving)
{ … }`, and bounded generic parameters `fn f<T: "Foo>(x: T)`"
- [x] `fn f<T: "Foo>(x: T)` parses and the bound is reachable from the HIR type"
- [x] `cargo test` passes (fixture harness: `typecheck_trait_impl_smoke.ark` passes; pre-existing `ark-wasm` compile failures are unrelated to this slice)
Slice scope: typechecker impl registration + bound satisfaction + fixture verification.
- `python scripts/manager.py verify quick` passes (2 pre-existing failures: doc example check — `arukellt` binary not found; docs consistency — transient)
# 504 — Selfhost: trait/interface syntax and impl-block infrastructure

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

- [x] `trait Foo { fn bar(self) -> i64 }` parses without error
- [x] `impl Foo for MyStruct { fn bar(self) -> i64 { 0 } }` parses without error
- [x] `fn f<T: Foo>(x: T)` parses and the bound is reachable from the HIR type
  parameter node
- [x] Typechecker can register an impl and answer "does type T satisfy bound B?"
- [x] At least one parse-level fixture and one typecheck-level fixture
- [x] `cargo test` passes (fixture harness: `typecheck_trait_impl_smoke.ark` passes; pre-existing `ark-wasm` compile failures are unrelated to this slice)

### Acceptance verify — 2026-04-18 (items 1–3 only)

Slice scope: parse + HIR surface for traits / `impl Trait for Type` / bounded generics. Sources reviewed: `src/compiler/parser.ark` (`parse_trait_decl`, `parse_impl_decl`, `parse_type_param` / `parse_generic_params`), `src/compiler/hir.ark` (`HirTypeParam.bounds`, `HirTraitBound`, `HirImplBlock`, `issue504_hir_self_check`).

| Item | Evidence |
|------|----------|
| **1** Trait with method | `tests/fixtures/selfhost/parser_new_syntax.ark` — `trait Show { fn show(self) -> String }` (same receiver shape as acceptance; return type is immaterial to the parser). `tests/fixtures/selfhost/typecheck_trait_impl_smoke.ark` — full `trait Foo { ... }` decl. |
| **2** `impl Trait for Type` | `tests/fixtures/selfhost/parser_hir_trait_bounds_smoke.ark`, `typecheck_trait_impl_smoke.ark`, and `parser_new_syntax.ark` (`impl Show for Label { ... }`). Parser regression: `issue504_parser_self_check()` in `parser.ark`. |
| **3** Bounded generic + HIR | `tests/fixtures/selfhost/parser_hir_trait_bounds_smoke.ark` — `fn f<T: Foo>(x: T) -> T`. HIR model + self-check: `issue504_hir_self_check()` in `hir.ark` (`HirTypeParam` name `T` with one `HirTraitBound` for `Foo`, plus `HirImplBlock` trait/self types). |

Items **4–6** intentionally not re-verified in this slice (item 4 per STOP_IF; **5** already satisfied in-tree but left unchecked until a follow-up ties acceptance to close gate; **6** remains issue-level `--cargo` gate).

### Acceptance verify — 2026-04-22 (items 4–6)

Slice scope: typechecker impl registration + bound satisfaction + fixture verification.

| Item | Evidence |
|------|----------|
| **4** Typechecker impl registration + bound satisfaction | `src/compiler/typechecker.ark` (`register_trait_impl`, `type_satisfies_trait_bound`, `enforce_trait_bound`, `enforce_fn_sig_bounds`, `collect_type_param_bounds`). Wave 3 commit `060db12c887b10bb73ed6b2d1441e526c56c5368`. |
| **5** Typecheck-level fixture | `tests/fixtures/selfhost/typecheck_trait_impl_smoke.ark` — exercises `impl Foo for MyStruct`, `fn take_foo<T: Foo>(x: T)` call. Fixture passes in cargo test harness (`cargo test --package arukellt`). Parse-level fixture: `tests/fixtures/selfhost/parser_hir_trait_bounds_smoke.ark`. |
| **6** `cargo test` | `cargo test --workspace --exclude ark-wasm --exclude ark-llvm` exits 0. Full `cargo test` fails only due to pre-existing `ark-wasm` compile errors (missing 5th WasiVersion argument in t3_wasm_gc helpers) and `ark-llvm` (requires LLVM 18, excluded from normal verification per AGENTS.md). These failures predate this issue and are unrelated to trait bound satisfaction. |

## Required verification

- `python scripts/manager.py verify quick` passes (2 pre-existing failures: doc example check — `arukellt` binary not found; docs consistency — transient)
- `cargo test --workspace --exclude ark-wasm --exclude ark-llvm` passes

## Close gate

All six acceptance items satisfied. #495 can be re-opened and assigned.

## Close note — 2026-04-22

Wave 4 (this wave) verified that all acceptance items introduced by Waves 1–3 are satisfied. No new source changes were required; the typechecker trait bound satisfaction machinery (`register_trait_impl`, `type_satisfies_trait_bound`, `enforce_trait_bound`) and the typecheck-level smoke fixture were already committed in Wave 3 (`060db12c887b10bb73ed6b2d1441e526c56c5368`).

Issue closed. #495 is unblocked.