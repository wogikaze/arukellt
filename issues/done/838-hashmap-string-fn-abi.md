---
Status: done
Created: 2026-07-25
Updated: 2026-07-25
ID: 838
Track: compiler-internal
Depends on: 833, 837
Related: ADR-033, #831, #833
Orchestration class: implementation-ready
Orchestration upstream: 833
Blocks v{N}: none
Priority: 2
Source: #833 Non-goal — HashMap&lt;String, fn&gt; after HashMap&lt;i32, fn&gt;
---

# HashMap&lt;String, fn&gt; funcref value ABI

## Summary

#833 で `HashMap&lt;i32, fn&gt;`（`__hm_if_*`）は閉じた。
`HashMap&lt;String, fn&gt;` は未実装で、String key（`__hm_ss_*`）と funcref value（`__hm_if_*`）の組み合わせが無い。

## Strategy

- **分類**: 実装拡張（#833 / monomorphic HashMap 特化の穴埋め）。新 ADR 不要。
- **ABI**: GC-only。keys=`A_ref0`(String)、vals=`A_fnref`、flags=`A_i32`。`fnref_array` は #833 と共用。
- **公開面**: `HashMap_String_fn_*` / `HashMap_new_String_fn` builtins + `std::collections::hash_str_fn`（`hash_fn` と同型のコンパイラ向け helper）。
- **正本**: `src/compiler/wasm/**` intrinsics / GC types、`std/collections/hash_str_fn.ark`、loader auto-load。
- **検証**: fixtures + `verify lane --gate t3` + `verify quick`。

## Acceptance

- [x] `hashmap_string_fn_call` validate + hosted run
- [x] `hashmap_string_fn_builtin`（use なし）validate + hosted run
- [x] 既存 `hashmap_i32_fn_*` / `hashmap_str_str_*` が緑
- [x] `python3 scripts/manager.py verify lane --gate t3`
- [x] `python3 scripts/manager.py verify quick`
