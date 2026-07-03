# ADR-040: Semantic Type Spine — 意味情報を保存する背骨

ステータス: **Accepted; implementation in progress** (PR-3 完了。PR-4 は wide-audit フェーズ: 監査・compat 修正は並列可、emit 切替は PR-4-switch 単独)

### PR-4 実施レーン（emit 境界で分割）

| Lane | 目的 | emit変更 | 並列 |
|------|------|----------|------|
| A | registry vs legacy VT mismatch 分類・集計 | なし | 可 |
| B | `legacy_vt_compat` 修正（semantic spine は維持、旧 emit ABI 互換のみ） | なし | 可 |
| C | MonoInstanceTable subst/return_types 監査・修正 | なし | 可 |
| D | 通常 CALL の一致分のみ registry 経由（`registry_return_switch.ark`） | **あり** | A/B 後・単独 |
| E | trait/generic/host intrinsic 除外漏れ修正 | なし | 可 |
| F | docs / ADR 更新 | なし | 可 |

**PR-4-wide-audit**: Lane A–C,E を並列（fixture/callee カテゴリ分割可）。emit は原則不変。  
**PR-4-switch**: Lane D のみ。一致率が十分になってから単独 PR。  
**PR-4b-trait-generic**: trait/generic/mono の registry 切替。単独または少数 PR。

決定日: 2026-07-03

---

## 文脈

### 現状の崩壊

T3 WASM validation が 376/441 まで改善したが、残り41件の validate-fail は
局所修正の限界に出ている。Wave 6 では修正と新規失敗が相殺した。

根本原因は個別の型推論バグではなく、**コンパイルパイプラインの各段階で
意味情報（型、シグネチャ、ABI）が失われていること**。その結果、Wasm emitter
が「考古学のように名前やスタックから情報を掘り返す」状態になっている。

### 設計原則

> emitter を賢くするではなく、emitter を馬鹿にできるようにする。

Wasm emitter は、Typed MIR を機械的に Wasm へ変換するだけにする。
型不明なら fallback せず即 internal compiler error にする。

### 具体的な症状

1. **emitter での型推論**: `find_stack_value_source` (128ステップ制限) と
   `infer_ref_local_gc_type_depth` (深さ制限3) が、スタック履歴を遡って
   local の型を推論している。

2. **名前からの型逆引き**: `mono_return_vt.ark` が `pair__i32__String` のような
   マングル名を `substring` と `split` でパースして `(i32, String)` を復元。

3. **nullability の欠落**: `MirLocal.val_type` は `VT_GC_REF` 一種類。
   `(ref null $Point)` と `(ref $Point)` の区別がない。

4. **MonoInstance の置換マップ欠落**: `type_args: Vec<TypeInfo>` はあるが、
   「どの型パラメータに何が束縛されたか」のマッピングがない。

5. **TypedFn の情報不足**: 戻り値型のみ。パラメータ型、trait情報、ABI情報がない。

6. **スクラッチローカルの固定index依存**: 追加すると全インデックスが
   シフトして大規模リグレッション。

7. **host intrinsic の ABI 未定義**: `read_to_string` が i32 を返すが
   呼び出し元が Result の GC ref を期待。

---

## 決定事項

### D1: 3層型システム — TypeId / MirValueType / WasmValueType

言語レベルの型、コンパイラ内部の値型、Wasm出力型を明確に分離する。

```
TypeId        = 意味型（Point, String, Vec<T>, Result<T,E> ...）
MirValueType  = コンパイラ内部の値型（TypeId + repr + nullability）
WasmValueType = Wasm出力型（i32, f64, (ref null $N), (ref $N) ...）
```

#### Layer 1: TypeId — 意味型

