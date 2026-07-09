---
Status: open
Created: 2026-07-15
Updated: 2026-07-10
Track: compiler-internal
Depends on: 724
Related: ADR-040, #707, #716, #690
Orchestration class: implementation-ready
Blocks v4 exit: True
---
# T3 WASM validation failures: validate-fail 修正（GC ref 型推論バグ）

## Summary

T3 WASM validation で `validate-fail` が残っており、`verify quick` が block されている。
pre-commit hook が `verify quick` を実行するため、現状コミットに `--no-verify` が必要な状態。
残失敗の主因は GC ref 型推論の不整合（ネストした enum/option/result/tuple のペイロード型名喪失）。

### 現在のベースライン（2026-07-10 再測定 #4）

| 状態 | 件数 |
|------|------|
| pass | 401 |
| **validate-fail** | **20** |
| compile-fail | 1 |
| skip | 22 |
| **total** | **444** |

> 進捗メモ（並列 wave/726-{1..4} → master マージ, 2026-07-10）:
>
> - 397/24 → **401/20**（+4 pass, -4 validate-fail）
> - **新規 PASS**: `host/sockets/connect_read_write.ark`（lane 4 Vec/Result payload）、
>   `structs/struct_in_vec.ark` / `stdlib_hashmap/hashmap_typed_remove_extend.ark`（lane 1）、
>   `generics_v1/generic_method_call.ark`（lane 3 mono method return）
> - **回帰修正**: `Vec<Option<T>>` を structref-vec と誤判定していた問題を
>   `gc_vec_elem_name_is_enum_like` で除外（`nested_option` / `no_nested_generics`）
> - lane 2（enum variant_slot）は回帰なしでマージ（pass 数は据え置き）
> - array-subtype は 0 件（タスク6 完了）

### エラー型別内訳（2026-07-10 #4）

| エラー型 | 件数 |
|----------|------|
| ref-vs-ref (expected (ref null $type), found (ref null $type)) | 16 |
| ref-vs-i32 (expected (ref null $type), found i32) | 0 |
| i32-vs-ref (expected i32, found (ref null $type)) | 4 |
| empty-stack | 0 |
| array-subtype (expected subtype of arrayref) | 0 |
| **合計 validate-fail** | **20** |

#### 個別 fixture（2026-07-10 #4）

- **ref-vs-ref (16)**: generics_v1/trait_dispatch_stdlib,
  iterator/custom_iterator,
  stdlib_io/fs_read_error, stdlib_io/fs_read_write, stdlib_json/json_perf_decode,
  stdlib_trait/debug_trait, stdlib_trait/debug_vec,
  stdlib_trait/buf_read, stdlib_trait/display_trait_vec,
  stdlib_trait/io_backward_compat, stdlib_trait/io_copy, stdlib_trait/read_write,
  stdlib_trait/seek, stdlib_csv/csv_perf,
  stdlib_toml/toml_full_inline_dotted, stdlib_toml/toml_full_table_header
- **ref-vs-i32 (0)**: （タスク2 で解消）
- **i32-vs-ref (4)**: generics_v1/nested_generic_call,
  stdlib_trait/iterator_adapters, stdlib_wit/wit_names, trait/builtin_method
- **array-subtype (0)**: （タスク6 で解消 — `connect_read_write`）
- **compile-fail (1)**: stdlib_wit/wit_ast_parse

### 旧ベースライン（2026-07-09 再測定 #3）

| 状態 | 件数 |
|------|------|
| pass | 397 |
| **validate-fail** | **24** |
| compile-fail | 1 |
| skip | 22 |
| **total** | **444** |

