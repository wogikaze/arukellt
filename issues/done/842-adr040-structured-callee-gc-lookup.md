---
Status: open
Created: 2026-07-28
Updated: 2026-07-28
ID: 842
Track: compiler-internal
Depends on: #725
Related: ADR-040, #724, #729
Orchestration class: design-then-implement
Blocks v4 exit: False
Priority: 4
Source: Split from #725 Step 3 — remaining string-based callee GC type matching
---

# ADR-040: builtin / string callee GC lookup → SignatureRegistry

## Summary

`#725` Step 3 のうち、host stub は SignatureRegistry フォールバックで
構造化経路に載せた。残るのは **Mir 関数テーブルに無い builtin / string /
hashmap / Vec_new_* callee** のハードコード名 matching を SignatureRegistry
（または同等の構造化登録）へ移す作業。

## Why blocked historically

`inst_ctx::resolve_fn_index` は Mir function name index のみを見る。
`String_new` / `__hm_si_new` / `Vec_new_*` / `parse_*` などはテーブルに無い
ことがあり、ハードコード matching を消すと T3 が 389→350 pass に悪化した
（#725 2026-07-08 調査）。

## Scope

- `code_ref_locals_infer_callee.ark` の残り:
  - `infer_string_callee_gc_type`
  - `infer_hashmap_callee_gc_type`
  - `infer_builtin_callee_gc_type` / `infer_vec_new_callee_gc_type`
- `code_ref_locals_types.ark::mir_fn_returns_option_by_name` の名前リスト
- 必要なら builtin 戻り値型を SignatureRegistry に登録する lowering 側拡張

## Non-goals

- host intrinsic stub 本体（Phase 7 / #724 完了）
- 命令トレーサ再導入
- 全 TypeTable intern の一括完了（別途 #729 / typed spine と協調）

## Acceptance

- [ ] 上記ハードコード matching を削除しても T3 pass が baseline を下回らない
- [ ] builtin / string / hashmap / Vec_new_* の戻り値 GC 型が SignatureRegistry
      （または GcLayoutTable + TypeId）から解決される
- [ ] `verify lane` PASS（emitter 変更時は `selfhost build-compiler` 後）

## 参照

- #725 — Phase 5e tracer 削除（親・トレーサ側完了）
- #724 — ADR-040 umbrella（Phase 7 host adapter 完了）
- ADR-040 / RFC-002 Semantic Type Spine
