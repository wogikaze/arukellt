---
Status: open
Created: 2026-07-15
Updated: 2026-07-09
ID: 726
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

### 現在のベースライン（2026-07-09 再測定）

> ⚠️ **重要な罠**: セルフホスト再構築時、`.build/selfhost/flat-overlay-cache.json`
> （ディスクキャッシュ）が `src/compiler` の **古いフラットオーバレイ** を返し、
> ソース編集が s2.wasm に反映されない。結果として T3 の失敗数が変化しないように
> 見える。コンパイラ変更を試すたびに以下を削除してから再構築すること：
>
> ```bash
> rm -f .build/selfhost/flat-overlay-cache.json
> rm -rf .build/selfhost/flat-src .build/selfhost/arukellt-s2.wasm \
>        .build/selfhost/s2-hash.txt .build/selfhost/arukellt-s3.wasm
> ARUKELLT_FIXPOINT_NO_CACHE=1 python3 scripts/manager.py selfhost fixpoint --build
> ```

| 状態 | 件数 |
|------|------|
| pass | 392 |
| **validate-fail** | **29** |
| compile-fail | 1 |
| skip | 22 |
| **total** | **444** |

> 進捗メモ（master `check-t3-wasm-validate.py`, 2026-07-09）:
>
> - 旧ベースライン: 389 pass / 32 validate-fail
> - 現在: **392 pass / 29 validate-fail**（件数は不変・クラス内訳は変化）
> - 解消済み例: `t3/string_gc.ark`, `stdlib_io/clock_random.ark`（`59865e134`）、
>   question_mark 系 T1（#690）、**empty-stack クラス全体**（タスク5）
> - タスク2 部分進捗: match/try/HOF の GC enum コンテナ VT 昇格で
>   `fs_read_error` / `fs_read_write` / `json_perf_decode` が **ref-vs-i32 → ref-vs-ref**
> - 残 ref-vs-i32（4）: `debug_vec`, `ord_sort_by`, `toml_full_*` — match 昇格以外の経路
> - `from_trait/from_auto_convert.ark` は **まだ fail**
> - `fix/726-t3-validate` の 393/28 は worktree WIP 測定であり、master には未マージ

### エラー型別内訳（master）

| エラー型 | 件数 |
|----------|------|
| ref-vs-ref (expected (ref null $type), found (ref null $type)) | 19 |
| ref-vs-i32 (expected (ref null $type), found i32) | 4 |
| i32-vs-ref (expected i32, found (ref null $type)) | 5 |
| empty-stack | 0 |
| array-subtype (expected subtype of arrayref) | 1 |
| **合計 validate-fail** | **29** |

#### 個別 fixture（master `check-t3-wasm-validate.py`）

- **ref-vs-ref (19)**: from_trait/from_auto_convert, generics/two_params,
  generics_v1/trait_dispatch_stdlib, integration/word_counter,
  iterator/custom_iterator, stdlib_hashmap/hashmap_typed_remove_extend,
  stdlib_io/fs_read_error, stdlib_io/fs_read_write, stdlib_json/json_perf_decode,
  stdlib_string/string_split, stdlib_trait/debug_trait, stdlib_trait/buf_read,
  stdlib_trait/display_trait_vec, stdlib_trait/io_backward_compat,
  stdlib_trait/io_copy, stdlib_trait/read_write, stdlib_trait/seek,
  structs/struct_in_vec, stdlib_csv/csv_perf
- **ref-vs-i32 (4)**: stdlib_trait/debug_vec, stdlib_trait/ord_sort_by,
  stdlib_toml/toml_full_inline_dotted, stdlib_toml/toml_full_table_header
- **i32-vs-ref (5)**: generics_v1/generic_method_call, generics_v1/nested_generic_call,
  stdlib_trait/iterator_adapters, stdlib_wit/wit_names, trait/builtin_method
- **array-subtype (1)**: host/sockets/connect_read_write
- **compile-fail (1)**: stdlib_wit/wit_ast_parse

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