> 進捗メモ（`fix/726-task1-ref-vs-ref` 作業中, 2026-07-09）:
>
> - 393/28 → 396/25 → **397/24**（+4 pass, -4 validate-fail）
> - **新規 PASS**: `integration/word_counter.ark`, `stdlib_string/string_split.ark`,
>   `from_trait/from_auto_convert.ark`, `enums/nested_enum.ark`
> - enum payload variant: レイアウト型名 + `variant_slot` 伝播で
>   `Result::Err(AppError)` / `Outer::Wrap(Inner)` の ref-vs-ref を解消
> - 既存 PASS 維持: `generics/two_params`, `stdlib_option_result/option_map`,
>   `stdlib_io/clock_random`, `stdlib_trait/ord_sort_by`

### エラー型別内訳（2026-07-09 #3）

| エラー型 | 件数 |
|----------|------|
| ref-vs-ref (expected (ref null $type), found (ref null $type)) | 18 |
| ref-vs-i32 (expected (ref null $type), found i32) | 0 |
| i32-vs-ref (expected i32, found (ref null $type)) | 5 |
| empty-stack | 0 |
| array-subtype (expected subtype of arrayref) | 1 |
| **合計 validate-fail** | **24** |

#### 個別 fixture（2026-07-09 #3）

- **ref-vs-ref (18)**: generics_v1/trait_dispatch_stdlib,
  iterator/custom_iterator, stdlib_hashmap/hashmap_typed_remove_extend,
  stdlib_io/fs_read_error, stdlib_io/fs_read_write, stdlib_json/json_perf_decode,
  stdlib_trait/debug_trait, stdlib_trait/debug_vec,
  stdlib_trait/buf_read, stdlib_trait/display_trait_vec,
  stdlib_trait/io_backward_compat, stdlib_trait/io_copy, stdlib_trait/read_write,
  stdlib_trait/seek, structs/struct_in_vec, stdlib_csv/csv_perf,
  stdlib_toml/toml_full_inline_dotted, stdlib_toml/toml_full_table_header
- **ref-vs-i32 (0)**: （タスク2 で解消）
- **i32-vs-ref (5)**: generics_v1/generic_method_call, generics_v1/nested_generic_call,
  stdlib_trait/iterator_adapters, stdlib_wit/wit_names, trait/builtin_method
- **array-subtype (1)**: host/sockets/connect_read_write
- **compile-fail (1)**: stdlib_wit/wit_ast_parse

### 旧ベースライン（2026-07-09 再測定 #2）

| 状態 | 件数 |
|------|------|
| pass | 396 |
| **validate-fail** | **25** |
| compile-fail | 1 |
| skip | 22 |
| **total** | **444** |

> 進捗メモ（`fix/726-task1-ref-vs-ref` 作業中, 2026-07-09）:
>
> - 393/28 → **396/25**（+3 pass, -3 validate-fail）
> - **新規 PASS**: `integration/word_counter.ark`, `stdlib_string/string_split.ark`
>   （`vec:string` 型名 + `VT_I32` 誤宣言の修正、`call_type_fallback` の GC ref vt 保持）
> - 既存 PASS 維持: `generics/two_params`, `stdlib_option_result/option_map`,
>   `stdlib_io/clock_random`, `stdlib_trait/ord_sort_by`
> - `from_trait/from_auto_convert.ark` は **まだ fail**（match 枝の variant cast 誤り）

### エラー型別内訳（2026-07-09 #2）

| エラー型 | 件数 |
|----------|------|
| ref-vs-ref (expected (ref null $type), found (ref null $type)) | 20 |
| ref-vs-i32 (expected (ref null $type), found i32) | 0 |
| i32-vs-ref (expected i32, found (ref null $type)) | 5 |
| empty-stack | 0 |
| array-subtype (expected subtype of arrayref) | 1 |
| **合計 validate-fail** | **25** |

#### 個別 fixture（2026-07-09 #2）

