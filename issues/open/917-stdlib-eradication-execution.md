---
Status: open
Created: 2026-10-11
Updated: 2026-10-27
ID: 917
Track: stdlib-api
Depends on: "709"
Orchestration class: incremental
Orchestration upstream: "#709 policy, #703 monomorphic cutover, #718 inventory"
Blocks v{N}: none
Priority: 1
Source: ADR-046 eradication execution umbrella split from #709 inventory
---

# 917 — Free-function eradication execution (must-delete / associated-or-method waves)

## Summary

Issue #709 produced the policy, classification table, and generated
scorecard for ADR-046 free-function eradication
(`docs/data/stdlib-api-eradication.toml`, `docs/stdlib/eradication.md`).
This issue owns the **execution**: actually deleting, deprecating, or
replacing every public free / `prelude_wrapper` / monomorphic helper
classified there, in dependency-safe waves.

## Current state (2026-10-26)

- `docs/data/stdlib-api-eradication.toml` now classifies both
  `std/manifest.toml` `[[functions]]` entries and top-level `pub fn`
  declarations under `std/**/*.ark`: 816 manifest functions (815 public) +
  413 unregistered declarations = **1228** public symbols
  (`must delete` 122, `associated-or-method` 1106, `intrinsic-only` 1).
- `scripts/check/check-stdlib-eradication.py` enforces the dual surface:
  no unclassified manifest entry or std declaration, no manifest entry
  without a source declaration, no dead rule, every public class has an owner.
- The generated scorecard counts remaining user-reachable free symbols;
  **zero** is the goal.
- Migration precedent: #718 (done) delivered the Tier-1 wave — method /
  associated replacements emitted before frees retired.
- Deletions are gated by ADR-014 (manifest 0.1.0, `remove_in 1.0.0`) and by
  the surface replacements owned by #703/#697/#708/#693/#710/#702. The
  0.x exception in `docs/stdlib/stability-policy.md` allows experimental
  entries and already-gated deprecated prelude bindings only; the stable 64
  wait for the ADR-014 addendum and the `std::seq` families wait for W0009.

### Landed slice (wave/917-eradication)

- Deleted the 2 `experimental` items: `std::wasm::memory_copy` /
  `memory_fill` (manifest entries + `pub fn` wrappers). The
  `tests/fixtures/stdlib_wasm/memory_ops.ark` fixture now verifies the
  non-public `__intrinsic_memory_*` bridge; the module overview documents
  that no public replacement exists (ADR-046 D2 / ADR-014 D4).
- Deleted the 4 deprecated, already-gated prelude wrappers `concat`,
  `char_at`, `print`, `println` under the 0.x exception (manifest entries +
  `std/prelude.ark` decls). Their E0100 `removed_names` hints stay active;
  internal bare calls in `std/prelude.ark`, `std/json.ark`,
  `std/json/parser.ark`, and `std/toml/parser.ark` were qualified.
- Retired 6 `std::collections::sort` free duplicates (`sort_by`,
  `sort_by_key`, `sort_unstable_by`, `sort_unstable_by_key`, `partition`,
  `is_sorted_by`; 5 counted — `sort_by` shares the registered name with
  `std::seq::sort_by`). Their `Vec<T>` methods landed in #697; the 4
  remaining no-argument frees (`sort`, `sort_unstable`, `is_sorted`,
  `select_nth`) still have no method spelling.
- Scorecard delta: 1257 -> **1246** public symbols (`must delete` 128 ->
  122, `associated-or-method` 1129 -> 1124); 1430 -> 1418 `pub fn`
  declarations; 436 -> 431 unregistered; 98 -> 96 `prelude_wrapper` entries.

### Landed slice (wave/917-stable64, 2026-10-26)

- **Stable-64 lifecycle conversion (ADR-087 Wave 1)**: 60 `stable` entries
  now carry `stability = "deprecated"`, `deprecated_by`,
  `deprecated_since = "0.1.0"`, and `remove_in = "1.0.0"` — 16 numeric
  conversions, 9 vec intrinsic-compat frees, 4 assertion frees, and 31
  seq/combinator frees. `i8_to_i32` is an individual deferral: its method
  frame still traps under wasm32-gc, so it stays stable. `filter_i32` stays
  provisional.
- **Retired stale non-callable entries (3)**: `clear`,
  `map_option_String_String`, `map_result_i32_i32` had no declaration and no
  resolver registration; they were removed with E0100 hints and method /
  `match` replacements.
- **Replacement surface**: canonical scalar conversion methods in
  `std/core/convert.ark` (`u8/u16/i16::to_i32`, `i32::to_u8_wrapping` /
  `to_i8_wrapping` / `to_u32_bits`, `u32::to_u64`, `i32::into`,
  `i64::to_i32_checked`, `f32::to_f64`, `f64::to_f32`, `f64::to_bits_*`);
  `Vec::clear` / `as_slice` and `Slice::len` / `get`;
  `VecIter::any` / `find` / `IteratorContains<String>`;
  `std/core/container.ark` (`Option<T>::map`, `Result<i32,i32>::map`).
- **W0009**: 60 bare-name entries in `src/compiler/lint/deprecated_table.ark`
  with per-symbol unit assertions and live diag fixtures on wasm32-gc and
  `--target wasm32`. The existing qualified `std::seq` coverage for the 13
  deprecated helpers is unchanged.
- **Fixture evidence**: 4 run fixtures (`scalar/numeric_conversion_methods`,
  `collections/vec_method_replacements`, `iterator/seq_method_replacements`,
  `stdlib_test/assertion_replacements`) plus 8 diag fixtures (4 symbols × 2
  targets) — 12 new manifest entries; `run:` count 1823 -> 1827.
- **Scorecard delta**: 1246 -> **1243** public symbols; `must delete`
  122 -> 119; `associated-or-method` 1124 unchanged. Stability: `stable`
  378 -> **315**, `deprecated` 37 -> **97**; manifest public entries 815 ->
  **812**; `prelude_wrapper` entries stay 96 (deprecation, not deletion).
- **Verification (wave/917-stable64)**: `verify fixtures` 1827 compile+validate
  PASS / 0 FAIL (1823 + 4 new run fixtures); `selfhost diag-parity` PASS with
  the rebuilt s2 (114 diag fixtures, including the 8 new W0009 fixtures);
  `docs check` 6/6; `check-stdlib-eradication` and `check-compat-orphans`
  clean; `verify lane` PASS; `check-cross-backend-parity.py` 4/4 new run
  fixtures parity (wasm32-gc component vs wasm32 linear).
- **Residuals**: the 31 Wave-2 deletions, the 1097
  `associated-or-method` replacements, the `i8_to_i32` deferral, the 3
  deferred intrinsic bodies (`i32_to_u16`, `i32_to_i16`, `u64_to_u32`), the
  i64/f64/String iterator consumer lanes, and the compiler/std migration of
  the ~9k vec intrinsic-compat call sites.
- Provisional decision (1): `filter_i32` is deferred with the deprecated
  `std::seq::filter_i32` family (W0009 qualified coverage and the ADR-014
  window). The 20 `HashMap_*` builtins were retired on 2026-10-27
  (`wave/917-hashmap`); their compiler registrations and the i32→i32
  adapters were removed together, so no still-callable builtin remains.

### Landed slice (wave/917-1124)