```ark
struct TypeId(i32)

struct TypeTable {
    entries: Vec<TypeEntry>,
}

// intern: 同じ意味型には常に同じ TypeId を返す
fn TypeTable_intern(table: TypeTable, kind: TypeKind, name: String, type_param_ids: Vec<TypeId>) -> TypeId

struct TypeEntry {
    id: TypeId,
    kind: TypeKind,              // 意味型の分類
    name: String,                // "Point" / "String" / "Vec" / "Result"
    type_param_ids: Vec<TypeId>, // ジェネリック型パラメータ（Vec<T>のT等）
}

enum TypeKind {
    Primitive,   // i32, i64, f64, bool, char
    Struct,      // ユーザー定義構造体
    Enum,        // ユーザー定義列挙型
    Vec,         // Vec<T>
    Option,      // Option<T>
    Result,      // Result<T, E>
    HashMap,     // HashMap<K, V>
    HashSet,     // HashSet<T>
    String,      // String
    Closure,     // クロージャ
    TypeVar,     // ジェネリック型パラメータ T, U, ...
    TraitObject, // trait object（将来用）
}
```

**重要**: `TypeId` は言語レベルの型のみを表す。`Point` という `TypeId` は
1つだけ。`Nullable Point` と `NonNull Point` で別の `TypeId` を作らない。
nullability は `MirValueType` 側に持たせる。

**canonical化（intern規則）**: `TypeTable_intern(kind, name, type_param_ids)`
は、同じ `(kind, name, type_param_ids)` の組み合わせに対して常に同じ
`TypeId` を返す。`Vec<String>` を2回 intern しても同じ `TypeId` になる。
これにより、trait impl lookup や generic substitution で
「同じ意味型なのに別 ID」事故を防ぐ。

#### Layer 2: MirValueType — コンパイラ内部の値型

```ark
struct MirValueType {
    type_id: TypeId,         // 意味型
    repr: ValueRepr,         // 値の内部表現
    nullability: Nullability, // refの場合のnullability
}

enum ValueRepr {
    Scalar,         // i32, i64, f64, bool, char（スタック直置き）
    GcRef,          // GC参照型（struct, vec, string, closure等）
    LinearPtr,      // 線形メモリポインタ（host intrinsic互換用）
}

enum Nullability {
    Nullable,   // (ref null $type) — Option/Result/match分岐先等
    NonNull,    // (ref $type) — 直後にstruct.new等で生成された値
    NotRef,     // i32, f64等（nullability無関係）
}
```

**タプルの扱い**: `TupleExpand` は `ValueRepr` に入れない。
タプルは「値の表現」ではなく「lowering上の展開戦略」であり、
`MirInst.result_types: Vec<MirValueType>` で「複数の MirValueType を返す」
として自然に表現する。各要素はそれぞれ `Scalar` や `GcRef` を持つ。

#### Layer 3: WasmValueType — Wasm出力型

```ark
enum WasmValueType {
    WasmI32,
    WasmI64,
    WasmF64,
    WasmRef(WasmRefType),
}

struct WasmRefType {
    type_idx: i32,        // Wasm type section内のインデックス
    nullable: bool,       // true = (ref null $N), false = (ref $N)
}
```

**lowering**: `MirValueType -> WasmValueType` は GcLayoutTable (D5) が担う。
`TypeId` 単独では nullable/non-null や linear memory 互換表現が決まらないため、
lowering の入力は `MirValueType` とする。

### D2: SignatureRegistry — 関数シグネチャの中央台帳

trait impl、generic instance、stdlib関数、host intrinsic、builtin methodを
全部、名前文字列ではなく `FunctionId` で管理する。

```ark
struct FunctionId(i32)

struct SignatureRegistry {
    entries: Vec<SignatureEntry>,
    id_to_mangled: Vec<(FunctionId, String)>,  // 本体: ID → 出力名
    // name_to_id は resolver/debug/互換層用のみ（D7禁止規則参照）
}

struct SignatureEntry {
    id: FunctionId,
    base_name: String,              // "pair" / "to_string" / "read_to_string"
    param_types: Vec<MirValueType>, // パラメータの値型
    return_types: Vec<MirValueType>,// 戻り値の値型（複数値対応）
    receiver_type: Option<MirValueType>,  // メソッドのレシーバ型
    trait_id: Option<TraitId>,      // 属するtrait
    impl_id: Option<ImplId>,        // 属するimpl
    mono_instance_id: Option<MonoInstanceId>,  // モノモーフ化済みの場合
    abi_kind: AbiKind,              // Normal / HostIntrinsic / Builtin / TraitDispatch
}

enum AbiKind {
    Normal,         // 通常のArukellt関数
    HostIntrinsic,  // host境界関数（D6参照）
    Builtin,        // コンパイラ組み込み
    TraitDispatch,  // trait経由ディスパッチ
}
```

