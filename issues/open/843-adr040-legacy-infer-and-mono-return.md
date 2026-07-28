---
Status: open
Created: 2026-07-28
Updated: 2026-07-28
ID: 843
Track: compiler-internal
Depends on: #724
Related: ADR-040, #725, #842, #729
Orchestration class: design-then-implement
Blocks v4 exit: False
Priority: 3
Source: Split from #724 — PR-4 / Phase 5 leftover legacy inference
---

# ADR-040: legacy GC infer と mono_return 名前逆引きの除去

## Summary

`#724` umbrella（Phase 3b–7）の完了後に残った、emitter / lowering の
**legacy 型推論経路**を消す。ADR-040 Phase 5 / PR-4 の未達受け入れ条件を
本 issue が引き継ぐ。

## Scope

1. **`mono_return_type_name` の名前逆引き = 0**
   - `src/compiler/mir/lower/mono_return_vt.ark`
   - mangled suffix / `method_concrete_return_from_mono_fn_name` 依存をやめ、
     MonoInstanceTable / SignatureRegistry から返す
2. **旧推論経路が呼ばれない**
   - `infer_ref_local_gc_type` の call site（現状 ~11 箇所: `code_locals*.ark`,
     `inst_locals.ark`, `inst_struct_record.ark`, `intrinsic_vec_access_gc.ark`）
   - local GC 型・return 決定を `MirLocal.value_type` / GcLayoutTable /
     SignatureRegistry 優先に切替し、infer を削除または dead 化
3. **PR-4 残**: local GC 型・全面 return 決定の registry 本線化

## Non-goals

- host intrinsic adapter（#724 Phase 7 done）
- 命令トレーサ（#725 done）
- builtin callee 名 matching（#842）
- T3 validate-fail=0 全体（#726）/ bootstrap pin（#730）

## Progress

### Slice 1 (2026-07-28): call-time mangled 削除

- `mono_return_vt.ark`: spine 優先、`method_concrete_return_from_mono_fn_name` 削除
- `call_type_fallback.ark`: `mark_mono_call_result_type` / `extract_mono_suffix` 削除
- `signature_registry_build_mono.ark`: 空 return 名でも mangled 具象（Point/String/float）を
  MonoInstance に登録（call-time 逆引きの代替）
- 検証: `eq_trait` / `eq_trait_string` / `self_return_add` / `vec_generic` OK、`verify lane` PASS

## Acceptance

- [x] `mono_return_type_name` の mangled / 名前逆引き経路が削除されている
      （または呼び出し回数 = 0）
- [ ] `infer_ref_local_gc_type` の本番 call site = 0（テスト専用を除く）
- [ ] local GC 型と CALL return 決定が SignatureRegistry / value_type /
      GcLayoutTable から解決される
- [ ] `selfhost build-compiler` + `verify lane` PASS（必要なら host/T3 部分集合）

## Evidence baseline (2026-07-28, pre-work)

- `mono_return_type_name` callers: `mono_return_vt.ark`, `call_type_fallback.ark`
- `infer_ref_local_gc_type` callers: 11 production sites（上記 Scope）
- `#724` Phase 5 未チェック 2 件をここへ移管

## 参照

- #724 — umbrella（Phase 3b–7、本 issue へ残作業移管後 close）
- #725 — tracer 削除（done）
- #842 — builtin callee GC lookup
- ADR-040 / `docs/plans/typed-mir-signature-registry.md` Phase 5
