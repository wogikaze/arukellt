# Proof-Driven Development Rollout

関連 ADR: [ADR-051](../adr/ADR-051-verifiable-compiler-architecture.md)

関連 RFC: [RFC-009](../rfcs/009-verifiable-compiler-architecture.md)、[RFC-010](../rfcs/010-proof-driven-development.md)

ステータス: DRAFT

日付: 2026-07-30

---

## 方針

このplanはcompiler architecture移行と並行して、ユーザーが日常的に使うproof-driven workflowを構築する。

優先順位は次とする。

1. 仕様を型付きartifactにする。
2. 失敗をcounterexampleとして返す。
3. proofをbuild/release gateへ接続する。
4. spec weakeningを検出する。
5. AI repair loopを安全に接続する。

## Track A: Specification foundation

### A1. Package proof policy

導入:

- `off | check | optional | required | certified`
- integer model
- unknown/axiom policy
- proof coverage policy

Gate:

- unknown policyのsilent downgrade 0件
- proof-required packageのreceipt bypass 0件
- manifest roundtrip test

### A2. Logic side

導入:

- `.arkp` またはlogic module
- predicate/model/lemma/ghost/axiom
- program sideからlogic sideへの禁止方向
- pure program functionの明示export

Gate:

- runtime artifactへlogic definition混入 0件
- impure functionをproof expressionから呼ぶ経路 0件
- unresolved logic symbol 0件

### A3. Formal specification surface

順序:

1. requires / ensures / result
2. proof_assert
3. loop invariant
4. decreases
5. old
6. reads / modifies
7. model / representation invariant
8. lemma

各sliceはparser、typed VerifiedCore、negative fixtures、serialization、independent decoderまでを一つのgateとする。

## Track B: Developer loop

### B1. `arukellt prove`

MVP:

- package/function選択
- VC generation
- Why3 adapter
- solver portfolio
- structured status
- proof receipt
- semantic-hash cache

Gate:

- proved/disproved/unknown/unsupportedの混同 0件
- cache keyへsource absolute pathや時刻を含めない
- stale receipt受理 0件

### B2. Counterexamples

導入:

- parameter model
- failing path
- violated clause
- source span
- heap projection
- loop iteration

Gate:

- counterexample JSON schema
- deterministic minimization
- LSP diagnostic fixture
- solver stdoutを直接diagnosticへ流す経路 0件

### B3. `arukellt verify`

実行内容:

- check
- generated examples tests
- ordinary tests
- prove
- specification audit
- receipt validation
- TrustManifest generation

Gate:

- required/certified policyで一つでも未証明なら非0終了
- artifact hashとproof receiptのbinding
- dependency receipt validation

## Track C: Vibe-style specification UX

### C1. Intent

`intent`はhuman/AI向けmetadataとして追加する。

Gate:

- intentがproof semanticsへ入らない
- intent変更だけでproof hashが変わらない
- docs/LSPで参照できる

### C2. Examples

examplesをtestへ展開する。

Gate:

- example failureがsource spanへ戻る
- exampleはproof扱いされない
- contractとexampleの明白な矛盾を検出する

### C3. Effects

effectをfirst-class contractとして表示・推移追跡する。

Gate:

- undeclared IO/HostCall/Unsafe/Concurrencyを拒否
- unknown effectをPureにしない
- public API docsへeffect summaryを出す

## Track D: Specification quality

### D1. Specification lock

CI mode:

- implementation repair: spec hash固定
- specification edit: implementation hash固定
- combined change: human approval必須

Gate:

- AI workflowがspecとimplementationを同時自動承認できない
- normative spec changeを明示labelなしでmergeできない

### D2. Weakening analysis

検査:

- precondition strengthening
- postcondition weakening
- clause removal
- frame condition weakening
- axiom追加
- coverage低下

Gate:

-差分report必須
- certified packageで未承認weakening 0件

### D3. Non-vacuity and mutation

導入:

- requires satisfiability
- vacuous postcondition detection
- controlled implementation/spec mutations
- surviving mutation report

Gate:

- `requires false`でcertified不可
- mutation score threshold
- boundary comparison mutation corpus

## Track E: AI repair loop

### E1. Machine-readable diagnostics

全failureをstable schemaで返す。

- parser/type/effect
- test/example
- proof/counterexample
- spec audit
- stale receipt

### E2. Repair modes

- code repair
- invariant/lemma guidance
- specification proposal

specification proposalは自動適用せず、別patchとして提示する。

### E3. Closed loop gate

```text
spec lock
→ AI patch
→ check/test/prove/audit
→ deterministic receipt
```

成功条件はassistantの自己申告ではなく、repository gateのreceiptだけで判定する。

## Track F: Proof-driven compiler development

compiler moduleを順次proof-requiredへ昇格する。

優先対象:

1. canonical serialization
2. TypeId/Signature registry
3. VerifiedCore encoder/decoder
4. constant folding
5. branch folding
6. CFG simplification
7. ABI/layout lookup

各compiler PRに必須:

- invariant delta
- validator
- negative fixture
- differential/metamorphic test
- translation witnessまたはLean theorem reference
- receipt impact

## Promotion levels

### Experimental

- requires/ensures
- prove command
- Why3/SMT
- structured status

### Preview

- assertions/invariants/decreases
- counterexamples
- receipt-gated package
- dependency contracts
- machine integer safety

### Stable

- specification lock
- weakening/non-vacuity gate
- mutation audit
- model-based verification
- LSP proof workflow
- compiler selected-pass verification

### Certified profile

- axiom deny
- unknown deny
- unsupported deny
- machine semantics only
- complete TrustManifest
- reproducible proof replay
- proof and mutation coverage thresholds

## PR分割案

1. RFC-010 + rollout plan
2. proof policy manifest schema
3. logic module parser and resolver
4. typed ProofExpr core
5. requires/ensures extraction
6. Why3 adapter skeleton
7. obligation and receipt schema
8. prove command MVP
9. structured counterexample
10. examples lowering
11. intent metadata
12. effect display/gate
13. assertions
14. loop invariants
15. decreases/termination
16. old + frame conditions
17. dependency proof receipts
18. specification lock
19. weakening/non-vacuity checker
20. mutation audit
21. LSP proof diagnostics
22. AI repair protocol
23. certified package profile

各PRは一つのartifact boundaryまたは一つのuser-visible featureだけを扱う。