**重要**: `return_types` は `Vec<MirValueType>` とする。
単一戻り値でも `Vec` 長1。タプル返しは `Vec` 長N。
`nothing on stack` エラーの多くはタプル分解の型情報欠落が原因のため、
複数値対応を最初から持たせる。

### D3: Typed MIR — MirInst/MirLocal の型必須化

```ark
pub struct MirInst {
    op: i32,
    dest: i32,
    arg0: i32,
    arg1: i32,
    int_val: i32,
    float_val: f64,
    str_val: String,
    val_type: i32,                    // ← 廃止対象（互換期間中は派生して残す）
    result_types: Vec<MirValueType>,  // ← 新設: この命令の結果型（複数値対応）
    func_id: Option<FunctionId>,      // ← 新設: CALL命令の場合
}

pub struct MirLocal {
    name: String,
    val_type: i32,                    // ← 廃止対象（互換期間中は派生して残す）
    value_type: MirValueType,         // ← 新設: このlocalの値型
    type_name: String,                // ← デバッグ用のみ（意味情報の保存先ではない）
    ssa_name: String,
    ssa_version: i32,
}
```

**設計判断**: `result_types: Vec<MirValueType>` とした。
単一結果でも `Vec` 長1。値を返さない命令（store等）は `Vec` 長0。
タプルを返すが複数localに展開される命令は、展開後の各localに
それぞれ `MirValueType` が設定される。

**移行期間中の互換性**: `val_type` は `result_types[0]` または
`value_type` から派生して残す。`VT_I32` / `VT_F64` / `VT_GC_REF` は
`MirValueType.repr` から計算可能。

### D4: MonoInstanceTable — モノモーフ化の型情報保存

```ark
struct MonoInstanceId(i32)

struct MonoInstanceTable {
    entries: Vec<MonoInstanceEntry>,
    id_to_mangled: Vec<(MonoInstanceId, String)>,  // 本体: ID → 出力名
}

struct MonoInstanceEntry {
    id: MonoInstanceId,
    base_function_id: FunctionId,       // 元のジェネリック関数
    mangled_name: String,               // 出力名（意味情報の保存先ではない）
    subst: Vec<(String, TypeId)>,       // {T -> i32, U -> String} の明示的マッピング
    instantiated_return_types: Vec<MirValueType>,  // subst適用済みの戻り値型
    instantiated_param_types: Vec<MirValueType>,   // subst適用済みのパラメータ型
    signature_id: FunctionId,           // モノモーフ化後の関数のSignatureEntry
}
```

**効果**: `pair__i32__String` から `substring` と `split` で型を復元する処理を廃止。
`MonoInstanceId` から `instantiated_return_types` が一発で取れる。
`subst` マップが明示的に保存されるため、型パラメータ→具象型の対応が失われない。

### D5: GcLayoutTable — MirValueType から WasmValueType への lowering

```ark
struct GcLayoutId(i32)

struct GcLayoutTable {
    entries: Vec<GcLayoutEntry>,
}

struct GcLayoutEntry {
    id: GcLayoutId,
    type_id: TypeId,              // 対応するArukellt意味型
    wasm_ref_type: WasmRefType,   // (ref null $N) または (ref $N)
    struct_type_idx: Option<i32>, // Wasm type section内のインデックス
    field_layouts: Vec<FieldLayout>,
}

struct FieldLayout {
    field_name: String,
    field_type_id: TypeId,
    wasm_field_type: WasmValueType,  // WasmI32 / WasmI64 / WasmF64 / WasmRef(...)
    byte_offset: i32,     // 線形メモリ互換用
}
```

