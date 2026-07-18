# Semantic Type Spine 移行計画

ステータス: 実装計画（決定記録ではない）
関連 ADR: ADR-040 / [RFC-002](../rfcs/002-semantic-type-spine.md)

---

## ゴール

MIR から Wasm emitter まで意味情報（型、シグネチャ、ABI）を欠落させず伝播し、
emitter の型推論・名前逆引きを廃止する。

## フェーズ順序

```
Phase 1: TypeTable + SignatureRegistry 骨格 + HostIntrinsicSpec 型定義
  ↓
Phase 2: MonoInstanceTable（subst マップ保存）
  ↓
Phase 3: Typed MIR（MirInst/MirLocal に MirValueType 追加）
  ↓
Phase 4: GcLayoutTable（MirValueType → WasmValueType lowering）
  ↓
Phase 5: emitter から型推論を削除
  ↓
Phase 6a: Symbolic Alias（直接local番号を消す）
Phase 6b: LocalAllocator（alias の実体を差し替え）
  ↓
Phase 7: host intrinsic adapter 実装
```

**Phase 2 を Phase 3 の前に移動した理由**: Typed MIR の CALL に正しい
戻り値型を付けるために、MonoInstance の subst が先に必要。
Phase 1 で SignatureRegistry の骨格を作り、Phase 2 で MonoInstance の
subst を保存し、Phase 3 で Typed MIR がそれを参照する。

---

## Phase 1: TypeTable + SignatureRegistry 骨格

- `src/compiler/corehir/type_table.ark` — TypeTable, TypeEntry, TypeId
- `src/compiler/corehir/mir_value_type.ark` — MirValueType, ValueRepr, Nullability
- `src/compiler/corehir/signature_registry.ark` — SignatureRegistry, SignatureEntry, FunctionId, AbiKind
- `src/compiler/corehir/host_intrinsic_spec.ark` — HostIntrinsicSpec, HostAbiKind（型定義のみ）
- `src/compiler/corehir/type_contracts.ark` — TypedFn に param_types, trait_id, impl_id, abi_kind 追加
- `src/compiler/typechecker.ark` — 型チェック結果を TypeTable と SignatureRegistry に登録

## Phase 2: MonoInstanceTable

- `src/compiler/mir/lower/mono_instance_table.ark` — MonoInstanceTable, MonoInstanceEntry, MonoInstanceId
- `src/compiler/mir/lower/mono_param_subst.ark` — モノモーフ化時に subst マップを保存
- `src/compiler/mir/lower/fn_index_mono.ark` — MonoInstanceTable に登録

## Phase 3: Typed MIR

- `src/compiler/mir/inst_record.ark` — MirInst に result_types, func_id 追加
- `src/compiler/mir/local_record.ark` — MirLocal に value_type 追加
- `src/compiler/mir/lower/*.ark` — MIR lowering 時に MirValueType を必ず設定
- `src/compiler/mir/verify.ark`（新設）— MIR verifier: type 未設定箇所をログで報告

## Phase 4: GcLayoutTable

- `src/compiler/wasm/gc_layout_table.ark` — GcLayoutTable, GcLayoutEntry, WasmRefType, WasmValueType
- `src/compiler/wasm/sections_type_plan.ark` — 全 defined type index と recursive group の owner
- `src/compiler/wasm/lower_value_type.ark` — semantic value 用の `lower_value_type`
- storage lowering — context-aware な `lower_storage_type`。Vec backing と aggregate field を区別
- `src/compiler/wasm/sections_types_gc.ark` — 型セクションエミッション時に GcLayoutTable を参照
- `src/compiler/wasm/ctx_gc_type.ark` — MirValueType から GcLayoutId をルックアップ

## Phase 5: emitter から型推論を削除

削除対象:
- `code_ref_locals_infer.ark::find_stack_value_source`
- `code_ref_locals_infer.ark::infer_ref_local_gc_type_depth`
- `code_ref_locals.ark::infer_ref_local_gc_type`
- `mono_return_vt.ark::mono_return_type_name` の名前逆引き部分

修正対象:
- `src/compiler/wasm/code_locals.ark` — local型を `value_type` から直接取得
- `src/compiler/wasm/call_fallback.ark` — callee型を `func_id` から直接取得
- `src/compiler/mir/verify.ark` — warning を fail に切り替え（INV-5 完全執行）

## Phase 6a: Symbolic Alias

- `src/compiler/wasm/code_scratch_locals.ark` — 直接番号を symbolic alias に置き換え
- `src/compiler/wasm/ctx_scratch.ark` — 同上
- スクラッチローカル番号を直接書く全箇所

## Phase 6b: LocalAllocator

- `src/compiler/wasm/local_allocator.ark` — LocalAllocator, ScratchPool
- `src/compiler/wasm/code_scratch_locals.ark` — alias の実体を LocalAllocator に差し替え
- `src/compiler/wasm/ctx_scratch.ark` — スクラッチをプールから借用

## Phase 7: host intrinsic adapter 実装

- `src/compiler/wasm/call_host.ark` — HostIntrinsicSpec に従ってABI変換
- `src/compiler/wasm/code_body.ark` — host intrinsic のスタブ化を HostIntrinsicSpec で統一

---

## 検証コマンド

```bash
python3 scripts/manager.py verify quick
# 各 Phase 後:
# selfhost fixpoint --build で安定ビルドを確認
```

## リスクと対策

### R1: 自己ホストの連鎖的影響

コンパイラソースを変更すると、コンパイラ自身の挙動が変わる。

**対策**: 各 Phase 後に `selfhost fixpoint --build` で安定ビルドを確認。
Phase 1-3 は既存コードと並行動作させる（互換層を残す）。
最初の PR を小さく切り、emit 経路には触れない。

### R2: 移行期間中のパフォーマンス

TypeTable, SignatureRegistry のルックアップコスト。

**対策**: Vec 線形探索で十分（関数数は数百程度）。必要なら HashMap に移行。

### R3: スクラッチローカル移行のリグレッション

Phase 6 で LocalAllocator に移行する際、既存の固定 index コードが
全て壊れる可能性。

**対策**: Phase 6a で symbolic alias を先に導入し、直接番号を消してから
Phase 6b で LocalAllocator に差し替える。2 段階移行でリスクを分散。

### R4: 巨大差分による自己ホスト崩壊

**対策**: PR を小さく切る。型定義追加 → verifier ログ → CALL の func_id →
通常関数の registry 切替 → trait/generic 拡張、の順で段階的に進める。
各 PR 後に selfhost fixpoint を確認。

## 関連

- [ADR-040: Semantic Type Spine](../adr/ADR-040-typed-mir-signature-registry.md)
- [ADR-042: Intrinsic Layer Separation](../adr/ADR-042-intrinsic-layer-separation.md)
