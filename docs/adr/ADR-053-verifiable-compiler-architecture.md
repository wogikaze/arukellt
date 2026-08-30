# ADR-053: 検証可能なコンパイラ・アーキテクチャ

ステータス: **PROPOSED** — 検証可能なコンパイラ構造と検証境界を提案する

提案日: 2026-07-30

関連: [ADR-040: Semantic Type Spine](ADR-040-typed-mir-signature-registry.md)、[ADR-052: Proof-Driven Development](ADR-052-proof-driven-development.md)、[RFC-002](../rfcs/002-semantic-type-spine.md)、[RFC-009](../rfcs/009-verifiable-compiler-architecture.md)、[RFC-010](../rfcs/010-proof-driven-development.md)

---

## 文脈

Arukellt のバグは、個別の条件分岐や opcode 実装だけではなく、コンパイル段階の境界が弱いことから発生している。

- 後段が前段で失われた型・シグネチャ・ABI・nullability を名前や履歴から再推論する。
- 巨大な可変 table と整数 index が広範囲に共有され、部分的に不正な状態を表現できる。
- IR の constructor が不変条件を保証せず、validator が後から不正状態を発見する。
- source semantics、machine semantics、runtime ABI、backend semantics が同じ型や文字列で表される。
- optimizer と lowering が同時に意味変換と表現変換を行い、各 pass の責務が大きい。
- コンパイラ本体、外部 verifier、solver、runtime のどこを信頼するかが artifact に記録されない。
- selfhost 制約のため、値コピーや巨大構造の巡回が correctness と性能の両方を損なう。

形式検証機能を既存構造へ追加するだけでは、誤った IR を正しく証明したように見せる危険がある。先に、コンパイラそのものを検証しやすい構造へ変更する必要がある。

## 提案する決定

Arukellt は、コンパイラを **小さな不変 snapshot の列**として再構成する。

```text
Source
  -> ParsedModule
  -> ResolvedModule
  -> TypedModule
  -> SemanticCore
  -> VerifiedCore
  -> TypedCFG
  -> TargetIR
  -> Artifact
```

各矢印は次を満たさなければならない。

1. 入力 snapshot は不変である。
2. 出力型は、その段階で必要な不変条件を型または smart constructor で表す。
3. pass は入力を変更せず、新しい snapshot と provenance を返す。
4. pass 直後に独立 validator を実行できる。
5. 不明な意味情報を fallback 推論しない。
6. serialization は versioned、canonical、unknown-field reject とする。
7. ID は domain ごとに異なる nominal type を使い、裸の `i32` を公開境界に出さない。

### 1. Semantic Core と Verified Core を分離する

`SemanticCore` は実行プログラムの型付き意味を表す。`VerifiedCore` は証明用の論理モデルを表す。証明契約を文字列や実行 AST の補助 node として保持しない。

`VerifiedCore` は少なくとも次を構造化して持つ。

- 型付き論理式
- 関数の事前条件・事後条件
- effect summary
- heap region と frame condition
- machine integer の overflow policy
- `old` snapshot
- loop invariant と variant
- source span と origin ID

実行 IR と論理 IR は、共有する `TypeId` / `FunctionId` を介して対応付けるが、同じ node table を共有しない。

### 2. 型付き式を文字列へ退化させない

証明式、callee、型、ABI、layout を文字列で表現しない。文字列は診断表示または serialization 上の名前に限定する。

```text
ProofExprId -> ProofExpr
CallTarget  -> FunctionId
ValueType   -> TypeId + Repr + Nullability
Layout      -> LayoutId
```

S式やWhyMLは外部出力形式であり、コンパイラ内部の正本ではない。

### 3. IR は構築時に妥当である

public field を持つ巨大 record と、後段 validator による一括検査を縮小する。IR node は smart constructor または builder だけが生成する。

- `TypedExpr` は `Error` 型を保持できない。
- `Call` は引数数と型が一致した `ResolvedCall` だけを保持する。
- `Branch` 条件は `BoolValueId` を要求する。
- `Return` は関数の `ReturnShapeId` と一致する値だけを受け取る。
- nullable/non-null、owned/shared、scalar/ref を異なる型で扱う。
- invalid state が必要な parser recovery は syntax 層に閉じ込める。

