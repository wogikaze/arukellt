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

