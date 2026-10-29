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

### Landed slice (wave/json-csv, 2026-10-27)

- **Surface**: the final `std::json` / `std::toml` / `std::csv` document
  entry points are associated functions / methods:
  `JsonValue::parse(text)` / `v.to_text()`, `TomlValue::parse(text)` /
  `v.to_text()`, `JsonParseStep::ok` / `err` with `step.value()` /
  `step.next()`, and `TomlValStep` / `TomlKeyStep` `ok` / `err`.
  `json::parse`, `toml::toml_parse`, `toml::toml_stringify`, and the eight
  unregistered parse-step helpers are deleted. All were `experimental`
  (ADR-014 D4) or outside the manifest, so no W0009 window applies; qualified
  old calls get E0100 hints from `removed_json_member_hint` /
  `removed_toml_member_hint`. `std::csv` already had no public free helper.
- **Payload inference (retested)**: under the rebuilt s2 (which contains
  `3472c53af` plus the payload/condition inference fixes), an unannotated
  `match json::JsonValue::parse(doc)` / `match toml::TomlValue::parse(doc)`
  infers `Result::Ok` / `Result::Err` correctly and validates on
  `wasm32-gc` and `--target wasm32`. The committed pinned bootstrap predates
  those fixes and cannot compile the associated form; the fixture gate uses
  the current s2 and records the pin only as a non-fatal parity note.
- **i64 payload (fixed)**: direct `TomlValue::Integer(42)` construction
  stored an unannotated `i32` literal into the declared `i64` variant field
  (`struct.set` type mismatch in WAT; `expected i64, found i32` on
  wasm32-gc). `emit_gc_variant_payload_fields` now resolves the variant
  layout and widens an `i32` payload to `i64` before `gc_struct_set`. Parsed
  TOML integers were already i64 and remain valid.
- **Same-name helper collision (fixed)**: a non-main module using both
  `JsonValue::to_text` and `TomlValue::to_text` bound the bare private
  `stringify_value` to the JSON implementation inside the TOML method (blank
  output on linear, `ref.cast` trap on wasm32-gc). The private parser helpers
  are now `json_*` / `toml_*` prefixed
  (`json_parse_document` / `json_stringify_value` / `json_stringify_array`
  and TOML counterparts); no duplicate top-level helper name remains.
- **Fixtures**: `stdlib_json/json_associated_surface*.ark` and
  `stdlib_toml/toml_text_surface*.ark` now cover the unannotated parse match,
  the error path, and integer `to_text`;
  `modules/non_main_associated_call/` calls both parsers from a helper
  module. The E0100 goldens are
  `diagnostics/removed_json_toml_csv_free*.ark`. The 244 migrated
  fixture/benchmark call sites compile and validate; targeted parity for the
  three stdlib families is 262 compile+validate, 0 FAIL.
- **Scorecard**: 358 -> **347** public symbols; `associated-or-method`
  288 -> **277**; manifest functions 325 -> **322**; unregistered declarations
  34 -> **26**; `must delete` 70 and `intrinsic-only` 1 unchanged. Generated
  scoreboard: 324 -> **321** APIs (experimental 54 -> **51**), fixture
  coverage 241/256 -> **238/253**. The module families now have zero
  manifest function entries and `docs/stdlib/eradication.md` has no live
  json/toml/csv rule.
- **Verification**: `verify fixtures` **1895/1895 PASS**, 0 FAIL (110
  runtime smoke, 2 runtime-unsupported, 1 `test:` fixture PASS); both-target
  compile + `wasm-tools validate` for the surface and two-module fixtures
  (`wasm32-gc` P2 component and `--target wasm32` linear); `selfhost
  diag-parity` PASS; `docs check` 6/6; `verify lane` PASS; ADR-029 fixpoint
  PASS: s2 == s3 at sha256
  `cf1bc74e6a5abcc94a5c345fb86291703afd7297f523a68a256f76b14b48ce1e`
  (pinned base `04d4446b…`).
- **Residual**: the pinned bootstrap cannot lower the associated document
  entry points, so the flat-overlay `bootstrap_toml_*` adapter stays until
  the pin is refreshed. `Vec<String>::concat` remains invalid on wasm32-gc
  independently of this wave.

### Landed slice (wave/host-env, 2026-10-27)

- **Surface**: the two `std::host::clock` reads now have the `Clock` namespace
  type: `Clock::monotonic_now()` (monotonic nanoseconds) and
  `Clock::now_ms()` (Unix-epoch milliseconds) from
  `use std::host::clock::Clock`. The WASI clock intrinsics stay behind the
  impl; no user-reachable free function was added.
