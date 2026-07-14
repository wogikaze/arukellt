# ADR-042: Intrinsic Layer Separation — 意味と実装の分離

ステータス: **PROPOSED** — intrinsic を 5 層に分離し emitter から stdlib 実装を除去する方針を提案

提案日: 2026-07-10

---

## 文脈

### 設計上の問題

intrinsic 周辺の構造には以下の問題がある:

1. **callee 文字列による dispatch**: `call_dispatch_table.ark` 等が MIR に保存された
   callee 文字列を見て `host → seq → text → scalar → parse → vec → simd` の
   順で振り分ける。各層で `eq(clone(callee), "starts_with")` のような文字列比較を
   行っている。`func_id_raw` が既に存在するにもかかわらず、intrinsic 判定
   は文字列ベースである。

2. **emitter が実装を持っている**: `starts_with`、`trim`、`sort`、`map` 等、
   Arukellt コードで書ける操作の実装が Wasm emitter 内にある。これらは
   言語プリミティブではなく、単に「今インライン化がないから emitter に書いた」
   だけのものである。

3. **GC / LM 二重実装**: `intrinsic_string_split.ark` と
   `intrinsic_string_split_gc.ark` のように、多くの intrinsic に GC 版と
   linear-memory 版の二つの実装がある。stdlib 操作が representation-specific
   lowering まで抱えているために起きている。

4. **prelude がスタブ**: `std/prelude.ark` は型チェック用の宣言のみで、
   本体は backend から除外されている。一部の関数は Ark で書かれているように見えても、
   実際にはドキュメント兼シグネチャ用スタブである。

5. **effect 情報不足**: 現在の LICM は opcode を列挙して pure かどうかを判定
   している。CALL の effect が SignatureRegistry にあれば、pure な stdlib 関数の
   ループ外移動、DCE、CSE が可能になる。

6. **ADR-040 との不整合**: ADR-040 は「emitter を馬鹿にする」方向で
   `FunctionId`、`SignatureRegistry`、Typed MIR を導入した。intrinsic dispatch が
   callee 名ベースのままでは、ADR-040 の spine が intrinsic dispatch まで到達しない。

### 根本的な認識の誤り

現在の設計では「コンパイラが知っているか／知らないか」の二択で境界を引いている。
これは間違いである。分離すべきは 3 つの軸である:

1. **コンパイラが意味を知っているか** (semantic knowledge)
2. **実装本体を誰が持つか** (implementation ownership)
3. **最終的なコード生成を誰が行うか** (code generation)

Arukellt では、**コンパイラは多数の標準ライブラリ操作の意味を知っていてよいが、
その実装まで Wasm emitter が持つべきではない**。

## 提案する決定

### D1: 5 層構造の採用

| 層 | 実装場所 | コンパイラの知識 | 例 |
|----|----------|-----------------|-----|
| 言語プリミティブ | MIR/backend | 完全に知る | GC 参照操作, raw array access, trap, indirect call |
| ランタイム ABI | runtime/host | ABI と effect だけ知る | allocator, panic handler, WASI/WIT imports |
| semantic stdlib | Ark 標準ライブラリ | 意味だけ知る | `Vec.len`, bounds check, String byte length |
| 通常 stdlib | Ark 標準ライブラリ | 通常関数として扱う | `split`, `trim`, `sort`, `map`, `parse` |
| target intrinsic | target 固有 module | 対象 backend だけ知る | raw V128 load/store, relaxed SIMD, Wasm 固有 memory/table/ref 操作, Wasm 固有 trap/value category |

fixed SIMD arithmetic（例: `I32x4` add）は target intrinsic ではなく portable `std::simd`（D7/D8）。

### D2: semantic stdlib 層の導入

Swift が Array、String、Dictionary を Swift 標準ライブラリ自身で実装しつつ
`@_semantics("array.count")` のような意味タグを付けるのと同様に、
Arukellt でも `Vec.len` は Ark 関数として本体を持たせながら、
SignatureRegistry 上では `CoreOpId::VecLen` を持たせる。

通常時は普通に Ark 関数を呼ぶ。最適化時だけ意味に基づく変換・置換を行う。
「std に実装がある」と「コンパイラが意味を知っている」を両立する。

### D3: 真の primitive の判定条件

compiler intrinsic（言語プリミティブ層）にしてよいのは、**次のいずれかを満たすもののみ**:

- それ自身なしでは定義できず、循環する（例: GC ref 操作、trap）
- Ark の型や値では保存できない意味がある（例: provenance、volatile）
- GC、atomic、write barrier など、最適化時にも保持すべき特殊な意味がある
- コンパイラがユーザーコード中に自動挿入する（例: bounds check 失敗時の trap）

target intrinsic（`std::wasm` 等）にしてよいのは、次を満たす場合のみ:

