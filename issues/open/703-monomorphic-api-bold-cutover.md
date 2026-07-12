---
Status: open
Created: 2026-06-27
Updated: 2026-07-12
ID: 703
Track: stdlib-api
Depends on: "700, 701, 691, 695"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#700 builtin method syntax, #701 associated function syntax, #691 Iterator, #695 Ord"
Blocks v{N}: none
Priority: 2
Source: Method-syntax-first stdlib direction 2026-06-27 / ADR-036 D2 / ADR-046
---

# 703 — Monomorphic API cutover (ADR-036 D2 + ADR-046)

## Summary

Arukellt's stdlib has **parallel monomorphic and (planned) generic** APIs
for `Vec` operations, sorting, and higher-order sequence functions.
These monomorphic free APIs must be **removed** from the user-reachable
surface ([ADR-046](../../docs/adr/ADR-046-free-function-eradication.md)).

Deletion timing follows **ADR-014 + ADR-046 D4** (not “delete stable
without notice”):

1. Provide the generic / method replacement (#700 / #701 / #691 / #695).
2. Mark `stable` symbols deprecated (`deprecated_by` + W0009) for at least
   one release with a migration guide.
3. Delete after the deprecation window (`experimental` may delete sooner).

Bold cutover without deprecation applies only to `experimental` or
already-deprecated symbols (ADR-036 D2 as clarified by ADR-046).

Why remove the parallel surface:

1. Monomorphic and generic variants coexisting confuses LLM code
   generation.
2. Free `*_i32` helpers block trait-first docs and scorecard zero.
3. Deprecation metadata in `std/manifest.toml` already marks direction
   for some symbols; this issue finishes the rest under ADR-014.

## Current state

**Monomorphic APIs to remove:**

- `Vec_new_i32()`, `Vec_new_i64()`, `Vec_new_f64()`, `Vec_new_String()`,
  `Vec_new_i32_with_cap(n)`, etc.
  → replaced by `Vec::new<T>()`, `Vec::with_capacity<T>(n)` (#701)
- `Vec_push_i32(v, x)`, `Vec_get_i32(v, i)`, `Vec_set_i32(v, i, x)`,
  `Vec_len_i32(v)`, etc.
  → replaced by `v.push(x)`, `v.get(i)`, `v.set(i, x)`, `v.len()` (#700)
- `sort_i32(v)`, `sort_i64(v)`, `sort_f64(v)`, `sort_String(v)`
  → replaced by `v.sort()` / `sort<T: Ord>(v)` (#695, #700)
- `map_i32_i32(v, f)`, `filter_i32(v, f)`, `fold_i32_i32(v, init, f)`,
  `any_i32(v, f)`, `find_i32(v, f)`, `contains_i32(v, x)`,
  `reverse_i32(v)`, `remove_i32(v, x)`
  → replaced by `v.map(f)`, `v.filter(f)`, `v.fold(init, f)`, etc.
  via `Iterator` adapters (#691) and `impl Vec<T>` methods (#700)
- `sum_i32(v)`, `product_i32(v)`
  → replaced by `v.sum()` / `Iterator::sum<T: Add>` (#691)
- `std::seq` module (`unique`, `seq_reverse`, `count_eq`,
  `binary_search`, `min_i32`, `max_i32`)
  → replaced by `std::iter` + `Vec<T>` methods

**Already deprecated in `std/manifest.toml`:**
- `Vec_new_i32` (deprecated_by: `Vec::new`)
- `Vec_new_i64` (deprecated_by: `Vec::new`)
- `filter_i32` (deprecated_by: `Vec::filter`)

**Not yet deprecated (need generic replacement first):**
- `sort_i32`, `map_i32_i32`, `fold_i32_i32`, `Vec_push_i32`, etc.

## Required work

### Prerequisite (blocked by upstream issues)

- [ ] #700 lands `impl Vec<T>` with `push` / `get` / `set` / `len` /
      `is_empty` / `clear` methods.
- [ ] #701 lands `Vec::new<T>()` / `Vec::with_capacity<T>(n)`.
- [ ] #691 lands `Iterator` trait with `map` / `filter` / `fold` /
      `any` / `find` / `sum` / `product` adapters.
- [ ] #695 lands `Ord` trait for `sort` / `binary_search`.

### Cutover

- [ ] **Deprecate** (if `stable`) then **delete** all monomorphic `Vec_*`
      APIs from prelude (`std/prelude.ark`): `Vec_new_i32`, `Vec_push_i32`,
      `Vec_get_i32`, `Vec_set_i32`, `Vec_len_i32`, `Vec_new_i32_with_cap`,
      etc. Follow ADR-014 / ADR-046 D4.
- [ ] **Deprecate** (if `stable`) then **delete** all monomorphic
      sort/map/filter/fold functions from `std/seq/mod.ark` and prelude:
      `sort_i32`, `sort_i64`, `sort_f64`, `sort_String`, `map_i32_i32`,
      `filter_i32`, `fold_i32_i32`, `any_i32`, `find_i32`, `contains_i32`,
      `reverse_i32`, `remove_i32`, `sum_i32`, `product_i32`.
- [ ] **Delete** `std::seq` module entirely — replace with `std::iter`
      (from #691) and `impl Vec<T>` methods (from #700).
- [ ] **Update** `std/manifest.toml` — remove deleted entries; ensure
      deprecation metadata existed for former `stable` symbols.
- [ ] **Update** all in-tree callers (compiler source, fixtures,
      benchmarks) to use method syntax / associated function syntax /
      `Iterator` adapters.
- [ ] **Migration guide**: Update `docs/stdlib/migration-guidance.md`
      with before/after table for each deleted API.
- [ ] **[breaking]** Create breaking change notice with migration guide.

### Fixtures

- [ ] All fixtures updated to use method syntax / generic API.
- [ ] No fixture references `Vec_new_i32`, `sort_i32`, `map_i32_i32`,
      etc.
- [ ] `python3 scripts/manager.py verify --full` exits 0.

## Acceptance

- [ ] `std::seq` module deleted.
- [ ] No monomorphic `Vec_*_i32` / `sort_*` / `map_*_*` / `fold_*_*`
      functions in prelude or stdlib.
- [ ] All in-tree code uses `Vec::new<T>()`, `v.push(x)`, `v.sort()`,
      `v.map(f)`, etc.
- [ ] `docs/stdlib/migration-guidance.md` documents the cutover.
- [ ] `python3 scripts/manager.py verify --full` exits 0.

## References

- Depends on: #700 (builtin method syntax), #701 (associated function
  syntax), #691 (Iterator), #695 (Ord)
- Related: #697 (Vec operation extension), #709 (eradication policy),
  #718 (free-function inventory)
- ADR-046 (free-function eradication)
- ADR-036 D2 (deletion policy; stable via ADR-014)
- `std/prelude.ark`, `std/seq/mod.ark`, `std/manifest.toml`
- `docs/stdlib/migration-guidance.md`, `docs/stdlib/trait-stdlib-redesign.md`