- **ADR-087 status**: both free entries are `deprecated` with
  `deprecated_by = "Clock::monotonic_now"` / `"Clock::now_ms"`,
  `deprecated_since = "0.1.0"`, and `remove_in = "1.0.0"`. They remain
  callable through the window; W0009 covers the `clock::`, `host::clock::`,
  and `std::host::clock::` spellings with unit assertions in
  `lint::deprecated_table`. No E0100 hint applies yet because the physical
  deletion is Wave 2.
- **Lowering probe**: the namespace shape lowers on both targets for scalar
  clock returns (including a non-main helper-module call) and emits the
  expected `wasi:clocks/monotonic-clock` / `wall-clock` imports rather than
  an unresolved `drop; unreachable`. The same shape is blocked for
  `std::host::env`: a namespace method calling `__intrinsic_args` or
  `__runtime_abi_env_var` produces `expected i64, found ref` in its own frame
  on `wasm32-gc` (the env intrinsics only lower at the registered free-call
  site), so `Env::args()` / `Env::var()` do not execute. `std::host::env`
  stays free and is reported as a residual instead of shipping a namespace
  spelling that cannot run.
- **Compiler**: the driver, wasm, and MIR timing call sites use `Clock::`.
  The committed pin predates loading the namespace source into the flat
  overlay and stubs driver timing to i32, so
  `scripts/selfhost/checks.py::_patch_bootstrap_host_clock_paths` rewrites the
  overlay call spelling and reproduces the stub / keep-clock form
  (`0`, `i32_to_i64(0)`, or i32 milliseconds). Checked-in source keeps the
  namespace; delete the adapter when the pin is refreshed (ADR-029).
- **Fixtures**: dual-target `stdlib_host/clock_namespace_surface*` compile,
  validate, and run the namespace on `wasm32-gc` (P2 component) and
  `--target wasm32` (linear); the W0009 goldens are
  `diagnostics/deprecated_host_clock_free*`. The migrated `stdlib_host`,
  `stdlib_time`, `stdlib_io`, `deny_clock`, and `bench_stdlib_time` call
  sites use `Clock::`; the benchmark also moved its remaining
  `time::duration_*` calls to `Duration::between(...).as_*()`.
- **Scorecard**: eradication scope stays **330** symbols (`pub fn`
  declarations 359, manifest public entries 305, unregistered 25); the
  `host/core namespace frees` family drops 40 -> **38** because the two frees
  are reclassified under `deprecated module frees` (6 -> **8**). Stability moves `stable`
  178 -> **176** and `deprecated` 72 -> **74**; `must delete` 74 -> **76**
  and `associated-or-method` 256 -> **254**; the fixture registry grows
  3361 -> **3365**. The `Clock` type is registered as `provisional`.
- **Verification**: `verify fixtures` **1901/1901 PASS**, 0 FAIL (1901
  compile+validate, 110 runtime smoke, 2 runtime-unsupported, 1 `test:`
  fixture PASS); `selfhost diag-parity` PASS; `docs check` 6/6;
  `verify lane` PASS; `check-stdlib-eradication`, `check-compat-orphans`,
  `check-comment-policy`, and `check-ark-code-quality` clean; ADR-029
  fixpoint PASS: s2 == s3 at sha256
  `802144555fc4a42ad43eff6ea6c227ac0e128bf9d6e7085a621691c7d48ac506`
  (pinned base `04d4446b…`).
- **Residual**: the two stable clock frees await the Wave 2 deletion (and its
  E0100 hints). `std::host::env` / `std::env` remain free because the env
  intrinsics do not lower behind a namespace method on `wasm32-gc`;
  `std::host::random` (`next_f64` is still a `0.0` stub), `std::host::process`,
  `std::process`, `std::cli`, and `std::signal` are untouched.

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
      - Landed 2026-10-27 (`wave/v128-memory`): the last 5 `std::wasm`
        v128 memory frees retired onto `V128Memory` after the 0xfd
        linear-memory lowering landed.
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

### Landed slice (wave/v128-memory, 2026-10-27)

- **Emitter**: added the 0xfd v128 linear-memory lowering. Ten new core
  operations (`wasm.v128.load8x8.s/u`, `load16x4.s/u`, `load32x2.s/u`,
  `wasm.v128.load_splat`, `store`, `load_lane`, `store_lane`) join the
  existing `wasm.v128.load`; the eleven `__wasm_v128_*` intrinsics are bound
  to them and the new `wasm::call_simd_memory` dispatch emits the 0xfd
  instruction plus a natural-alignment memarg. The runtime `offset` operand is
  added to the address (`i32.add`, or `i64.add` under memory64), so the memarg
  offset is zero. `load_lane` / `store_lane` read the lane as a compile-time
  immediate; the lane-position predicate now handles the fourth argument.