- **操作の意味そのものが Wasm 固有**である、または
- Wasm の raw value category / linear memory / relaxed semantics /
  Wasm 固有 trap 規則を**直接露出**する

「ターゲットに 1 対 1 の命令が存在する」だけでは不十分である。
命令が存在しても、意味がターゲット非依存の semantic operation なら
`std::bits`・整数メソッド・`std::simd` 等に置き、backend が対応命令へ lowering する。

**次の理由だけでは intrinsic にしない**（実装都合は ADR に固定しない）:

- 現在インライン化がない
- Ark 実装より手書き Wasm の方が速い（可能性）
- ホストアクセスが必要（→ runtime/import でよい）
- emitter で書く方が簡単
- いつか最適化するかもしれない
- Wasm（や他 ISA）に同名命令がある（例: `popcnt`）

### D4: Vec/String の内部表現は sealed raw API 経由のみ

Vec/String の内部表現は一般ユーザーコードには公開しない。
stdlib 専用の sealed raw API からのみ触れるようにする。

**モジュール名は未決。** 候補: `core::raw`、`core::rt`、`core::intrinsics`。
最終決定は別 RFC で行う。

概念的には `raw_array_new<T>`、`raw_array_len<T>`、
`raw_array_get_unchecked<T>`、`raw_array_set_unchecked<T>`、
`raw_array_grow<T>` 等の最小 API に依存し、その上に Ark で
`Vec::new`、`Vec::push`、`Vec::get` 等を実装する。

`len`/`push`/`get` そのものを言語構文や真の primitive にはしない。

### D5: SignatureEntry の拡張

ADR-040 の SignatureEntry に以下を追加する:

| 情報 | 例 | 説明 |
|------|-----|------|
| `core_op_id` | `string.starts_with`, `vec.len`, `simd.i32x4.add` | core-ops 内の canonical id |
| `effect` | memory / allocates / may_trap / noreturn / external_io / nondeterminism / atomic / volatile | 直交属性セット |
| `const semantics` | 定数評価可否、overflow/NaN/trap 規則 | |
| `inline_policy` | `never`, `hint`, `always` | `always` は意味上の保証でなく、強い inlining hint |
| `lowering` | `normal_call` / `mir_op` / `runtime_call` / `target_intrinsic` | variant ごとに必要な payload を含む |
| `fallback` | 安定 implementation symbol path | public path ではなく内部実装 symbol。FunctionId は永続化しない |
| `exposure` | `public` / `internal` / `runtime` / `target_raw` | 公開性と binding 必須性 |
| `binding_required` | bool | `public` でも `std/manifest.toml` への即時 binding が不要な場合がある |

**宣言の単一正本（designated SSOT）:** `data/core-ops.toml`（GHC `primops.txt.pp` 方式）。
人間向け概要は [`docs/compiler/core-ops-registry.md`](../compiler/core-ops-registry.md)。

- `core-ops.toml` は **将来の authoritative 正本**であり、現状では `status = "scaffold"`。
  ADR-042 採択後、#798 で compiler との統合が完了するまで authoritative 入力ではない。
- `core-ops.toml` が semantic types / CoreOpId / effect / lowering / fallback symbol / inline policy / signature / exposure の正本となる。
- `std/manifest.toml` は公開 path・docs・stability・deprecation の正本であり、
  `core_op_id` / `type_id` で `core-ops.toml` を **参照**する（manifest 全体を core-ops から生成しない）。
- 一つの semantic operation を複数の公開 API から参照してよい
  （例: `i32.popcount()` と `std::bits::popcount(i32)` が同じ `bits.popcount.i32`）。
- **public path を core-ops の fallback として書かない。**
  fallback は内部実装 symbol または primary public binding 用の専用 ID に向ける。
  public path は `std/manifest.toml` のみが持つ。
- generator / checker は `core-ops.toml` と `std/manifest.toml` の
  参照存在、signature 互換、effect/lowering 整合、
  public binding の未参照・衝突、fallback 条件を検査する。
- resolver / typechecker / MIR / docs / runtime ABI 表は将来、
  検査済み `core-ops.toml`（および検査済み参照）から読む。手作業二重登録禁止。

**signature の統一形式:**

`core-ops.toml` の `signature` は receiver 非依存の `inputs` 列に統一する。
`receiver_index`（0-based）でどの input が receiver かを示す。

例:

```toml
[operations.signature]
inputs = [
  { name = "s", type = "string" },
  { name = "prefix", type = "string" },
]
receiver_index = 0
outputs = [{ type = "bool" }]
```

`std/manifest.toml` の `params` は `inputs` と位置・型が対応するように書く。
型は `type_id` で core-ops 型を参照するか、組み込み primitive 名（`i32`, `bool`, `unit` 等）を使う。