- [ ] タスク1: ref-vs-ref（17件）解決 — MIR local 型一貫性（emitter 局所パッチは非推奨）
- [~] タスク2: ref-vs-i32 — 7→4（match/try/HOF コンテナ昇格）。残4は別経路
- [ ] タスク3: i32-vs-ref（5件）解決 — `method_emit.ark` の `mir_mark_call_result_type` 修正
- [x] タスク4: i32-vs-f64 — 現行 clean ベースラインでは 0 件（解消済み）
- [x] タスク5: empty-stack — 解消済み（`MIR_CALL_INDIRECT` を store policy consumer に追加）
- [ ] タスク6: array-subtype（1件）解決 — vec 型推論の個別修正
- [ ] `verify quick` が validate-fail=0 で pass する
- [ ] pre-commit hook が `--no-verify` なしで通過する

## エラーパターン別分類（現行 29 件）

| エラー型 | 件数 | エラーメッセージ |
|----------|------|-----------------|
| **ref-vs-ref** | 19 | `expected (ref null $type), found (ref null $type)` |
| **ref-vs-i32** | 4 | `expected (ref null $type), found i32` |
| **i32-vs-ref** | 5 | `expected i32, found (ref null $type)` |
| **empty-stack** | 0 | （タスク5で解消） |
| **array-subtype** | 1 | `expected subtype of arrayref, found (ref null (id 21))` |

## タスク1: ref-vs-ref（17件）— 最大グループ

### エラー

`expected (ref null $type), found (ref null $type)` — 異なる2つの ref 型が混同

### 対象fixture

- `from_trait/from_auto_convert.ark`
- `generics/two_params.ark`
- `generics_v1/nested_generic_call.ark`
- `generics_v1/trait_dispatch_stdlib.ark`
- `integration/word_counter.ark`
- `iterator/custom_iterator.ark`
- `stdlib_csv/csv_perf.ark`
- `stdlib_hashmap/hashmap_typed_remove_extend.ark`
- `stdlib_string/string_split.ark`
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

## タスク2: ref-vs-i32（残り4件）— 部分進捗

### エラー

`expected (ref null $type), found i32`

### 対象fixture

- ~~`stdlib_io/fs_read_error.ark`~~ — match 昇格後 **ref-vs-ref** へ移行
- ~~`stdlib_io/fs_read_write.ark`~~ — 同上
- ~~`stdlib_json/json_perf_decode.ark`~~ — 同上
- `stdlib_toml/toml_full_inline_dotted.ark` — 関数 param が i32 のまま call に渡る
- `stdlib_toml/toml_full_table_header.ark` — 同上
- `stdlib_trait/debug_vec.ark` — return 経路で i32（unreachable stub 後）
- `stdlib_trait/ord_sort_by.ark` — `call_indirect` 型解決（array_get i32 vs ref param）

### 実施済み（2026-07-09）

- `mir_promote_gc_enum_container_vt`（`call_type_fallback.ark`）
- match / CoreHIR match / HOF Option / try のコンテナ local 昇格
- `mark_vec_get_result_type`: Option コンテナに `set_ref_vt_for_type_name`
- `cmp` / `cmp__*` / `::cmp__*` の `enum:Ordering` 伝播拡張

### 残件の修正方針

- toml: ラッパ関数の **param VT** を GC ref にする（local 昇格だけでは不足）
- `debug_vec`: String return / intrinsic stub 経路
- `ord_sort_by`: `call_indirect` の type index 選択（タスク5 で revert した実験の再設計）

## タスク3: i32-vs-ref（5件）— i32 のべき local が ref に

### エラー

`expected i32, found (ref null $type)` — i32 のべき local が ref 型として宣言

### 対象fixture

- `generics_v1/generic_method_call.ark`
- `generics_v1/nested_generic_call.ark`
- `stdlib_trait/iterator_adapters.ark`
- `stdlib_wit/wit_names.ark`
- `trait/builtin_method.ark`

### 根本原因

メソッド呼び出し結果（文字列や ref 型）の dest local が `VT_GC_REF` に正しく設定されていない。`mir_mark_call_result_type` に `first_arg_idx`（レシーバ local）を渡していることが型推論を妨げている可能性。

### 修正方針

`method_emit.ark` の `mir_mark_call_result_type` 呼び出しで、`first_arg_idx` ではなく `dest` を渡す、または型推論ロジックを修正。

