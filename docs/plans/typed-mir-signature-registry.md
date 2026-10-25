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

### 進捗（2026-10-25/26, typed-MIR Phase 5.0/5.1 レーン）

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

Phase 5.1 の試行錯誤（履歴）: 仕様どおり `infer_call_result_gc_type_for_inst` を
`mir_inst_result_type_at(inst, 0)` + `SelfEmitCtx_wasm_storage_ref_type_idx_for_mir_value_type`
に置き換えると、次の wasm32-gc fixture が invalid Wasm になった
（pinned 58cc153e0 では valid）。

- `question_mark/from_error` / `question_mark/error_ecosystem` /
  `question_mark/user_error_type` / `enums/enum_struct_variant`

原因: `impl From<String> for AppError { fn from(...) -> AppError }` のような impl
method で、SignatureRegistry の return MVT（= call result TypeId）が `String` になり、
callee MIR 関数の宣言 return（= 実際の wasm signature、`AppError`）と食い違う。
INV-3 はこのクラスで成立しない。旧 emitter の
`infer_call_result_gc_type_from_fn_at` rung がこの食い違いを吸収していた。
W009 では検出できない（TypeId は存在するが誤っている）。

5.1 実装（2026-10-26、wave/phase5-1）: valid `func_id` かつ MIR body がある呼び出しは
`func_id -> SelfEmitCtx_mir_index_for_func_id` で callee MIR 関数の宣言 return を
唯一の型源にする（callee 文字列を読まないので INV-4 は保たれる）。registry-by-id は
body がない場合だけ、intrinsic/builtin-by-name table は `fid < 0` の場合だけ使う。
`ctx_enrich_inst_result_types` は registry return の TypeId が invalid な CALL に
`result_types` を捏造せず空のまま残す（W009 が拾う）。

- `code_ref_locals_infer_callee.ark`: `infer_call_result_gc_type_from_fn_at` /
  `resolve_fn_index_for_inference` / from_fn rung / 名前だけの entry point を削除。
  `infer_call_result_gc_type_from_mir_function`（body あり）/
  `infer_call_result_gc_type_from_registry_id`（body なし）/
  `infer_intrinsic_call_result_gc_type_by_name`（fid < 0）へ整理。
- bare `Result` / `Option` の return 名は関数 signature（`gc_enum_return_type_sig`）と
  同じく enum-open に解決する（INV-5）。
- typechecker 5.7 前提: `impl_method_sig` は
  `corehir::decl_impl_target::impl_method_mangled_name` を使い、重複していた
  `impl_self_type_name` を削除。`__<source>` suffix は canonical FnSig 名になり、
  source-level `Owner::method` は index alias として残す（associated call の lookup
  を維持）。non-generic impl の `Self` return は
  `associated_types::resolve_impl_self_return` で impl target に解決する。
- enum-open / structref rung の削除は 5.7 本体（concrete enum payload TypeId /
  typechecker の overload 解決）待ち。

5.1 の検証（wasm32-gc / wasm32）:
- 回帰 4 件 + `structs/vec_elem_field_get` は両 target で compile+validate。
  `from_error` / `error_ecosystem` / `user_error_type` は component 実行で `OK`、
  `enum_struct_variant` は `75`/`24`、`vec_elem_field_get` は `x`。WAT は 5.0
  baseline（f03493a36 の s2）と byte-identical。`from_error` の `$process` の From
  dest `$_t5` は `(ref null 14)`（enum-open）で、`(ref null 0)`（String）ではない。
  wasm sha256 は pinned 58cc153e0 の出力とも一致（`820a3c41…`）。
- #693 IO lane probe（inherent / trait impl の `Result<Vec<i32>, E>` 直接呼び出し）は
  call dest が structref fallback から `(ref null 14)` になり、`ref.cast` も消える。
  free-call と generic-bound control の WAT は不変。
- `verify lane` / `verify fixtures`（1805 compile+validate、102 runtime smoke）green。
- `selfhost fixpoint`: 本レーンは未達（s2 `c01861e1…` vs s3 `bca6a14c…`）。
  emitter/typechecker 変更が自己コンパイル出力を変えるためで、5.0 以降 main 側で
  続いている fixpoint 診断とは独立。pinned 再 pin までは未達のまま。
- 既知の残り: enum-open / structref rung の削除（5.7 本体）、`resolve_call_fn_ret_vt`
  の call ABI vt は registry-first のまま（dest は MIR return と一致）、
  `mono_return_vt` の名前逆引き（R2）、typechecker の source-spelling alias。

### 進捗（2026-10-26, typed-MIR 5.7 前提）

5.1 が残した「`Self` を返す impl method」の signature 同一性と INV-5 を実装した。