- **Migration**: the five experimental `std::wasm` frees move onto the new
  `V128Memory` `[[types]]` entry (`load`, `store`, `load_splat`, `load_lane`,
  `store_lane`, plus the six widening loads); their five `[[functions]]`
  entries and `pub fn` declarations are deleted. `load_splat` is
  `v128.load8_splat`; the lane functions move one 32-bit lane (`lane` = 0..3).
  E0100 hints in `resolver::removed_names` name each replacement.
- **Fixtures**: `stdlib_wasm/v128_memory_surface.ark` (wasm32-gc memory32 P2
  component) and `_wasm32.ark` (`--target wasm32`, memory32 linear) store and
  reload vectors at a fixed scratch page, assert lane values, and cover
  `load_splat`, `load_lane`, `store_lane`, and the widening loads.
  `diagnostics/removed_wasm_v128_memory_free.ark` + `_wasm32.ark` pin the
  E0100 hints on both targets.
- **Scorecard**: 358 -> **353** public symbols; `associated-or-method`
  288 -> **283**; manifest public entries 324 -> **319**; `pub fn`
  declarations 389 -> **384**; `must delete` 70, `intrinsic-only` 1, and
  `prelude_wrapper` 65 unchanged. The `std::wasm v128 memory frees` rule is
  removed from `docs/data/stdlib-api-eradication.toml`.
- **Verification (both targets)**: the wasm32-gc memory32 P2 component and
  the `--target wasm32` memory32 linear core both compile,
  `wasm-tools validate`, and run to completion with every lane assertion
  passing. `wasm-tools print` of the GC core shows `v128.load`, `v128.store`,
  `v128.load8_splat`, the six widening loads, `v128.load32_lane 0..3`, and
  `v128.store32_lane 0..3` (18 sites, no `unreachable`). A wasm32-gc memory64
  core compiles to `i64.add` + `v128.load` and validates under `memory64`; the
  P2 host profile cannot run a memory64 core directly, so that path is
  validate-only.
- **Gates**: `verify fixtures` **1897/1897 PASS / 0 FAIL** (1897
  compile+validate, 110 runtime smoke, 1 `test:` fixture PASS, 2 runtime
  contracts delegated to native-cpp); `docs check` 6/6; `verify lane` PASS;
  `check-core-ops`, `check-core-op-compiler-validator`,
  `check-native-cpp-capabilities`, `check-stdlib-eradication`,
  `check-compat-orphans`, `check-semantic-debt`, and
  `check-legacy-dispatch-inventory` clean; ADR-029 fixpoint PASS: s2 == s3 at
  sha256 `16d2af795aacca9013865653fe9e1d416c9dea496f5a0b9020a480c7a992a404`
  (pinned base `04d4446b`).
- **Residuals**: the fixture gate compiles `run:` fixtures as
  wasm32-gc memory32 P2 components only, so the memory64 `i64.add` path is
  validate-only; `std::simd` still owns the raw bitwise surface while
  `V128Memory` owns linear-memory access (ADR-042 D7); the wider lane widths
  (`load16_lane` / `load64_lane`, `load16_splat` / `load64_splat`) and
  `v128.load32_zero` / `load64_zero` stay unexposed until a caller needs them.

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

### Landed slice (wave/wave2-fs-clock, 2026-10-28)

- **ADR-087 Wave 2 physical deletion (7 stable frees)**: the retained host
  fs / clock stable frees from the 2026-10-27 host waves are gone —
  `std::host::fs::is_readable_file`; `std::fs::{read_string, write_string,
  exists, is_readable_file}`; `std::host::clock::{monotonic_now, now_ms}`.
  The manifest entries, the `pub fn` declarations, and the W0009 rows were
  deleted together.
- **Replacement coverage (verified before deletion)**: `Fs::read_to_string`,
  `Fs::write_string`, and `Fs::is_readable_file` exist in
  `std/host/fs.ark` with the documented signatures and semantics
  (`is_readable_file` is the read probe, not a path-existence query);
  `Clock::monotonic_now()` / `Clock::now_ms()` exist in `std/host/clock.ark`
  and are the only host clock surface. The two namespace-surface fixture
  pairs still compile, validate, and run on both targets.
- **E0100 hints**: `resolver::removed_names::removed_host_fs_member_hint`
  already named the `Fs` replacements for all five fs names; this wave adds
  `removed_host_clock_member_hint` for the `clock::` / `host::clock::` /
  `std::host::clock::` spellings of `monotonic_now` / `now_ms`.
  `lint::deprecated_table` drops the two host W0009 branches, and its tests
  now assert that no W0009 entry remains for the seven names.