**lowering関数**: `lower_mir_value_type(MirValueType) -> WasmValueType`

```
MirValueType { repr: Scalar, ... }     → WasmI32 / WasmI64 / WasmF64
MirValueType { repr: GcRef, type_id, nullability }
    → GcLayoutTable[type_id].wasm_ref_type
      (nullability が Nullable なら nullable=true で上書き)
MirValueType { repr: LinearPtr, ... }  → WasmI32
```

**効果**: `String` → `(ref null $A_i8)`、`Vec<i64>` → `(ref null $A_i64)`、
`Point` → `(ref null $struct:Point)` の対応が明示的になる。
emitter は `MirValueType` を `lower_mir_value_type` に渡すだけで
`WasmValueType` が得られる。

### D6: HostIntrinsicSpec — host境界のABI明示的定義

```ark
struct HostIntrinsicSpec {
    name: String,              // "read_to_string"
    param_types: Vec<MirValueType>,
    return_types: Vec<MirValueType>,  // [Result<String, Error>]
    abi_kind: HostAbiKind,     // LinearMemoryPtr / GcRef / Scalar
    adapter: Option<FunctionId>,  // i32ポインタ → Result<String> の変換関数
    signature_id: FunctionId,  // SignatureRegistry内のエントリ
}

enum HostAbiKind {
    LinearMemoryPtr,  // 内部実装はi32ポインタ、adapter経由でGC refに変換
    GcRef,            // 内部実装もGC ref
    Scalar,           // スカラー値を直接返す
}
```

**効果**: `read_to_string` が i32 を返すなら、adapter 関数が
i32 → `(ref null $Result_String_Error)` に変換。
呼び出し元は常に `Result<String, Error>` を受け取る。
host intrinsic にも通常関数と同じ `SignatureEntry` が存在するため、
emitter 側は呼び出し経路を区別する必要がない。

### D7: LocalAllocator — 固定index依存の排除（中間段階付き）

#### 中間段階: Symbolic Alias

固定indexに名前を付け、直接local番号を書く箇所を0にする。

```ark
// Phase 6a: symbolic alias の導入
const SCRATCH_I32_0: i32 = 0   // 旧: 直接 0 を書いていた
const SCRATCH_I32_1: i32 = 1
const SCRATCH_F64_0: i32 = 12
const SCRATCH_GC_0: i32 = 16
// ...
```

全ての `emit_leb128_u(w, 16)` のような直接番号を
`emit_leb128_u(w, SCRATCH_GC_0)` に置き換える。

#### 最終段階: LocalAllocator

```ark
struct LocalAllocator {
    locals: Vec<AllocatedLocal>,
    scratch_pool: Vec<ScratchLocal>,
}

struct AllocatedLocal {
    mir_local_idx: i32,     // MIR内の論理インデックス
    wasm_local_idx: i32,    // Wasm出力でのインデックス
    value_type: MirValueType,
    kind: LocalKind,        // User / Param / Scratch
}

struct ScratchLocal {
    wasm_local_idx: i32,
    value_type: MirValueType,
    in_use: bool,
}
```

**移行順序**:
1. symbolic alias 導入（直接番号を消す）
2. alias の実体を LocalAllocator へ差し替え
3. スクラッチはプールから借りる方式に移行

---

## 不変条件 (Invariants)

以下の条件は compile-time invariant として扱い、違反は
internal compiler error とする。

### INV-1: CALL命令は必ず func_id を持つ

```
全ての MIR CALL命令は func_id: Some(FunctionId) を持つ。
func_id がない CALL は MIR 検証で落とす。
```

### INV-2: MirLocal は必ず MirValueType を持つ

```
全ての MirLocal は value_type: MirValueType を持つ。
VT_I32 fallback は禁止。未設定なら MIR 検証で落とす。
```

### INV-3: mangled_name を型解決に使うことを禁止

```
mangled_name は Wasm 出力名のみに使用。
semantic lookup（型解決、シグネチャ取得、戻り値型取得）で
mangled_name をキーに検索することを禁止。
型解決は必ず FunctionId / MonoInstanceId / TypeId を経由する。
```