### 修正対象ファイル

- `src/compiler/mir/lower/method_emit.ark` — `mir_emit_method_call`
- `src/compiler/mir/lower/call_types.ark` — `mir_mark_call_result_type`

## タスク4: i32-vs-f64 — 解消済み

現行 clean ベースラインでは 0 件。旧リストの `enums/tuple_variant.ark` /
`stdlib_io/f64_to_string.ark` は validate 通過済み。

## タスク5: empty-stack — 解消済み

### エラー（旧）

`expected (ref null $type) but nothing on stack`

### 対象fixture

- ~~`stdlib_io/clock_random.ark`~~ — 解消済み（`59865e134`）
- ~~`stdlib_trait/ord_sort_by.ark`~~ — empty-stack 解消（2026-07-09）。残失敗は
  `expected (ref null $type), found i32`（タスク2 / ref-vs-i32）

### 根本原因

`MIR_CALL_INDIRECT` が store policy の consumer 扱いに入っておらず、直前の値が
`local.set` で消えて `call_indirect` 時にスタックが空になる。

### 修正内容

`src/compiler/wasm/inst_store_policy.ark` で `MIR_CALL_INDIRECT` を
`is_direct_consumer_op` / `is_load_followed_by_consumer` /
`should_skip_store_after_early_tee` の next2 判定に追加。

### 検証

- `ord_sort_by` の empty-stack は消滅
- T3 全体: **392 pass / 29 validate-fail**（回帰なし・pass 増なし）
- 型解決を広げた `call_indirect` 実験は 261 pass まで悪化したため revert

## タスク6: array-subtype（1件）— 配列サブタイプチェック失敗

### エラー

`expected subtype of arrayref, found (ref null (id 21))` — 配列型のサブタイプでない ref が配列操作に渡される

### 対象fixture

- `host/sockets/connect_read_write.ark`

### 根本原因

vec 型推論の誤り。id 21 の ref が配列のサブタイプでないのに配列操作に使用されている。

### 修正方針

vec 型推論で正しい配列型を返すように修正。

## 修正の優先順位

| 優先度 | タスク | 件数 | 効果 |
|--------|--------|------|------|
| **高** | タスク1: ref-vs-ref | 19 | 19件解決で 29→10 |
| **高** | タスク2: ref-vs-i32 | 4 | 残り4（別経路） |
| **中** | タスク3: i32-vs-ref | 5 | 5件解決で 6→1 |
| **済** | タスク4: i32-vs-f64 | 0 | 解消済み |
| **済** | タスク5: empty-stack | 0 | store policy で解消（残件はタスク2へ） |
| **低** | タスク6: array-subtype | 1 | vec 型推論の個別修正 |

タスク1 で19件、残タスク2+3+6 で10件。match 昇格はタスク1 件数を増やした（クラス移行）。

## Notes

### 関連コミット

- `59865e134` — string_gc concat + MIR_REF_CAST producer 型推論（clock_random / string_gc 解消）
- `aef1089ba` — `fix/verify-green` を master へマージ
- `fc0ee1525` — #690 From 変換 / #707 String clone（T3: 392/29）
- タスク5 — `inst_store_policy.ark` に `MIR_CALL_INDIRECT` consumer 追加（empty-stack 0）
- タスク2 部分 — `mir_promote_gc_enum_container_vt`（ref-vs-i32 7→4）
- `fix/726-t3-validate` — 継続調査・WIP（worktree 測定 393/28 は未マージ）

### 関連 Issue

- #724 — ADR-040 Phase 5-7 残作業（本 issue の上位）
- #725 — ADR-040 Phase 5e tracer removal（関連）
- #707 — trait self return type support
- #716 — GC intrinsic stub completion
- #690 — `?` / From 変換（question_mark fixture 側は解消）

### 実装メモ

- worktree: `fix/726-t3-validate`（`.worktrees/fix-726-t3-validate`）で作業中
- selfhost rebuild 時は **必ず** `flat-overlay-cache.json` を削除してから測定する
- 修正ごとに T3 validate-fail 数の増減を確認（stale s2 に注意）