- **Goldens (both targets)**: `diagnostics/deprecated_host_fs_free*` ->
  `diagnostics/removed_host_fs_stable_free*` and
  `diagnostics/deprecated_host_clock_free*` ->
  `diagnostics/removed_host_clock_free*`. The four fixtures now assert
  `error[E0100|resolve]: undefined module member: ...` with the exact
  replacement hint on `wasm32-gc` and `--target wasm32`; the compiler output
  matches each golden byte-for-byte.
- **In-tree call sites**: none remained. A repo-wide scan for
  `fs::read_string`, `fs::write_string`, `fs::exists`,
  `fs::is_readable_file`, `clock::monotonic_now`, and `clock::now_ms` finds
  only the four new E0100 goldens; the earlier host waves had migrated the
  compiler, stdlib, fixture, example, and benchmark call sites to `Fs::` /
  `Clock::`. The bootstrap overlay adapters still rewrite `Fs::` / `Clock::`
  call spellings for the committed pin; they are unchanged and remain
  pin-refresh debt (ADR-029).
- **Docs**: migration-guidance now lists the five fs and two clock entries
  under "Removed in Wave 2" with E0100 semantics; the changelog entry is
  2026-10-28; the generated `std::fs` / `std::host::fs` module page,
  reference, name index, scoreboard, monomorphic-deprecation table, and
  eradication scorecard were regenerated. `std::fs` remains documented as a
  deprecated `Fs` re-export facade.
- **Scorecard delta**: eradication scope 330 -> **323** (`pub fn`
  declarations 359 -> 352, manifest public entries 305 -> 298); `must delete`
  76 -> **69**; `deprecated module frees` 8 -> **1** (`std::env::get_var`);
  `deprecated` entries 74 -> **67**; `associated-or-method` stays 254,
  `prelude_wrapper` stays 65, unregistered declarations stay 25, fixture
  registry stays 3365. `check-stdlib-eradication.py`: 299/299 manifest
  functions and 25/25 unregistered declarations classified.
- **Verification**: `verify fixtures` **1901/1901 PASS / 0 FAIL** (1901
  compile+validate, 110 runtime smoke, 2 runtime-unsupported, 1 `test:`
  fixture PASS); `selfhost diag-parity` PASS; `docs check` 6/6; `verify lane`
  PASS; `selfhost build-compiler` succeeded (43.3 s) after the compiler-table
  edits.
- **Out of scope (unchanged)**: the experimental `fd_seek` / `fd_tell` /
  `fd_fdstat_errno` frees stay (`t3 = false`, wasm32 / WASI P1 only, no
  gc parity fixture); `std::env::get_var` remains the only
  `deprecated module frees` entry; `std::host::env` stays on free functions
  because `Env::*` still fails to lower on `wasm32-gc`.

### Landed slice (wave/env-getvar, 2026-10-28)

- **ADR-087 Wave 2 deletion (1 manifest + 2 declarations)**: the last
  `deprecated module frees` entry `std::env::get_var` and the duplicate
  `std::host::env::get_var` alias are gone. Qualified old calls fail with
  **E0100**; `resolver::removed_names::removed_host_env_member_hint` covers
  the `env`, `host::env`, `std::env`, and `std::host::env` spellings and names
  `Env::var(name)` from `use std::host::env::Env`. The bare-name hint is
  retargeted to the same replacement.
- **Surface**: `std::host::env::Env` (`provisional`) exposes `Env::var`,
  `Env::arg_count`, `Env::arg_at`, and `Env::var_or_default`. `Env::var` /
  `Env::arg_at` bind the intrinsic result to `Option<String>` to pin the
  Option GC layout; without the binding the direct intrinsic call leaves the
  MIR temp scalar. The four methods compile, validate, and run on `wasm32-gc`
  (P2 component) and `--target wasm32` (linear), including from a non-main
  module (`helper.ark` probe).
- **Blocked, measured — `Env::args`**: a direct `__intrinsic_args` call inside
  a namespace method still emits an invalid i64/ref frame on `wasm32-gc`.
  `wasm-tools validate` reports `func 18 failed to validate: type mismatch:
  expected i64, found (ref null $type) (at offset 0xa98)`; WAT:
  `(func $Env::args (;18;) (type 60) (result (ref null 19)) (local $_t0 i64) …`
  and the intrinsic result is stored with `local.set 0` into that i64 local
  (`local.get 23; ref.cast (ref null 19); local.set 0`). Root cause: the MIR
  call-result typing has no env intrinsic mapping (`__intrinsic_args` →
  `vec:String`, `__runtime_abi_env_var` / `__intrinsic_arg_at` →
  `Option_String`), unlike the existing `Fs::read_to_string` mapping in
  `mir/lower/call_type_fallback_stdlib.ark`. The pin predates that mapping and
  cannot compile an unannotated `Env::args` in the overlay, so `args` stays a
  free function and the direct spelling is deferred to a pin refresh.