- **ref-vs-ref (20)**: from_trait/from_auto_convert, generics_v1/trait_dispatch_stdlib,
  iterator/custom_iterator, stdlib_hashmap/hashmap_typed_remove_extend,
  stdlib_io/fs_read_error, stdlib_io/fs_read_write, stdlib_json/json_perf_decode,
  stdlib_trait/debug_trait, stdlib_trait/debug_vec,
  stdlib_trait/buf_read, stdlib_trait/display_trait_vec,
  stdlib_trait/io_backward_compat, stdlib_trait/io_copy, stdlib_trait/read_write,
  stdlib_trait/seek, structs/struct_in_vec, stdlib_csv/csv_perf,
  stdlib_toml/toml_full_inline_dotted, stdlib_toml/toml_full_table_header
- **ref-vs-i32 (0)**: （タスク2 で解消）
- **i32-vs-ref (5)**: generics_v1/generic_method_call, generics_v1/nested_generic_call,
  stdlib_trait/iterator_adapters, stdlib_wit/wit_names, trait/builtin_method
- **array-subtype (1)**: host/sockets/connect_read_write
- **compile-fail (1)**: stdlib_wit/wit_ast_parse

### 旧ベースライン（2026-07-09 初回）

| 状態 | 件数 |
|------|------|
| pass | 393 |
| **validate-fail** | **28** |
| compile-fail | 1 |
| skip | 22 |
| **total** | **444** |

> 進捗メモ（master `check-t3-wasm-validate.py`, 2026-07-09）:
>
> - 旧ベースライン: 389 pass / 32 validate-fail → 392/29 → **393/28**
> - 解消済み例: `t3/string_gc.ark`, `stdlib_io/clock_random.ark`（`59865e134`）、
>   question_mark 系 T1（#690）、**empty-stack クラス全体**（タスク5）、
>   **`stdlib_trait/ord_sort_by.ark`**（タスク2 個別経路）
> - タスク2 完了: **ref-vs-i32 = 0**。残4件はすべて **ref-vs-ref** へ移行
>   （`debug_vec`, `toml_full_*`）または PASS（`ord_sort_by`）
> - `from_trait/from_auto_convert.ark` は **まだ fail**（match 枝 variant cast）

> ⚠️ **重要な罠**: セルフホスト再構築時、`.build/selfhost/flat-overlay-cache.json`
> （ディスクキャッシュ）が `src/compiler` の **古いフラットオーバレイ** を返し、
> ソース編集が s2.wasm に反映されない。コンパイラ変更を試すたびに以下を削除してから再構築すること：
>
> ```bash
> rm -f .build/selfhost/flat-overlay-cache.json
> rm -rf .build/selfhost/flat-src .build/selfhost/arukellt-s2.wasm \
>        .build/selfhost/s2-hash.txt .build/selfhost/arukellt-s3.wasm
> ARUKELLT_FIXPOINT_NO_CACHE=1 python3 scripts/manager.py selfhost fixpoint --build
> /bin/cp -f .build/selfhost/arukellt-s2.wasm .build/selfhost/arukellt-s2-runtime.wasm
> ```

### 2026-07-08 調査で判明した根本ブロッカー

`emit_gc_local_val_type`（`src/compiler/wasm/code_locals.ark`）の local 宣言型と、
`emit_gc_ref_cast_to_dest`（`inst_locals.ark`）が `local.set` 時に出力する `ref.cast`
の型が一致しないことが直接原因。しかし単純な「宣言型 = ref.cast 型」の統一は
**不安全**であることが分かった：

1. `ref.cast` は `SelfEmitCtx_wasm_ref_type_idx_for_local` の戻りが -1 の場合、
   `code_ref_locals::infer_ref_local_gc_type`（ブロック走査）へフォールバックする。
2. このブロック走査は **宣言出力時（body 出力前）と body 出力時で結果が異なる**
   （state-dependent）。そのため `emit_gc_local_val_type` から同じ走査を呼ぶと、
   通っていた fixture（例: `stdlib_option_result/option_map`）が壊れる。
3. 実際の ref-vs-ref 不一致は、多くの場合 `ref.cast` を経由せずに**直接格納**される
   値の型と宣言型が食い違っている local にある。よって宣言型を ref.cast 型に合わせても
   解決しない。

