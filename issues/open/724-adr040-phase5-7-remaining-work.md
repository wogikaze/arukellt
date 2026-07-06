---
Status: open
Created: 2026-07-15
ID: 724
Track: compiler-internal
Depends on: —
Related: ADR-040, #707
Orchestration class: design-then-implement
Blocks v4 exit: False
---
# ADR-040 Phase 3b-7: Typed MIR Signature Registry 残作業

## Summary

ADR-040 (Semantic Type Spine) は Phase 1-2, 4 と PR-4 が完了したが、**Phase 3（Typed MIR）は部分完了**のまま
Phase 5-7 が未実装。本 issue は残作業の追跡と完了基準の明確化を目的とする。

## 現在の完了状態

### Phase 1-2, 4: 完了

- Phase 1: TypeTable + SignatureRegistry 骨格 — `src/compiler/corehir/type_table.ark`, `signature_registry.ark`
- Phase 2: MonoInstanceTable (subst マップ保存)
- Phase 4: GcLayoutTable (MirValueType → WasmValueType lowering) — `gc_layout_table.ark`

### PR-4: 部分完了

- PR-4-wide-audit: Lane A-C,E 完了 (T3 reg-vt-audit mismatched=0)
- PR-4-switch: Lane D 完了 (void-return 判定に registry 使用)
- PR-4b-trait-generic: trait/generic/mono CALL の registry 切替完了
- **未完了**: local GC 型・全面 return 決定は legacy 推論経路が残存

### Phase 3: 部分完了（Critical Blocker）

- **完了**: `MirInst.func_id_raw`、spine 永続化、MIR verifier warn-only
- **未完了**: `MirLocal.value_type`、`MirInst.result_types` フィールド未追加
- lowering は `val_type`/`type_name` のみ設定

### Phase 6a: 完了

- `ctx_scratch::SelfEmitCtx_scratch_local(ctx, offset)` accessor 経由に全面統一済み
- ADR 原文の `SCRATCH_GC_0` 定数化は未実施だが目的は達成

## 残作業

### Phase 3b: Typed MIR フィールド完了（Phase 5 の前提）

**PR-3b-1**: `inst_record.ark` / `local_record.ark` に `result_types` / `value_type` 追加（emit 不変）

**PR-3b-L1〜L4**: lowering 全面配線（カテゴリ分割）

**PR-3c**: verifier 拡張（warn-only、INV-8 骨格）

**完了条件**:
- [ ] 全 MirLocal に `value_type` 設定
- [ ] 値を返す全 MirInst に `result_types` 設定
- [ ] T3 pass 数悪化なし

### Phase 5: emitter から型推論を削除

**削除する関数**:
- `code_ref_locals_infer.ark::find_stack_value_source`
- `code_ref_locals_infer.ark::infer_ref_local_gc_type_depth`
- `code_ref_locals.ark::infer_ref_local_gc_type`
- `mono_return_vt.ark::mono_return_type_name` の名前逆引き部分

**修正するファイル**:
- `src/compiler/wasm/code_locals.ark` — local型を `value_type` から直接取得
- `src/compiler/wasm/call_fallback.ark` — callee型を `func_id` から直接取得
- `src/compiler/mir/verify.ark` — warning を fail に切り替え（INV-5 完全執行）

**完了条件**:
- [ ] `find_stack_value_source` の呼び出し回数 = 0
- [ ] `infer_ref_local_gc_type_depth` の呼び出し回数 = 0
- [ ] `mono_return_type_name` の名前逆引き回数 = 0
- [ ] 旧推論経路が呼ばれないことを確認
- [ ] MIR verifier が type 未設定を fail にする（INV-5 完全執行）
- [ ] CALL/local/result の型整合が MIR verifier で検査される（INV-8, INV-9）

### Phase 6b: LocalAllocator

**作成するファイル**:
- `src/compiler/wasm/local_allocator.ark` — LocalAllocator, ScratchPool

**完了条件**:
- [ ] `src/compiler/wasm/local_allocator.ark` が存在する
- [ ] スクラッチローカル追加で index シフトしない
- [ ] 既存テスト pass 数が悪化しない

### Phase 7: host intrinsic adapter 実装

**修正するファイル**:
- `src/compiler/wasm/call_host.ark` — HostIntrinsicSpec に従ってABI変換
- `src/compiler/wasm/code_body.ark` — GC target unreachable stub 削除

**完了条件**:
- [ ] 全 host intrinsic が SignatureRegistry 経由で呼び出される
- [ ] adapter 関数が i32 → GC ref 変換を行う
- [ ] 経路依存（func 12 OK / func 28 NG）0 件
- [ ] T3 host intrinsic 系 validate-fail 0 件

## 実装順序

Phase 3b → 3c → 5 → 6b ∥ 7

- Phase 3b は Phase 5 の前提（`value_type` / `result_types` 必須）
- Phase 6a は完了済み（作業不要）
- Phase 6b と Phase 7 は Phase 5 完了後に並列可能

## 参照

- [ADR-040: Semantic Type Spine](../../docs/adr/ADR-040-typed-mir-signature-registry.md)
- #707 — trait self return type support (ADR-040 関連)
