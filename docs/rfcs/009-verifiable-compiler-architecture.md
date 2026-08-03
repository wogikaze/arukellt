# RFC-009: Verifiable Compiler Architecture

ステータス: DRAFT

関連 ADR: [ADR-051](../adr/ADR-051-verifiable-compiler-architecture.md)

関連 RFC: [RFC-010: Proof-Driven Development](010-proof-driven-development.md)

関連 plan: [verifiable-compiler-migration.md](../plans/verifiable-compiler-migration.md)

日付: 2026-07-30

---

## 1. 目的

本RFCは、Arukelltを「後段が不足情報を推測するコンパイラ」から、「各段階が検証済みsnapshotを生成するコンパイラ」へ変更する詳細仕様を定める。

対象はユーザープログラムの契約検証だけではない。frontend、Core、CFG、MIR、backend、runtime ABIまで含めて、次を達成する。

- 不正状態を表現しにくい。
- passの責務が小さく、単体で検証できる。
- コンパイル結果が決定的である。
- 各境界をserializeして独立checkerへ渡せる。
- バグ発生時に、どのpassが意味を壊したか特定できる。
- 形式証明を段階的に導入できる。
- RFC-010が定めるproof-driven development loopの信頼できる基盤になる。

## 2. 全体パイプライン

```text
bytes
  │ decode
  ▼
SourceFileSet
  │ lex + parse
  ▼
ParsedModule
  │ name resolution
  ▼
ResolvedModule
  │ type/effect checking
  ▼
TypedModule
  │ semantic desugaring
  ▼
SemanticCore
  ├───────────────┐
  │ executable     │ logical extraction
  ▼                ▼
TypedCFG       VerifiedCore
  │ SSA            │ VC generation
  ▼                ▼
TypedSSA       VerificationBundle
  │ repr select     │ backend adapter
  ▼                ▼
ReprIR          Why3 / Lean / SMT
  │ target lower
  ▼
TargetIR
  │ emit
  ▼
Artifact + TrustManifest
```

`SemanticCore`が実行意味と証明意味の最後の共有点である。それ以降、実行系と論理系は別の所有構造を持つ。

## 3. 共通基盤

### 3.1 nominal ID

裸の整数indexをmodule境界へ出さない。

```text
FileId
SpanId
ModuleId
ItemId
FunctionId
TypeId
ExprId
LocalId
BlockId
ValueId
ProofExprId
RegionId
LayoutId
SignatureId
```

異なるdomainのIDは代入不能でなければならない。serialization時も `{kind, index}` またはdomain別fieldを使い、単一の汎用整数IDへ潰さない。

### 3.2 arena ownership

各snapshotは自分のarenaを所有する。arena間の参照は、そのsnapshot内のnominal IDだけで行う。

- snapshotをまたぐ直接node参照は禁止。
- global mutable singleton tableは禁止。
- passはinput arenaを変更しない。
- output arenaはbuilderが所有し、freeze後は不変とする。
- debug dumpのために巨大arenaを値コピーしない。

### 3.3 canonical hash

各snapshotはcanonical serializationからhashを計算する。

```text
SnapshotHash = hash(schema_version || semantic_version || canonical_payload)
```

map順、source absolute path、時刻、乱数、process固有値をhashへ含めない。

### 3.4 provenance

全nodeはoriginを持つ。

```text
Origin {
  primary_span: SpanId,
  parent_origin: Option<OriginId>,
  produced_by: PassId,
  source_node: Option<StableNodeId>
}
```

内部生成nodeも「最寄りspan」だけでなく、どのnodeからどのpassが生成したかを追跡する。

## 4. ParsedModule

parser recoveryを許す唯一の層である。

- missing token、error node、曖昧nodeを保持してよい。
- parserは名前解決・型解決・operator選択を行わない。
- syntax extensionは明示nodeとして保持する。
- `ParsedModule`を実行backendへ渡すAPIは存在しない。

validatorはtree well-formedness、span containment、token ownershipを確認する。

## 5. ResolvedModule

すべての名前参照がsymbolへ解決される。

```text
ResolvedName = Local(LocalId)
             | Item(ItemId)
             | Type(TypeId)
             | Module(ModuleId)
             | Builtin(BuiltinId)
```

文字列名は診断表示にのみ残す。method/operator/trait dispatchの候補集合も構造化し、最終選択をtypecheckerが行う。

不変条件:

- unresolved nameがない。
- scope外LocalIdがない。
- import aliasの再解析が不要。
- item identityがmangled nameに依存しない。

## 6. TypedModule

### 6.1 typed node wrapper

型検査済みexpressionは、型を別tableから推測せずnode自身のheaderで参照する。