- **Fixtures**: `stdlib_host/env_namespace_surface*` (run, both targets) and
  `diagnostics/removed_env_get_var*` (E0100, both targets); registry
  3365 -> **3369**.
- **Scorecard**: eradication scope 323 -> **322**; `pub fn` declarations
  352 -> **350**; manifest public entries 298 -> **297**; `must delete`
  69 -> **68**; `deprecated` entries 67 -> **66**; `associated-or-method`
  stays 254, `prelude_wrapper` stays 65, monomorphic helpers stay 51, and
  unregistered declarations stay 25. The `deprecated module frees` rule is
  removed from `docs/data/stdlib-api-eradication.toml` (no live match).
- **Verification**: `verify fixtures` **1903/1903 PASS / 0 FAIL** (1903
  compile+validate, 110 runtime smoke, 2 runtime-unsupported, 1 `test:`
  fixture PASS); `selfhost diag-parity` PASS with the two new E0100 goldens;
  `docs check` 6/6; `verify lane` PASS; `check-stdlib-eradication` (298/298
  manifest functions, 25/25 unregistered), `check-compat-orphans`,
  `check-comment-policy`, `check-ark-code-quality`, `check-semantic-debt`, and
  `check-legacy-dispatch-inventory` clean; `selfhost build-compiler` produced
  the current s2 in 1625.0 s. Both-target `wasm-tools validate` and runtime
  output (`env-var-unset`, `env-namespace-ok`) recorded for the new pair.
- **Residuals**: `Env::args` (above); the other env frees (`args`,
  `arg_count`, `arg_at`, `var`, `var_or_default`, `has_flag`) remain for their
  own wave, and `std::env` stays a deprecated module path.


### Landed slice (wave/env-args, 2026-10-28)

- **MIR fix (the blocker from `wave/env-getvar`)**: the env intrinsic call
  results now carry the same explicit typing as the fs file intrinsics in
  `mir/lower/call_type_fallback_stdlib.ark`: `__intrinsic_args` ->
  `vec:String`, `__runtime_abi_env_var` / `__intrinsic_arg_at` ->
  `Option_String`. `mir/lower/call_type_vec.ark` also marks an `Env::args`
  call site as `vec:String` because a namespace method has no registered
  free-call spine entry. The `local.set` of the Vec ref no longer targets an
  i64 local; `wasm-tools validate
  --features gc,function-references,memory64` accepts the generated core
  module on `wasm32-gc` and `--target wasm32`.
- **Surface**: `std::host::env::Env` (`provisional`) is the only env surface:
  `Env::args`, `Env::var`, `Env::arg_count`, `Env::arg_at`,
  `Env::var_or_default`, and `Env::has_flag`. The temporary `Option<String>`
  bindings are removed. `Env::arg_at` indexes `Env::args`, preserving
  argument lookup on `wasm32-gc` where the direct `__intrinsic_arg_at`
  lowering is still a constant `None`.
- **Removed (11 declarations, 10 manifest entries)**: the
  `std::host::env::{args, var, arg_count, arg_at, has_flag}` entries plus the
  unregistered `var_or_default`, and the legacy `std::env` module
  (`args` / `arg_count` / `arg_at` / `var` / `var_or_default`). Qualified old
  calls fail with **E0100** from
  `resolver::removed_names::removed_host_env_member_hint`; the bare-name hints
  cover the retired members. `std::env` no longer resolves, so its W0002 entry
  is removed from `lint::deprecated_table`.
- **Bootstrap overlay debt (ADR-029)**: the committed pin cannot lower
  `Env::args`, so `_patch_bootstrap_host_env_paths` rewrites the in-tree
  compiler's `Env::args` / `Env::arg_count` calls to the free builtin spelling
  for stage-2, and `_patch_bootstrap_main_host_to_prelude` accepts the
  `Env::args()` main form. Delete both when the pin is refreshed.
- **Fixtures**: new dual-target `stdlib_host/env_args_surface*` (compile,
  validate, run) covers `args` / `var` / `arg_count` / `has_flag`; the four
  `stdlib_env/*` free-function fixtures and their `t3-*` entries are retired;
  the remaining `stdlib_host`, `stdlib_cli`, `host`, `wasi_p2_native`,
  `native_cpp_*`, and `lsp_perf` call sites use `Env::`.
