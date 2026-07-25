---
Status: done
Created: 2026-07-25
Updated: 2026-07-25
Closed: 2026-07-25
ID: 833
Track: compiler-internal
Depends on: 832
Related: ADR-033, #831, #832, #722
Orchestration class: implementation-ready
Orchestration upstream: 832
Blocks v{N}: none
Priority: 2
Source: #832 Non-goal — HashMap&lt;K, fn&gt; after Vec&lt;fn&gt; funcref ABI
---

# HashMap&lt;K, fn&gt; funcref value ABI

## Summary

#832 で `Vec&lt;fn&gt;` / `Option&lt;Vec&lt;fn&gt;&gt;` の funcref 配列 ABI は閉じた。
#832 Non-goals に残した **`HashMap&lt;K, fn&gt;`** を本 issue で実装する。

現行の HashMap は monomorphic 特化のみ:

| 形 | 実装 |
|----|------|
| `HashMap&lt;i32, i32&gt;` | `Vec&lt;i32&gt;` flat (`hashmap_new` / `hashmap_set`) |
| `HashMap&lt;String, i32&gt;` | GC `__hm_si_*` |
| `HashMap&lt;i32, String&gt;` | GC `__hm_is_*` |
| `HashMap&lt;String, String&gt;` | GC `__hm_ss_*` |
| `HashMap&lt;i32, fn(...)&gt;` | ❌ typecheck（`i32` API に押し込むと E0208） |
| `HashMap&lt;String, fn(...)&gt;` | ❌ 同上 |

fn 値は i32 スロットに載せられない（semantic-debt 禁止）。
`Vec&lt;fn&gt;` と同型の **nullable funcref 配列**を value region に使う。

## Baseline（2026-07-25, s2）

```ark
let m: HashMap<i32, fn(i32)->i32> = HashMap_i32_i32_new()
HashMap_i32_i32_insert(m, 1, double)  // E0200: i32 vs struct/fn
```

専用 builtin / GC layout / stdlib 経路が無い。

## Root causes / 必要作業

### A. GC layout — `HashMap&lt;i32, fn&gt;`（MVP）

- 新規 type sig: keys=`A_i32`, vals=`A_fnref`(offset 26), flags=`A_i32`
  - 例: `SubF_GS_f0_i32_f1_i32_f2_ref1_f3_ref26_f4_ref1` → gcref30
- `SelfEmitCtx_hashmap_i32_fn_type` / plan / locals / scratch（fnref array）

### B. Primitives `__hm_if_*`（i32 key, fnref value）

`__hm_is_*`（i32→String）を鏡写し:

- `__hm_if_new` / `_cap` / `_size` / `_set_size`
- `_get_flag` / `_set_flag` / `_get_key` / `_set_key`
- `_get_val` / `_set_val`（funcref array.get/set + 必要なら `ref.as_non_null`）

### C. Typecheck / resolver / ann

- builtins: `HashMap_new_i32_fn` / `HashMap_i32_fn_insert|get|…`
- `get`/`remove` → `Option&lt;fn&gt;`（`_f1_fnref` / VT_FUNCREF）
- ann: `HashMap&lt;i32, fn(...)&gt;` → `hashmap:i32fn`（`hashmap_type_alias`）
- aliases: `HashMap_i32_fn_*` → stdlib / `__hm_if_*`

### D. Stdlib wrappers

- `std/collections/hash_fn.ark`（または `hash_string.ark` 併置）:
  `hashmap_i32_fn_new/set/get/contains/remove/len`
- `hash_map.ark` に薄い公開 wrapper（既存 i32_str と同型）

### E. Fixture

- `tests/fixtures/collections/hashmap_i32_fn_call.ark`
  - insert `double` → get → `f(5)` → `10`
- manifest: `run:` / `t3-compile:` / `t3-run:`

### F. 後続（本 issue の非 MVP / 別 commit 可）

- `HashMap&lt;String, fn&gt;`（`__hm_sf_*`）
- `HashMap&lt;fn, V&gt;`（key が fn — 要 hash 方針、非ゴール寄り）
- `remove` / `contains` / grow / clear のフル行列
- linear-memory (`wasm32`) 経路（GC primary のみでも MVP 可）

## Workstreams（推奨順）

1. **A+B** — GC type + `__hm_if_*` emitter
2. **C+D** — builtins / ann / stdlib
3. **E** — fixture + `verify lane --gate t3`
4. **F** — String-key / 拡張（余力）

## Primary paths

- `src/compiler/wasm/sections_types_gc*.ark` / `ctx_gc_type.ark`
- `src/compiler/wasm/intrinsic_hashmap_str_*.ark` / `hash_aliases.ark` / `call_hash.ark`
- `src/compiler/typechecker/builtins_hash.ark` / `resolver/builtins_hash.ark`
- `src/compiler/corehir/hashmap_type_name.ark` / `type_ann_local_name*.ark`
- `src/compiler/mir/lower/call_type_hash.ark` / `ctx_gc_enum_sig.ark`（Option&lt;fn&gt;）
- `std/collections/hash_*.ark`

## Non-goals

- 完全汎用 `HashMap&lt;K,V&gt;` の自動 monomorphize（既存方針の特化追加に留める）
- `HashMap&lt;fn, _&gt;` key
- Class C `call_indirect` 全廃
- ADR-035 `br_on_null`

## Acceptance

- [x] `HashMap&lt;i32, fn(i32)-&gt;i32&gt;` の new/insert/get/call が validate + hosted run
  - evidence: `tests/fixtures/collections/hashmap_i32_fn_call.ark` → hosted stdout `10`
- [x] get の `Option&lt;fn&gt;` match で `call_ref`（unreachable にならない）
- [x] 回帰 fixture を manifest 登録
  - `run:` / `t3-compile:` / `t3-run:` in `tests/fixtures/manifest.txt`
- [x] `python3 scripts/manager.py verify lane --gate t3`（T3 gate PASS; quality changed は無関係 dirty で別失敗しうる）
- [x] フェーズ完了時 `python3 scripts/manager.py verify quick`（docs fixture count sync 後）

## Notes

- value 配列は #832 の `A_fnref`（nullable）を再利用する。新規 array type は不要。
- `#832` 完了後の直接後続。upstream は #832（Vec&lt;fn&gt; ABI）。
- `std/collections/hash_fn.ark` の `get`/`remove` は early `return Some/None` を使う。
  `let mut result: Option&lt;fn&gt; = None` 後に `result = Some(__hm_if_get_val(...))`
  する形は、スロットに funcref が入っていても `ref.as_non_null` で trap する
  （`Option&lt;String&gt;` の同型パターンは問題なし）。lowering 修正は follow-up。
- MVP 正経路は `use std::collections::hash_fn`。`HashMap_i32_fn_*` builtin だけの
  自動 load は現状 `unreachable` に落ちる（`HashMap_i32_String_*` も同型の既知ギャップ）。

## Related

- #832 nested container / funcref typing matrix（upstream）
- #831 call_ref emitter
- ADR-033 HOF / typed funcref migration