- typechecker `associated_types::resolve_impl_self_return` は generic impl でも
  `Self` を impl head に解決する。impl head の型引数は type variable のまま残すため、
  MonoInstance が後から具体型を代入できる。`impl<T> Wrap<T> { fn make(value: T) -> Self }`
  の body 戻り型検査が通る（base は E0200）。
- corehir `return_type_name::return_index_type_name_for_impl` は impl method の
  return 注釈の `Self` token を impl target 名へ置換する。MIR lowering と
  SignatureRegistry は impl context を持たないため、`Self` のままだと call dest が
  scalar/structref に宣言される。`impl Factory<i32> for Boxed { fn make(...) -> Self }`
  の call dest は `(ref null 2)`（concrete struct TypeId）になり、base で
  `type mismatch: expected i64` だった module が validate する。
- `wasm/registry_return_resolve::resolve_call_fn_ret_vt` は call ABI も
  `func_id -> SelfEmitCtx_mir_index_for_func_id -> MirFunction declared return` から
  読む（INV-5）。registry view は audit 計測専用に残す。
- 5.7 class の rung を 2 本削除した。`code_locals_struct::emit_gc_local_named_type` の
  Option→enum-open 分岐は、同じ prefix を無条件で先に処理する
  `emit_gc_local_enum_like` の後には到達しない。`code_locals::emit_gc_local_val_type`
  の variant_slot rung は enum_like の variant 分岐と同じ guard で、census 0。
  enum-open の名前分岐と structref fallback は残す（下表）。

5.7 前提レーンの検証（s2 sha256 `747e8396…`、base s2 `60649bd6…`）:
- 4 question_mark/enums fixture + `try_from_parse` / `try_from_narrow` +
  `debug_result` / `debug_option` + `structs/vec_elem_field_get` の 9 件を
  wasm32-gc / wasm32 で compile+validate。18 出力の WAT と wasm は base s2 と
  byte-identical（sha256 一致）。
- P2 component 実行は base と同じ: `from_error` / `error_ecosystem` /
  `user_error_type` = `OK`、`enum_struct_variant` = `75`/`24`、
  `vec_elem_field_get` = `x`、try_from/debug は assert で exit 0。
- `from_error` WAT: `$AppError::from__String` の Io constructor dest は
  `(ref null 40)`（type 40 = `(sub final 14 (struct (field i32) (field (ref null 0))))`）、
  `$process` の Result::Err payload dest は `(ref null 42)`、Ok dest は
  `(ref null 37)`。From call dest は callee の enum ABI と同じ `(ref null 14)`。
- Self probe（fixture 外、base で fail する形）: non-generic trait impl `-> Self` は
  invalid Wasm → valid + `7`、generic inherent impl `-> Self` は E0200 → valid + `7`、
  両者を同時に呼ぶ probe は `16`。wasm32-gc component と wasm32 core の両方で確認。
- `verify lane` PASS。`verify fixtures` PASS（1810 compile+validate、104 runtime
  smoke）。`verify quick` / fixpoint は本レーンのゲートではない。

残り:
- generic **trait** impl の `Self` return を associated call すると、corehir の
  return 名が impl owner の bare 名（`G`）になるため call dest が String のままに
  なる probe がある（impl 側を `-> G<T>` と明示すると base でも valid）。修正は
  owner を impl head の instantiation 綴り（`G<T>`）で置換することだが、
  本レーンは 1 build 制約のため未変更・未検証。
- enum-open 名前分岐 / structref fallback: `open_ref_fallback` は対象 9 fixture では
  0 だが 5.0 census では全 fixture で 7 件あり、削除は full set で concrete TypeId が
  揃うまで行わない。
- `stmt_let_store` の enum/Result annotation 伝播と `mono_return_vt` の名前 fallback
  （R2）は残る。
- `selfhost fixpoint` は本レーンで未実行。fixpoint の可否はこの変更の主張に含めない
  （pinned / 5.1 base の fixpoint 状態は本レーンとは独立）。

削除対象（履歴）:
- `code_ref_locals_infer.ark::find_stack_value_source` — 削除済み
- `code_ref_locals_infer.ark::infer_ref_local_gc_type_depth` — 削除済み
- `code_ref_locals.ark::infer_ref_local_gc_type` — 削除済み
- `code_ref_locals_infer_callee.ark::infer_call_result_gc_type_from_fn_at` — 5.1 で削除
- `code_ref_locals_infer_callee.ark::resolve_fn_index_for_inference` — 5.1 で削除
- `typechecker/module_fns.ark::impl_self_type_name`（`decl_impl_target` との重複） —
  5.1 で削除
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