**結論**: 真の修正は emitter 側のパッチではなく、MIR 段階での local 型一貫性
（body から local 型を決定する post-pass、または local 宣言を body 出力後に
行う並べ替え）が必要。emitter 側で局所的に塞ぐのは不良Regressionのリスクが高い。

### 評価済みだが不発だったアプローチ（2026-07-08）

- `call_type_fallback.ark` / `return_typeinfo.ark` の継承 WIP 編集:
  fresh s2 で評価したが T3 件数に変化なし（inert）。stale s2 の罠に隠れていた。
- `emit_gc_local_val_type` で `SelfEmitCtx_wasm_ref_type_idx_for_local` を
  全域 primary にする: 7 fixture が regression（配列/option/struct）。
- `should_gc_ref_cast_to_dest` を宣言時に呼ぶ（scan フォールバック込み）:
  `option_map` が regression（走査の state-dependence のため）。

## Acceptance Criteria

- [ ] タスク1: ref-vs-ref（16件残）解決 — MIR local 型一貫性（emitter 局所パッチは非推奨）
- [x] タスク2: ref-vs-i32 — 解消済み（7→0。残件はタスク1へ移行 or PASS）
- [ ] タスク3: i32-vs-ref（4件残）解決 — `generic_method_call` は PASS（mono return suffix）。残り4件
- [x] タスク4: i32-vs-f64 — 現行 clean ベースラインでは 0 件（解消済み）
- [x] タスク5: empty-stack — 解消済み（`MIR_CALL_INDIRECT` を store policy consumer に追加）
- [x] タスク6: array-subtype — 解消済み（`connect_read_write` PASS、Vec/Result payload typing）
- [ ] `verify quick` が validate-fail=0 で pass する
- [ ] pre-commit hook が `--no-verify` なしで通過する

## エラーパターン別分類（現行 20 件）

| エラー型 | 件数 | エラーメッセージ |
|----------|------|-----------------|
| **ref-vs-ref** | 16 | `expected (ref null $type), found (ref null $type)` |
| **ref-vs-i32** | 0 | （タスク2で解消） |
| **i32-vs-ref** | 4 | `expected i32, found (ref null $type)` |
| **empty-stack** | 0 | （タスク5で解消） |
| **array-subtype** | 0 | （タスク6で解消） |

## タスク1: ref-vs-ref（20件）— 最大グループ

### エラー

`expected (ref null $type), found (ref null $type)` — 異なる2つの ref 型が混同

### 対象fixture

- `from_trait/from_auto_convert.ark`
- `generics_v1/trait_dispatch_stdlib.ark`
- `iterator/custom_iterator.ark`
- `stdlib_csv/csv_perf.ark`
- `stdlib_hashmap/hashmap_typed_remove_extend.ark`
- `stdlib_trait/buf_read.ark`
- `stdlib_trait/debug_trait.ark`
- `stdlib_trait/display_trait_vec.ark`
- `stdlib_trait/io_backward_compat.ark`
- `stdlib_trait/io_copy.ark`
- `stdlib_trait/read_write.ark`
- `stdlib_trait/seek.ark`
- `structs/struct_in_vec.ark`

### 根本原因

local 宣言時と ref.cast 時で異なる型推論パスを使用している：
- **local 宣言時** (`code_locals.ark`): `type_name` の文字列マッチ → `enum_open_type`（汎化型、例: type 14）
- **ref.cast 時** (`inst_locals.ark`): `variant_slot` の構造化ルックアップ → 特定の `enum variant type`（個別型、例: type 15+）

結果: local は `(ref null $type14)` と宣言されるが、`ref.cast` が `(ref null $type15)` を生成し、validator が型不一致を検出。

### 修正方針

`code_locals.ark` の local 型宣言で `variant_slot` ルックアップを優先的に使用し、ref.cast 時と同じ型推論パスに統一する。

