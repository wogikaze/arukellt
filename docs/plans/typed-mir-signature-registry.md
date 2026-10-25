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

### 進捗（2026-10-25, typed-MIR Phase 5.0/5.1 レーン）

Phase 5.0 完了:
- `src/compiler/mir/inst_record.ark` / `src/compiler/mir/block_inst_access.ark` —
  packed result は valid TypeId を要求する（INV-1）。legacy vt 由来の
  `TYPE_ID_INVALID` は result として数えない。
- `src/compiler/mir/inst_draft.ark` — `MirInstDraft_new_full` が legacy vt から
  fake result を作らない。
- `src/compiler/mir/lower/ctx_typed_mir_sync.ark` — valid な result TypeId を
  legacy vt で上書きしない（INV-2）。
- `src/compiler/mir/verify_local.ark` / `verify_call.ark` / `verify_report.ark` —
  W009（valid func_id + GC/ref dest + valid result TypeId なし）と W010（valid local
  TypeId + GcLayoutTable binding なし）を warning-only で追加。W007 は spine 側に
  valid TypeId がない呼び出しを要求しない。
- `src/compiler/wasm/code_locals.ark` — `--time` 時に rung 決定カウンタを出力する
  （emit は非変更）。fused emit（#851 Phase 3）では本体が lowering 中の ctx で
  吐かれるため、`mir/lower/emit_inloop.ark` の drain から同じフラグで出力する。
- 未参照だった `wasm/code_ref_locals_infer.ark`（9 行）と
  `wasm/code_ref_locals_stack_scan.ark`（72 行）を削除。
- 既存 regression の修正: `wasm/intrinsic_vec_access_gc.ark` の
  `build_get_unchecked_field_types` が非 GC target でも GcLayoutTable index を
  返していた。wasm32 ではその index が func type を指すため、`get_unchecked`
  struct temp が `(ref null <func type>)` で宣言され、i32 を store できず
  invalid module になっていた（`structs/vec_elem_field_get.ark` の wasm32、
  および fixpoint stage-3 の compiler wasm。7f5d56e50 系の main checkout でも
  再現する 5.0 以前の regression）。`is_gc_target` で gate して修正。

5.0 時点の census（`--time`、wasm32-gc、`stdlib_trait` / `stdlib_io` /
`question_mark` / `enums` / `structs` / `stdlib_string` の `run:` 447 件）:
`scalar_override=22997 direct_producer=9243 copied_local_set=4145 vec_type_name=3007
spine_string_early=1913 scalar_width=1431 named_type=458 vec_constructor=482
payload_extract=333 return_vec=250 enum_like=494 funcref=221 vec_access=90
i64_demotion=38 open_ref_fallback=7 get_unchecked_field=4 branch_join=1`。
`array_literal` / `direct_producer_late` / `spine_string_late` / `variant_slot` は 0。
W009=0, W010=0（`mir-verify` summary 自体が出ない = 全 warning 合計 0。
W009/W010 はその合計に含まれる）。

Phase 5.1（CALL result を callee 名から再導出しない）は保留:
仕様どおり `infer_call_result_gc_type_for_inst` を
`mir_inst_result_type_at(inst, 0)` + `SelfEmitCtx_wasm_storage_ref_type_idx_for_mir_value_type`
に置き換えると、次の wasm32-gc fixture が invalid Wasm になる
（pinned 58cc153e0 では valid）。

- `question_mark/from_error` / `question_mark/error_ecosystem` /
  `question_mark/user_error_type` / `enums/enum_struct_variant`

原因: `impl From<String> for AppError { fn from(...) -> AppError }` のような Self を
含む impl method で、SignatureRegistry の return MVT（= call result TypeId）が
`String` になり、callee MIR 関数の宣言 return（= 実際の wasm signature、`AppError`）
と食い違う。INV-3 はこのクラスで成立しない。旧 emitter の
`infer_call_result_gc_type_from_fn_at` rung がこの食い違いを吸収していた。
W009 では検出できない（TypeId は存在するが誤っている）。

次の 5.1 案: valid fid かつ MIR body がある呼び出しは
`func_id -> SelfEmitCtx_mir_index_for_func_id` で callee MIR 関数の宣言 return を
唯一の型源にする（callee 文字列を読まないので INV-4 は保たれる）。registry-by-id と
intrinsic table は body がない場合だけ使う。GcLayoutTable 参照は `is_gc_target` で
gate する（wasm32 に func type index を漏らさない）。enum-open / structref rung の
削除には引き続き typechecker の Self 解決（R9）が必要。

削除対象（履歴）:
- `code_ref_locals_infer.ark::find_stack_value_source` — 削除済み
- `code_ref_locals_infer.ark::infer_ref_local_gc_type_depth` — 削除済み
- `code_ref_locals.ark::infer_ref_local_gc_type` — 削除済み
- `mono_return_vt.ark::mono_return_type_name` の名前逆引き部分 — 未着手（R2）

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
