# #874 — fallback managed value: move/borrow parity audit

ステータス: **done**（#874 closed 2026-09-27）
日付: 2026-09-27
関連: [#874](../../issues/done/874-fallback-managed-value-lowering.md)、
[lowering plan](../plans/wasm32-linear-memory-lowering.md)、
ADR-075（linear-memory tracing runtime）、ADR-002（memory model）、
#889（impl-path `Vec::new<T>` element grow）、#890（`Option<tuple>` ABI）

## スコープ

#874 acceptance の最後の未消化項だった
「Wasm GC backend と同じ move/borrow semantics を保つ」
「共有・循環参照を reject せず実行する」の書面監査。
比較対象は source-visible semantics であり、heap 表現やランタイム内部の
一致ではない。managed layout・allocator 接続・root/collect の個別実装は
issue receipt と `wasm32-linear-memory-lowering.md` が正本であり、
本書は parity 判定とその証拠の一覧だけを置く。

## 監査項目と証拠

| # | 監査項目 | wasm32 fallback の仕組み | parity evidence | 判定 |
|---|----------|--------------------------|-----------------|------|
| 1 | struct/enum/Option/Result layout | record が `emit_heap_alloc_from_stack` → `LINEAR_GC_ALLOC`、enum は tag+payload record | `enums/enum_struct_variant` = `75/24`、`vec_option_i32` = `7`、`generics/gc_struct_field_vec_string` = `1` を両 backend で実行 | OK |
| 2 | move/copy value semantics | 値コピー（struct は i32 index 参照ではなく値） | `closure_capture/move_local`、`copy_capture_snapshot` が両 backend で同一出力 | OK |
| 3 | borrow/capture + write-through | capture cell は managed record、borrow rebind は cell 越しの write-through | `closure_capture` 6 fixture（borrow/move/copy/rebind/struct/escape*）全て両 backend で同一出力 | OK |
| 4 | Vec 要素族（i32/i64/f64/String/fn/struct） | `{len, cap, data}` flat layout + 要素幅別 store/load | `vec_pop_kinds`（`pop -> Option<T>` 全要素族）、`vec_fn_methods`、`vec_fn_param_get`、`vec_impl_path_numeric`（#889, impl-path i64/f64）が両 backend で同一出力 | OK |
| 5 | Option/Result payload | i32 系は `{tag:i32, payload:i32}` 8B、i64/f64 は `{tag:i32, payload:8B}` 12B（`emit_i64_ok_result` と同一 layout） | `vec_pop_kinds`・`option_fn_return_boundary` = `14/14/0/6`、#889 の wide `vec_get` も同一 record layout に統一 | OK |
| 6 | String allocation family | concat/from_bytes/slice/trim/case/join/repeat/pad/replace/split/chars 全て `LINEAR_GC_ALLOC` 経由（length-prefix ABI 維持） | 各 family fixture が wasm32 linear core / wasm32-gc component で全 pass（receipt: 8df8fa460, 196da76fa, bf94dbe98, d00da2f1c, 26513a797, b31fe4557） | OK |
| 7 | HashMap managed runtime 接続 | flat slot array は managed `Vec<i32>` backing 内、key/value String は transitive marking で保持 | `target_capability/linear_gc_hashmap_survives`（concat 生成 key/value が 160MiB churn + 2回超の entry-gated collection を越えて get/len/contains_key 維持）、`hm_ii_methods` = `2 10`、`hm_fn_ops` = `6 true false 1 8 0 true`、`hm_strfn_ops` = `10 true 12` 両 backend 同一 | OK |
| 8 | fn value / closure env | `{fnptr, env}` pair record（managed closure ABI） | `vec_fn_methods`、`vec_fn_param_get`、`option_vec_fn`、`result_vec_fn`、`option_fn_return_boundary`、`hm_fn_ops`、`hm_strfn_ops`、escape fixture 群が両 backend で同一出力 | OK |
| 9 | bounds/error observable parity | `emit_vec_trap_if_oob`（`idx >= len` で `unreachable`、負 index も unsigned 比較で捕捉） | `get_unchecked(100)`・`v.set(50, _)` が両 backend で trap（LM: unreachable、GC: out of bounds array access — いずれも実行中断）= ADR-075 §4 一致 | OK |
| 10 | root / automatic collect | root frame enter/push/update（LOCAL_SET 後）、64MiB watermark gate が function entry で collect を起動 | `linear_gc_entry_collect` がソースから 2回以上の collection を駆動、`test_function_entry_collects_when_watermark_exceeds_threshold`、`test_source_level_live_data_survives_entry_collection` | OK |
| 11 | 共有・循環参照 | source では真の pointer cycle は書けない（struct は managed ref ではなく i32 index を持つ）。実 cycle は runtime marker が marked set で処理 | `test_rooted_cycle_survives_and_unreachable_cycle_is_reused`（rooted cycle 生存 + unreachable cycle が free list 再利用）、#7 の共有 String graph が transitive marking で保持 | OK |

## 記録済みの相違・残存事項

- **conservative pointer-candidate scan（継続）**: ADR-075 §3 が bootstrap
  表現に限り許可。false positive は一時保持を許し、false negative による
  live object 回収は禁止。exact pointer-field descriptor は bootstrap
  外でのみ必要となり、本 issue の条件ではない。
- **`Option<tuple>` payload ABI**: 両 backend 同一に invalid wasm
  を生成する pre-existing gap。parity 上の相違ではなく共通欠陥。
  → #890 で追跡。
- **HashMap receiver の E0200 表示名**: 型名が `?` と描画される
  cosmetic gap（診断品質・両 backend 同一）。
- **`hashmap_builtin_alias` の `HashMap_i32_fn_insert -> _set`**
  bool/Option 戻り値の非対称: 内部 alias 経路の cosmetic 差異。
  公開 semantics には到達しない。
- **impl-path `Vec::new` の容量**: `raw_array_new(0)` は GC で hint を
  そのまま反映し cap=0 で始まる（pseudo ctor は既定 8）。容量は
  user-observable ではなく、#889 の grow clamp 以後は両 backend で
  push/get/pop が同一に動く。
- **unannotated `Vec::new<(A,B)>` turbofish**: receipt では残存と
  記録されていたが、#888 の tuple type-name 経路で解消済み。probe
  で `v.push((1,2)); get_unchecked(0)` が両 backend で `1 2` と
  destructure できることを確認済み。

## 判定

acceptance 全項 OK:#874 close を承認する。exact descriptors は
ADR-075 §3 の bootstrap 許可により本 issue の外、`Option<tuple>` は
#890、E0200 表示名と alias 非対称は cosmetic 残件として receipt に
記録する。