- Retired the 22 public free functions of the `std::core` value-helper
  family (18 scorecard-counted): the 12 `std::core::either` helpers became
  `Either` enum constructors / inherent methods, the 7 `std::core::math`
  numeric helpers became `i32::gcd` / `i32::lcm` plus `Integer` trait methods
  (`gcd` was name-collided with the prelude wrapper and is not counted), and
  the 3 Option-returning `std::core::convert` parse adapters became
  `TryFrom<String>` associated functions (`i32::try_from`, …); those three
  declarations were already uncounted because the prelude `parse_*` entries
  own their names.
- All removals are unregistered source declarations, so no manifest
  deprecation window or ADR-087 exception applied. `std::core::math` remains
  as the owner of the checked / saturating arithmetic trait impls.
- Added both-target fixtures: `stdlib_core/either_methods*.ark`,
  `stdlib_core/integer_methods*.ark`, and
  `stdlib_trait/try_from_parse_wasm32.ark`; migrated the existing
  either/math/convert fixtures to the method surface.
- Removed the 5 dead `math::*` W0009 table entries and the
  `convert::parse_*` signature-registry / body-defer special cases from
  `src/compiler` (stage-2 rebuilt once).
- Scorecard delta: 1246 -> **1228** public symbols (`associated-or-method`
  1124 -> 1106); 1418 -> 1396 `pub fn` declarations; 431 -> 413
  unregistered; manifest 815 and `prelude_wrapper` 96 unchanged.
- Residual in this family: `std::core::mod::identity` (no method receiver) and
  the 20 `std::core::raw` representation bridges (need a RawArray/RawString
  namespace surface). No stable manifest entry was touched.

### Landed slice (wave/917-seq33, 2026-10-27)