### 修正対象ファイル

- `src/compiler/wasm/code_locals.ark` — local 型宣言に `variant_slot` ルックアップを追加
- `src/compiler/wasm/code_ref_locals_infer.ark` — 推論パスの統一
- `src/compiler/wasm/post_pass_type_propagate.ark` — `variant_slot` 伝播の確認

## タスク2: ref-vs-i32 — 解消済み

### 結果

**ref-vs-i32 = 0**。T3: **393 pass / 28 validate-fail**（`ord_sort_by` が PASS）。

### 対象fixture（旧）

- ~~`stdlib_io/fs_read_*` / `json_perf_decode`~~ — match 昇格後 **ref-vs-ref**
- ~~`stdlib_toml/toml_full_*`~~ — qualified enum param 昇格後 **ref-vs-ref**
- ~~`stdlib_trait/debug_vec`~~ — `format_debug__?t0` → `T::fmt_debug` 後 **ref-vs-ref**
- ~~`stdlib_trait/ord_sort_by`~~ — **PASS**

### 実施済み（2026-07-09）

1. match/try/HOF コンテナ VT 昇格（`mir_promote_gc_enum_container_vt`）
2. `params_fn.ark`: `parser::TomlValue` 等の qualified enum/struct を `VT_GC_REF` に
3. `call_text` / `call_types` / `call_mono*`: `__intrinsic_string_slice` +
   `format_debug__?t0` → `T::fmt_debug`（`call_rewrite` は format_debug のみ再解決。
   全域 `mono_by_type` は `modules/multi_import` を壊すため不可）
4. `call_indirect.ark`: unary は `identity` 優先、binary fold は WASI type index 1
   （`(i32,i32)->i32`）を固定。MirFunction 探索は `map_i32_i32` の誤マッチを起こす
5. `sections_types_gc_phase7.ark`: 未置換型変数 `T`/`t0` を structref 配列判定から除外
   （未マングル `min_by` シェルが `Vec<i32>` を structref 配列として読むのを防止）

## タスク3: i32-vs-ref（4件残）— i32 のべき local が ref に

### エラー

`expected i32, found (ref null $type)` — i32 のべき local が ref 型として宣言

### 対象fixture（残）

- `generics_v1/nested_generic_call.ark`
- `stdlib_trait/iterator_adapters.ark`
- `stdlib_wit/wit_names.ark`
- `trait/builtin_method.ark`

### 解消済み

- `generics_v1/generic_method_call.ark` — mono method return を mangled suffix
  （`Picker__echo__String`）から解決（lane 3）

### 根本原因

メソッド呼び出し結果（文字列や ref 型）の dest local が `VT_GC_REF` に正しく設定されていない。
CoreHIR に `__return` ann が無い generic method では return TypeInfo が空になり、
mono 実体化後も i32/ref が混線する。

### 修正方針

- `fn_index_method_return_type` / `entry_methods_mono` で mono suffix から concrete return を復元（一部完了）
- 残り4件は nested mono / trait method / iterator adapter の同様パスを拡張

### 修正対象ファイル

- `src/compiler/mir/lower/fn_index_method_return_type.ark`
- `src/compiler/mir/lower/entry_methods_mono.ark`
- `src/compiler/mir/lower/method_emit.ark`
- `src/compiler/wasm/sections_types_sigs_mono.ark`

## タスク4: i32-vs-f64 — 解消済み

現行 clean ベースラインでは 0 件。旧リストの `enums/tuple_variant.ark` /
`stdlib_io/f64_to_string.ark` は validate 通過済み。

## タスク5: empty-stack — 解消済み

### エラー（旧）

`expected (ref null $type) but nothing on stack`

### 対象fixture

- ~~`stdlib_io/clock_random.ark`~~ — 解消済み（`59865e134`）
- ~~`stdlib_trait/ord_sort_by.ark`~~ — empty-stack 解消後、タスク2 で **PASS**

