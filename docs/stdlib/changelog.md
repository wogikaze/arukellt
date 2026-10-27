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

## 2026-10-27 — hash-family adapters removed; HashMap methods are the lowering target (#917)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2/D4 and
  the [#917](https://github.com/wogikaze/arukellt/issues/917) execution lane.
  All 101 symbols were unregistered top-level `pub fn` declarations, so no
  manifest deprecation window, ADR-087 exception, or W0009 row applied.
- **Lowering first**: the compiler now rewrites the `HashMap<K, V>` methods
  directly to the per-pair implementation functions
  (`hash_string::hashmap_str_get`, `hash_string::hashmap_i32_str_insert`,
  `hash_fn::hashmap_i32_fn_insert`, `hash_str_fn::hashmap_str_fn_get`, …).
  The old `hash_map::hashmap_*` adapters are gone; `insert` keeps the
  previous-value `Option<V>` contract. The implementation functions stay as
  module-private MIR targets, not public API.
- **Removed (101)**: the 50 `std::collections::hash_map` adapters, the 29
  `std::collections::hash_string` implementation frees, and the 11 + 11
  `std::collections::hash_fn` / `hash_str_fn` implementation frees. The two
  `hashmap_str_val_*` forwarders and the six duplicate
  `hash_map::hashset_str_*` frees were deleted with them (fixtures now use
  the `hash_set::hashset_str_*` facade).
- **Replacement**: `HashMap::new<K, V>()` plus `insert` / `get` /
  `contains_key` / `remove` / `len` / `is_empty` / `keys` / `values` /
  `entries` on the receiver. Qualified old calls fail with an **E0100**
  hint from `resolver::removed_names` (bare and module-qualified forms);
  every before/after row is in [migration-guidance.md](migration-guidance.md).
- **In-tree callers**: 11 fixtures migrated to the method surface. The
  fn-valued maps previously used `hash_fn::hashmap_i32_fn_set` /
  `hash_str_fn::hashmap_str_fn_set`; `m.insert(k, v)` has the same
  store-and-return-previous semantics.
- **Scorecard (regenerated)**: public surface 1047 → **946** symbols;
  `associated-or-method` 977 → **876**; unregistered declarations 375 →
  **274**; `pub fn` declarations 1222 → **1115**. Manifest public entries
  (672), `must delete` (70), `prelude_wrapper` (65), and `intrinsic-only`
  (1) are unchanged. The four hash adapter rules retired from
  `docs/data/stdlib-api-eradication.toml`; the `hash set monomorphic
  adapters` rule keeps 15 rows with the missing `HashSet<T>` surface as the
  recorded blocker. The committed scorecard was 13 symbols stale
  (1060/685 vs the real 1047/672) before this regeneration.
- **Evidence**: stage-2 rebuild; the 11 migrated fixtures run with identical
  stdout on `wasm32-gc` and linear `wasm32` (cross-backend parity 11/11);
  `verify fixtures` **1839/1839** compile+validate PASS; `selfhost
  diag-parity` **119/119** PASS including the new
  `diagnostics/removed_hashmap_adapter*.ark` goldens;
  `check-stdlib-eradication` and `docs check` clean after the
  `project-state.toml` fixture-registry sync (3260 → 3262).
- **Residual**: the 15 `hashset_i32_*` adapters remain blocked on a
  `HashSet<T>` nominal type (no receiver surface for a method rewrite; the
  `Vec<i32>`/`Vec<String>` facades are not types). Deleting them now would
  remove the only user spelling without a replacement.

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