```text
TypedExpr {
  id: ExprId,
  type_id: TypeId,
  effect_id: EffectId,
  value_category: ValueCategory,
  kind: TypedExprKind,
  origin: OriginId
}
```

`TypeId::Error`はTypedModuleに存在できない。型検査失敗時はTypedModuleを生成しない。

### 6.2 resolved call

```text
ResolvedCall {
  callee: FunctionId,
  signature: SignatureId,
  substitutions: SubstitutionId,
  args: Vec<TypedValueId>,
  dispatch: DispatchKind
}
```

callee名、引数型、戻り値型をbackendが再推論しない。

### 6.3 effect system

証明可能性のため、少なくとも次をeffectとして明示する。

```text
Pure
Read(RegionSet)
Write(RegionSet)
Allocate
Panic
IO
HostCall
Unsafe
Diverge
```

初期実装では精密なeffect polymorphismを要求しない。保守的なsummaryでよい。ただし未知effectをPureとして扱ってはならない。

## 7. SemanticCore

surface syntaxを除去した、target非依存の小さな意味IRとする。

### 7.1 許可する要素

- primitive literal
- local/global value
- resolved direct call
- explicit closure construction/call
- algebraic data constructor/project/test
- explicit control flow expression
- explicit memory/effect operation
- explicit checked/wrapping arithmetic
- structured match

### 7.2 除去する要素

- method-call syntax
- operator syntax
- import syntax
- implicit conversion
- implicit dereference/copy
- backend layout
- mangled-name解析
- target opcode

### 7.3 integer semantics

整数演算はopcodeだけでなくoverflow modeを持つ。

```text
IntAdd { mode: Checked | Wrapping | Saturating | Mathematical }
IntDiv { signedness, zero_policy, overflow_policy, rounding }
Shift  { signedness, count_policy }
```

`Mathematical`は証明式用であり、実行IRへlowerするにはrange proofまたは明示変換が必要である。

## 8. VerifiedCore

### 8.1 文字列禁止

契約式の正本はtyped treeである。

```text
ProofExpr {
  type_id: LogicTypeId,
  kind: ProofExprKind,
  origin: OriginId
}
```

主要kind:

```text
BoolLit, IntLit, Var, Result, Old,
Not, And, Or, Implies,
Eq, Lt, Le, Gt, Ge,
Add, Sub, Mul, Div, Mod,
IfThenElse,
Let,
Quantifier,
FunctionApp,
Field,
Index,
Length
```

unsupported runtime constructは文字列化せず、抽出時にdiagnosticとする。

### 8.2 契約

```text
FunctionContract {
  function: FunctionId,
  requires: Vec<ProofExprId>,
  ensures: Vec<ProofExprId>,
  modifies: RegionSet,
  reads: RegionSet,
  decreases: Option<ProofExprId>,
  assumptions: Vec<AxiomId>
}
```

### 8.3 heap model

初期heap modelはregion-basedとする。

- immutable valueは数学値として扱う。
- mutable objectは`Heap × ObjectId × FieldId -> Value`として扱う。
- function call前後のheapを`heap_pre`/`heap_post`で区別する。
- `modifies`外のregionはframe conditionで不変とする。
- aliasing不明の場合はregionを統合し、証明精度を下げるが不健全にはしない。

### 8.4 purity

proof expressionから呼べる関数は次のいずれかに限る。

- builtin logic function
- `pure`かつtotalと認定された関数
- `logic fn`として別定義された関数
- axiomatized function

通常関数を暗黙に純粋扱いしない。

## 9. TypedCFG / TypedSSA

### 9.1 CFG

structured control flowからexplicit CFGを生成する。各blockはtyped parameterを持つ。

```text
Block {
  params: Vec<(ValueId, ValueType)>,
  insts: Vec<InstId>,
  terminator: Terminator
}
```

terminatorは`Return`、`Jump`、`Branch`、`Switch`、`Unreachable`の閉じた集合とする。

### 9.2 SSA

SSA形成はCFG構築後の独立passとする。phiはblock parameterで表す。

validator:

- dominance
- single definition
- use-before-def禁止
- edge argumentとblock parameterの型一致
- terminator completeness
- call signature一致

## 10. ReprIR

意味型と表現型を分離する。

```text
ValueType {
  semantic: TypeId,
  repr: ReprId,
  nullability: Nullability,
  ownership: Ownership
}
```

`ReprId`はGcRef、scalar、linear pointer、multi-valueなどを明示する。layout選択後に名前やsource typeから再推論しない。

## 11. TargetIR

backendごとにlegality済みIRを持つ。

- WasmIR
- C99IR

