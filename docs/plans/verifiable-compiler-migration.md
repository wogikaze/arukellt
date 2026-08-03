# Verifiable Compiler Migration Plan

関連 ADR: [ADR-051](../adr/ADR-051-verifiable-compiler-architecture.md)、[ADR-052](../adr/ADR-052-proof-driven-development.md)

関連 RFC: [RFC-009](../rfcs/009-verifiable-compiler-architecture.md)、[RFC-010](../rfcs/010-proof-driven-development.md)

関連 plan: [proof-driven-development.md](proof-driven-development.md)

ステータス: DRAFT

日付: 2026-07-30

---

## 原則

この移行を一つの巨大PRで実施しない。各sliceは次を満たす。

- 新しい型または境界を一つだけ導入する。
- 旧経路と新経路を同じ入力で実行し、差分をreceipt化する。
- validatorと失敗fixtureを実装より先に追加する。
- fallbackを増やさない。
- baseline数値の単なる更新で品質gateを回避しない。
- hard gateを通るまで次sliceへ進まない。
- proof-driven user workflowは別planのTrack A–Fと同期する。

## Phase 0: 設計凍結と現状計測

成果物:

- ADR-051/ADR-052採択
- RFC-009/RFC-010レビュー
- 全pass inventory
- 現在のfallback、名前解析、型再推論、巨大table copyの一覧
- compiler invariant violationの分類と件数baseline
- reproducible fixture corpus

Gate:

- 各compiler stageのownerと入出力が一覧化されている。
- backendの推論箇所がファイル・関数単位で機械可読に列挙されている。
- 同じ入力を3回buildしてstage hashが一致する。
- proof policyとproof receiptのtrust boundaryが文書化されている。

## Phase 1: Foundation IDs and Snapshots

導入:

- nominal `TypeId`, `FunctionId`, `ExprId`, `BodyId`, `BlockId`, `ValueId`
- `SnapshotHeader`
- canonical hash
- `Origin` / provenance
- `PassReceipt`

禁止を追加:

- 新規public APIで裸のi32 IDを返すこと
- input snapshotのin-place mutation
- nondeterministic map iteration

移行方法:

旧整数IDとnominal IDの変換は単一compat moduleに隔離する。新コードから旧変換を直接呼ばない。

Gate:

- ID domain混同のcompile-fail test
- snapshot determinism 3連続一致
- pass receipt golden test
- compatibility module以外の新規裸ID 0件

## Phase 2: TypedModule Boundary

導入:

- `TypedExpr`
- `ResolvedCall`
- `SignatureId`
- conservative `EffectSummary`
- typed-module validator

切替:

- typechecker成功時だけTypedModuleを生成
- CoreHIR builderはTypedModuleだけを受け取る
- backend向けcallee文字列復元を禁止

Gate:

- `Type::Error` node 0件
- call arity/type/return shape validator
- unknown effectをPureに落とす経路 0件
-既存fixtureのtyped snapshot deterministic hash一致

## Phase 3: SemanticCore v2

導入:

- 小さなtarget-independent node set
- explicit integer semantics
- explicit conversion/copy/borrow
- structured ownership
- SemanticCore v2 validator

二重実行:

旧CoreHIRとSemanticCore v2を同時生成し、同じMIRへlowerしてsemantic fixtureを比較する。

Gate:

- method/operator/import raw syntax残存 0件
- implicit conversion 0件
- integer opcodeにoverflow/rounding policyが必須
- semantic fixture全件等価

## Phase 4: VerifiedCore v1

既存PR #16の試作をそのままmergeしない。必要な知見だけを移植する。

導入:

- structured `ProofExpr`
- `FunctionContract`
- logic type table
- purity/effect checker
- machine integer model
- strict versioned serialization
- independent host validator

初期surface:

- `requires`
- `ensures`
- `result`
- boolean/integer/field/index/length expression
- pure total function call

初期非対応:

- mutable aliasを伴うheap update
- unsafe/FFI
- float
- concurrency
- higher-order quantification

非対応constructはhard diagnosticにする。

Gate:

- proof expression文字列正本 0件
- unsupported nodeのsilent drop 0件
- compiler serializerと独立decoderのroundtrip
- unknown field/version reject
- property/fuzz tests

RFC-010のTrack Aはこのphaseから開始する。logic side、package proof policy、formal specification surfaceはVerifiedCoreと別の正本を作らない。

## Phase 5: TypedCFG and SSA

導入:

- explicit CFG
- block parameters
- typed values
- dominance validator
- SSA pass

Gate:

- use-before-def 0件
- edge/block parameter type mismatch 0件
- terminator欠落 0件
- CFG→SSA differential execution一致

## Phase 6: Representation Selection

導入:

- `ReprId`
- `LayoutId`
- nullability/ownershipを含む`ValueType`
- GcLayoutRegistry
- ABI registry

切替:

- emitterの名前解析・stack-history推論を削除
- unknown layoutはICE

Gate:

- backend name parsing 0件
- backend source-type inference 0件
- all calls resolved by FunctionId/SignatureId
- wasm validator全件成功

## Phase 7: TargetIR and Dumb Emitters

導入:

- WasmIR validator
- C99IR validator
- emitterをserialization専用化

Gate:

- emitter内のCFG修復 0件
- emitter内の型/ABI/layout選択 0件
- TargetIR rejection fixtures
- emitted artifact deterministic hash一致

## Phase 8: Translation Validation

対象passを順番に有効化する。

1. constant folding
2. branch folding
3. copy propagation
4. dead block elimination
5. CFG simplification

各passはtransform witnessを生成する。validationが`unknown`またはfailureなら、release buildでも変換前snapshotへ戻す。

Gate:

- validationなしで対象pass出力を採用する経路 0件
- known historical miscompile fixtureを全てreject
- signed division、overflow、NaN、aliasingの境界fixture

## Phase 9: External Verification Backend

導入:

- VerificationBundle
- Why3 adapter
- solver driver
- TrustManifest
- proof receipt

結果分類:

- proved
- disproved
- unknown
- unsupported
- assumed
- skipped

Gate:

- solver unknownをsuccessに変換しない
- artifact hashとreceipt hashの結合
- solver/options/version記録
- reproducible proof run
- proof-required packageのstale receipt bypass 0件

このphaseでRFC-010のTrack Bを実用化し、`arukellt prove`と`arukellt verify`を正式surfaceへ昇格する。

## Phase 10: Lean Proof Project

独立directory `formal/` を追加する。

最初の証明対象:

- VerifiedCore expression typing
- encoder/decoder roundtrip
- constant folding preservation
- branch folding preservation
- CFG simplification preservation

Gate:

- theorem statementが実装schema versionと結び付く
- generated theorem inputを信頼せず独立decodeする
- CIでLean build

## Phase 11: Proof-driven quality gates

導入:

- specification lock
- weakening analysis
- non-vacuity check
- implementation/spec mutation audit
- proof coverage report
- dependency proof receipt validation
- machine-readable counterexample

Gate:

- AI workflowがformal specとimplementationを同時自動承認できない
- `requires false`などのvacuous proofをcertifiedとして受理しない
- normative spec weakeningはhuman approval必須
- certified packageでunknown/axiom/unsupported 0件

## Phase 12: Removal

削除対象:

- old CoreHIR compat APIs
-裸ID public functions
- string proof expressions
- backend inference helpers
- name-based ABI dispatch
- giant mutable cross-stage tables
- legacy validator bypass
- receiptなしのproof-required release path

Gate:

- compatibility allowlist 0件
- fallback allowlist 0件
- old pipeline build flag削除
- strict CI 3連続PASS

## PR分割案

1. Design only: ADR/RFC/plan
2. Nominal IDs and snapshot header
3. Pass receipts and deterministic hashing
4. Typed call/signature boundary
5. Effect summary
6. SemanticCore v2 skeleton and validator
7. Structured VerifiedCore schema
8. Contract parser to VerifiedCore extraction
9. Independent Proof IR decoder/validator
10. Proof policy manifest schema
11. Logic-side module foundation
12. TypedCFG
13. SSA validator
14. Repr/Layout registries
15. WasmIR boundary
16. C99IR boundary
17. Translation validator MVP
18. Why3 adapter and TrustManifest
19. `arukellt prove` MVP
20. Structured counterexample
21. Specification lock and weakening audit
22. Mutation/non-vacuity audit
23. Lean formal project
24. Legacy removal

各PRは原則500行程度までとし、機械生成物を除いて1000行を超える場合は分割する。

## PR #16の扱い

PR #16は実験として有用だが、そのままmerge対象にはしない。

再利用するもの:

- language surfaceの検討
- external verifier boundary
- strict schema/version rejection
- frontend negative tests
- selfhost環境で巨大table値コピーがmemory trapになる知見

破棄・再設計するもの:

- proof expressionの文字列/S式正本
- function indexの裸i32
- CoreHIR横付けtable
- debug dumpを正式artifact境界として使う設計
- 50ファイル規模の一括導入

設計PR採択後、PR #16はcloseし、必要な変更を小PRへcherry-pickではなく再実装する。