- **Scorecard**: scope 320 -> **310** (`pub fn` declarations 348 -> **337**,
  manifest public entries 295 -> **285**), `associated-or-method` 254 ->
  **244**, `must delete` 66, `prelude_wrapper` 65, monomorphic helpers 50, and
  unregistered declarations 25; fixture registry 3379 -> **3375**.
- **Verification**: both targets compile, validate, and run the new fixture
  with the same golden; the `wasm32-gc` core validates with
  `wasm-tools validate --features gc,function-references,memory64` and the
  `--target wasm32` core with plain `wasm-tools validate`. A P2 component with
  one real argument reports `arg_count=1` / `has_flag=true` /
  `arg_at(0)`=the argument; `gate-668-p2-args-env` PASS. `verify fixtures`
  **1904/1904 PASS**, 0 FAIL (109 runtime smoke, 2 runtime-unsupported, 1
  `test:` fixture PASS); `docs check` 6/6; `verify lane` PASS; eradication /
  compat-orphans / comment-policy / code-quality / semantic-debt /
  legacy-dispatch-inventory / docs-consistency / docs-freshness /
  deprecated-docs clean; ADR-029 fixpoint PASS at
  `aa39ccd602d9b98cd87ec72ee7344ac1cbd9c2d69527ac125508651ebc2e2e45`
  (pinned base `8df2698c…`), `build-compiler` 1657.7 s. The two
  `diagnostics/removed_env_get_var*` E0100 goldens still match byte-for-byte,
  and the new bare/qualified hints were exercised with `arukellt check`.

### Landed slice (wave/associated-remain, 2026-10-29)

- **Group**: `std::collections::compiler` — the 14 `arena_*` / `slotmap_*` /
  `interner_*` free functions. All entries were `experimental`, so
  [ADR-014](../adr/ADR-014-stability-labels.md) D4 allowed direct removal
  without a deprecation window or W0009 entries; this is the same lifecycle
  path used for the `std::wasm` memory helpers and the `std::wit` experimental
  frees.
- **Replacement surface**: `Arena` (`new` / `alloc` / `get` / `len`),
  `SlotMap` (`new` / `insert` / `get` / `remove` / `contains` / `len`), and
  `Interner` (`new` / `intern` / `lookup` / `len`) in
  `std/collections/compiler.ark`. The backing stores are private fields; the
  module's `[[types]]` and `[[modules]]` manifest entries document the new
  surface. `std/component/handle.ark` now stores a `SlotMap` in
  `HandleTableI32` and calls the receiver methods.
- **E0100**: `resolver::removed_names::removed_compiler_member_hint` covers
  the `compiler`, `collections::compiler`, and `std::collections::compiler`
  spellings (and the bare-name path) with the per-symbol associated / method
  replacement. Six new diag goldens
  (`diagnostics/removed_compiler_{arena,slotmap,interner}_free*`) assert the
  exact hint on both targets.
- **Migration**: the five `stdlib_collections_compiler` /
  `stdlib_compiler` fixtures moved to the type surface; expected stdout is
  unchanged. `docs/stdlib/migration-guidance.md` has the before/after table,
  and `docs/stdlib/changelog.md` records the wave.
- **Scorecard**: scope 310 -> **296**; `associated-or-method` 244 ->
  **230**; `must delete` 66 and `intrinsic-only` 1 unchanged; `pub fn`
  declarations 337 -> **323**; manifest public entries 285 -> **271**;
  unregistered declarations stay 25 and `prelude_wrapper` stays 65. The
  `std::collections frees` rule drops 100 -> **86** manifest entries (20
  unregistered source rows unchanged).
- **Verification**: `check-cross-backend-parity.py` reports parity=5 for the
  migrated compiler-collections fixtures and parity=2 for the two
  `stdlib_component/handle_basic*` fixtures; every gc component validates with
  `wasm-tools validate --features gc,function-references,memory64` and every
  linear core with plain `wasm-tools validate`. `wasmtime` runs the gc
  components with the same goldens. `verify fixtures` **1891/1891 PASS**, 0
  FAIL (109 runtime smoke, 2 runtime-unsupported, 36 `test:` fixtures PASS);
  `selfhost diag-parity` PASS with the six new E0100 goldens; `docs check`
  6/6; `verify lane` PASS; `check-stdlib-eradication` 272/272 manifest
  functions and 25/25 unregistered declarations classified; `quality quick`,
  `check-compat-orphans`, `check-comment-policy`, `check-semantic-debt`, and
  `check-legacy-dispatch-inventory` clean. `selfhost build-compiler`
  2145.5 s; ADR-029 fixpoint PASS at s2 == s3 sha256
  `53fdf6978bada5700fd49993bbf633c8aacefb83295a125ac08a926724750d05`
  (pinned base `8df2698c…`).
