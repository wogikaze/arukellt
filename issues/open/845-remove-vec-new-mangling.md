---
Status: open
Created: 2026-07-28
Updated: 2026-07-29
ID: 845
Track: stdlib-api
Depends on: #700, #701, #703
Related: ADR-046, ADR-036, #842, #718
Orchestration class: design-then-implement
Blocks v4 exit: False
Priority: 2
Source: Front-load #703 Vec ctor cutover — remove Vec_new_* mangling (#701 rewrite debt)
---

# Remove `Vec_new_*` mangling and monomorphic constructors

## Summary

`#701` shipped `Vec::new<T>()` by rewriting AST text to `Vec_new_T`.
That legacy mangling must go. Target: real `Vec::new<T>()` / associated
path → mono → `raw::raw_array_new`, then delete in-tree `Vec_new_*` and
prelude / `#842` Vec registry entries.

## Scope

1. `impl Vec<T> { fn new() / with_capacity() }`
2. Stop parser rewrite `Vec::new` → `Vec_new_`
3. Migrate all in-tree `Vec_new_<Type>()` to `Vec::new<Type>()`
4. Delete prelude / manifest `Vec_new_*`, resolver `Vec_new_` escape,
   `#842` `sig_reg_register_vec_news` / `infer_vec_new_return_gc_type`

## Non-goals

- Full `#703` (sort/map/filter monomorphics, `std::seq`)
- `String::from` / `i32::from` rewrite (follow-up after Vec green)

## Acceptance

- [x] Parser does not rewrite `Vec::new` → `Vec_new_`
- [x] `tests/fixtures/associated_fn/vec_new.ark` passes without rewrite
- [x] No in-tree `Vec_new_(i32|i64|f64|String|v128|…)\(` constructors
- [x] Prelude / manifest user-reachable `Vec_new_*` removed
- [ ] `selfhost build-compiler` + `verify lane` (+ `--gate t3` at end) PASS
  - build-compiler + `verify lane`: PASS (2026-07-29)
  - `--gate t3`: in progress

## Implementation notes

- Real `impl Vec<T> { new / with_capacity }` → `raw::raw_array_new`
- Turbofish accepts named / `fn` / tuple type starts
- Bare `Vec::new<T>()` (no let annotation): typechecker records mono from
  path children (`call_vec_ctor.ark`); MIR marks `vec:Elem` from rewrite
- Do not auto-load `std::collections::vec` into every program — inherent
  method shells steal `push` and break `format_i32` (use CoreOp path)

## References

- #703 — parent monomorphic cutover (Vec ctor portion moved here)
- #701 — associated syntax (rewrite debt)
- #842 — registry still lists `Vec_new_*` (delete after migration)