### INV-4: emitter から TypeTable の文字列名を parse することを禁止

```
emitter は MirValueType -> WasmValueType の lowering のみを行う。
TypeTable.name や type_name 文字列を substring/split/starts_with
等で parse して型を推論することを禁止。
```

### INV-5: unknown type は i32 ではなく internal compiler error

```
型不明の場合、i32 に fallback せず internal compiler error とする。
ただし移行期間中は MIR verifier が warning を出すのみで
fail しない（Phase 3 完了後に fail に切り替え）。
```

### INV-6: result_types は命令の実際のスタック効果と一致する

```
MirInst.result_types の長さは、その命令がスタックに積む値の数と一致する。
値を返さない命令（store等）は result_types 長0。
タプルを返す命令は展開後の値数と一致。
```

### INV-7: TypeTable は intern 方式で同一性を保証する

```
TypeTable_intern(kind, name, type_param_ids) は同じ意味型に対して
常に同じ TypeId を返す。同じ意味型に複数の TypeId が存在することを禁止。
```

### INV-8: SignatureEntry と MirInst の型整合性

```
SignatureEntry.param_types と MirInst の引数型は MIR verifier で照合される。
SignatureEntry.return_types と MirInst.result_types は MIR verifier で照合される。
不一致は internal compiler error。
```

### INV-9: MonoInstanceEntry の整合性

```
MonoInstanceEntry.signature_id が指す SignatureEntry の param/return は、
instantiated_param_types / instantiated_return_types と一致する。
不一致は internal compiler error。
```

---

## 移行計画

### フェーズ順序（修正版）

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

### 各フェーズの詳細と終了条件

#### Phase 1: TypeTable + SignatureRegistry 骨格

**作成するファイル**:
- `src/compiler/corehir/type_table.ark` — TypeTable, TypeEntry, TypeId
- `src/compiler/corehir/mir_value_type.ark` — MirValueType, ValueRepr, Nullability
- `src/compiler/corehir/signature_registry.ark` — SignatureRegistry, SignatureEntry, FunctionId, AbiKind
- `src/compiler/corehir/host_intrinsic_spec.ark` — HostIntrinsicSpec, HostAbiKind（型定義のみ）

**修正するファイル**:
- `src/compiler/corehir/type_contracts.ark` — TypedFn に param_types, trait_id, impl_id, abi_kind 追加
- `src/compiler/typechecker.ark` — 型チェック結果を TypeTable と SignatureRegistry に登録

**終了条件**:
- 全既存 TypedFn が SignatureRegistry へ登録される
- AbiKind::HostIntrinsic の SignatureEntry が最低限存在する（adapter実装は不要）
- 既存テスト pass 数が悪化しない
- semantic lookup で mangled_name を使う新規コードがない

#### Phase 2: MonoInstanceTable

**作成するファイル**:
- `src/compiler/mir/lower/mono_instance_table.ark` — MonoInstanceTable, MonoInstanceEntry, MonoInstanceId

**修正するファイル**:
- `src/compiler/mir/lower/mono_param_subst.ark` — モノモーフ化時に subst マップを保存
- `src/compiler/mir/lower/fn_index_mono.ark` — MonoInstanceTable に登録

**終了条件**:
- 全モノモーフ化インスタンスが MonoInstanceTable に登録される
- subst マップが明示的に保存されている
- 既存の名前逆引きコードはまだ並行動作（削除しない）
- 既存テスト pass 数が悪化しない

#### Phase 3: Typed MIR

**修正するファイル**:
- `src/compiler/mir/inst_record.ark` — MirInst に result_types, func_id 追加
- `src/compiler/mir/local_record.ark` — MirLocal に value_type 追加
- `src/compiler/mir/lower/*.ark` — MIR lowering 時に MirValueType を必ず設定
- `src/compiler/mir/verify.ark`（新設）— MIR verifier: type 未設定箇所をログで報告

**終了条件**:
- 全 MirLocal に value_type が入る
- 全 CALL 命令に func_id が付く
- MIR verifier が type 未設定箇所を warning で報告（まだ fail しない）
- emitter はまだ旧 val_type を読む（並行動作）
- 既存テスト pass 数が悪化しない

