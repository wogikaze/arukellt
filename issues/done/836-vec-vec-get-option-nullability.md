---
Status: done
Created: 2026-07-25
Updated: 2026-07-25
ID: 836
Track: compiler-internal
Depends on: 832
Related: ADR-033, #726, #844, #835
Orchestration class: implementation-ready
Orchestration upstream: 832
Blocks v{N}: none
Priority: 2
Source: post-#844 / #845 follow-up — isolate Vec&lt;Vec&gt; vs flat Vec gaps
---

# `get` on `Vec&lt;Vec&lt;T&gt;&gt;` nullability validate-fail

## Summary

#844 F で `Vec&lt;Vec&lt;i32&gt;&gt;` + `get_unchecked` は閉じた。
本 issue は **flat `Vec` と nested `Vec&lt;Vec&gt;` を切り分けた結果**、残る穴を追跡する。

切り分け結論（2026-07-25, s2）:

| 形 | 結果 |
|----|------|
| `Vec&lt;i32&gt;` + `get` → `Option&lt;i32&gt;` | OK |
| `Vec&lt;String&gt;` + `get` → `Option&lt;String&gt;` | OK |
| `Vec&lt;Vec&lt;i32&gt;&gt;` + `get_unchecked` | OK（既存 `vec_vec_i32`） |
| `Vec&lt;Vec&lt;i32&gt;&gt;` outer `get_unchecked` → inner `get` | OK |
| **`Vec&lt;Vec&lt;i32&gt;&gt;` outer `get` → `Option&lt;Vec&lt;i32&gt;&gt;`** | **FIXED** |
| `Vec&lt;Vec&lt;String&gt;&gt;` + `get_unchecked` | OK |
| `Vec&lt;Vec&lt;fn&gt;&gt;` + `get_unchecked` / call | OK |
| `Vec&lt;Option&lt;Vec&lt;i32&gt;&gt;&gt;` + `get_unchecked` | OK |

## Root cause

`emit_vec_get_gc_some_ref` が structref 要素の `Option::Some` を選ぶとき、
コンテナ要素名（`vec:i32` 等）を認識せず既定 `_f1_ref0`（String）へ落ちていた。
WAT では `struct.new` が Some(String) になり、payload に Vec ref を載せた結果
`expected (ref null $type), found (ref $type)` として validate-fail した。

## Fix

`intrinsic_vec_access_gc::option_some_suffix_for_container_elem` を追加し、
MIR の `gc_struct_container_ref_suffix` と同じ固定スロット（例: `vec:i32` → `_f1_ref10`）
で Option::Some を選ぶ。

## Acceptance

- [x] outer `get(outer, i)` → `Option&lt;Vec&lt;i32&gt;&gt;` match が validate + hosted run
- [x] 既存 `vec_vec_i32` / flat `get` fixtures が緑
- [x] fixture を `run:` / `t3-compile:` / `t3-run:` 登録（`vec_vec_i32_get`, `vec_vec_string_get`）
- [x] `python3 scripts/manager.py verify lane --gate t3`

## Related

- #844 nested container / funcref matrix（F は get_unchecked のみ）
- #835 Option mut open-enum cast（近縁）
- #726 GC ref 型推論