**effect モデルは単一 enum にしない。**
`pure` は memory=none / allocates=false / may_trap=false / noreturn=false / external_io=false / nondeterminism=deterministic 等の導出値である。
最低限必要な直交属性は:

- `memory`: `none` / `read` / `write` / `read-write`
- `allocates`: bool
- `may_trap`: bool
- `noreturn`: bool
- `external_io`: bool
- `nondeterminism`: `deterministic` / `clock` / `random` / `external`
- `atomic`: bool
- `volatile`: bool

`pure` と `readnone` や `may_trap` と別 `may_trap` フラグを併存させるような
重複表現は避ける。

**lowering variant の payload:**

| `lowering.kind` | 必要な情報 |
|-----------------|------------|
| `normal_call` | `[lowering]` 内の `kind` と `[fallback]` の `implementation_symbol` |
| `mir_op` | `[lowering.mir]` の `opcode` または `operation` |
| `runtime_call` | `[lowering.runtime]` の `function`, `abi`（`native` / `wit`）, 任意で `interface` / `version` |
| `target_intrinsic` | `[lowering.target]` の `target_family`, `target_id`, `required_features` |

**portable SIMD lowering:**

portable `std::simd` 操作の `lowering.kind` は `normal_call` とする。
scalar フォールバックを `[fallback]` に置き、
ネイティブ SIMD 特殊化は `[[operations.specializations]]` で表す。

例:

```toml
[operations.lowering]
kind = "normal_call"

[[operations.specializations]]
when = { portable_simd_lowering = "NativeSimd" }
backend = "wasm"
target_op = "i32x4.add"

[operations.fallback]
implementation_symbol = "example.invalid.fallback.simd_i32x4_add"
required = true
```

`portable_simd_lowering` は `NativeSimd | Scalar | Unsupported` の 3 値をとる。
raw `std::wasm` の `v128` 操作は `wasm_raw_v128 = Enabled` が必要で、
relaxed SIMD 操作は `wasm_relaxed_simd = Enabled` が必要（詳細は ADR-037）。

**validation ルール:**

`core-ops.toml` の `[validation]` でチェックを有効化する。
各 checker は `operations.exposure` と `operations.binding_required` を見て
条件を変える。

- `check_unreferenced_public_bindings`: `exposure = "public"` かつ `binding_required = true` の operation が `std/manifest.toml` から `core_op_id` で参照されていることを確認。
- `check_public_binding_collisions`: 同じ public symbol（正規化 path）が複数の `core_op_id` に対応しないことを確認。
- `check_signature_compat`: `core_op_id` 経由で結ばれた manifest entry の `params` / `returns` が `core-ops` の `signature` と位置・型で対応することを確認。
- `check_effect_lowering_consistency`: `lowering.kind` と `effect` が矛盾しないことを確認。
- `check_fallback_resolvable`: `fallback.required = true` な operation に `fallback.implementation_symbol` が設定されていることを確認。`target_intrinsic` 等で fallback 不要な場合は `fallback.required = false`。

移行中は既存 manifest を拡張してよいが、実装開始前に core-ops 参照モデルへ切り替える（本 ADR の D5 決定）。

resolver、typechecker、MIR、docs、runtime ABI 表は SSOT から生成する方針とする。
現状 `std/manifest.toml` と resolver/typechecker 間に不一致があるため、
生成パイプラインは実装計画で具体化する。

### D6: callee 文字列 dispatch の廃止

intrinsic 判定を callee 名から MIR に保存された `FunctionId` による
`SignatureRegistry` 参照へ移行する。
`SignatureRegistry` から `CoreOpId` / `LoweringKind` を取得し、
それによって dispatch する。

`func_id_raw` は `FunctionId` を `MirInst` 内に保存するための物理表現にすぎない。
backend が raw な `FunctionId` 値を意味キーとして扱うと、
登録順・monomorphization・module 結合順に依存する暗黙 ABI になるため禁止する。
名前は診断表示にだけ使う。

close gate はファイル名や特定の構文ではなく、次の semantic invariant を検査する:

- call lowering が診断用途以外で callee 文字列を参照しない
- 全 CALL の `FunctionId` が `SignatureRegistry` entry へ解決される
- non-normal lowering が必ず一意な `CoreOpId` を持つ
- host operation の lowering owner が runtime ABI 層である
- public binding と semantic signature が整合する
- fallback と specialized lowering の differential test が通る

### D7: target intrinsic の名前空間

target-specific intrinsic の公開場所は **`std::wasm`** とする（ADR-037 と整合）。

- **portable SIMD**: `std::simd`（nominal lane 型 `I32x4` / `F32x4`、`MaskN`、
  `select` / `bit_select`、同幅 `reinterpret`、bitwise and/or/xor/not、any/all/bitmask 等）