#### Phase 4: GcLayoutTable

**作成するファイル**:
- `src/compiler/wasm/gc_layout_table.ark` — GcLayoutTable, GcLayoutEntry, WasmRefType, WasmValueType
- `src/compiler/wasm/lower_value_type.ark` — `lower_mir_value_type(MirValueType) -> WasmValueType`

**修正するファイル**:
- `src/compiler/wasm/sections_types_gc.ark` — 型セクションエミッション時に GcLayoutTable を参照
- `src/compiler/wasm/ctx_gc_type.ark` — MirValueType から GcLayoutId をルックアップ

**終了条件**:
- `lower_mir_value_type` が全 MirValueType に対して WasmValueType を返す
- 既存の ctx_gc_type の型判定が GcLayoutTable 経由に切り替わる
- 既存テスト pass 数が悪化しない

#### Phase 5: emitter から型推論を削除

**削除する関数**:
- `code_ref_locals_infer.ark::find_stack_value_source`
- `code_ref_locals_infer.ark::infer_ref_local_gc_type_depth`
- `code_ref_locals.ark::infer_ref_local_gc_type`
- `mono_return_vt.ark::mono_return_type_name` の名前逆引き部分

**修正するファイル**:
- `src/compiler/wasm/code_locals.ark` — local型を `value_type` から直接取得
- `src/compiler/wasm/call_fallback.ark` — callee型を `func_id` から直接取得
- `src/compiler/mir/verify.ark` — warning を fail に切り替え（INV-5 完全執行）

**終了条件**:
- `find_stack_value_source` の呼び出し回数 = 0
- `infer_ref_local_gc_type_depth` の呼び出し回数 = 0
- `mono_return_type_name` の名前逆引き回数 = 0
- 旧推論経路が呼ばれないことを確認
- MIR verifier が type 未設定を fail にする（INV-5 完全執行）
- CALL/local/result の型整合が MIR verifier で検査される（INV-8, INV-9）
- T3 validate-fail の減少は副作用として扱う（目標ではなく結果）

#### Phase 6a: Symbolic Alias

**修正するファイル**:
- `src/compiler/wasm/code_scratch_locals.ark` — 直接番号を symbolic alias に置き換え
- `src/compiler/wasm/ctx_scratch.ark` — 同上
- スクラッチローカル番号を直接書く全箇所

**終了条件**:
- `emit_leb128_u(w, 16)` のような直接番号記述が 0 件
- 全て `emit_leb128_u(w, SCRATCH_GC_0)` のような alias 使用
- 既存テスト pass 数が悪化しない

#### Phase 6b: LocalAllocator

**作成するファイル**:
- `src/compiler/wasm/local_allocator.ark` — LocalAllocator, ScratchPool

**修正するファイル**:
- `src/compiler/wasm/code_scratch_locals.ark` — alias の実体を LocalAllocator に差し替え
- `src/compiler/wasm/ctx_scratch.ark` — スクラッチをプールから借用

**終了条件**:
- スクラッチローカル追加で index シフトしない
- 既存テスト pass 数が悪化しない

#### Phase 7: host intrinsic adapter 実装

**修正するファイル**:
- `src/compiler/wasm/call_host.ark` — HostIntrinsicSpec に従ってABI変換
- `src/compiler/wasm/code_body.ark` — host intrinsic のスタブ化を HostIntrinsicSpec で統一

**終了条件**:
- 全 host intrinsic が SignatureRegistry 経由で呼び出される
- adapter 関数が i32 → GC ref 変換を行う
- `func 12では対応済みだがfunc 28では別経路` のような経路依存が 0 件
- T3 validate-fail の host intrinsic 系が 0 件

---

## 実装の粒度 — 最初のPR

**重要**: このADRを一度に全部実装しない。最初のPRは以下のみとする。

### PR-1: 型定義追加 + registry 構築（emit経路には未使用）

