# Stdlib API Changelog

> Stdlib の公開 API lifecycle（deprecation / removal / tier change）を記録する手動管理の changelog。
>
> ADR-014 と [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) が要求する changelog
> entry の置き場である。現在の stability と replacement は `std/manifest.toml` と生成された
> `reference.md` / `eradication.md` を正とし、このファイルは変更履歴を持つ。生成物では
> ない。entry は日付順に追記し、過去の entry を書き換えない。

## 記録ルール

- 1 entry = 1 日付 + 1 wave。ADR / issue、対象シンボル、変更、replacement を書く。
- `stable` の削除は [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) の 3 条件
  （W0009、migration guide、この changelog）を満たす。条件を満たさない削除を先に
  記録しない。
- このファイルの過去 entry は削除時点の履歴であり、現在の API 一覧ではない。現在の
  事実は manifest と生成 docs から取得する。

## 2026-10-27 — HashSet aggregate, String, and ADR-086 trait surface (#917, ADR-046 / ADR-086)

- **Basis**: [ADR-086](../adr/ADR-086-linear-collection-adts.md) fixes the
  collection receiver surface, and [ADR-046](../adr/ADR-046-free-function-eradication.md)
  requires the replacement to land before the free/adapter spelling retires.
  The six-method `HashSet<i32>` membership surface landed earlier the same day.
- **Landed (i32)**: `to_vec`, `union`, `intersection`, `difference`,
  `capacity`, `reserve`, `shrink_to_fit`, `clear`, and `extend` as receiver
  methods over the flat `hash::hashset_*` store. Set-returning methods keep the
  receiver marker on the temporary, so `s.union(t).len()` and chained
  membership calls resolve to the set.
- **Landed (String)**: `HashSet::new<String>()` plus `insert`, `contains`,
  `remove`, `len`, and `is_empty` over the `Vec<String>` facade.
- **Landed (traits)**: `IntoIterator` (snapshots `to_vec` into the shared
  `VecIter<i32>`), `Clone`, `Eq`, and `Debug` for `HashSet<i32>`. `for value in
  set` now calls the set's `into_iter`, not the raw Vec cursor over the bucket
  layout.
- **Retired (21 adapters)**: the 15 `hash_set::hashset_i32_*` adapters and the
  6 `hash_set::hashset_str_*` facade functions (including `hashset_str_new`).
  Qualified calls fail resolution with an E0100 hint from
  `resolver::removed_names::hashset_method_replacement_hint`. The
  `hash::hashset_*` / `hash_string::hashset_str_*` implementation functions
  remain internal lowering targets. The scorecard delta is 15 because the six `hashset_str_*` names duplicate manifest-registered implementation functions and were therefore not counted as unregistered declarations.
- **Compiler fixes**: the HashSet receiver marker now also applies to
  concrete impl owners (`HashSet_i32` / `HashSet_String`) and to method
  parameters; the method target table covers the aggregate and ADR-086 trait
  names; the clone result emits the flat Vec alias; and both the legacy and
  CoreHIR `for` lowering consult the marker before the Vec cursor. The
  marker-less fallback stays limited to `insert` / `contains` / `remove`, so
  the String store's own `hashset_str_len` body is not rewritten to itself.
- **Fixtures**: the existing `stdlib_hashset/hashset_methods.ark` +
  `_wasm32.ark` pair now covers membership, aggregates, chained calls, the
  String lane, `Clone` / `Eq` / `Debug`, direct `into_iter`, and `for`
  iteration on `wasm32-gc` and `--target wasm32`. Four `stdlib_hashmap`
  fixtures moved off the adapters. The new
  `diagnostics/removed_hashset_adapter*.ark` pair pins the E0100 hint on both
  targets.
- **Scorecard**: 515 → **500** public symbols; unregistered declarations
  146 → **131**; `pub fn` declarations 569 → **548**;
  `associated-or-method` 445 → **430**. Manifest public entries 369,
  `prelude_wrapper` 65, monomorphic manifest helpers 57, `must delete` 70,
  and `intrinsic-only` 1 are unchanged.
- **Blocked / residual**: `FromIterator` stays unlanded because the resolver
  cannot select `HashSet::from_iter` for a concrete
  `impl FromIterator<VecIter<i32>> for HashSet<i32>` (the impl is registered
  under `HashSet_i32::from_iter__VecIter_i32`, and `HashSet<i32>::from_iter`
  is not valid source syntax), so no source spelling reaches it.
  `HashSet<String>` aggregates and trait impls await store-level snapshot
  accessors for the `Vec<String>` facade; only its membership methods are
  public, and a `for` over a String set is rejected as non-iterable until
  then.

## 2026-10-27 — std::json / std::toml / std::csv associated conversions (#917, ADR-046)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and the
  [ADR-014](../adr/ADR-014-stability.md) D4 experimental rule. Every removed
  manifest entry was `experimental`; unregistered declarations were never in
  the stable 64, so no ADR-087 W0009 window applies. E0100 hints come from
  `removed_json_member_hint` / `removed_toml_member_hint` /
  `removed_csv_member_hint` and the bare-name fallback.
- **Compiler unblock**: a `module::Type::method` call in a loaded module kept
  the module prefix (the resolver only canonicalizes the entry module), while
  the signature spine registers `Type::method`. Lowering left `fid=-1`; the
  fused in-loop emitter emitted `drop; unreachable` instead of deferring, and
  the reachability worklist could drop the impl body. `callee_spelling_variants`
  now canonicalizes the spelling in lowering, the typechecker, edge discovery,
  the in-loop oracle, and the drain-time name fallback.