- **raw / Wasm-specific**: `std::wasm`（`std::wasm::V128`、`v128.load` / `v128.store`、
  Wasm linear-memory アドレス空間の露出、relaxed SIMD、Wasm 固有 trap/value category）

portable nominal SIMD は `normal_call`  lowering と `specializations` で実現する。
raw `std::wasm::V128` は `target_intrinsic`  lowering とする。

| 概念 | `lowering.kind` | 条件 |
|------|----------------|------|
| portable `std::simd` | `normal_call` + `specializations` | `portable_simd_lowering` |
| raw `std::wasm` | `target_intrinsic` | `wasm_raw_v128 = Enabled` |
| relaxed SIMD | `target_intrinsic` | `wasm_relaxed_simd = Enabled` |

一般的な `Vec::push` や `String::split` が Wasm API に依存してはならない。
`std::arch::wasm` は採用しない（`std::wasm` に統一）。

### D8: target intrinsic にしない例（negative examples）

Wasm に命令が存在しても、次は **target intrinsic にしない**
（semantic stdlib / 整数メソッド / `std::simd` へ）:

| 操作 | 置き場の例 | 理由 |
|------|-----------|------|
| `popcnt` / `clz` / `ctz` | `std::bits` または整数メソッド | ターゲット非依存のビット演算 |
| `rotate_left` / `rotate_right` | 同上 | 同上 |
| `byte_swap` | 同上 | 同上 |
| fixed SIMD arithmetic（`f32x4.add` 等） | `std::simd` | portable vector semantics |
| `bit_select`（bit-wise） | `std::simd` | 128-bit bit 選択。意味は ISA 非依存に定義可能（lane `select` とは別） |
| lane `select(MaskN, …)` | `std::simd` | boolean lane 選択 |
| 同幅ビット列としての `reinterpret` | 型変換 API（portable） | 「同じ幅のビットを別型として見る」は非 Wasm 固有 |

`std::wasm` に置くのは、意味が Wasm 固有、または raw / relaxed / trap /
linear-memory 規則を直接露出する場合に限る。

## 要別 ADR / RFC

以下は本 ADR のスコープ外とし、別文書で決定する:

- **prelude のコンパイル対象復帰**: `combine_loaded_and_main_decls_skip_prelude` の
  廃止と prelude 本体の backend 通過。型チェック専用スタブからの移行は
  破壊的変更を伴うため、専用 ADR / RFC が必要。
- **sealed raw API のモジュール名と公開面**（D4 候補の最終決定）

## 等価性検証

semantic lowering には必ず Ark fallback body を残す。
最適化 ON/OFF、GC/LM、各 target について、Ark fallback 版と
optimized lowering 版を同じ入力で実行し、結果と副作用が一致する
differential test を置く。compiler 特殊化が std の意味から乖離するのを防ぐ。

旧文字列 dispatch から新 `SignatureRegistry` dispatch への移行には、
切り替え前に両方の結果を比較する shadow mode を用いる。
一致率 100% を新 dispatch への切り替え gate とする。

## ADR-040 との関係

本 ADR は ADR-040 / [RFC-002](../rfcs/002-semantic-type-spine.md) の拡張である。
ADR-040 は型情報の spine 化を行った。
本 ADR はそれを呼び出し意味・effect・intrinsic lowering まで拡張する。
実質的に ADR-040 を型情報だけで終わらせず、semantic spine を完成させる。

移行段階・機能の移動先・目標規模は
[`docs/plans/intrinsic-layer-separation.md`](../plans/intrinsic-layer-separation.md) を参照。

## 参考文献

- Swift: `@_semantics` attribute と HighLevelSILOptimizations
  ([swift/docs/HighLevelSILOptimizations.rst](https://github.com/swiftlang/swift/blob/main/docs/HighLevelSILOptimizations.rst))
- LLVM: Intrinsic の意味・型・メモリ効果定義
  ([LLVM Language Reference Manual](https://llvm.org/docs/LangRef.html))
- Rust: MIR における binary/unary/index 操作と lang items
  ([Rust Compiler Development Guide](https://rustc-dev-guide.rust-lang.org/mir/construction.html),
  [Lang Items](https://rustc-dev-guide.rust-lang.org/lang-items.html))
- Go: 高水準構文から runtime call への lowering
  ([Go Compiler README](https://go.dev/src/cmd/compile/README))
- GHC: `primops.txt.pp` + `genprimopcode` による primitive 宣言の表化
  ([GHC primops.txt.pp](https://gitlab.haskell.org/ghc/ghc/-/blob/master/compiler/GHC/Builtin/primops.txt.pp))
- Rust `core::intrinsics`: 標準ライブラリ実装詳細としての intrinsic
  ([core::intrinsics](https://doc.rust-lang.org/core/intrinsics/index.html))