- **Residuals**: the remaining 230 `associated-or-method` symbols are
  unchanged by this slice. Manifest-backed 205: prelude 55 (including
  `panic`), hash 42, ordered 37, test 18, signal 10, seq 9, linear 7, cli 6,
  io 5, host stdio 4, host random 4, host process 3, host fs 3, legacy
  process 2. Unregistered source declarations 25: `std::collections::sort`
  no-argument frees 4, `bimap` 10, `trie` 7, `hashmap_entry_pairs`,
  `btree_set_find_idx`, `new_i32`, and `io_error_message`. The
  `std::collections::compiler` module has no free symbol left; the next
  coherent source-only groups are `bimap` (10) and `trie` (7), which still
  need type surfaces.

### Landed slice (wave/bimap-trie, 2026-10-29)

- **Group**: `std::collections` — the 10 unregistered `bimap_i32_i32_*` and
  7 unregistered `trie_*` top-level declarations. Neither family had a
  manifest entry, so ADR-014 D4 allowed direct removal: no deprecation
  window, no W0009 rows, and no `[[functions]]` entries to delete.
- **Replacement surface**: `Bimap` (`new`, `insert`, `get_by_key`,
  `get_by_value`, `contains_key`, `contains_value`, `remove_by_key`,
  `remove_by_value`, `len`, `is_empty`) and `Trie` (`new`, `insert`, `get`,
  `contains`, `has_prefix`, `remove`, `count_with_prefix`) in
  `std/collections/{bimap,trie}.ark`; the pair/node storage is a private
  field. Both types are monomorphic over the flat `Vec<i32>` store (`i32`
  keys/values and `String` keys / `i32` values); generic `Bimap<K, V>` /
  `Trie<V>` spellings need a different representation and are not landed.
  `Trie` has no `len`; `count_with_prefix("")` is the O(n) total count.
- **Manifest**: the `std::collections::bimap` and `std::collections::trie`
  modules and the two `struct` types are registered as `provisional`; no
  function entry was added.
- **E0100**: `resolver::removed_names::removed_bimap_member_hint` /
  `removed_trie_member_hint` cover the bare, `bimap`/`trie`,
  `collections::*`, and `std::collections::*` spellings. Four new goldens
  (`diagnostics/removed_bimap_free*`, `removed_trie_free*`) assert the exact
  hint on both targets.
- **Fixtures**: `stdlib_bimap/bimap_basic` migrated to the method surface;
  added `stdlib_bimap/bimap_basic_wasm32` and the new
  `stdlib_trie/trie_basic` + `trie_basic_wasm32` pair (compile, validate,
  run). `run:` registry 1891 -> **1894**.
- **Build blocker repaired**: `removed_names.ark` contained two top-level
  definitions of `hashmap_method_replacement_hint` (E0101 in one compilation
  unit), introduced by the 2026-10-27 simd wave and present at the base
  commit. The later `text::concat` definition is kept; this is a prerequisite
  for building s2, not part of the bimap/trie surface.
- **Scorecard**: scope 296 -> **279**; `associated-or-method` 230 ->
  **213**; `pub fn` declarations 323 -> **306**; unregistered declarations
  25 -> **8**; manifest public entries 271, `must delete` 66,
  `intrinsic-only` 1, and `prelude_wrapper` 65 unchanged. Fixture registry
  3461 -> **3468**.
- **Verification**: `verify fixtures` **1894/1894 PASS**, 0 FAIL (110 runtime
  smoke, 2 runtime-unsupported, 36 `test:` fixtures PASS);
  `selfhost diag-parity` PASS; cross-backend parity **2/2**
  (`stdlib_bimap/bimap_basic`, `stdlib_trie/trie_basic`); `docs check` 6/6;
  `verify lane` PASS; `check-stdlib-eradication` 272/272 manifest functions
  and 8/8 unregistered declarations classified; `check-compat-orphans`,
  `check-comment-policy`, `check-ark-code-quality`, `check-semantic-debt`
  clean. `selfhost build-compiler` 55.5 s (gc pin); ADR-029 fixpoint PASS at
  s2 == s3 sha256
  `e6fbb279ffab2c89786f511762edb91d55ebe89cc74460eab41ed6a3c188ecbf`
  (pinned base `35a2585a…`).
- **Residuals**: the remaining 213 `associated-or-method` symbols are
  unchanged by this slice; the 8 unregistered declarations are the four
  no-argument `std::collections::sort` frees, `hashmap_entry_pairs`,
  `btree_set_find_idx`, `new_i32`, and `io_error_message`. Generic storage
  for `Bimap<K, V>` / `Trie<V>`, a `Trie::len`, and a public iterator surface
  for either collection remain out of scope.

### Landed slice (wave/hash-group, 2026-10-29)

