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

### Landed slice (wave/host-core, 2026-10-27)

- Retired the 15 `stable` free helpers of the three pure receiver modules and
  the unregistered `path::last_index_of` helper: `std::path` (9 + 1),
  `std::time` (3), and `std::random` (3). The replacements are the
  `Path` / `Duration` / `Rng` receiver types; no host capability or
  non-main-module associated call is involved (all methods take a plain
  receiver and the fixtures compile on both targets).
- **ADR-087 two-step**: Wave 1 (commit `4be2ec4e5`) added the receiver
  surface, converted the manifest entries to `deprecated`
  (`deprecated_since = "0.1.0"`, `remove_in = "1.0.0"`) and pinned live
  **W0009** qualified-call warnings with six diag goldens (3 symbols × 2
  targets). Wave 2 deleted the manifest entries and `pub fn` declarations and
  replaced the W0009 rows with E0100 hints in
  `resolver::removed_names::removed_{path,time,random}_member_hint` plus the
  `path::last_index_of` hint.
- **Replacement surface**: `Path::new` / `into_string` / `join` / `parent` /
  `file_name` / `extension` / `with_extension` / `is_absolute` / `stem` /
  `normalize` / `components`; `Duration::between` / `as_ms` / `as_us` /
  `as_ns`; `Rng::new` / `next` / `range` / `shuffle`. The private free
  helpers stay as implementation details of the methods.
- **Fixtures**: migrated the 12 existing path/time/random call-site fixtures
  plus the quickstart example; added the dual-target
  `stdlib_path/path_method_surface*`, `stdlib_time/duration_methods*`, and
  `stdlib_random/rng_methods*` pairs (compile + execute on `wasm32-gc` and
  `--target wasm32`); renamed the six W0009 diag fixtures to
  `removed_{path,time,random}_free*` E0100 goldens (no fixture-count change).
- **Scorecard delta**: 588 -> **572** public symbols; `associated-or-method`
  518 -> **502**; manifest functions 417 -> **402**; `pub fn` declarations
  642 -> **626**; unregistered declarations 172 -> **171**; `must delete`
  85 -> **70**; `prelude_wrapper` 65 and `intrinsic-only` 1 unchanged. The
  `host/core namespace frees` group drops from 78 manifest + 22 source to
  **63 + 21**.
- **Residual in the group**: the host side (`std::host::fs` / `env` /
  `process` / `clock` / `random`, `std::fs` / `env` / `cli` / `signal`) and
  `std::core` (`Range` / `RangeInclusive` / `Error::message` / `combine` /
  `identity` plus the 20 `std::core::raw` bridges). The host families depend
  on the separate non-main-module associated-call lane; `std::host::stdio`
  stays with #693. The raw bridges need a `RawArray` / `RawString` namespace
  surface.

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
      - Landed 2026-10-27 (`wave/host-core`): the 15 pure
        `std::path` / `std::time` / `std::random` frees and the unregistered
        `path::last_index_of` retired after the `Path` / `Duration` / `Rng`
        receiver surface landed; the stable entries completed the ADR-087
        W0009 window in Wave 1 before deletion. The remaining host/core
        families stay in the rule with their lowering constraints recorded.
- [x] Keep the scorecard at zero-unclassified while waves land: update
      the TOML rules as symbols retire so the checker stays green.
- [x] Update user-facing docs/examples in the same PR that retires a
      symbol family (docs-sync gate).

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

