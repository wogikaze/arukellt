---
Status: open
Created: 2026-06-27
Updated: 2026-10-26
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
- Wave 1 landed 2026-10-26 (branch `wave/917-stable64`): 31 of the 34 prelude
  monomorphic frees carry `stability = "deprecated"`, `deprecated_by`,
  `deprecated_since = "0.1.0"`, and `remove_in = "1.0.0"`, and their bare names
  emit W0009. Two entries (`map_option_String_String`, `map_result_i32_i32`)
  were stale non-callable builtins and were retired directly; `filter_i32`
  remains provisional (individual deferral). The i64/f64/String iterator
  consumer lanes are still planned, so some replacement rows describe the
  target shape rather than a verified call path.
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
- [x] Add W0009 coverage for qualified `std::seq` calls (the 13 deprecated
      helpers), with both-target diagnostic fixtures. See
      [W0009 coverage status](#w0009-coverage-status).

### Wave 1 — deprecate the 34 stable monomorphic frees

- [x] Add `deprecated_by`, `deprecated_since`, and `remove_in` to the 31
      callable prelude entries in `std/manifest.toml` (ADR-014 window).
      `map_option_String_String` / `map_result_i32_i32` were retired as stale
      non-callable entries; `filter_i32` stays provisional (defer).
- [x] Add the matching rows to `docs/stdlib/migration-guidance.md`.
- [x] W0009 diagnostic coverage for the 31 stable frees (bare-name table +
      per-symbol unit assertions + live diag fixtures on wasm32-gc/wasm32).
- [ ] Update in-tree callers (compiler source, fixtures, benchmarks) to
      `v.sort()`, `v.iter().map(f).collect()`, `v.iter().sum()`, … as the
      method replacements land (#697 for `Vec::sort`). This is Wave 2 work:
      the i64/f64/String consumer lanes and collected-Vec receiver methods
      still hit compiler emitter gaps.

### Wave 2 — delete inside the 1.0.0 window

- [ ] Delete the 49 remaining manifest entries and their prelude / `std::seq`
      implementations after the W0009 window / ADR-087 conditions are
      recorded. Two stale entries were already retired in Wave 1.
- [ ] Do **not** delete the whole `std::seq` module in this issue: the
      nine non-deprecated generic helpers belong to #917.
- [ ] Update `std/manifest.toml`, generated docs, and the fixture set
      (104 fixtures / 252 manifest lanes need the cutover; do not migrate
      them before the deletion wave).
- [ ] Publish the breaking-change notice with the before/after table.

## W0009 coverage status

`src/compiler/lint/deprecated_table.ark` maps every deprecated
`std::seq` helper to a replacement message, and
`deprecated_qualified_call_message` dispatches the `seq` entries.
`scripts/check/check-compat-orphans.py` maps the `seq` warning alias to
`std::seq`, so the table cannot silently retain dead entries.

Coverage is limited to qualified calls (`use std::seq` plus
`seq::map_i32_i32(...)`) because the bare helper names overlap the
still-stable prelude frees (`sum_i32`, `sort_i32`, ...), which must not
warn before Wave 1 deprecates them.

Live-call evidence:

- `tests/fixtures/diagnostics/deprecated_seq_map_i32_i32.ark` exercises
  the default `wasm32-gc` target and expects the W0009 replacement.
- `tests/fixtures/diagnostics/deprecated_seq_sum_i32_wasm32.ark` passes
  `--target wasm32 --wasi-version wasi-p1` through its `.flags` sidecar
  and expects the W0009 replacement.
- Both fixtures are listed under `diag:` in
  `tests/fixtures/manifest.txt`, so
  `python3 scripts/manager.py selfhost diag-parity` compares their output
  with the committed goldens.

The four deprecated bare prelude wrappers (`concat`, `char_at`, `print`,
`println`) remain outside W0009: the ADR-072 v2 prelude gates them before
lint with **E0100** and a `resolver::removed_names` migration hint. The
34 stable prelude monomorphic frees remain warning-free until their
`deprecated_by` metadata lands in Wave 1.

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