1. `TypeId`, `MirValueType`, `FunctionId`, `SignatureEntry` の型定義を追加
2. 既存の型情報から SignatureRegistry を埋める
3. **既存 emit 経路にはまだ使わせない**（並行して存在するだけ）

### PR-2: MIR verifier 追加（ログのみ）

1. MIR verifier を追加し、`type_id` 未設定箇所をログで数える
2. **最初から fail にはしない**（warning のみ）

### PR-3: CALL に func_id を付ける

1. CALL命令のみ `func_id` を付ける
2. まだ戻り値型は旧経路で取得

### PR-4a: 通常関数 CALL の戻り値型取得を registry へ切り替え

1. 通常関数（非trait・非generic）の CALL の戻り値型取得のみ
   `func_id -> SignatureEntry.return_types` へ切り替え
2. 旧経路（名前逆引き・val_type）はtrait/generic用に残す
3. **通常関数でregistry経路が安定することを確認**

### PR-4b: trait/generic call の戻り値型取得を registry へ切り替え

1. trait/generic call の戻り値型取得を registry へ切り替え
2. MonoInstanceTable の `instantiated_return_types` を使用
3. **この段階で41件のうちかなり動くはず**

---

## 健康指標

pass数ではなく、以下を0に近づけることを目標とする:

| 指標 | 現状 | 目標 |
|------|------|------|
| `find_stack_value_source` 呼び出し回数 | 1箇所 (128ステップ制限) | 0 |
| `infer_ref_local_gc_type_depth` 呼び出し回数 | 1箇所 (深さ制限3) | 0 |
| `mono_return_type_name` 名前逆引き回数 | 1箇所 (substring/split) | 0 |
| `val_type` のみで型判定している箇所 | 多数 | 0 (MirValueTypeに置き換え) |
| `type_name` 文字列パース回数 | 多数 | 0 |
| FunctionIdなしのCALL命令 | 全CALL命令 | 0 |
| nullability 未設定の MirLocal | 全MirLocal | 0 |
| i32 default に落ちた local | 不明 | 0 (internal compiler error) |
| mangled_name を semantic lookup に使う箇所 | 多数 | 0 |
| 直接local番号を書く箇所 | 多数 | 0 (symbolic alias経由) |

---

## リスクと対策

### R1: 自己ホストの連鎖的影響

コンパイラソースを変更すると、コンパイラ自身の挙動が変わる。

**対策**: 各Phase後に `selfhost fixpoint --build` で安定ビルドを確認。
Phase 1-3 は既存コードと並行動作させる（互換層を残す）。
最初のPRを小さく切り、emit経路には触れない。

### R2: 移行期間中のパフォーマンス

TypeTable, SignatureRegistry のルックアップコスト。

**対策**: Vec線形探索で十分（関数数は数百程度）。必要ならHashMapに移行。

### R3: スクラッチローカル移行のリグレッション

Phase 6 で LocalAllocator に移行する際、既存の固定indexコードが
全て壊れる可能性。

**対策**: Phase 6a で symbolic alias を先に導入し、直接番号を消してから
Phase 6b で LocalAllocator に差し替える。2段階移行でリスクを分散。

### R4: 巨大差分による自己ホスト崩壊

サブエージェントが巨大差分を作ってコンパイラを壊す可能性。

**対策**: PRを小さく切る。PR-1は型定義のみ。PR-2はverifierログのみ。
PR-3はCALLのfunc_idのみ。PR-4aで通常関数のemit経路に触れ、
PR-4bでtrait/genericに拡張する。
各PR後にselfhost fixpointを確認。

---

## 期待される効果

1. **T3 validate-fail 41件 → 0**: emitter が正しい型情報を取得できるため
2. **新規フィクスチャの追加時に型エラーが出ない**: パイプライン全体で型が保存されるため
3. **コンパイラの保守性向上**: 型推論の複雑さが MIR lowering に集約され、emitter が単純化
4. **デバッグの容易化**: 型不明の場合に即 internal compiler error が出るため、問題の早期発見
5. **スクラッチローカル追加の安全化**: LocalAllocator により index シフト問題が解消