emitterはTargetIRを逐語的にserializeする。emitter内で型推論、CFG修復、ABI選択、fallback loweringを行わない。

TargetIR validatorが失敗した場合はemitterを呼ばない。

## 12. Pass API

```text
trait Pass<In, Out> {
  fn run(input: &In, context: &PassContext) -> Result<PassOutput<Out>, DiagnosticSet>
}

PassOutput<Out> {
  snapshot: Out,
  receipt: PassReceipt,
  validation: ValidationReceipt
}
```

passはglobal状態を読まず、必要なregistryは`PassContext`に明示する。

禁止:

- inputのin-place mutation
- diagnostic文字列をcontrol flowとして解析
- function名によるsemantic dispatch
- validator failure後の継続
- nondeterministic iteration order

## 13. Validator設計

各snapshotに独立validatorを持つ。

```text
validate_parsed
validate_resolved
validate_typed
validate_semantic_core
validate_verified_core
validate_cfg
validate_ssa
validate_repr_ir
validate_target_ir
```

validatorは次を満たす。

- read-only
- deterministic
- panic-free
- 一件目だけでなくboundedな複数diagnosticを返す
- validator自身のunit/property/fuzz testを持つ
- compiler本体と別processからartifactを検査できる

## 14. Translation validation

最適化passは、前後snapshotと対応mapを出力する。

```text
TransformWitness {
  input_hash,
  output_hash,
  value_map,
  block_map,
  side_conditions
}
```

初期対象:

- constant folding
- branch folding
- copy propagation
- dead block elimination
- CFG simplification

難しいpassは、検証器が`unknown`を返した場合その変換を採用せず、入力版へrollbackできる設計にする。

## 15. 機械証明

Lean 4側では最初から実コンパイラ全体をmodel化しない。

1. `VerifiedCore`の小step semantics
2. TypedCFGのsemantics
3. selected passのpreservation
4. Proof IR encoder/decoder roundtrip

を順に証明する。

compiler生成artifactはLean側の独立decoderで読み、compiler内部serializerを信頼しすぎない。

## 16. Artifact format

JSONはdebug用、正式interchangeはcanonical binaryを推奨する。初期はstrict JSONでもよいが次を必須とする。

- schema name/version
- semantic version
- exact field set
- canonical ordering
- integer range明示
- unknown field reject
- hash
- feature/capability list
- unsupported construct list

expressionをS式文字列fieldへ格納しない。構造化node tableとしてserializeする。

## 17. Trust Manifest

```text
TrustManifest {
  compiler_build,
  source_semantics,
  schemas,
  passes,
  validators,
  verifier,
  solvers,
  runtime_abi,
  axioms,
  unsupported,
  status
}
```

proof receiptはsolver stdoutの保存だけではなく、input artifact hashとmanifestへ暗号学的に結び付ける。

## 18. テスト戦略

各passに次を要求する。

- constructor unit test
- validator rejection test
- golden serialization test
- determinism test
- roundtrip test
- metamorphic test
- differential test
- fuzz test
- regression fixture

backendは複数runtimeで同一semantic testを実行する。検証対象passはtranslation validatorをCI hard gateにする。

## 19. エラー分類

```text
UserDiagnostic
UnsupportedFeature
InvalidSnapshot
CompilerInvariantViolation
VerificationDisproved
VerificationUnknown
ToolchainFailure
RuntimeFailure
```

`InvalidSnapshot`と`CompilerInvariantViolation`をユーザーコードエラーへ偽装しない。

## 20. Proof-driven developmentとの接続

本RFCが定めるsnapshot、VerifiedCore、VerificationBundle、TrustManifestは、RFC-010の `check → test → prove → specification audit → receipt` で共通利用する。

`intent` と examples はdeveloper UXおよび仕様監査へ使うが、証明意味論へ混入させない。proof-required packageでは、source artifact、VerifiedCore hash、dependency contract hash、semantics version、prover configurationに一致するreceiptがなければreleaseできない。

AI-assisted repairでは、compiler/proverの機械可読diagnosticだけを判定根拠とし、assistantの自己申告をproof statusとして扱わない。

## 21. 完了条件

本設計への移行完了は、次をすべて満たす状態とする。

- backendが名前・履歴から型/ABI/layoutを推論しない。
-主要境界にversioned artifactと独立validatorがある。
- optimizerが不正変換を採用しないtranslation-validation経路を持つ。
- proof contractがtyped VerifiedCoreとして表現される。
- solver結果にTrustManifestが付く。
-旧巨大mutable table APIがcompiler-privateにも残らない。
-各passの入力・出力・責務が文書化されている。
- RFC-010のproof-required packageがstaleまたは不完全なreceiptでreleaseできない。
