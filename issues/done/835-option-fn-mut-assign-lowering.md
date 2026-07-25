---
Status: done
Created: 2026-07-25
Updated: 2026-07-25
Closed: 2026-07-25
ID: 835
Track: compiler-internal
Depends on: 833
Related: ADR-033, #831, #833
Orchestration class: implementation-ready
Orchestration upstream: 833
Blocks v{N}: none
Priority: 2
Source: #833 Notes — Option&lt;fn&gt; mut assign traps on ref.as_non_null
---

# Option&lt;fn&gt; mut 代入 lowering

## Summary

#833 で `HashMap&lt;i32, fn&gt;` MVP は閉じた。stdlib は early `return Some/None` で回避していた。
`let mut result: Option&lt;fn&gt; = None` 後に `result = Some(...)` する形は、
スロットに funcref が入っていても `ref.as_non_null` で trap していた。
`Option&lt;String&gt;` の同型パターン（`hash_string`）は問題なし。

本 issue は null funcref の **生産側** を直し、`hash_fn` を `hash_string` と同型の
mut パターンへ戻した。`as_non_null` の無条件削除はしていない。

## Root cause

mut `Option` / `Result` 一時 local が None 構築後に sticky な concrete variant type
（例: type 38）へ `ref.cast` されていた。後から `Some`（type 45）を代入すると
非互換 final subtype への cast で trap。正しくは open-enum（type 21）へ cast。

## Fix

- [`inst_locals.ark`](../../src/compiler/wasm/inst_locals.ark): GC_STRUCT_NEW dest 以外で
  concrete enum-variant へ cast しない。open-enum へ寄せる。
- multi-variant LOCAL_SET 検出を強化（推論・local decls）
- `hash_fn` get/remove を mut `Option&lt;fn&gt;` パターンへ復帰

## Acceptance

- [x] `option_fn_mut_assign` validate + hosted run（stdout `10`）
- [x] `hashmap_i32_fn_mut_get` validate + hosted run（stdout `10`）
- [x] `hash_fn` get/remove が mut パターンで同じ挙動
- [x] 既存 `option_fn_call` / `hashmap_i32_fn_call` が緑
- [x] `python3 scripts/manager.py verify lane --gate t3`

## Related

- #833 HashMap&lt;i32, fn&gt; funcref value ABI（upstream）
- #831 call_ref emitter
- ADR-033