- **Removed (32 manifest + 25 unregistered)**: the JSON text helpers
  (`quote_string`, `json_escape`, `skip_ws`, `parse_int_at`,
  `parse_content_length`, `find_key_pos`, `json_hex_byte`, `json_hex_value`,
  `json_parse_string_at`, `json_decode_escape[_at]`,
  `json_decode_unicode_ascii`, `json_get_str[_after]`, `json_get_int`), the
  JSON primitive constructors/serializers (`json_stringify_*`, `json_null`,
  `json_parse_*`, `stringify_pretty`, `json_field`, `json_str`, `json_i32`,
  `json_bool`, `json_null_val`, `json_raw`, `json_obj_new`, `json_arr_new`,
  `json_obj_string`, `json_arr_string`, `json_encode_string`), the TOML text
  scanners (`toml_parse_line`, `toml_find_*`, `find_toml_*`,
  `toml_parse_string_array`), and the CSV frees (`csv_split_line`,
  `csv_parse_row`, `csv_stringify_row`, `csv_count_rows`, `csv_get_row_raw`,
  `csv_parse_row_at`, `csv_parse`, `csv_stringify`, `csv_parse_with_header`).
- **Replacement**: `JsonText` (`quote`, `escape`, `skip_ws`, `parse_int_at`,
  `content_length`, `find_key_pos`, `hex_byte`, `hex_value`,
  `parse_string_at`, `decode_escape_at`, `decode_unicode_ascii`, `get_str`,
  `get_int`), `JsonValue` constructors (`null`, `bool`, `int`, `string`,
  `raw`, `array`, `object`), `JsonField::new`, `JsonValue::pretty_text`,
  the `JsonValue` primitive helpers (`stringify_*`, `parse_*`,
  `encode_string`, `null_text`), `JsonStrResult` / `JsonEscapeResult`
  accessors, `TomlText` (`parse_line`, `find_section`, `find_value`,
  `find_raw_value`, `parse_string_array`), and `Csv` (`split_line`,
  `parse_row`, `stringify_row`, `count_rows`, `get_row_raw`, `parse_row_at`,
  `parse`, `stringify`, `parse_with_header`). The full before/after table is
  in the [migration guide](migration-guidance.md#associated-conversion-wave-2026-10-27-917--wavenon-main-module).
- **Fixtures**: `stdlib_json/json_associated_surface*.ark`,
  `stdlib_toml/toml_text_surface*.ark`,
  `stdlib_csv/csv_namespace_surface*.ark`, and the two-module regression
  `modules/non_main_associated_call/`. The pairs compile and run on
  `wasm32-gc` (P2 component) and `--target wasm32` (linear).
- **Scorecard**: 588 → **531** public symbols; `associated-or-method`
  518 → **461**; manifest functions 417 → **385**; unregistered declarations
  172 → **147**. `must delete` 70 and `intrinsic-only` 1 unchanged.
- **Residuals**: `json::parse` / `toml::toml_parse` / `toml::toml_stringify`
  stay free (unannotated associated `parse` loses the Result payload type;
  `TomlValue::to_text` still emits an i64 return on wasm32-gc);
  `Vec<String>::concat` remains invalid on wasm32-gc independently of this
  wave (join with an explicit loop / use `len` / `get_unchecked`). The
  committed pin predates the fix, so the generated flat overlay routes the
  compiler's own manifest scans through free `bootstrap_toml_*` shims
  (`scripts/selfhost/checks.py::_patch_bootstrap_manifest_scanner_paths`);
  delete the adapter when the pin is refreshed.

## 2026-10-27 — std::json / std::toml value methods (#917, ADR-046)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and the
  [ADR-014](../adr/ADR-014-stability.md) D4 experimental rule. All removed
  manifest entries were `experimental`; no ADR-087 W0009 window applies.
- **Removed (18 manifest + 9 unregistered)**: `json_get`, `json_get_index`,
  `json_as_bool/string/i32/f64/array`, `is_null/bool/number/string/array/object`,
  `stringify`, `parse_error_message`, `toml_get`, `toml_table_keys`, and
  `toml_as_bool/int/string/float/array/table`.
- **Replacement**: `JsonValue` methods (`v.is_*()`, `v.as_*()`,
  `v.get_field(key)`, `v.get_index(i)`, `v.to_text()`,
  `e.message()` on `JsonParseError`) and `TomlValue` methods
  (`v.as_*()`, `v.get_field(key)`, `v.table_keys()`). See the
  [migration guide](migration-guidance.md#stdjson--stdtoml-value-method-wave-adr-046-2026-10-27).
- **Retained by lowering blocks**: the primitive helpers, DOM constructors,
  `JsonText` / `TomlText` scanner, CSV frees, `json::parse`,
  `toml::toml_parse`, and `toml::toml_stringify`. Associated calls from a
  non-main module emit an unresolved-call fallback, and associated `parse`
  loses its `Result` payload type in an unannotated `match`.
- **Scorecard**: 728 → **701**; `associated-or-method` 658 → **631**;
  manifest functions 507 → **489**; unregistered declarations 222 → **213**.

## 2026-10-27 — std::bytes free surface removed (#917, ADR-087 Wave 2)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2/D4, the
  [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) 0.x exception, and the
  [#917](https://github.com/wogikaze/arukellt/issues/917) execution lane. The
  45 entries were `stable` and completed their Wave 1 window in the
  2026-10-27 bytes surface wave (`deprecated` + W0009,
  `deprecated_since = "0.1.0"`, `remove_in = "1.0.0"`).
- **Removed (45)**: `bytes_new`, `bytes_len`, `bytes_get`, `bytes_push`,
  `bytes_eq`, `bytes_from_string`, `string_from_bytes`, `bytes_concat`,
  `bytes_slice`, `byte_length`, `hex_encode`, `hex_decode`,
  `leb128_encode_u32`, `leb128_encode_i32`, `u16_to_le_bytes`,
  `u16_from_le_bytes`, `u32_to_le_bytes`, `u32_from_le_bytes`,
  `u32_to_be_bytes`, `u32_from_be_bytes`, `bytes_from_array`, `buf_new`,
  `buf_with_capacity`, `buf_push_u8`, `buf_push_u16_le`, `buf_push_u32_le`,
  `buf_push_u64_le`, `buf_extend`, `buf_freeze`, `buf_len`, `cursor_new`,
  `cursor_pos`, `cursor_remaining`, `read_u8`, `read_u16_le`, `read_u32_le`,
  `read_u64_le`, `read_u32_be`, `read_bytes`, `base64_encode`,
  `base64_decode`, `leb128_decode_u32`, `leb128_decode_i32`,
  `leb128_encode_u64`, `leb128_decode_u64`. The manifest entries and the
  `pub fn` declarations are gone.
- **Replacement**: `Vec<i32>` byte-vector methods (`b.push_byte`, `b.len`,
  `b.get_unchecked`, `a.equals_bytes`, `a.concat_bytes`, `b.slice_bytes`,
  `b.to_hex`, `b.to_base64`, the endian `to_*` / `push_*` pairs,
  `b.extend_bytes`, `b.freeze`, `data.byte_cursor()` with the `c.read_*` /
  `c.try_read_*` / `c.read_leb128_*` readers) plus the `Bytes` namespace for
  receiver-less constructors/decoders (`Bytes::from_hex`,
  `Bytes::from_base64`, `Bytes::from_array`, `Bytes::from_u16_le`,
  `Bytes::from_u32_le`, `Bytes::from_u32_be`, `Bytes::from_leb128_*`).
  Import `use std::bytes` for the methods and `use std::bytes::Bytes` for the
  associated functions.
- **Diagnostics**: the 45 W0009 rows became **E0100** hints from
  `resolver::removed_names::removed_bytes_member_hint`; the eight
  unregistered helpers retired in Wave 1 (`hex_char`, `hex_val_char`,
  `try_hex_val_char`, and the five `try_read_*` readers) keep their hints in
  the same table (the earlier merge dropped the table, so this wave restores
  it with the 45 additions). The W0009 goldens were replaced by
  `diagnostics/removed_bytes_free*.ark` on both targets.
- **Call sites**: 150 fixture files (869 `bytes::*` calls) and the
  `modules/use_qualified_call` qualifier fixture moved to the method /
  associated surface; the cookbook `wasm-binary` code blocks and the language
  spec section were updated.
- **Scorecard**: 858 → **813** public symbols; `must delete` 115 → **70**;
  manifest functions 626 → **581**; `pub fn` declarations 981 → **936**;
  `associated-or-method` 743 and `intrinsic-only` 1 unchanged. The
  `deprecated std::bytes frees` rule is removed from the eradication TOML (no
  remaining symbol matches it).
- **Verification**: `verify fixtures` **1845/1845 PASS**, `selfhost
  diag-parity` 124 PASS / 0 FAIL, both targets (wasm32-gc P2 and linear
  wasm32 P1) compile+validate for every changed fixture, `docs check`,
  `check-stdlib-eradication`, and `check-compat-orphans` clean.
- **Residual**: `Bytes::from_base64` still skips invalid quartets
  (compatibility behavior), `try_read_u64_le` stays deferred (constructing
  `Some(i64)` emits invalid Wasm), and the `std::io` /
  `std::host::fs` byte-layout adapters remain under their own lanes.

## 2026-10-27 — concrete `HashSet<i32>` membership surface (#917)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and
  [ADR-086](../adr/ADR-086-linear-collection-adts.md) §2/§4: public collection
  types use associated constructors and receiver methods. This is an
  incremental slice of the [#917](https://github.com/wogikaze/arukellt/issues/917)
  execution lane; the aggregate and `String` lanes are not part of it.
- **Added**: `HashSet<i32>` (`std::collections::hash_set::HashSet`) with
  `HashSet::new<i32>()` and the receiver methods `insert`, `contains`,
  `remove`, `len`, `is_empty`. They lower through the compiler method rewrite
  to the flat `hash::hashset_*` store.
- **Retained, deprecated (20)**: the 15 `hash_set::hashset_i32_*` adapters and
  the 5 `hash_set::hashset_str_*` `Vec<String>` facade adapters. They stay for
  the aggregate i32 and `String` lanes, whose GC lowering is not fixed yet;
  no manifest lifecycle or E0100 removal applies.
- **Fixtures**: `stdlib_hashset/hashset_methods.ark` and
  `stdlib_hashset/hashset_methods_wasm32.ark` (both targets, compile + run).
  No aggregate, `String`, or `union`/`reserve`/`extend` fixture is registered.
- **Residual**: `HashSet<T>` has no aggregate methods (`to_vec`, `union`,
  `intersection`, `difference`, `capacity`, `reserve`, `shrink_to_fit`,
  `clear`, `extend`) or `HashSet<String>` methods until the GC method
  lowering is fixed; those users keep the deprecated adapters. No
  `IntoIterator` / `FromIterator` / `Clone` / `Eq` / `Debug` impls yet.

## 2026-10-27 — iterator collect lane fixes (#917, #691)

- **Basis**: the iterator-collect residuals reported by the
  [#917](https://github.com/wogikaze/arukellt/issues/917) `next1054` lane and
  the lazy-adapter lane of [#691](https://github.com/wogikaze/arukellt/issues/691).
  No public signature is removed or renamed in this entry.
- **Fixed**: `UniqueIter` / `RevIter` `.collect()` now emits the method body;
  the AST edge worklist could infer the receiver of
  `a.iter().unique().collect()` as the inner `VecIter`, keep
  `VecIter::collect`, and drop `UniqueIter::collect`, so the call lowered to
  the unresolved `drop; unreachable` fallback while `.next()` / `.count()`
  worked.
- **Fixed**: `TakeIter` / `SkipIter` `.collect()` instantiated inside a std
  module resolved its `for item in self` desugar to the first concrete
  `into_iter` variant (`TakeIter<MapIter<..>>`), which does not match the
  emitted `TakeIter<VecIter<i32>>` signature. Both adapters now drain through
  an explicit `self.next()` loop in their collect hook (the same item
  protocol, without the `for` desugar), so `I::Item` resolves per instance.
  The deprecated
  `std::seq::take_i32` / `skip_i32` shims are lazy again
  (`v.iter().take(n).collect()` / `v.iter().skip(n).collect()`) with the same
  signatures and boundary results.
- **Fixed**: `std::core::iter` method calls in a module that omits
  `use std::core::iter` now fail type checking with
  ``method `<name>` requires `use std::core::iter` `` instead of silently
  lowering to the unresolved `drop; unreachable` fallback. Auto-loading the
  module from the prelude is not viable yet: its `Pair<A, B>` / `Range`
  declarations collide with same-named user types in the shared type registry,
  so the diagnostic keeps the explicit-import contract that the migration
  docs already use.
- **Fixtures**: `stdlib_trait/iterator_unique_rev_collect*.ark`,
  `stdlib_seq/seq_take_skip_collect*.ark`, and the
  `diagnostics/iterator_methods_no_import.ark` E0200 golden; the
  `iterator_method_surface*.ark` pair now asserts the `unique` / `rev`
  `collect` values instead of only `next` / `count`. Each pair compiles,
  validates, and runs on `wasm32-gc` (memory32 / P2) and `--target wasm32`
  (wasi-p1).
- **Residuals**: a negative `n` passed to the `std::seq` take shim still traps
  on linear-memory `wasm32`; the same call traps with the previous eager
  implementation, so it is a separate pre-existing linear lowering bug. The
  new fixtures pin the supported 0 / positive / over-length boundaries. The
  underlying generic-`for` site selection still pins the first concrete
  `into_iter` instance for other adapter bodies; fixing that needs
  per-enclosing-instance call-site records and is not required for the two
  collect hooks fixed here.

## 2026-10-27 — eager iterator constructor/collector frees removed (#917)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2/D4 and
  the [#917](https://github.com/wogikaze/arukellt/issues/917) execution lane.
  All 44 symbols were `provisional` manifest entries with top-level `pub fn`
  declarations; the provisional lifecycle allows a minor-version change with
  changelog notice, so no ADR-087 `stable` exception and no W0009 deprecation
  window applied.
- **Removed (44)**: the eager `std::core::iter` constructors (`vec_iter`,
  `range_iter`, `range_inclusive_iter`, `map_iter`, `filter_iter`, `take_iter`,
  `skip_iter`, `chain_iter`, `enumerate_iter`, `zip_iter`, `flat_map_iter`,
  `peek_iter`, `step_by_iter`, `scan_iter`, `unique_iter`, `rev_iter`) and the
  collectors/consumers (`collect_vec_iter` … `collect_range_iter`,
  `collect_map_i32`, `collect_filter_i32`, `collect_take_i32`,
  `collect_skip_i32`, `collect_chain_i32`, `collect_range_i32`,
  `collect_map_filter_i32`, `collect_map_filter_take_i32`, `iter_fold_i32`,
  `iter_count_i32`, `iter_sum_i32`, `iter_any_i32`, `iter_all_i32`,
  `iter_contains_i32`, `iter_count_eq_i32`, `iter_unique_i32`,
  `iter_reverse_i32`).
- **Replacement**: the lazy adapter method surface on `VecIter<T>`.
  `chain`, `enumerate`, `zip`, and `all` landed in the same change so every
  removed constructor has a method spelling; the collectors are
  `collect` / `count` and the numeric capabilities
  (`sum`, `fold`, `contains`, `count_eq`) are trait methods. `iter::*`
  qualified calls now fail resolution with an **E0100** hint from
  `resolver::removed_names`; every before/after row is in
  [migration-guidance.md](migration-guidance.md).
- **In-tree callers**: the 10 deprecated `std::seq` compatibility helpers were
  migrated too; map / filter / fold now use the methods, while take / skip
  keep an eager bounded copy because the `TakeIter` / `SkipIter` collector
  mis-resolves its item type when instantiated from a std module (#691; the
  same chain compiles from user code).
- **Scorecard**: public surface 1104 → **1060** symbols; manifest public
  entries 729 → **685**; `pub fn` declarations 1279 → **1235**;
  `associated-or-method` 1021 → **977**; monomorphic `*_i32` manifest helpers
  114 → **97**. `must delete` 83, `intrinsic-only` 1, `prelude_wrapper` 65,
  and the 375 unregistered declarations are unchanged. The
  `std::core::iter frees` classification rule retired with the family, and the
  module's 44-API row dropped from the coverage scoreboard.
- **Evidence**: stage-2 rebuild; full fixture parity **1839/1839**
  compile+validate PASS with 105 runtime smoke fixtures (the two new
  `stdlib_trait/iterator_method_surface*.ark` fixtures cover the method
  surface on `wasm32-gc` and linear `wasm32`); the 56 migrated
  `stdlib_core/iter_*` and `stdlib_trait/iterator_*` fixtures compile and
  validate on both targets; `selfhost diag-parity` **117/117** PASS including
  the two new `diagnostics/removed_iter_constructor*.ark` E0100 goldens;
  cross-backend parity **2/2** for the new pair; `verify lane` PASS;
  `check-stdlib-eradication`, `check-compat-orphans`, and `docs check` 6/6
  clean.
- **Residuals**: the `.unique().collect()` / `.rev().collect()` runtime lane
  traps inside the shared generic `collect_iterator` on both targets
  (pre-existing: `.next()` / `.count()` work, and the retired
  `iter_unique_i32` / `iter_reverse_i32` frees used the same trapping collect
  path), so the new fixture pins the reachable next/count paths. Methods of
  `std::core::iter` resolve without importing the module but miscompile, so
  migrated call sites keep `use std::core::iter`; this loader/lowering gap is
  separate from the eradication wave.

## 2026-10-27 — stable seq/combinator frees removed (ADR-087 Wave 2, #703 / #917)

- **Basis**: [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) D2/D3,
  [ADR-046](../adr/ADR-046-free-function-eradication.md), and the
  [#703](https://github.com/wogikaze/arukellt/issues/703) /
  [#917](https://github.com/wogikaze/arukellt/issues/917) execution lanes.
- **Conditions**: (1) the 31 entries carried `deprecated_since = "0.1.0"` and
  `remove_in = "1.0.0"` from the Wave 1 deprecation on 2026-10-26, with live
  W0009 coverage and diagnostic fixtures; (2) `docs/stdlib/migration-guidance.md`
  records the before/after and the reachable replacement for every symbol;
  (3) this entry. The 31 bare names now fail resolution with **E0100** and the
  `resolver::removed_names` hint, replacing the W0009 warning.
- **Removed (31)**: `sort_i32`, `sort_i64`, `sort_f64`, `sort_String`,
  `map_i32_i32`, `map_i64_i64`, `map_f64_f64`, `map_String_String`,
  `filter_i64`, `filter_f64`, `filter_String`, `fold_i32_i32`,
  `fold_i64_i64`, `fold_f64_f64`, `any_i32`, `any_i64`, `any_f64`,
  `any_String`, `find_i32`, `find_i64`, `find_f64`, `find_String`,
  `contains_i32`, `contains_String`, `reverse_i32`, `reverse_String`,
  `remove_i32`, `sum_i32`, `product_i32`, `map_option_i32_i32`, and
  `contains_string` (manifest entries and `std/prelude.ark` declarations).
- **Reason**: ADR-046 eradication of the user-reachable monomorphic prelude
  surface; the replacement is the `Vec` / iterator method surface, with the
  concrete reachable fallback recorded per row in the migration guide.
  `filter_i32` remains `provisional` (individual deferral) and was not
  removed.
- **Replacement**: `v.sort_by(cmp)` for `Vec<i32>` / `Vec<String>` and
  `std::seq::sort_by(v)` for the `i64` / `f64` sort lanes;
  `v.iter().map(f).collect()`, `v.iter().filter(f).collect()`,
  `v.iter().fold(init, f)`, `v.iter().sum()`, `v.iter().any(f)`,
  `v.iter().find(f)`, `v.iter().contains(x)`, and
  `v.iter().rev().collect()` for the verified item lanes; `v.drain(i, i + 1)`
  for `remove_i32`; `o.map(f)` via `std::core::container` for
  `map_option_i32_i32`; and `s.contains(sub)` for `contains_string`. The
  `i64` / `f64` numeric consumers, the `Vec<String>` reverse/contains lanes,
  and the generic `Vec::sort_by` receiver for `i64` / `f64` still emit invalid
  Wasm, so those rows document an explicit loop / `std::seq::sort_by`
  fallback until the lane lands.
- **Scorecard**: public surface 1168 → **1137** symbols; manifest public
  entries 776 → **745**; `prelude_wrapper` entries 96 → **65**; `pub fn`
  declarations 1359 → **1328**; `must delete` 114 → **83**
  (`associated-or-method` unchanged at 1054). The family count is 32 → **1**
  (`filter_i32` only).
- **Evidence**: stage-2 rebuild; full fixture parity **1836/1836**
  compile+validate PASS (105 runtime smoke, 1 `test:` fixture PASS);
  cross-backend parity **81/81** for the changed run fixtures
  (`wasm32-gc` component vs `wasm32` linear core, stdout identical);
  `selfhost diag-parity` PASS with the new E0100 goldens;
  `check-stdlib-eradication`, `check-compat-orphans`, and `docs check` 6/6
  clean. `verify lane` PASS after forcing single-file lint with the pinned
  bootstrap CLI: the stage-2 stub host traps in its `lint` `parse_args` path
  before per-file fallback (pre-existing tooling residual, not a source
  failure). The ADR-029 fixpoint result is recorded in the wave receipt.

## 2026-10-27 — std::io Phase F completion (ADR-085 Phase F, #693)

- **Basis**: [ADR-085](../adr/ADR-085-io-trait-unification.md) D2/D3/D5/D8/D9,
  [ADR-046](../adr/ADR-046-free-function-eradication.md) D3/D4, and the
  [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) 0.x removal exception.
  The BufReader dispatch residual that blocked the buffered wrapper migration
  was fixed by `ebfb2f8be` (calls inside `if` conditions now record their mono
  demand edge).
- **Conditions**: (1) all 13 removed entries carried
  `deprecated_since = "0.1.0"` / `remove_in = "1.0.0"` from the
  2026-10-27 Phase E entry, with live W0009 coverage and the
  `deprecated_io_free_function` diagnostic fixture; (2) the before/after rows in
  [migration-guidance.md](migration-guidance.md) name the replacement and the
  deleted `buf_*` layout behavior; (3) this entry. The removed names now fail
  resolution with **E0100** and a `resolver::removed_names` hint, replacing the
  W0009 warning.
- **Removed (13)**: `reader_from_bytes`, `buf_reader_new`, `buffered_reader`,
  `buf_reader_read_line`, `buf_writer_new`, `buffered_writer`,
  `buf_writer_write_str`, `buf_writer_flush`, `writer_new`, `write_all`,
  `write_string`, `writer_flush`, and `writer_to_bytes` (manifest entries and
  `std/io/mod.ark` declarations).
- **Reason**: the generic `BufReader<R: Read>` / `BufWriter<W: Write>`
  adapters and the `Reader` / `Writer` / `Io` methods fully replace these
  wrappers. Keeping the legacy `[cursor, ...]` / `[fd_tag, ...]` layouts would
  keep a second public representation of the same handles alive after its
  consumers migrated.
- **Replacement**: `Reader::from_bytes`, `BufReader::new` / `br.read_line()`,
  `BufWriter::new` / `bw.write_string(s)` / `bw.flush()`, `Writer::new` /
  `w.write(data)` / `w.write_string(s)` / `w.flush()` / `w.into_bytes()`, and
  `Io::write_all`.
- **Retained (2)**: `writer_write` and `writer_write_str` stay `deprecated`
  under the same window. Exact blocker: `stdout()` and `stderr()` return tagged
  `Vec<i32>` handles, not `Writer` values, and `std::host::stdio` does not yet
  expose typed handles with `Write` implementations, so there is no receiver to
  migrate these two calls to (ADR-085 D9). Deletion condition: those typed
  handles land.
- **Scorecard**: deprecated std::io compat frees 15 → **2**; `pub fn`
  declarations under `std/**/*.ark` 1279 → **1266**; manifest public entries
  729 → **716**; eradication scope 1104 → **1091**; `must delete` 83 → **70**
  (all 13 removed entries were already in that class). `std/manifest.toml` and
  the generated docs are the current source of truth.
- **Evidence**: stage-2 rebuild from `ebfb2f8be`; the 25 migrated `stdlib_io`
  / `stdlib_io_rw` / `stdlib_trait` run fixtures compile, validate, and run on
  both `wasm32-gc` (memory32, WASI P2 component) and `--target wasm32` (core),
  with identical stdout and matching `.expected` goldens (25/25). Full
  `verify fixtures`: **1837/1837** compile+validate PASS, 0 failed, 105 runtime
  smoke, 1 `test:` fixture PASS. `selfhost diag-parity` PASS with the updated
  W0009 golden and the new E0100 golden
  (`tests/fixtures/diagnostics/removed_io_compat_free_function.ark`).
  `check-compat-orphans` clean, `check-stdlib-eradication`
  717/717 manifest functions + 375/375 unregistered declarations classified,
  `docs check` **6/6**, `verify lane` PASS, and the ADR-029 fixpoint reached
  (`sha256(arukellt-s2.wasm) == sha256(arukellt-s3.wasm)` =
  `27fe9ef5c9e55ea1edfed3b2422159353ea3f7c112c26b51e3a95ced36521474`).

## 2026-10-27 — std::io trait cutover (ADR-085 Phase E/F, #693)

- **Basis**: [ADR-085](../adr/ADR-085-io-trait-unification.md) D1–D10,
  [ADR-046](../adr/ADR-046-free-function-eradication.md) D3/D4, and the
  [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) 0.x removal exception.
  The canonical surface is `Reader` / `Writer`, `Read` / `Write` / `BufRead` /
  `Seek`, the `Io` algorithms, the generic `BufReader<R>` / `BufWriter<W>`
  adapters, and `std::host::fs::File`.
- **Conditions**: (1) W0009 is live for the retained compatibility family in
  `src/compiler/lint/deprecated_table.ark` with unit coverage and a live diag
  fixture; (2) [migration-guidance.md](migration-guidance.md) records every
  before/after and the behavior differences; (3) this entry.
- **Converted to `deprecated` (15)** with
  `deprecated_since = "0.1.0"` and `remove_in = "1.0.0"`:
  `reader_from_bytes` → `Reader::from_bytes`; `buf_reader_new` /
  `buffered_reader` → `BufReader::new`; `buf_reader_read_line` →
  `BufReader::read_line`; `buf_writer_new` / `buffered_writer` →
  `BufWriter::new`; `buf_writer_write_str` → `BufWriter::write_string`;
  `buf_writer_flush` → `BufWriter::flush`; `writer_new` → `Writer::new`;
  `writer_write` → `w.write`; `writer_write_str` / `write_string` →
  `w.write_string`; `write_all` → `Io::write_all`; `writer_flush` →
  `w.flush`; `writer_to_bytes` → `w.into_bytes`.
  Two reasons keep this window open instead of deleting the family now:
  `reader_from_bytes` / `buf_*` retain the legacy `Vec<i32>` layout until the
  generic-adapter dispatch residual (#693) allows their fixtures to move, and
  `writer_*` keeps the tagged `stdout()` / `stderr()` handles writable until
  `std::host::stdio` exposes typed `Write` handles.
- **Removed (18)** with E0100 replacement hints in
  `src/compiler/resolver/removed_names.ark`:
  `reader_eof`, `reader_read`, `reader_read_exact`, `reader_read_all`,
  `reader_read_line`, `copy_bytes`, `copy`, `copy_trait`, `new_memory_buffer`,
  `write_bytes`, `read_bytes`, `seek_to`, `buffer_position`, `buffer_len`,
  `fill_buffer`, `file_from_fd`, `file_read`, `file_write` (the last three were
  the experimental fd lane; ADR-014 allowed direct retirement). The 17
  manifest entries were removed and `copy_trait` was source-only.
- **Guidance fix**: `use std::io` no longer emits the retired
  "moved to `std::host::stdio`" W0009; `std::io` is the canonical abstraction
  module (ADR-085 D1). `docs/compiler/error-codes.md` E0104 now uses the
  `std::time` → `std::host::clock` example.
- **Scorecard**: eradication scope 1226 → **1208** (manifest public entries
  813 → **796**, unregistered std declarations 413 → **412**); `must delete`
  119 → **134**, `associated-or-method` 1107 → **1074**. Stability:
  `stable` 315 → **287**, `experimental` 318 → **314**, `deprecated`
  97 → **112**.
- **Evidence**: W0009
  `tests/fixtures/diagnostics/deprecated_io_free_function.ark`, E0100
  `tests/fixtures/diagnostics/removed_io_free_function.ark`, and the migrated
  `stdlib_io*` / `stdlib_trait` / `question_mark` fixtures compiled and run on
  both `wasm32-gc` and `wasm32`. `python3 scripts/manager.py verify fixtures`
  reports 1835/1835 compile+validate, 0 failed, 105 runtime smoke; the 68
  changed io fixtures additionally produced identical `wasm32-gc` component and
  `wasm32` core output.

## 2026-10-26 — 0.x deprecation exception (ADR-014 addendum)

- **Policy**: [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) を採択した。
  ADR-014 は維持し、0.x 期に限り `stable` の削除窓を明示的例外で短縮する。
- **Scope**: 0.x 期の `stable` 公開 stdlib エントリのみ。language spec、CLI、ABI、
  target、compiler phase、stdlib 以外の API には適用しない。
- **Conditions**: 削除 wave ごとに (1) W0009 が対象 API に対して実装・発火する、
  (2) `migration-guidance.md` に before/after と到達可能な replacement がある、
  (3) この changelog に対象シンボル・削除理由・replacement を記録する。
- **`remove_in`**: 0.x 例外の下では上限として扱う。`remove_in = "1.0.0"` の
  `stable` エントリを 0.x の minor で削除してよいが、`remove_in` より後には削除しない。
- **1.0.0**: 今は切らない。release-blocking な 3 項目（fixture harness、selfhost CLI
  parity、selfhost diag parity）の再証跡が揃うまでタグを切らない。1.0.0 以降は
  ADR-014 の通常方針に戻り、この例外は失効する。
- **Immediate wave**: `experimental` の `must delete` 2 件、`provisional` の
  `must delete` 21 件（個別判断）、`associated-or-method` 1129 件の replacement 追加。
- **Not in this wave**: `stable` の `must delete` 64 件。特に prelude monomorphic
  seq/combinator の 33 件は W0009 実装（別 lane）まで削除しない。314 件の `stable`
  free→method 置換は replacement の追加として進める。

## 2026-10-26 — stable 64 + seq 33 wave（ADR-087）

- **Basis**: [ADR-087](../adr/ADR-087-0x-deprecation-exception.md) D2（0.x 期の
  `stable` 削除例外）。条件 1 の W0009 は全対象シンボルの bare 名に対して
  `src/compiler/lint/deprecated_table.ark` に実装し、単体テストで 61 名を
  カバーする。条件 2 は [migration-guidance.md](migration-guidance.md)、
  条件 3 は本 entry。`remove_in = "1.0.0"` は上限であり、削除は Wave 2。
- **Converted to `deprecated` (60)**: `stability = "deprecated"`、
  `deprecated_by`、`deprecated_since = "0.1.0"`、`remove_in = "1.0.0"` を
  `std/manifest.toml` に設定した。理由は ADR-046 の free-function 根絶と
  ADR-044 の trait / method 正規化。replacement は次の各 family のとおり。
  - **Numeric conversions (16)**: `u8_to_i32`, `u16_to_i32`,
    `i16_to_i32`, `i32_to_u8`, `i32_to_u16`, `i32_to_i8`, `i32_to_i16`,
    `u32_to_u64`, `u64_to_u32`, `i32_to_i64`, `i64_to_i32`, `f32_to_f64`,
    `f64_to_f32`, `i32_to_u32`, `f64_bits_lo`, `f64_bits_hi` →
    `u8/i8/u16/i16::to_i32()`、`i32::to_u8_wrapping()` ほか narrow methods、
    `u32::to_u64()`、`u64::to_u32_wrapping()`、`i32::into()`、
    `i64::to_i32_checked()`、`f32::to_f64()`、`f64::to_f32()`、
    `f64::to_bits_lo()` / `to_bits_hi()`。新しい scalar conversion method は
    `std/core/convert.ark` に追加した。`i8_to_i32` は個別 deferral: 現行 GC
    emitter では method frame からの sign-extension が trap するため stable の
    まま保持し、`i8::to_i32` を追加しない。`i32_to_u16` / `i32_to_i16` /
    `u64_to_u32` の method は free と同じ deferred intrinsic body を共有して
    おり、Wave 2 まで runtime trap が残る。
  - **Vec intrinsic-compat (9)**: `push`, `pop`, `len`, `get`, `set`,
    `get_unchecked` → `Vec<T>` methods。`as_slice`, `slice_len`, `slice_get`
    → `v.as_slice(s, e)`, `view.len()`, `view.get(i)`。
  - **Assertion frees (4)**: `assert_eq` → `std::test::assert_eq_i32`、
    `assert_ne` → `assert_ne_i32`、`assert_eq_i64` → `assert_eq_i64`、
    `assert_eq_str` → `assert_eq_string`。
  - **Seq/combinator frees (31)**: `sort_i32/i64/f64/String` →
    `v.sort_by(cmp)`; `map_i32_i32`, `map_i64_i64`, `map_f64_f64`,
    `map_String_String` → `v.iter().map(f).collect()`; `filter_i64`,
    `filter_f64`, `filter_String` → `v.iter().filter(f).collect()`;
    `fold_i32_i32`, `fold_i64_i64`, `fold_f64_f64` →
    `v.iter().fold(init, f)`; `any_i32/i64/f64/String` → `v.iter().any(f)`;
    `find_i32/i64/f64/String` → `v.iter().find(f)`; `contains_i32`,
    `contains_String` → `v.iter().contains(x)`; `reverse_i32` →
    `v.iter().rev().collect()`; `reverse_String` → 同形（String rev lane は
    planned）; `remove_i32` → `v.drain(i, i + 1)`; `sum_i32` →
    `v.iter().sum()`; `product_i32` → `v.iter().fold(1, mul)`;
    `map_option_i32_i32` → `Option::map`（`std::core::container`）;
    `contains_string` → `s.contains(sub)`。`filter_i32` は provisional の
    個別判断として据え置く。i64/f64/String の consumer lane は planned で
    あり、Wave 2 前に到達可能化を完了させる。
- **Removed (3)**: `clear`, `map_option_String_String`,
  `map_result_i32_i32` は宣言も resolver 登録も持たない stale な builtin
  entry だったため、deprecated 化せず削除した。`clear` は `v.clear()`、
  残りは `match` / `std::core::container` の `Option::map` が replacement。
  E0100 の removed-name hint を `resolver::removed_names` に追加した。
- **Scorecard**: eradication scope 1246 → **1243**（`must delete`
  122 → 119、`associated-or-method` 1124 のまま）。stability は
  `stable` 378 → **315**、`deprecated` 37 → **97**、manifest 公開 entry
  815 → **812**。`prelude_wrapper` entry は 96 のまま（deprecated 化のみで、物理削除は Wave 2）。
- **Evidence**: W0009 の live diag fixture を wasm32-gc / wasm32 の両 target
  で追加し、replacement の run fixture を `scalar/`, `collections/`,
  `iterator/`, `stdlib_test/` に追加した。`check-cross-backend-parity.py` で
  両 target の compile + execute を確認する。
- **Wave 2**: 60 件の物理削除は W0009 window の後。`i8_to_i32` の deferral と
  3 件の deferred-body trap も Wave 2 の前提。vec intrinsic-compat は
  compiler / std 内の約 9k 呼び出しの移行、i64/f64/String consumer lane の
  実装、call-site migration を前提とする。

## 2026-10-27 — HashMap builtin retirement（#917）

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) の free-function
  根絶と #917 の実行 wave。対象は `provisional` の monomorphic `HashMap_*` builtin 20 件で
  あり、`stable` ではないため ADR-087 の deprecation window は適用されない。移行は
  W0009 ではなく、未解決名に対する **E0100** の removed-name hint で案内する。
- **Removed (20 manifest builtins)**: `HashMap_i32_i32_{new,insert,get,contains_key,len}`、
  `HashMap_new_String_i32` / `HashMap_String_i32_{insert,get,contains_key,len}`、
  `HashMap_new_i32_String` / `HashMap_i32_String_{insert,get,contains_key,len}`、
  `HashMap_new_String_String` / `HashMap_String_String_{insert,get,contains_key,len}`。
  manifest entry だけでなく、`resolver::builtins_hash` / `typechecker::builtins_hash` /
  `mir::lower::call_type_hash` / `wasm::hash_aliases` / `native_c::core_call` の lowering
  登録と、`std/collections/hash_map.ark` の i32→i32 adapter 10 件を同時に削除した。
- **Replacement**: `impl HashMap<K,V>` の `new` / `insert` / `get` / `contains_key` /
  `remove` / `len` / `is_empty` / `keys` / `values` / `entries` のみが公開綴りである。
  `HashMap<i32, i32>` は `std::collections::hash` の flat store 実装へ直接 lower し、
  他の pair は `hash_map::` typed adapter を維持する。旧 spelling は
  `src/compiler/resolver/removed_names.ark` の `HashMap_` prefix hint で
  `HashMap::new<K, V>()` と receiver methods を案内する。
- **Evidence**: `hashmap_basic`, `hashmap_insert_return`, 6 pair family fixture、
  `hashmap_fn_growth`, `typed_i32_i32_remove`, `collections/hashmap_entries_values`、
  `collections/hashmap_*_method` を wasm32-gc / `--target wasm32` の両方で
  compile + execute し、stdout を比較した。t3 compile fixtures、native-cpp scenario、
  benchmark も method 綴りへ移行した。
- **Scorecard**: eradication scope 1226 → **1196**（`must delete` 119 → 99、
  `associated-or-method` 1107 → 1097）。manifest 公開 entry 813 → **793**、
  `pub fn` 宣言 1397 → **1387**、unregistered 413 → **403**。
  `prelude_wrapper` 96 は不変。`HashMap` methods の `keys` / `values` / `entries` は
  引き続き typechecker が直接型付けする。

## 2026-10-27 — WIT builder wave（#917、wave/917-1106 の記録復元）

- **Note**: この節は wave/917-1106 で追加された changelog 行を、
  #693 Phase E/F のマージで失われた後に復元したものである。コード側の
  `std::wit::world` と `WitParam::new` は既に method / associated 形で
  land している。
- **対象**: `std::wit::world` の builder free 9 件 (`world_new`,
  `world_import`, `world_export`, `world_name`, `world_exports_len`,
  `interface_new`, `interface_add_func`, `wit_func_new`, `wit_param_new`)
  と、その forwarder だった `std::wit::types::wit_param`。
- **変更**: `World::new` / `Interface::new` / `WitFunc::new` /
  `WitParam::new`、`w.add_import(iface)` / `w.add_export(iface)` /
  `w.name()` / `w.exports_len()` / `iface.add_func(func)`。
- **Evidence**: `stdlib_wit/world_build.ark` と
  `stdlib_wit/wit_print.ark` を method surface へ移行し、
  `stdlib_wit/wit_methods_wasm32.ark` が linear target の runtime を担う。

## 2026-10-27 — WIT type / text / printer method wave（#917）

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) の
  free-function 根絶と [ADR-044](../adr/ADR-044-trait-method-syntax-adopted.md)
  の method-first surface。対象は `std::wit` の非 AST 面 **33 記号**。
  16 件は `experimental` の manifest entry なので ADR-014 D4 により
  deprecation window なしで直接削除した。残り 17 件は未登録 source 宣言で
  W0009 の対象外。全 call が module-qualified だったため、E0100 は
  `resolver::removed_names::removed_module_member_hint` の `wit` /
  `types` / `scan` / `names` / `printer` alias にだけ追加した。
- **Replacement (types 20)**: `WitType::Bool` … `WitType::String` の enum
  variant、`WitType::list(inner)` / `WitType::option(inner)` の associated
  constructor、`ty.id()` / `ty.name()` / `ty.wit_text()` の receiver
  method、`WitType::from_id(id)` / `WitType::name_from_ark(name)` の
  associated function。`std::wit` の 16 個の forwarding wrapper も同じ
  wave で削除し、`std::wit` は module doc だけを持つルートになった。
- **Replacement (scan 8)**: `WitText` namespace type の
  `find_char` / `trim` / `strip_version` / `before_colon` / `split_param` /
  `split_csv_items` / `line_start` / `line_end`。`std::wit::scan` は
  `std::wit::types` に依存しないまま（bootstrap の module loading 制約）。
- **Replacement (names 4)**: `WitName` namespace type の
  `to_kebab` / `to_snake` / `to_pascal` / `capitalize`。
- **Replacement (printer 1)**: `print_wit(w)` → `w.to_wit()`。private の
  `wit_type_print` は `WitType::wit_text` と重複していたため削除し、
  `wit_func_print` / `wit_iface_print` は `WitFunc::to_wit` /
  `Interface::to_wit` へ移した。
- **Proof**: `WitType` の recursive `List` / `Option` payload bind を含む
  method を wasm32-gc で compile し、`wasm-tools validate
  --features gc,function-references,memory64` と公式 WASI P2 WIT での
  component packaging 後の実行で linear と同一 stdout を確認した。
  `std::wit::ast` の deferral は `WitNode` 固有で、`WitType` には及ばない。
- **Scorecard**: eradication scope 1168 → **1135**（`associated-or-method`
  1054 → **1021**、`must delete` 114 と `intrinsic-only` 1 は不変）。
  manifest 公開 entry 776 → **760**、`pub fn` 宣言 1359 → **1310**、
  unregistered 392 → **375**。`prelude_wrapper` 96 は不変。
- **Evidence**: `stdlib_wit/wit_basic.expected` と `wit_types.expected` は
  wasm32-gc component と `--target wasm32` の両方で一致。
  `world_methods_wasm32` は `wit_methods_wasm32` に改名し、types / scan /
  names / printer の method surface を linear target で実行する。
  全 8 個の `stdlib_wit` fixture は wasm32-gc で compile + validate 済み。
- **Residual**: `std::wit::ast` の 23 宣言、`parser` の 3 free
  (`parse_full` / `parse_full_import_text` / `parse_wit`)、
  `std::component` と `std::component::handle` の family は #917 に残る。