### 根本原因

`MIR_CALL_INDIRECT` が store policy の consumer 扱いに入っておらず、直前の値が
`local.set` で消えて `call_indirect` 時にスタックが空になる。

### 修正内容

`src/compiler/wasm/inst_store_policy.ark` で `MIR_CALL_INDIRECT` を
`is_direct_consumer_op` / `is_load_followed_by_consumer` /
`should_skip_store_after_early_tee` の next2 判定に追加。

### 検証

- `ord_sort_by` の empty-stack は消滅
- T3 全体（タスク5時点）: 392 pass / 29 validate-fail
- 型解決を広げた `call_indirect` 実験は 261 pass まで悪化したため revert
- タスク2 後: **393 pass / 28 validate-fail**（`ord_sort_by` PASS）

## タスク6: array-subtype — 解消済み

### エラー

`expected subtype of arrayref, found (ref null (id 21))` — 配列型のサブタイプでない ref が配列操作に渡される

### 対象fixture

- `host/sockets/connect_read_write.ark` — **PASS**（lane 4）

### 根本原因 / 修正

`Result<Vec<i32>, _>` の Ok ペイロードが `enum_open` ref になり `array.len` が失敗。
vec:i32 mono key で正しい GC array ref を選ぶよう `call_rewrite_vec` /
`match_payload_bind_types` / `return_typeinfo_names` を修正。

## 修正の優先順位

| 優先度 | タスク | 件数 | 効果 |
|--------|--------|------|------|
| **高** | タスク1: ref-vs-ref | 16 | 16件解決で 20→4 |
| **済** | タスク2: ref-vs-i32 | 0 | 解消済み（393/28） |
| **中** | タスク3: i32-vs-ref | 4 | 4件解決で 20→16（generic_method_call 済） |
| **済** | タスク4: i32-vs-f64 | 0 | 解消済み |
| **済** | タスク5: empty-stack | 0 | store policy で解消 |
| **済** | タスク6: array-subtype | 0 | `connect_read_write` PASS |

タスク1 で16件、残タスク3 で4件。現行 T3: **401 pass / 20 validate-fail**。

## Notes

### 関連コミット

- `9c8a10817` — merge lane 3 i32-vs-ref / generic method mono return（401/20）
- `221e2a75f` — Option/Result/enum vec elems を non-structref に（nested_option 回帰修正）
- `00f18c24c` — Vec payload variant typing（`connect_read_write`）
- `e562bf840` — struct/container GC refs（`struct_in_vec` / hashmap extend）
- `59865e134` — string_gc concat + MIR_REF_CAST producer 型推論（clock_random / string_gc 解消）
- `aef1089ba` — `fix/verify-green` を master へマージ
- `fc0ee1525` — #690 From 変換 / #707 String clone（T3: 392/29）
- タスク5 — `inst_store_policy.ark` に `MIR_CALL_INDIRECT` consumer 追加（empty-stack 0）
- タスク2 部分 — `mir_promote_gc_enum_container_vt`（ref-vs-i32 7→4）
- タスク2 完了 — toml param / format_debug / call_indirect / type-param structref
  （ref-vs-i32 0、`ord_sort_by` PASS、393/28）

### 関連 Issue

- #724 — ADR-040 Phase 5-7 残作業（本 issue の上位）
- #725 — ADR-040 Phase 5e tracer removal（関連）
- #707 — trait self return type support
- #716 — GC intrinsic stub completion
- #690 — `?` / From 変換（question_mark fixture 側は解消）

### 実装メモ

- 並列レーン: `.worktrees/wave-726-{1..4}-*`（マージ後クリーンアップ可）
- selfhost rebuild 時は **必ず** `flat-overlay-cache.json` を削除してから測定する
- 修正ごとに T3 validate-fail 数の増減を確認（stale s2 に注意）
- emitter 局所パッチは regression リスク大 — MIR 段階の型一貫性を優先