- **Group**: `std::collections::hash` — the 21 HashSet free functions
  (`hashset_*` 15, `hashset_str_*` 6). All entries were `stable`, so this is
  the ADR-087 two-phase lifecycle: Wave 1 (`0a1ba8c04`) moved the manifest
  entries to `deprecated` with `deprecated_by = "HashSet::<method>"`,
  `deprecated_since = "0.1.0"`, `remove_in = "1.0.0"` and added the W0009
  qualified-call table with dual-target `diagnostics/deprecated_hashset_free*`
  goldens. Wave 2 deletes the entries and the `pub fn` declarations after
  that window.
- **Replacement surface**: the already-landed `HashSet<i32>` /
  `HashSet<String>` methods (`a2abc9b18`, `03a9a7674`, `c918bc460`):
  `HashSet::new<i32>()` / `HashSet::new<String>()`, `insert`, `contains`,
  `remove`, `len`, `is_empty`, `to_vec`, `union`, `intersection`,
  `difference`, `capacity`, `reserve`, `shrink_to_fit`, `clear`, `extend`.
  The 21 declarations stay as non-public MIR lowering targets; the duplicate
  `Vec<String>` facade in `hash.ark` is removed, so `hash_string.ark` owns the
  String store.
- **E0100**: `resolver::removed_names::hashset_free_replacement_hint` covers
  `hash::hashset_*`, `collections::hash::hashset_*`, and
  `std::collections::hash::hashset_*`. Because the declarations remain
  private rather than absent, `resolver::scope_error::resolve_retired_private_member`
  turns the visibility error into the same "removed module member" E0100 with
  the method replacement. The old `hash_set::` adapter hints are unchanged.
  Dual-target `diagnostics/removed_hashset_free*` assert the exact message.
- **Fixtures**: 17 `stdlib_hashmap` fixtures migrated from the free calls to
  the `HashSet<T>` method surface (the 12 `run:` fixtures compile + validate +
  run on both targets; the 5 `test:` fixtures pass on both). The fixture
  registry stays **3475** (2 diag fixtures renamed, not added).
- **Scorecard**: eradication scope 274 -> **253**; `associated-or-method`
  208 -> **187**; manifest public entries 270 -> **249**; `pub fn`
  declarations 301 -> **274** (the 21 names, including the duplicate String
  declarations); the `std::collections frees` rule drops 86 -> **65**
  manifest rows; unregistered declarations stay 4, `must delete` 66,
  `intrinsic-only` 1, `prelude_wrapper` 64. Scoreboard stability:
  stable 164 -> **143**, deprecated 65 -> **65** (the Wave 1 step raised it to
  86 before deletion), hash family 42 -> **21** APIs.
- **Verification**: `verify fixtures` **1895/1895 PASS**, 0 FAIL (110 runtime
  smoke, 2 runtime-unsupported, 36 `test:` fixtures PASS); both-target
  `wasm-tools validate` for every migrated `run:` fixture (gc features +
  plain wasm32) and `arukellt test --target wasm32` PASS for the 5 test
  fixtures; `selfhost diag-parity` PASS with the two new E0100 goldens;
  `docs check` **6/6**; `verify lane` PASS; `check-stdlib-eradication`
  250/250 manifest functions and 4/4 unregistered declarations classified;
  `check-compat-orphans`, `check-comment-policy`, `check-ark-code-quality`,
  and `check-semantic-debt` clean. `selfhost build-compiler` 100.2 s (first,
  after the Wave 2 surface edits) and 56.7 s (second, after the resolver
  visibility-hint fix); ADR-029 fixpoint PASS at s2 == s3 sha256
  `e92b912e6251eedbe0c037c3c9086c0f95c72bf275a432949e032c3928ab1c5e`
  (pinned base `62aecf93…`).
- **Residuals**: the 21 `hashmap_*` flat-store frees in the same module
  (`new`, `get`, `get_option`, `contains`, `set`, `insert`, `size`,
  `is_empty`, `keys`, `values`, `entries`, `remove` plus the
  capacity/bulk-maintenance group `capacity`, `with_capacity`, `reserve`,
  `try_reserve`, `shrink_to_fit`, `clear`, `drain`, `remove_entry`,
  `extend`) stay public until the HashMap capacity/bulk method surface lands;
  the unregistered `hashmap_entry_pairs` remains public as the lowering
  target for `entries()` and is outside this 21-row group. Known inference
  gap, measured: `let u = s.union(t)` with an
  inferred local traps inside a `test` block (the annotated
  `let u: HashSet<i32> = ...` form works on both targets), so the migrated
  fixture uses the annotated form; the unannotated test-block lane is
  deferred with the method-return type inference.