- Deleted the 31 callable `stable` seq/combinator prelude frees (the
  [#703](703-monomorphic-api-bold-cutover.md) family) under ADR-087 Wave 2:
  `sort_*`, `map_*`, `filter_*` (except the provisional `filter_i32`),
  `fold_*`, `any_*`, `find_*`, `contains_i32` / `contains_String`,
  `reverse_*`, `remove_i32`, `sum_i32`, `product_i32`,
  `map_option_i32_i32`, and `contains_string`. Manifest entries and
  `std/prelude.ark` declarations are gone; bare calls now fail with E0100 and
  a `resolver::removed_names` hint.
- Migrated the in-tree call sites: compiler/std call sites, 84 fixture files,
  benchmarks, and the non-generated docs examples. The i32 and
  `Vec<String>` map/filter lanes use `v.iter()...`; the i64/f64 numeric
  consumers, the `Vec<String>` reverse/contains lanes, and the generic
  `Vec::sort_by` receiver for i64/f64 use the documented explicit loop /
  `std::seq::sort_by` fallback because their emitter lanes still fail Wasm
  validation.
- Diagnostics: the two `deprecated_seq_monomorphic*` fixtures now assert the
  E0100 removed-name hint; the qualified `std::seq` W0009 fixtures are
  unchanged.
- Scorecard delta: 1168 → **1137** public symbols; manifest public entries
  776 → **745**; `prelude_wrapper` 96 → **65**; `pub fn` declarations 1359 →
  **1328**; `must delete` 114 → **83**; `associated-or-method` 1054 unchanged.
  The monomorphic seq/combinator family drops from 32 rows to **1**
  (`filter_i32`, provisional deferral).
- Verification: `verify fixtures` 1836/1836 compile+validate PASS (105
  runtime smoke, 1 `test:` fixture PASS); cross-backend parity 81/81 for the
  changed run fixtures; `selfhost diag-parity` PASS; `check-stdlib-eradication`
  and `check-compat-orphans` clean; `docs check` 6/6; `verify lane` PASS with
  the pinned bootstrap lint CLI (the stage-2 stub host traps in `lint`
  `parse_args`, a pre-existing tooling residual). The ADR-029 fixpoint result
  is recorded in the wave receipt.
- Residuals: the i64/f64 numeric iterator consumer lanes, the
  `Vec<String>` reverse/contains lanes, the `Vec::sort_by` receiver gap for
  i64/f64, the provisional `filter_i32`, the 1054
  `associated-or-method` replacements, and the #693 io residuals.

### Landed slice (wave/917-hashmap, 2026-10-27)

- Retired the 20 `provisional` monomorphic `HashMap_*` builtins and the
  compiler registrations that kept them callable: `resolver::builtins_hash`
  (the 60-name typed list), `typechecker::builtins_hash` (return types),
  `mir::lower::call_type_hash`, `wasm::hash_aliases`,
  `wasm::sections_types_gc_override`, `mir::lower::signature_registry_builtin`,
  `mir::lower::ctx_fn_return_vt`, `wasm::intrinsic_hashmap_str`, the native-c
  callee map, and the loader's legacy-spelling auto-load rules.
- Retired the 10 `std/collections/hash_map.ark` i32→i32 adapters in the same
  change. `HashMap<i32, i32>` now lowers directly to the flat
  `std::collections::hash` store (`hashmap_new` / `hashmap_insert` /
  `hashmap_get_option` / `hashmap_contains` / `hashmap_remove` /
  `hashmap_size` / `hashmap_is_empty` / `hashmap_keys` / `hashmap_values` /
  `hashmap_entry_pairs`). The other supported pairs keep their typed
  `hash_map::` adapters; they consume only public source functions now.
- Added the E0100 removed-name hint (prefix `HashMap_`) and migrated fixtures,
  cookbook/migration docs, and the changelog. All 20 fixture call sites now
  use the methods.
- Scorecard delta: 1226 → **1196** public symbols (`must delete` 119 → 99,
  `associated-or-method` 1107 → 1097); manifest public entries 813 → 793;
  1397 → 1387 `pub fn` declarations; 413 → 403 unregistered.
- Verification: s2 rebuilt after the final compiler edits; `verify fixtures`
  1835/1835 compile+validate PASS; both-target (wasm32-gc / wasm32) parity for
  the 6 pair families, `hashmap_insert_return`, `hashmap_entries_values`, the
  two typed i32→i32 fixtures, and the renamed `collections/hashmap_*_method`
  fixtures; `docs check` 6/6, `check-stdlib-eradication`,
  `check-compat-orphans`, and `selfhost fixpoint` (s2 == s3 at
  `3617a54e…`; the pin-built stage-2 was one generation behind, so the
  self-built stage-3 was adopted as stage-2 for the final check).

### Landed slice (wave/917-nextgroup, 2026-10-27)

- Retired the 33 non-AST `std::wit` symbols: the 16 `experimental`
  `wit_type_*` manifest entries plus their `std::wit` forwarding wrappers,
  and the 17 unregistered `types` / `scan` / `names` / `printer`
  declarations. ADR-014 D4 allowed direct deletion for the experimental
  entries; no ADR-087 deprecation window or W0009 entry applies.
- Replacement surface: `WitType::Bool` … `WitType::String` variants,
  `WitType::list` / `WitType::option`, `ty.id()` / `ty.name()` /
  `ty.wit_text()`, `WitType::from_id` / `WitType::name_from_ark`;
  `WitText` namespace `find_char` / `trim` / `strip_version` /
  `before_colon` / `split_param` / `split_csv_items` / `line_start` /
  `line_end`; `WitName` namespace `to_kebab` / `to_snake` / `to_pascal` /
  `capitalize`; `World::to_wit()` (with `Interface::to_wit` /
  `WitFunc::to_wit` absorbing the private printer helpers).
- E0100: the qualified removals carry hints in
  `resolver::removed_names::removed_module_member_hint` for the `wit`,
  `types`, `scan`, `names`, and `printer` aliases, with unit assertions in
  the same file. The 16 manifest entries were experimental, so W0009 does
  not apply.
- Compiler-shape probe: `WitType`'s recursive `List` / `Option` payload
  binds compile and validate on wasm32-gc (method receiver and associated
  constructors), unlike the deferred `WitNode` accessors.
- Fixtures: migrated `wit_basic`, `wit_types`, `wit_type_basic`,
  `wit_names`, `wit_ast_parse`, `wit_print`, `world_build`; renamed
  `world_methods_wasm32` to `wit_methods_wasm32` and extended it over the
  types / scan / names / printer surface. `run:` count stays 1836.
- Scorecard: 1168 → **1135** public symbols (`associated-or-method`
  1054 → **1021**; `must delete` 114 and `intrinsic-only` 1 unchanged);
  777 → **761** manifest functions; 1359 → **1310** `pub fn` declarations;
  392 → **375** unregistered declarations. `prelude_wrapper` stays 96.
- Residual: `std::wit::ast` (23 declarations, deferred on the recursive
  `WitNode` receiver shape), the 3 `std::wit::parser` frees, and the
  `std::component` / `std::component::handle` families remain in the
  `std::wit/component frees` rule.

### Landed slice (wave/917-iter44, 2026-10-27)

- Retired the 44 `provisional` eager iterator constructor / collector frees in
  `std::core::iter`: the 16 adapter constructors (`vec_iter` … `rev_iter`) and
  the 28 collectors / consumers (`collect_*`, `iter_fold_i32`,
  `iter_count_i32`, `iter_sum_i32`, `iter_any_i32`, `iter_all_i32`,
  `iter_contains_i32`, `iter_count_eq_i32`, `iter_unique_i32`,
  `iter_reverse_i32`). Manifest entries and top-level `pub fn` declarations are
  gone. Provisional entries need a changelog notice and a migration row, not an
  ADR-087 window or W0009, so none were added; the replacement surface is the
  lazy method chain.
- Replacement surface: the `VecIter<T>` adapter methods
  (`map`/`filter`/`take`/`skip`/`flat_map`/`peekable`/`step_by`/`scan` plus the
  new `chain`, `enumerate`, `zip`, and `all`), `Iterator::collect` / `count`,
  and the `IteratorSum` / `IteratorFold` / `IteratorContains` /
  `IteratorCountEq` capabilities. `unique` / `rev` keep the concrete
  `VecIter<i32>` methods. The qualified old calls fail with an E0100 hint from
  `resolver::removed_names::removed_iter_member_hint` (44 entries plus unit
  assertions).
- Migrated the in-tree call sites: `std/seq/mod.ark` (the 10 deprecated
  helpers keep their signatures; map/filter/fold use the methods while
  take/skip keep an eager bounded copy because the `TakeIter`/`SkipIter`
  collector mis-resolves its item type when instantiated from a std module —
  #691; the same chain compiles from user code), 56 fixtures under
  `stdlib_core/` and `stdlib_trait/`, the two `deprecated_seq_*` diagnostic
  goldens, and the W0009 `deprecated_seq_qualified_message` hint strings that
  offered the removed collectors as alternatives.
- Added the dual-target `stdlib_trait/iterator_method_surface.ark` +
  `_wasm32.ark` pair and the dual-target
  `diagnostics/removed_iter_constructor*.ark` E0100 pair.
- `docs/language/spec.md` no longer describes the removed compatibility
  collectors; `docs/stdlib/migration-guidance.md` gains the full 44-row
  before/after table.
- Scorecard delta: 1104 → **1060** public symbols; manifest public entries 729
  → **685**; `pub fn` declarations 1279 → **1235**; `associated-or-method`
  1021 → **977**; monomorphic `*_i32` manifest helpers 114 → **97**;
  `must delete` 83, `intrinsic-only` 1, `prelude_wrapper` 65, and the 375
  unregistered declarations unchanged. The `std::core::iter frees` rule was
  removed from the eradication TOML (no remaining entry matches it).
- Both-target evidence: the 56 migrated fixtures and the new pair compile and
  validate on `wasm32-gc` and `--target wasm32` with the rebuilt s2;
  `verify fixtures` **1839/1839** compile+validate PASS (105 runtime smoke);
  `selfhost diag-parity` **117/117** PASS including the new E0100 goldens;
  cross-backend parity **2/2** for the new pair; `verify lane` PASS;
  `docs check` 6/6; `check-stdlib-eradication` and `check-compat-orphans`
  clean.
- Residuals: the `.unique().collect()` / `.rev().collect()` runtime lane traps
  in the shared generic `collect_iterator` on both targets (pre-existing;
  `.next()` / `.count()` and the retired frees' replacement rows are equally
  affected, and the fixtures pin the reachable paths); the `TakeIter` /
  `SkipIter` `collect` instantiation from a std module emits an invalid Wasm
  signature, so the deprecated `std::seq::take_i32` / `skip_i32` shims use an
  eager bounded copy; `std::core::iter` methods resolve without the module
  import but miscompile, so migrated call sites retain `use std::core::iter`;
  the remaining 977
  `associated-or-method` symbols (hash adapters, text, bytes, simd, wasm,
  wit/component, host/core, io, test, collections, json/toml/csv) and the
  `filter_i32` deferral are untouched.

### Landed slice (wave/iter-collect-fix, 2026-10-27)

- Fixed the three iterator-collect residuals from the `next1054` lane.
- `UniqueIter` / `RevIter` `.collect()`: the AST edge type memo keyed only on
  `(span_start, span_end)`, but every method-call node in one postfix chain
  shares that span; `a.iter().unique().collect()` cached `VecIter<i32>` for the
  `unique` receiver, kept `VecIter::collect`, and dropped
  `UniqueIter::collect`, so emit used the unresolved `drop; unreachable`
  fallback. The memo key now includes the method name, and a duplicated
  `(span, name)` key is treated as ambiguous instead of returning another
  node's type.
- `TakeIter` / `SkipIter` `.collect()`: a generic method body's `for`
  desugar records `TakeIter::into_iter` at one span per source site; the
  first concrete instantiation (`MapIter<String,String>`) pinned the site and
  every other instance emitted that wrong callee (invalid Wasm signature).
  Both adapters now drain through an explicit `self.next()` loop in their
  collect hook, so `I::Item` resolves per instance without the `for` desugar.
  `std::seq::take_i32` / `skip_i32` are lazy again. The generic-`for` site selection itself remains
  a residual for other adapter bodies (it needs per-enclosing-instance
  call-site records).
- Import-less `std::core::iter` methods: the typechecker silently returned
  `TY_UNKNOWN` when the module was not loaded, and lowering emitted the
  unresolved fallback. An unresolved iterator method on a `Vec` / iterator
  receiver now reports ``method `<name>` requires `use std::core::iter` ``
  (E0200). Auto-loading the module was rejected because iter's
  `Pair<A, B>` / `Range` declarations collide with same-named user types in
  the shared type registry.
- Fixtures: `stdlib_trait/iterator_unique_rev_collect*.ark`,
  `stdlib_trait/iterator_methods_no_import*.ark`,
  `stdlib_seq/seq_take_skip_collect*.ark`, and the updated
  `iterator_method_surface*.ark` pair, verified on `wasm32-gc` (memory32/P2)
  and `--target wasm32`.
- Residuals: negative `n` to the `std::seq` take shim still traps on linear
  `wasm32` (the eager implementation traps the same way, so it is a separate
  pre-existing linear lowering bug), and the generic-`for` site pinning
  remains for adapter bodies other than the two collect hooks fixed here.

### Landed slice (wave/hashset-typesurface, 2026-10-27)

- **Concrete `HashSet<i32>` surface**: `std::collections::hash_set::HashSet`
  now exposes `HashSet::new<i32>()` plus `insert`, `contains`, `remove`,
  `len`, and `is_empty`. The manifest `[[types]]` entry registers the six
  methods; MIR rewrites the shell calls to the flat `hash::hashset_*` store
  (`call_rewrite_hashset`), with a HashSet local marker so `len` / `is_empty`
  are not claimed by the Vec core-op rewrites.
- **Retained (20, deprecated)**: the 15 `hash_set::hashset_i32_*` adapters and
  the 5 `hash_set::hashset_str_*` `Vec<String>` facade adapters stay for the
  aggregate i32 and `String` lanes. Their GC lowering is not fixed yet, so
  no adapter was deleted and no E0100 removal or manifest lifecycle applies.
- **Fixtures**: `stdlib_hashset/hashset_methods.ark` +
  `hashset_methods_wasm32.ark` (compile + run on `wasm32-gc` and
  `--target wasm32`). Aggregate, `union`/`reserve`/`extend`, and
  `HashSet<String>` fixtures are intentionally not registered.
- **Docs**: module page, type registry, migration table, and changelog cover
  the six-method surface; `docs check` is part of the wave gate.
- **Verification**: `verify fixtures` **1845/1845 PASS** (0 FAIL, 106 runtime
  smoke, 1 `test:` fixture), `docs check` 6/6, `verify lane` PASS, and
  `selfhost fixpoint` reached (s2 == s3 at `2c623cbf…`, pinned base
  `04d4446b…`). The new pair compiles, validates, and runs on `wasm32-gc`
  (P2 component) and `--target wasm32` (linear).
- **Residual**: aggregate methods (`to_vec`, `union`, `intersection`,
  `difference`, `capacity`, `reserve`, `shrink_to_fit`, `clear`, `extend`),
  `HashSet<String>` methods, and the ADR-086 trait impls wait on the GC
  method lowering. The `hashset_i32_*` / `hashset_str_*` rows above stay in
  the `std::collections frees` family.

### Landed slice (wave/hashset-from-iter, 2026-10-27)

- **FromIterator (i32 + String)**: `impl FromIterator<VecIter<i32>> for
  HashSet<i32>` and `impl FromIterator<VecIter<String>> for
  HashSet<String>` are back in `std/collections/hash_set.ark`. The source
  spelling is `HashSet::from_iter(values.iter())`; the impl body drains the
  cursor with `next` because `collect` forwards to a generic helper whose
  mono instance is not specialized inside a concrete loaded-module body
  (the emitted stub trapped at runtime on both targets).
- **Resolver fallback**: concrete conversion impls register only under
  `HashSet_i32::from_iter__VecIter_i32` /
  `HashSet_String::from_iter__VecIter_i32`, which is not source syntax.
  `register_headers` and `register_imported_impl_exports` now export a
  `HashSet::from_iter` member on the declared base type for a concrete
  `FromIterator` impl, so `hash_set::HashSet::from_iter` resolves instead of
  failing E0100. The typechecker
  (`hashset_from_iter_candidate`) and MIR
  (`mir_try_resolve_associated_function`) select the concrete impl from the
  cursor's mangled element type (`VecIter_i32` / `VecIter_String`).
- **HashSet<String> traits**: `to_vec` plus `IntoIterator`, `Clone`, `Eq`,
  and `Debug`. The snapshot is the non-public
  `hash_string::hashset_str_to_vec` lowering target (clones the
  `Vec<String>` store; not a new public free function). `for word in set`
  now typechecks and lowers through `HashSet_String::into_iter`.
- **Result typing**: `return_index_type_name_for_impl` maps concrete
  `HashSet<i32>` / `HashSet<String>` returns to `vec:i32` / `vec:String`,
  and `mark_hashset_result_type` / `call_type_fallback` keep call
  destinations on the flat store alias. Without this the call result was
  typed as a String-like ref and failed Wasm validation.
- **Fixtures**: new `stdlib_hashset/hashset_from_iter.ark` +
  `hashset_from_iter_wasm32.ark` (`run:` registry 1877 -> 1879) cover i32
  and String `from_iter`, deduplication, a direct `for` over a
  `FromIterator` call, String `to_vec` / `Clone` / `Eq` / `Debug`, and
  String iteration.
- **Verification**: both targets compile with `wasm-tools validate` and run
  (P2 component on `wasm32-gc`, linear on `--target wasm32`); `verify
  fixtures` **1879 PASS / 0 FAIL** (1879 compile+validate, 110 runtime
  smoke, 1 `test:` fixture PASS); `selfhost diag-parity` PASS;
  `docs regenerate` / `docs check` 6/6;
  `verify lane` PASS (quality changed + 13-file lint ratchet);
  `check-stdlib-eradication` 362/362 + 130/130 classified, public surface
  491; `check-compat-orphans` and `check-comment-policy` clean; ADR-029
  fixpoint s2 == s3 at sha256
  `c0a9f92f0061e84b382525474c54e638eb536c1d24f08229bce9ae52c8c04e2a`
  (pinned base `bootstrap/arukellt-selfhost.wasm`, sha256
  `04d4446bbad63f33e7543c5b94d1126932f2852262a77bcbf9d48cecba9d1f14`).
- **Scorecard**: unchanged — no manifest function entry and no top-level
  `pub fn` was added (the snapshot accessor is a private lowering target).
- **Residual**: `HashSet<String>` set algebra (`union` / `intersection` /
  `difference`) remains i32-only. Standalone `arukellt test
  std/collections/hash_set.ark` still reports `E0100 undefined struct:
  VecIter`; the pre-change base reproduces it, so it is a test-mode module
  loading gap rather than this wave, and the dual-target fixture is the
  executable regression.

### Landed slice (wave/bytes-deletion, 2026-10-27)

- **ADR-087 Wave 2 physical deletion**: the 45 `stable` `std::bytes` free
  functions were deleted after their 2026-10-27 Wave 1 conversion
  (`deprecated` + W0009, `deprecated_since = "0.1.0"`,
  `remove_in = "1.0.0"`). Both the `std/manifest.toml` entries and the
  `pub fn` declarations in `std/bytes/mod.ark` are gone; the module keeps the
  `Bytes` namespace and the `Vec<i32>` byte methods.
- **E0100 conversion**: the 45 W0009 rows were replaced by
  `resolver::removed_names::removed_bytes_member_hint` entries with the same
  replacement text. The eight unregistered Wave-1 removals (`hex_char`,
  `hex_val_char`, `try_hex_val_char`, and the five `try_read_*` readers) keep
  their hints in the same table — the earlier `merge 917 wit ast methods`
  conflict resolution had dropped the whole table, so this wave restores it
  together with the 45 additions. `deprecated_bytes_qualified_message` and
  the `check-compat-orphans` `bytes` alias mapping are removed.
- **Call sites**: 150 fixture files (869 `bytes::*` calls) moved to the
  method surface (`b.push_byte`, `b.len`, `b.get_unchecked`, `a.equals_bytes`,
  `a.concat_bytes`, `b.slice_bytes`, `b.to_hex`, `b.to_base64`, the endian
  `to_*` / `push_*` pairs, `data.byte_cursor()` + `c.read_*`) and the `Bytes`
  namespace (`Bytes::from_hex` / `from_base64` / `from_array` /
  `from_u16_le` / `from_u32_le` / `from_u32_be` / `from_leb128_*`).
  `modules/use_qualified_call` now exercises `bytes::Bytes::from_hex`; the
  cookbook `wasm-binary` blocks and the language spec bytes section were
  updated.
- **Fixtures**: `diagnostics/deprecated_bytes_free*.ark` became
  `diagnostics/removed_bytes_free*.ark` (E0100 goldens on wasm32-gc and
  `--target wasm32`).
- **Scorecard delta**: 858 → **813** public symbols; `must delete` 115 →
  **70**; manifest functions 626 → **581** (public 625 → **580**); `pub fn`
  declarations 981 → **936**; `associated-or-method` 743 and
  `intrinsic-only` 1 unchanged. The `deprecated std::bytes frees` rule is
  removed from `docs/data/stdlib-api-eradication.toml` (no live match).
- **Verification**: `verify fixtures` **1845/1845 PASS** (0 FAIL);
  `selfhost diag-parity` **124 PASS / 0 FAIL** (the two
  `removed_hashmap_adapter*` goldens now pass with the `146f35153` hint);
  every changed fixture compiles and validates on `wasm32-gc` (P2) and
  `--target wasm32` (P1); `docs check` 6/6; `check-stdlib-eradication` and
  `check-compat-orphans` clean; ADR-029 fixpoint s2 == s3.
- **Residual**: `Bytes::from_base64` keeps the skipping decoder semantics;
  `try_read_u64_le` stays deferred (`Some(i64)` still emits invalid Wasm);
  the `std::io` / `std::host::fs` byte-layout adapters and the 743
  `associated-or-method` symbols remain in their own lanes.

### Landed slice (wave/host-core-json, 2026-10-27)

- Retired the 18 `std::json` / `std::toml` manifest free entries and 9
  unregistered source declarations for the **value-method** subset:
  `JsonValue` `is_null` / `is_bool` / `is_number` / `is_string` / `is_array` /
  `is_object`, `as_bool` / `as_string` / `as_i32` / `as_f64` / `as_array`,
  `get_field` / `get_index`, `to_text`, and `JsonParseError::message`;
  `TomlValue` `as_string` / `as_int` / `as_bool` / `as_float` / `as_array` /
  `as_table`, `get_field`, and `table_keys`. All removed manifest entries were
  `experimental` (ADR-014 D4), so no ADR-087 W0009 row applies; E0100 hints
  come from `resolver::removed_names::removed_{json,toml}_member_hint` with
  unit assertions in the same file.
- **Lowering block (measured, not faked)**: an associated function or namespace
  type (`JsonText`, `TomlText`, `Csv`, `JsonValue::*` constructors,
  `JsonField::new`, `JsonValue::pretty_text`) that returns a constructed
  value emits an unresolved-call `unreachable` when called from a **non-main
  module**. A two-module probe (`helper.ark` → `main.ark`) compiling with both
  the pinned bootstrap and the rebuilt s2 produced "compiler produced invalid
  core Wasm"; the same call from `main.ark` validates. The compiler source
  therefore stays on the free helpers (`JsonText` / `TomlText` / CSV frees were
  not adopted), and those free functions remain the public surface for
  multi-module callers. Associated `JsonValue::parse` / `TomlValue::parse`
  also fail unannotated `match` inference, and `TomlValue::to_text` emits an
  invalid `i64` return on wasm32-gc.
- **Fixtures**: the 258 json/toml/csv fixtures now use the value methods for
  the safe subset; the primitive helpers, DOM constructors, streaming scanner,
  and CSV calls stay free. Nested `Option` matches whose scrutinee is a
  method call need an explicit `Option<...>` binding (`toml_aot_*`,
  `toml_array_*`).
- **Scorecard delta**: 728 -> **701** public symbols; `associated-or-method`
  658 -> **631**; unregistered declarations 222 -> **213**; manifest
  functions 507 -> **489**. `must delete` 70 and `intrinsic-only` 1 unchanged.
- Verification: both-target fixture compile for the 258 json/toml/csv
  fixtures; full `verify fixtures`, `docs check`, `verify lane`, and the
  ADR-029 fixpoint are the wave gates.

### Landed slice (wave/non-main-module, 2026-10-27)

- **Compiler bug (measured)**: a `module::Type::method` associated call in a
  **non-main** module kept the module prefix because the resolver only
  canonicalizes the entry module. The signature spine registers the impl
  method as `Type::method`, so `call_func_id` left `fid=-1`. The fused
  in-loop emitter's fid<0 oracle (`emit_inloop_disc_hit`) stripped only the
  last `::` segment (`quote`) instead of the registered `Type::method`, so it
  treated the call as an intrinsic and emitted `drop; unreachable` instead of
  deferring; the reachability worklist also resolved only the last segment and
  could drop the impl body. The impl and its mono instance were present in the
  fn table/registry — this was a spelling/identity bug, not a missing
  registration.
- **WAT evidence**: before the fix, `helper.ark`'s `JsonText::quote` call
  emitted `local.get $s; drop; unreachable; local.get $_t1; return` while
  `$JsonText::quote` existed later in the module; the same call in `main.ark`
  emitted `call $JsonText::quote` and validated. After the fix the helper call
  emits `call $JsonText::quote` and the module validates.
- **Fix**: `corehir::callee_spelling::callee_spelling_variants` produces the
  canonical `Type::method` / after-first-segment spellings for 3+ segment
  names. It is applied in `call_rewrite` (dest typing/callee identity),
  `call_func_id` (fid + canonical call name), the typechecker sig lookup,
  edge discovery (impl/mono keep), `emit_inloop_disc_hit` (defer instead of
  intrinsic fallback), and the drain-time `resolve_fn_index` fallback.
- **Unblocked conversions (32 manifest + 25 unregistered declarations)**:
  `JsonText` (`quote`, `escape`, `skip_ws`, `parse_int_at`, `content_length`,
  `find_key_pos`, `hex_byte`, `hex_value`, `parse_string_at`,
  `decode_escape_at`, `decode_unicode_ascii`, `get_str`, `get_int`),
  `JsonValue` constructors (`null`, `bool`, `int`, `string`, `raw`, `array`,
  `object`), `JsonField::new`, `JsonValue::pretty_text`, the `JsonValue`
  primitive helpers (`stringify_*`, `parse_*`, `encode_string`, `null_text`),
  the `JsonStrResult` / `JsonEscapeResult` accessors, `TomlText`
  (`parse_line`, `find_section`, `find_value`, `find_raw_value`,
  `parse_string_array`), and `Csv` (`split_line`, `parse_row`,
  `stringify_row`, `count_rows`, `get_row_raw`, `parse_row_at`, `parse`,
  `stringify`, `parse_with_header`). The compiler call sites, including
  `loader/wit_manifest.ark::collect_wit_paths_from_manifest`, now use the
  associated surface.
- **Fixtures**: `stdlib_json/json_associated_surface*.ark`,
  `stdlib_toml/toml_text_surface*.ark`,
  `stdlib_csv/csv_namespace_surface*.ark`, and the two-module regression
  `modules/non_main_associated_call/` (helper.ark -> main.ark). Each pair
  compiles, validates, and runs on `wasm32-gc` (P2 component) and `--target
  wasm32` (linear). E0100 goldens:
  `diagnostics/removed_json_toml_csv_free*.ark`.
- **Scorecard delta**: 588 -> **531** public symbols; `associated-or-method`
  518 -> **461**; manifest functions 417 -> **385**; unregistered declarations
  172 -> **147**. `must delete` 70 and `intrinsic-only` 1 unchanged.
- **Verification**: the two-module regression compiles, validates, and runs
  on `wasm32-gc` (P2 component) and `--target wasm32` (linear) with the
  rebuilt s2; the same probe still fails under the committed pre-fix pin
  (expected). `verify fixtures` **1867/1867 PASS**, 0 FAIL (1867
  compile+validate, 108 runtime smoke, 1 `test:` fixture PASS);
  cross-backend parity **4/4** for the new surface + module fixtures;
  `selfhost diag-parity` 138 PASS / 2 pre-existing `removed_wasm_free*`
  golden failures (unrelated E0100 table residual); `docs check` 6/6;
  `verify lane` PASS; ADR-029 fixpoint PASS: s2 == s3 at sha256
  `28746bb30071549d1cc881407c8319ba0547452d860e8140c31321831267da3b`
  (pinned base `04d4446b…`).
- **Bootstrap adapter**: the committed pin cannot lower the compiler's own
  `collect_wit_paths_from_manifest` associated call. The generated flat
  overlay therefore routes the compiler's TOML manifest scans through free
  `bootstrap_toml_*` shims
  (`scripts/selfhost/checks.py::_patch_bootstrap_manifest_scanner_paths`);
  checked-in `std`/`src/compiler` source keeps the associated surface. Delete
  the adapter when the pin is refreshed (ADR-029).
- **Residuals**: `json::parse` / `toml::toml_parse` / `toml::toml_stringify`
  remain the document entry points (associated `parse` loses the Result
  payload type in an unannotated `match`; `TomlValue::to_text` still emits an
  i64 return on wasm32-gc). `Vec<String>::concat` remains invalid on
  wasm32-gc independently of this wave. The committed pinned bootstrap is the
  pre-fix artifact, so the two-module probe fails under it; the rebuilt s2
  (and its self-built s3 successor) pass.

### Landed slice (wave/host-core2, 2026-10-27)

- **ADR-087 Wave 2 conversion (8 manifest + 1 unregistered)**: the `std::core`
  frees `range_new` / `range_contains` / `range_len` /
  `range_inclusive_new` / `range_inclusive_contains` / `cmp_i32`,
  `std::core::error::error_message`, `std::core::hash::combine`, and the
  unregistered `std::core::identity`. Wave 1 (`3e1b307d8`) landed the
  receiver surface and `deprecated` manifest entries with live W0009
  qualified-call warnings and dual-target goldens; Wave 2 deletes the
  entries/declarations and leaves E0100 hints.
- **Replacement surface**: `Range::new` / `r.contains(value)` / `r.len()`,
  `RangeInclusive::new` / `r.contains(value)`, `a.cmp(b)` through the `Ord`
  trait, `Error::message`, and `i32::combine_hash`; `identity` has no
  replacement. `Range::new` is bound to the `core.range_new` CoreOp (new
  legacy alias + regenerated binding table), so the GC struct constructor and
  the linear-memory 8-byte record path keep their existing lowering.
- **Compiler fix (measured)**: an enum-variant struct literal
  (`Error::IndexOutOfBounds { .. }`) keeps the variant payload layout as the
  binding's local type, so `e.message()` resolved
  `IndexOutOfBounds::message`; the ambiguous bare-name fallback (`message` is
  also `JsonParseError::message`) emitted `drop; unreachable` and failed Wasm
  validation. Method-call resolution now maps a registered payload-variant
  receiver to the owning enum (`ctx_variant_owner_name` in
  `mir_initial_method_callee`) when that enum declares the method. Stamping
  the local as `enum:<Enum>` instead resolved the method but trapped on the
  linear target for struct-payload variants, so the resolution stays at the
  callee spelling and keeps the payload layout for emit. A value returned
  from a function or bound through an explicit annotation already lowered
  correctly.
- **Fixtures**: dual-target `stdlib_core/core_method_surface*` pair (compile,
  validate, run), `diagnostics/removed_core_free*` E0100 goldens, and the
  migrated `stdlib_core`, `stdlib_trait`, `stdlib_inline`,
  `stdlib_component`, and `generics_v1` call sites. The #914 shadow fixture
  is retargeted from the removed `std::core::identity` to the live
  `std::collections::string::to_lower`. Run count 1867 -> 1869.
- **Scorecard**: 515 -> **506** public symbols; `associated-or-method`
  445 -> **436**; manifest public entries 369 -> **361**; unregistered
  declarations 146 -> **145**; `pub fn` declarations 569 -> **560**;
  monomorphic `*_i32` helpers 57 -> **56** (`cmp_i32`); `must delete` 70,
  `intrinsic-only` 1, and `prelude_wrapper` 65 unchanged. The
  `host/core namespace frees` family drops from 63 manifest / 21 source to
  **55 manifest / 20 source**.
- **Verification**: `verify fixtures` **1869/1869 PASS**, 0 FAIL (1869
  compile+validate, 108 runtime smoke, 2 runtime-unsupported, 1 `test:`
  fixture PASS) with the rebuilt s2; `selfhost diag-parity` PASS with the
  new E0100 `removed_core_free*` goldens (the Wave 1 W0009 goldens fired on
  both `wasm32-gc` and `--target wasm32` before the rename); `docs check`
  6/6; `check-stdlib-eradication`, `check-compat-orphans`,
  `check-ark-code-quality`, and `check-comment-policy` clean; `verify lane`
  PASS. `stdlib_core/core_method_surface*` compiles, validates, and runs on
  `wasm32-gc` (P2 component) and `--target wasm32` (linear), including the
  `Error::IndexOutOfBounds { .. }` / `Error::ParseError { .. }` payload
  variants; `check-cross-backend-parity.py` reports 4/4 parity for the
  surface, `error_basic`, `range`, and `iterator_range_basic`. ADR-029
  fixpoint PASS: s2 == s3 at sha256
  `9779dc820e01a54cd09a8374b94c5e5941baa378602e0803cf3e89a5bbb57d4b`
  (pinned base `04d4446b…`).
- **Residuals**: the 20 `std::core::raw` representation bridges still need a
  `RawArray` / `RawString` namespace surface; the other host families
  (`std::host::{env,fs,clock,random,process}`, `std::fs`, `std::env`,
  `std::cli`, `std::process`, `std::signal`) are untouched.

### Landed slice (wave/core-raw, 2026-10-27)

- **Surface**: the 20 `std::core::raw` bridges are now associated functions on
  `RawArray` (`new`, `len`, `get_unchecked`, `set_unchecked`, `grow`,
  `set_len`), `RawString` (`new`, `from_bytes`, `len_bytes`,
  `byte_at_unchecked`, `clone`, `push_char`, `parse_i32`, `parse_i64`,
  `parse_f64`), and `RawFloat` (`sqrt`, `bits_lo`, `bits_hi`, `to_string`,
  `promote_f32`). The canonical `raw_*` spellings stay as private
  CoreOp-bound helpers, so `data/core-ops.toml` keeps driving GC/LM lowering
  and no CoreOp operation table changed.
- **E0100**: `resolver::removed_names::removed_raw_member_hint` covers the 20
  bare names and `raw::` members with public Vec / String / scalar
  replacement hints; the dual-target goldens are
  `diagnostics/removed_raw_free*`.
- **Compiler fixes (measured)**: the text parsers gained exact
  `result:i32:String` / `result:i64:String` / `result:f64:String` rules (the
  old leaf-name match also claimed `JsonValue::parse_i32`, an i32 reader), and
  `parse_i32` / `parse_i64` / `parse_f64` were removed from the
  Option-returning builtin list (they declare `Result`). `RawArray` namespace
  calls rewrite to the canonical `raw_array_*` CoreOp spellings before mono
  resolution, so `Vec::new<String>` no longer instantiates the i32 wrapper
  (the five fixture regressions this caught are fixed). The private
  `raw_string_new` shell uses `String_new()` because an emitted
  `__intrinsic_string_new` body defaults its temp to i64;
  `raw_f32_promote_f64` calls `f32_to_f64` so the emitted f64 return types.
- **Fixtures**: dual-target `stdlib_core/raw_consumer_surface*` (compile,
  validate, run; `raw consumers ok`) and the E0100 pair. The sealed module's
  `test mod` passes all three suites on `--target wasm32` and compiles and
  validates on `wasm32-gc`. The gc test wrapper does not package the P2 core
  module as a component, so direct test execution is a harness residual; the
  production consumer fixtures pass on both targets. This is reported as a
  test-harness residual, not a skip.
- **Scorecard**: 491 -> **471** public symbols; `associated-or-method`
  421 -> **401**; `pub fn` 539 -> **519**; unregistered 130 -> **110**;
  manifest public 361 unchanged. The `host/core namespace frees` family drops
  from 55/20 to **55/0**.
- **Verification**: the rebuilt s2 compiles/runs the consumer pair on
  wasm32-gc (P2) and `--target wasm32` (linear); the linear raw test module
  PASSes 3/3 suites and the gc test module compiles/validates; `verify
  fixtures` **1879/1879 PASS**, 0 FAIL (base gate 1875 + 4 new fixtures);
  `selfhost diag-parity` PASS; `docs check` 6/6; `verify lane` PASS;
  cross-backend parity 2/2 for the new pair; ADR-029 fixpoint PASS: s2 == s3
  at sha256 `5b0455216247ab3bf2563a67b0c7f6aa23bb77d468d4c5f8449255ab203646af`
  (pinned base `04d4446b…`).

## Required work

- [x] Phase 0 — measure top-level `pub fn` under `std/**/*.ark`, make the gate
      dual-surface (manifest + source declarations), and generate the real
      1258-symbol scorecard.
- [x] Wave A — `must delete` entries whose replacement already exists
      (deprecated prelude wrappers, vec intrinsic-compat frees,
      monomorphic numeric conversions, `HashMap_*` builtins once
      `impl HashMap<K,V>` methods cover them). Apply ADR-014
      deprecation where the symbol is still reachable.
      - Landed 2026-10-26: the 2 experimental `std::wasm` helpers and the
        4 deprecated gated prelude wrappers.
      - Landed 2026-10-26 (`wave/917-stable64`): 60 of the stable 64 were
        converted to `deprecated` with W0009 + migration + changelog
        (ADR-087); 3 stale non-callable entries retired. The physical
        deletion of the deprecated set is Wave 2. Residual: `i8_to_i32`
        deferral.
      - Landed 2026-10-27 (`wave/917-hashmap`): the 20 `HashMap_*` builtins
        and the i32→i32 adapters were retired; the compiler registrations
        and fixtures migrated to `HashMap<K, V>` methods.
      - Landed 2026-10-27 (`wave/917-seq33`): the 31 stable seq/combinator
        prelude frees were deleted after the Wave 1 W0009 window; E0100
        hints replace the W0009 rows and `filter_i32` remains provisional.
      - Landed 2026-10-27 (`wave/bytes-deletion`): the 45 stable `std::bytes`
        frees were deleted after their Wave 1 W0009 window; the E0100 member
        hints carry the method / `Bytes` replacements and the unregistered
        helper hints are restored.
- [ ] Wave B — `associated-or-method` entries: land the inherent /
      trait / associated replacement first (per-owner issues #693,
      #702, #703, #708, #710, and the future Test / IO / collection
      types), then retire the free symbol.
      - Landed 2026-10-26: 6 `std::collections::sort` frees retired after
        the #697 `Vec<T>` methods; the `std::core` value-helper family
        (18 counted) retired after the `Either` / `Integer` / `TryFrom`
        replacements.
      - Landed 2026-10-26 (`wave/917-stable64`): 31 seq/combinator frees
        deprecated (Wave 1) with W0009 + iterator/Vec method replacements;
        deletion is Wave 2.
      - Landed 2026-10-27 (`wave/917-nextgroup`): the 33 non-AST
        `std::wit` symbols retired after `WitType` methods, the `WitText` /
        `WitName` namespace types, and `World::to_wit` landed. `std::wit::ast`
        and the parser frees stay deferred with the recursive `WitNode`
        receiver shape.
      - Landed 2026-10-27 (`wave/917-iter44`): the 44 eager
        `std::core::iter` constructor / collector frees retired after the
        `VecIter` adapter method surface (`chain`/`enumerate`/`zip`/`all`
        landed with the wave) replaced them.
      - Landed 2026-10-27 (`wave/host-core2`): the 8 `std::core`
        range/order/error/hash manifest frees and the unregistered
        `std::core::identity` retired after the `Range` / `RangeInclusive` /
        `i32` / `Error` receiver surface landed.
- [x] Keep the scorecard at zero-unclassified while waves land: update
      the TOML rules as symbols retire so the checker stays green.
- [x] Update user-facing docs/examples in the same PR that retires a
      symbol family (docs-sync gate).

### Landed slice (wave/simd-lowering, 2026-10-27)

- **Emitter root causes fixed** (pre-existing, exposed by the lane wave):
  wrong draft 0xfd sub-opcodes for `i8x16`/`i16x8`/`i32x4`/`i64x2`/
  `f32x4`/`f64x2` arithmetic and `neg` (e.g. `i8x16.add` emitted
  `0x7e`, `f32x4.neg` aliased `abs`, `f64x2.add` aliased `min`); missing
  core-op registry entries, aliases, and dispatch branches for
  `i8x16::{le_s,le_u,ge_s,ge_u}`, the `i16x8` comparison set,
  `i16x8::extend_low_i8x16_u`, `i32x4::max_u`, `i64x2::gt_s`, and
  `f64x2::{min,max}`; no lowering at all for the four unsigned lane modules
  (their intrinsic names were unknown, so the ABI degraded to i64 and failed
  validation); `i8x16.mul` (no Wasm instruction) and the unsigned `i64x2`
  compares (no Wasm instruction) had no emulation; the MIR argument heuristic
  promoted the `i64x2` shift amount from `i32` to `i64` because the intrinsic
  name contains `i64`; and `__simd_v128_const` emitted the all-zero vector
  regardless of its byte arguments, which made `v128::new` return zero.
- **Fix shape**: corrected the opcode table; registered the missing operations
  and aliases (22 new `core_ops`, 92 aliases) and added the dispatch branches;
  mapped the 74 unsigned lane intrinsics onto the signed machine ops where the
  bits agree (adding the unsigned compare/min/max forms); emulated
  `i8x16.mul` with `i16x8` products + low-byte mask + unsigned narrowing and
  the unsigned `i64x2` compares with a sign-bit flip over the signed compare;
  added the second v128 scratch slot for both GC and linear frames; kept the
  i64x2 shift amount at Wasm `i32`; rebuilt `v128::new` from `i8x16` lanes.
- **Migration**: all 130 remaining free functions moved onto their lane
  namespace types and the 37 `std::simd` manifest `[[functions]]` entries were
  deleted (all `experimental`, ADR-014 D4). `U8x16`, `U16x8`, `U32x4`, and
  `U64x2` are new `[[types]]` entries; the existing type method lists gained
  the newly unblocked methods. The `std::simd frees` rule was removed from
  `docs/data/stdlib-api-eradication.toml`.
- **Fixtures**: `simd_conformance/{u8x16,u16x8,u32x4,u64x2,signed,float,v128_new}`
  lane fixtures plus `_wasm32` twins (`--target wasm32 --wasi-version wasi-p1
  --memory-width memory32`). They assert values, not just validity, and cover
  the unsigned-only compare/min/max lanes, saturating arithmetic, `i8x16.mul`,
  `v128::new`, `i8x16`/`i16x8` le/ge, `i16x8` extend_u, `i32x4::max_u`,
  `i64x2::gt_s`, and the `f32x4`/`f64x2` fixes.
- **Scorecard delta**: 491 → **378** public symbols; manifest functions 362 →
  **325** (public 361 → **324**); `associated-or-method` 421 → **308**;
  unregistered declarations 130 → **54**; `must delete` 70 and
  `intrinsic-only` 1 unchanged. The fixture registry grows 3331 → **3345**.
- **Verification**: `verify fixtures` **1891/1891 PASS / 0 FAIL** (110 runtime
  smoke, 1 `test:` fixture PASS); the 18 `simd_conformance` fixtures execute on
  both targets; `wasm-tools validate` passes the full-surface probe in the
  memory64 GC core, the memory32 GC component, and the linear wasm32 forms;
  `docs check` 6/6; `check-stdlib-eradication`,
  `check-core-op-compiler-validator`, `check-native-cpp-capabilities`,
  `check-compat-orphans`, and `check-semantic-debt` clean; `verify lane` PASS;
  ADR-029 fixpoint s2 == s3 at `093412d160d0a040` (pinned base
  `04d4446b`).
- **Residuals**: the `std::wasm` v128 memory frees and their `V128Memory`
  associated form still lack `v128.load`/`store` lowering; the scoreboard
  generator still counts manifest functions only, so the simd families (now
  type-associated, no function entries) no longer appear as rows while
  `reference.md` lists the types and methods; and the 308 remaining
  `associated-or-method` symbols stay in their own lanes.

## Acceptance

- [ ] Generated scorecard shows 0 public free functions, 0
      `prelude_wrapper` entries, 0 monomorphic helpers.
- [ ] `python3 scripts/check/check-stdlib-eradication.py` passes.
- [ ] `std/manifest.toml` contains no `kind = "prelude_wrapper"`.
- [ ] Each retired family points at its method/associated replacement in
      release notes or migration docs.

## References

- ADR-046 (free-function eradication), ADR-014 (stability), ADR-044
- #709 (policy + inventory), #718 (Tier-1 wave, done), #703, #693,
  #702, #708, #719
- `docs/data/stdlib-api-eradication.toml`,
  `scripts/check/check-stdlib-eradication.py`,
  `std/manifest.toml`

### Landed slice (wave/host-fs, 2026-10-27)

- **Surface**: `std::host::fs::Fs` is the namespace for the one-shot path
  operations: `Fs::read_to_string`, `Fs::read_bytes`, `Fs::write_string`,
  `Fs::write_bytes`, `Fs::is_readable_file`, `Fs::is_file`, `Fs::is_dir`, and
  the honest unsupported `Fs::read_dir` / `Fs::metadata` rejections. `File`
  keeps the image handle and the `Read` / `Write` / `Seek` impls. `std::fs`
  is now a deprecated facade that re-exports `Fs` and keeps four stable
  wrappers.
- **Removed (11)**: the provisional / experimental / deprecated frees
  `std::host::fs::{read_to_string, read_bytes, write_string, write_bytes,
  is_file, is_dir, exists}` and `std::fs::{is_file, is_dir, read_dir,
  metadata}`. Qualified old calls fail with **E0100** and the
  `resolver::removed_names` hint; `read_dir` / `metadata` state the
  unsupported host capability instead of promising a replacement.
- **Deprecated with live W0009 (5)**: `std::host::fs::is_readable_file`,
  `std::fs::read_string`, `std::fs::write_string`, `std::fs::exists`, and
  `std::fs::is_readable_file`. They stay callable for the ADR-087 window;
  physical deletion is the Wave 2 change.
- **Compiler**: added `Fs` result-type fallbacks for `read_bytes`,
  `write_string`, and `write_bytes`; the `Fs::` spellings lower on both
  targets. The committed pin predates loading the namespace source into the
  flat overlay and resolves the `std::host::fs` frees as builtins, so
  `scripts/selfhost/checks.py::_patch_bootstrap_host_fs_paths` rewrites
  `Fs::*` call sites to the free builtins **only in** the generated compiler
  overlay. Checked-in compiler source keeps `Fs`; delete the adapter when the
  pin is refreshed (ADR-029). The pin cannot resolve `std::host::fs::Fs` in a
  compiler module directly: the first attempt emitted `drop; unreachable` and
  trapped in `find_package_root`.
- **Stale manifest entry**: `std::host::stdio::read_to_string` had no source
  declaration (the implementation is `read_stdin`); the entry now matches the
  declaration. This removes one unregistered declaration from the scorecard
  without changing the callable surface.
- **Fixtures**: dual-target `stdlib_fs/fs_namespace_surface*` (compile,
  validate, run), `diagnostics/removed_host_fs_free*` E0100 goldens, and
  `diagnostics/deprecated_host_fs_free*` W0009 goldens on wasm32-gc and
  `--target wasm32`. Migrated the stdlib_fs / stdlib_io / stdlib_io_traits /
  native-cpp / host / quickstart / benchmark call sites to `Fs`.
- **Scorecard**: public surface 358 -> **346**; the
  `host/core namespace frees` family drops 55 -> **40** (10 retired + 5 stable
  entries now classified under `deprecated module frees`);
  `deprecated module frees` 2 -> **7**; `associated-or-method` 288 -> **272**;
  `must delete` 70 -> **74**; manifest functions 325 -> **314**; unregistered
  declarations 34 -> **33** (the stale stdio entry). `prelude_wrapper` stays
  65.
- **Residual / lowering note**: an inline `match Fs::read_bytes(...)` payload
  combined with the `File` image lane still types an emit temp as i64; the
  fixture uses an explicit `Result<Vec<i32>, String>` scrutinee binding. The
  experimental `fd_seek` / `fd_tell` / `fd_fdstat_errno` frees stay free:
  `t3 = false` (wasm32 / WASI P1 only), so they have no gc parity fixture.
- **Verification**: `verify fixtures` **1897 PASS / 0 FAIL**
  (1897 compile+validate, 110 runtime smoke, 2 runtime-unsupported, 1 `test:`
  fixture PASS); `selfhost diag-parity` PASS including the new W0009 / E0100
  goldens; `docs check` 6/6; `verify lane` PASS; both-target
  `stdlib_fs/fs_namespace_surface*` compile, `wasm-tools validate`, and run
  (`fs-namespace-ok`); ADR-029 fixpoint PASS: s2 == s3 at sha256
  `58861e17cbe0d4d9e05422ddd9c4f961a4911ae2b0e0179e7b00b8f1a1b0aa85`
  (pinned base `04d4446b…`).
