---
Status: open
Created: 2026-07-15
ID: 726
Track: compiler-internal
Depends on: 724
Related: ADR-040, #707, #716
Orchestration class: implementation-ready
Blocks v4 exit: True
---
# T3 WASM validation failures: 32件の validate-fail 修正（GC ref 型推論バグ）

## Summary

T3 WASM validation で32件の `validate-fail` が発生しており、`verify quick` が block されている。
pre-commit hook が `verify quick` を実行するため、現状コミットに `--no-verify` が必要な状態。
すべての失敗は GC ref 型推論の不整合に起因する。

### 現在のベースライン

| 状態 | 件数 |
|------|------|
| pass | 389 |
| **validate-fail** | **32** |
| compile-fail | 1 |
| skip | 22 |
| **total** | **444** |

## Acceptance Criteria

- [ ] タスク1: ref-vs-ref（16件）解決 — `code_locals.ark` の variant_slot ルックアップ統一
- [ ] タスク2: ref-vs-i32（6件）解決 — `try`/`match` payload 抽出の local VT 修正
- [ ] タスク3: i32-vs-ref（5件）解決 — `method_emit.ark` の `mir_mark_call_result_type` 修正
- [ ] タスク4: i32-vs-f64（2件）解決 — local 再利用の型衝突修正
- [ ] タスク5: empty-stack（2件）解決 — store policy の連続 `local.set` 修正
- [ ] タスク6: array-subtype（1件）解決 — vec 型推論の個別修正
- [ ] `verify quick` が validate-fail=0 で pass する
- [ ] pre-commit hook が `--no-verify` なしで通過する

## エラーパターン別分類

| エラー型 | 件数 | エラーメッセージ |
|----------|------|-----------------|
| **ref-vs-ref** | 16 | `expected (ref null $type), found (ref null $type)` |
| **ref-vs-i32** | 6 | `expected (ref null $type), found i32` |
| **i32-vs-ref** | 5 | `expected i32, found (ref null $type)` |
| **i32-vs-f64** | 2 | `expected i32, found f64` |
| **empty-stack** | 2 | `expected (ref null $type) but nothing on stack` |
| **array-subtype** | 1 | `expected subtype of arrayref, found (ref null (id 21))` |

## タスク1: ref-vs-ref（16件）— 最大グループ

### エラー
`expected (ref null $type), found (ref null $type)` — 異なる2つの ref 型が混同

### 対象fixture
- `from_trait/from_auto_convert.ark`
- `generics/two_params.ark`
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

## タスク2: ref-vs-i32（6件）— ref が i32 に退化

### エラー
`expected (ref null $type), found i32` — ref 型のべき local が i32 として宣言

### 対象fixture
- `stdlib_io/fs_read_error.ark`
- `stdlib_io/fs_read_write.ark`
- `stdlib_json/json_perf_decode.ark`
- `stdlib_toml/toml_full_inline_dotted.ark`
- `stdlib_toml/toml_full_table_header.ark`
- `stdlib_trait/debug_vec.ark`

### 根本原因
`try` / `match` の payload 抽出で、`res_ptr` / `scrut_idx` などの local が GC target でも `VT_GC_REF` ではなく `VT_I32` として宣言されている。

### 修正方針
GC target の場合、これらの local を `VT_GC_REF` として宣言する。

### 修正対象ファイル
- `src/compiler/mir/lower/try.ark` — `mir_emit_try_unwrap` の `res_ptr` local
- `src/compiler/mir/lower/match_payload_prepare.ark` — `mir_emit_payload_tag_local` の `scrut_idx` local
- `src/compiler/mir/lower/hof_option_setup.ark` — `emit_map_option_extract_tag` の `mopt_opt_local`
- `src/compiler/mir/lower/core_match_payload_info.ark` — `core_emit_payload_tag_local` の `scrut_idx` local

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

## タスク4: i32-vs-f64（2件）— f64 の local に i32 が格納

### エラー
`expected i32, found f64` — f64 local に i32 値が格納される

### 対象fixture
- `enums/tuple_variant.ark`
- `stdlib_io/f64_to_string.ark`

### 根本原因
local 再利用による型衝突。`f64_to_string` 内部で、f64 local 12 に `f64.ne` の結果（i32）が格納されている。local 12 は元々 f64 として宣言されたが、別の用途で i32 として再利用されている。

### 修正方針
local 再利用時の型衝突を防ぐ、または f64 演算結果の local 型を正しく推論。

## タスク5: empty-stack（2件）— スタックが空なのに local.set

### エラー
`expected (ref null $type) but nothing on stack` — スタックが空の状態で local.set が実行

### 対象fixture
- `stdlib_io/clock_random.ark`
- `stdlib_trait/ord_sort_by.ark`

### 根本原因
`clock_random` で `end` 後に `local.set 36` `local.set 35` が連続している。スタック上に値が1つしかないのに2つの local.set が実行される。store policy または peephole 最適化のバグ。

### 修正方針
store policy の連続 `local.set` 処理を修正、または peephole でスタック深さを正しく追跡。

### 修正対象ファイル
- `src/compiler/wasm/inst_store_policy.ark` — store policy
- `src/compiler/wasm/inst_dispatch_local.ark` — peephole 最適化

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
| **高** | タスク1: ref-vs-ref | 16 | 16件一括解決で 32→16 に減少 |
| **高** | タスク2: ref-vs-i32 | 6 | 6件解決で 16→10 に減少 |
| **中** | タスク3: i32-vs-ref | 5 | 5件解決で 10→5 に減少 |
| **低** | タスク4: i32-vs-f64 | 2 | local 再利用の型衝突、個別対応 |
| **低** | タスク5: empty-stack | 2 | store policy のバグ、個別対応 |
| **低** | タスク6: array-subtype | 1 | vec 型推論の個別修正 |

タスク1 + タスク2 で22件（69%）が解決し、32→10 に減少する見込み。

## Notes

### 関連コミット
- `59865e134` — Bug 4 (string_gc concat) + Bug 5 (MIR_REF_CAST producer 型推論) の修正
- master マージ済み（merge conflict 解消: `code_ref_locals_typename.ark`）

### 関連 Issue
- #724 — ADR-040 Phase 5-7 残作業（本 issue の上位）
- #707 — trait self return type support
- #716 — GC intrinsic stub completion

### 実装メモ
- worktree: `fix-verify-green` で作業中
- selfhost rebuild + T3 検証で効果確認
- 修正ごとに `verify quick` で validate-fail 数の増減を確認