validator は constructor の代替ではなく、serialization・unsafe boundary・debug assertion の二重化として残す。

### 4. pass を単機能化する

各 pass は「一種類の意味変換」だけを行う。

- desugar と型解決を混ぜない。
- CFG 構築と SSA 化を分ける。
- representation selection と target lowering を分ける。
- optimization と legality repair を分ける。
- emitter は target IR の逐語変換だけを行う。

pass は次の結果を返す。

```text
PassResult<Out> {
  output: Out,
  receipt: PassReceipt,
  diagnostics: Diagnostics
}
```

`PassReceipt` は input hash、output hash、pass version、設定、検証結果を持つ。

### 5. verification ladder を採用する

すべてを一度に定理証明しない。次の順で保証を強化する。

1. 型・constructor による不正状態排除
2. deterministic validator
3. differential / metamorphic test
4. translation validation
5. selected pass の機械証明
6. end-to-end theorem の検討

optimizer はまず translation validation を標準とする。各関数について最適化前後の等価性条件を外部 checker へ渡す。安定した小 pass は後から Lean 4 で恒久証明する。

### 6. trust manifest を artifact に付与する

証明結果と生成 artifact は、何を信頼したかを機械可読で記録する。

- compiler build ID
- frontend semantics version
- IR schema versions
- pass receipts
- verifier/backend version
- solver 名・version・options
- runtime ABI version
- axioms / unsupported features
- proof status: `proved | disproved | unknown | skipped`

solver の `unknown` と timeout は成功として扱わない。

### 7. selfhost compiler と verifier を分離する

selfhost compiler は source を型付き snapshot と versioned artifact へ変換する。Why3、SMT solver、Lean checker は host-side tool が実行する。

ただし外部 tool は JSON 内の自由文字列を再パースして意味を復元してはならない。canonical binary または構造化JSONから直接 typed model を復元する。

### 8. fail closed を原則とする

次の場合は fallback せずコンパイルまたは検証を停止する。

- unknown TypeId / FunctionId / LayoutId
- schema version 不一致
- unsupported proof expression
- ABI mismatch
- unmodelled effect
- overflow policy 未指定
- validator failure
- solver unknown を proof-required gate で受け取った場合

### 9. Proof-Driven Development の基盤とする

本アーキテクチャは単にproof artifactを出すだけではなく、ADR-052とRFC-010が定める `specification → implementation → check/test/prove → counterexample → repair → receipt` を通常の開発経路として成立させる。

`intent` と examples はdeveloper UXと仕様監査に使うが、形式証明の根拠にはしない。proof-required packageは、現在のsource、semantics、dependency contracts、prover configurationに一致するreceiptなしでreleaseできない。

AIは候補生成器に限定し、deterministic validator/proverだけがproof statusを決める。仕様を弱めてproofを通す変更を防ぐため、specification lock、weakening analysis、non-vacuity、mutation auditをarchitecture上の正式なconsumerとして扱う。

## 非目標

- 初期段階でコンパイラ全体をLeanで書き直すこと
- SMT solverをselfhost artifactへリンクすること
- unsafe、FFI、並行性、浮動小数点を即座に完全検証すること
- 既存MIRを一括置換する巨大PR

## 実装方針

実装は master から独立した小PRに分割する。各PRは一つの新しい境界とvalidatorだけを導入し、旧経路との二重実行・差分比較を経て切り替える。

詳細仕様は RFC-009、proof-driven workflowはRFC-010、移行順とgateは `docs/plans/verifiable-compiler-migration.md` と `docs/plans/proof-driven-development.md` を正本とする。

## 結果

この判断によりコード量は短期的に増えるが、後段の推論、文字列解析、巨大table共有、暗黙fallbackを削減できる。形式検証は追加機能ではなく、明示された意味論と小さな信頼境界の上に構築され、通常の開発完了条件へ組み込まれる。
