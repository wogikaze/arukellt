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
