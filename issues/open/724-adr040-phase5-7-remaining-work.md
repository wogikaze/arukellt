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
# ADR-040 Phase 5-7: Typed MIR Signature Registry 残作業

## Summary

ADR-040 (Semantic Type Spine) は Phase 1-4 が完了し、PR-4-wide-audit /
PR-4-switch / PR-4b-trait-generic も完了した。しかし Phase 5-7 が未実装のまま
残っており、これらを追跡する issue が存在しなかった。本 issue は残作業の
追跡と完了基準の明確化を目的とする。

## 現在の完了状態

### Phase 1-4: 完了 ✅

- Phase 1: TypeTable + SignatureRegistry 骨格 — `src/compiler/corehir/type_table.ark`, `signature_registry.ark`
- Phase 2: MonoInstanceTable (subst マップ保存)
- Phase 3: Typed MIR (MirInst/MirLocal に MirValueType 追加)
- Phase 4: GcLayoutTable (MirValueType → WasmValueType lowering) — `gc_layout_table.ark` (216行)

### PR-4: 完了 ✅

- PR-4-wide-audit: Lane A-C,E 完了 (T3 reg-vt-audit mismatched=0)
- PR-4-switch: Lane D 完了 (通常 CALL の registry 経由切替)
- PR-4b-trait-generic: trait/generic/mono CALL の registry 切替完了

## 残作業 (Phase 5-7)

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

### Phase 6a: Symbolic Alias

**修正するファイル**:
- `src/compiler/wasm/code_scratch_locals.ark` — 直接番号を symbolic alias に置き換え
- `src/compiler/wasm/ctx_scratch.ark` — 同上
- スクラッチローカル番号を直接書く全箇所

**完了条件**:
- [ ] `emit_leb128_u(w, 16)` のような直接番号記述が 0 件
- [ ] 全て `emit_leb128_u(w, SCRATCH_GC_0)` のような alias 使用
- [ ] 既存テスト pass 数が悪化しない

### Phase 6b: LocalAllocator

**作成するファイル**:
- `src/compiler/wasm/local_allocator.ark` — LocalAllocator, ScratchPool

**修正するファイル**:
- `src/compiler/wasm/code_scratch_locals.ark` — alias の実体を LocalAllocator に差し替え
- `src/compiler/wasm/ctx_scratch.ark` — スクラッチをプールから借用

**完了条件**:
- [ ] `src/compiler/wasm/local_allocator.ark` が存在する
- [ ] スクラッチローカル追加で index シフトしない
- [ ] 既存テスト pass 数が悪化しない

### Phase 7: host intrinsic adapter 実装

**修正するファイル**:
- `src/compiler/wasm/call_host.ark` — HostIntrinsicSpec に従ってABI変換
- `src/compiler/wasm/code_body.ark` — host intrinsic のスタブ化を HostIntrinsicSpec で統一

**完了条件**:
- [ ] 全 host intrinsic が SignatureRegistry 経由で呼び出される
- [ ] adapter 関数が i32 → GC ref 変換を行う
- [ ] `func 12では対応済みだがfunc 28では別経路` のような経路依存が 0 件

## 実装順序

Phase 5 → 6a → 6b → 7 の順序で実装する。

- Phase 5 は emitter から型推論を削除するため、Phase 1-4 の registry が完全に機能していることが前提
- Phase 6a は symbolic alias を導入し、直接番号を消す
- Phase 6b は alias の実体を LocalAllocator に差し替える（2段階移行でリスク分散）
- Phase 7 は host intrinsic の統一

## 参照

- [ADR-040: Semantic Type Spine](../../docs/adr/ADR-040-typed-mir-signature-registry.md)
- #707 — trait self return type support (ADR-040 関連)
