---
Status: open
Created: 2026-06-27
Updated: 2026-10-25
ID: 703
Track: stdlib-api
Depends on: "none"
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: Method-syntax-first stdlib direction 2026-06-27 / ADR-036 D2 / ADR-046
---

# 703 — Monomorphic API cutover (ADR-036 D2 + ADR-046)

## Summary

Remove the remaining **monomorphic free functions** from the user-reachable
stdlib surface ([ADR-046](../../docs/adr/ADR-046-free-function-eradication.md)).
The replacement shape is `Vec<T>` / `HashMap<K, V>` methods and the
`std::core::iter` pipeline; there is no permanent free-function bridge.

Deletion timing follows **ADR-014 + ADR-046 D4**: provide the replacement,
deprecate `stable` symbols for at least one complete release, then delete.
`experimental` / already-deprecated symbols may be removed sooner, but only
inside their recorded removal window.

## Ownership (Wave 0 correction)

The pre-2026-10 issue text claimed `Vec_new_*` / `Vec_push_*` constructors.
Those were removed by [#845](../done/845-remove-vec-new-mangling.md); the
remaining cutover is the following, single owner per family:

| Family | Symbols | Owner |
|--------|---------|-------|
| Deprecated prelude wrappers (`concat`, `char_at`, `print`, `println`) | 4 | #703 |
| Deprecated `std::seq` helpers (`map_i32_i32`, `filter_i32`, `take_i32`, `skip_i32`, `fold_i32_i32`, `binary_search`, `min_i32`, `max_i32`, `sum_i32`, `seq_reverse`, `seq_contains`, `unique`, `count_eq`) | 13 | #703 |
| Stable prelude monomorphic seq/combinator frees (`sort_i32/i64/f64/String`, `map_*`, `filter_*`, `fold_*`, `contains_*`, `reverse_*`, `remove_i32`, `sum_*`, `product_*`, `any_*`, `find_*`, `map_option_*`, `map_result_*`) | 34 | #703 |
| **#703 total** | **51** | **#703** |
| Every other eradication family (numeric conversions, `HashMap_*` builtins, io, collections, wasm, simd, wit, test, …) | — | #917 |

`docs/data/stdlib-api-eradication.toml` is the machine-readable owner of
this split: the `deprecated prelude wrappers`, `deprecated std::seq helpers`,
and `monomorphic seq/combinator frees` rules all carry `owner = "703"`.
#917 stays the execution umbrella for the other families and for the final
retirement of the `std::seq` module itself (its non-deprecated generic
helpers `lower_bound`, `upper_bound`, `binary_search_by`, `min_by`, … are
registered entries owned by #917).

## Current state (2026-10-25)

- Upstream code issues #700 (method syntax), #701 (associated syntax),
  #691 (`Iterator`), and #695 (`Ord`) are **closed**. #691 delivered the
  concrete `i32` / `String` iterator lanes, not the full adapter surface.
- The 4 prelude wrappers and 13 `std::seq` helpers already carry
  `deprecated_by` in `std/manifest.toml`.
- The 34 stable prelude monomorphic frees are **not yet deprecated** and
  still ship as `stable` prelude entries.
- `std/manifest.toml` is version `0.1.0`; `[deprecation_policy]` sets
  `default_remove_in = "1.0.0"` and `minimum_complete_releases = 1`.
  No #703 symbol may be deleted before that window closes.
- The eradication gate now measures the real surface: 1431 top-level
  `pub fn` declarations under `std/**/*.ark`, of which 447 have no manifest
  entry. Eradication scope is 811 manifest public entries + 447
  unregistered declarations = 1258 symbols
  (`docs/stdlib/eradication.md`).

## Required work

### Wave 0 — foundation (this lane)

- [x] Measure the real std surface in
      `scripts/check/check-stdlib-eradication.py` /
      `docs/stdlib/eradication.md`, including unregistered `pub fn`
      declarations and manifest entries without a declaration.
- [x] Correct the ownership split in
      `docs/data/stdlib-api-eradication.toml` (51 symbols here, #917 for
      the other families).
- [x] Stop advertising nonexistent generic frees (`sort<i32>(v)`,
      `map<i32, i32>(v, f)`, `reverse<i32>`, `sum<i32>`, `product<i32>`,
      …) in `docs/stdlib/migration-guidance.md`.
- [x] Teach `scripts/check/check-compat-orphans.py` about `Type::method`
      `deprecated_by` targets so Wave 1 can point at `Vec::sort()` and
      similar replacements without a false orphan.
- [ ] Add W0009 coverage for `std::seq` imports/calls — **blocked**, see
      [W0009 coverage blocker](#w0009-coverage-blocker).

### Wave 1 — deprecate the 34 stable monomorphic frees

- [ ] Add `deprecated_by`, `deprecated_since`, and `remove_in` to the 34
      prelude entries in `std/manifest.toml` (ADR-014 window).
- [ ] Add the matching rows to `docs/stdlib/migration-guidance.md`.
- [ ] W0009 / E0100 diagnostic coverage (blocked; see below).
- [ ] Update in-tree callers (compiler source, fixtures, benchmarks) to
      `v.sort()`, `v.iter().map(f).collect()`, `v.iter().sum()`, … as the
      method replacements land (#697 for `Vec::sort`).

### Wave 2 — delete inside the 1.0.0 window

- [ ] Delete the 51 manifest entries and their prelude / `std::seq`
      implementations after `remove_in = 1.0.0` is reachable.
- [ ] Do **not** delete the whole `std::seq` module in this issue: the
      nine non-deprecated generic helpers belong to #917.
- [ ] Update `std/manifest.toml`, generated docs, and the fixture set
      (104 fixtures / 252 manifest lanes need the cutover; do not migrate
      them before the deletion wave).
- [ ] Publish the breaking-change notice with the before/after table.

## W0009 coverage blocker

`src/compiler/lint/deprecated_table.ark` has no live warning for the
`std::seq` helpers, and the four deprecated prelude wrappers are gated
before lint:

- Bare prelude names (`concat`, `char_at`, `print`, `println`) fail name
  resolution under the ADR-072 v2 prelude contract with **E0100** and a
  `resolver::removed_names` migration hint. `deprecated_call_message` is
  intentionally empty, so no W0009 can fire for a gated bare name.
- `checks_extra.ark` only inspects `use` paths through
  `deprecated_use_message` (module renames) and qualified calls through
  `deprecated_qualified_call_message`, which dispatches `math` / `cmp` /
  `convert` / `hash` only. A `seq::map_i32_i32(...)` path therefore emits
  no warning.
- `check-compat-orphans.py` currently maps qualified warning aliases to
  `std::core::<mod>`; `seq` would need an explicit module mapping.

Adding this coverage changes compiler warning behavior and needs a new
`.selfhost.diag` golden, so it is out of scope for this docs/tooling-only
foundation lane. Concrete unblock steps: (1) add
`deprecated_seq_qualified_message` to `deprecated_table.ark`, (2) map
`seq` → `std::seq` in the orphan checker, (3) add the diagnostic golden,
(4) run the fixture suite. Tracked here and in #917.

## Acceptance

- [ ] All 51 #703 symbols are removed from the user-reachable surface after
      the ADR-014 window.
- [ ] No `sort_*` / `map_*_*` / `fold_*_*` / `reverse_*` / `sum_*` /
      `product_*` free function remains in prelude or `std::seq`.
- [ ] All in-tree code uses `v.sort()`, `v.iter().map(f).collect()`,
      `v.iter().sum()`, etc.
- [ ] `docs/stdlib/migration-guidance.md` and the generated eradication
      scorecard agree with `std/manifest.toml`.
- [ ] `python3 scripts/check/check-stdlib-eradication.py` and
      `python3 scripts/check/check-compat-orphans.py` pass.
- [ ] `python3 scripts/manager.py verify full` exits 0.

## References

- ADR-046 (free-function eradication), ADR-014 (stability labels),
  ADR-036 D2 (deletion policy)
- #709 (policy / scorecard, done), #917 (execution umbrella),
  #845 (`Vec_new_*` removal, done), #697 (`Vec<T>` operation extension)
- #700, #701, #691, #695 (closed upstream code work)
- `std/prelude.ark`, `std/seq/mod.ark`, `std/manifest.toml`
- `docs/data/stdlib-api-eradication.toml`,
  `docs/stdlib/eradication.md`, `docs/stdlib/migration-guidance.md`
- `scripts/check/check-stdlib-eradication.py`,
  `scripts/check/check-compat-orphans.py`
