# RFC-010: Proof-Driven Development

ステータス: DRAFT

関連 ADR: [ADR-051](../adr/ADR-051-verifiable-compiler-architecture.md)

関連 RFC: [RFC-009](009-verifiable-compiler-architecture.md)

関連 plan: [proof-driven-development.md](../plans/proof-driven-development.md)

日付: 2026-07-30

---

## 1. 目的

Arukellt の形式検証を、任意の追加コマンドではなく通常の開発ループにする。

目標は次の統合である。

- VibeLang 型の `intent`、examples、contracts、effects を中心にした開発 UX
- MoonBit 型の program side / logic side 分離、契約、loop invariant、lemma、外部 prover
- Arrukellt コンパイラ自身の snapshot validator、translation validation、proof receipt

ここで `intent` と examples は仕様理解と回帰防止を助けるが、形式証明の根拠にはしない。証明の根拠は型付き論理仕様、意味論、検証条件、solver または proof checker の結果だけである。

## 2. 開発ループ

標準ループを次に固定する。

```text
intent / examples / formal specification
                 │
                 ▼
        implementation or repair
                 │
                 ▼
 check → test → prove → specification audit
                 │
       ┌─────────┴─────────┐
       │                   │
 counterexample        proof receipt
       │                   │
       ▼                   ▼
 code/spec repair      build/release gate
```

`compile succeeded` は完了条件ではない。proof-required package では、現在の source、dependency contracts、compiler build、semantics version に一致する proof receipt が必要である。

## 3. Program side と Logic side

### 3.1 Program side

通常の `.ark` ファイルに実行コードを書く。

```text
@intent("returns the absolute value without overflow")
@examples {
    abs_checked(0) => Some(0)
    abs_checked(3) => Some(3)
    abs_checked(-3) => Some(3)
}
fn abs_checked(x: i32) -> Option<i32> proof {
    ensures: match result {
        Some(y) => y >= 0
        None => x == i32::MIN
    }
} {
    if x == i32::MIN { None }
    else if x < 0 { Some(-x) }
    else { Some(x) }
}
```

### 3.2 Logic side

`.arkp` または `logic` module に、runtime artifactへ入らない定義を書く。

- predicate
- model function
- representation invariant
- lemma
- theorem
- ghost type/value
- axiomatized declaration

```text
predicate abs_result(x: int, r: Option<int>) {
    match r {
        Some(y) => y >= 0 && (y == x || y == -x)
        None => x == -2147483648
    }
}
```

logic side は machine integer と mathematical integer を異なる型として扱う。暗黙変換は禁止する。

## 4. First-class specification surface

最終的な表面構文は別RFCで凍結するが、意味上は次を第一級要素とする。

### 4.1 非形式メタデータ

- `intent`: 人間とAI向けの目的説明。proof semanticsへ入れない。
- `examples`: executable testとspec sanity checkへ変換する。
- rationale/reasoning: proof guidanceまたは文書。定理としては扱わない。

### 4.2 形式仕様

- `requires`
- `ensures`
- `invariant`
- `assert`
- `decreases`
- `reads`
- `modifies`
- `old`
- `result`
- `lemma`
- `model`
- `pure`
- `axiom`

### 4.3 effect

最低限、次を推移的に追跡する。

```text
Pure
Read(region set)
Write(region set)
Allocate
Panic
IO
HostCall
Unsafe
Concurrency
Diverge
```

未分類effectを `Pure` として扱わない。proof expression から呼べる program-side function は、effect checkerとtermination checkerが `pure + total` と認定したものだけである。

## 5. Package proof policy

package manifest はproof policyを持つ。

```toml
[proof]
mode = "required"
integer_model = "machine"
unknown = "deny"
axioms = "deny"
coverage = "public"
```

mode:

- `off`: 証明artifactを生成しない。
- `check`: specificationの構文・型・effect・purityだけ検査する。
- `optional`: proofを実行するが、unknownで通常buildを止めない。
- `required`: 対象itemのproved receiptがなければbuild/releaseを失敗させる。
- `certified`: requiredに加え、axiom・unsupported・unmodelled effect・mathematical/machine mismatchを禁止する。

`required` と `certified` では timeout、unknown、unsupported、skipped を成功に変換しない。

## 6. CLI

### `arukellt check`

- parse / resolve / type / effect
- specification well-formedness
- proof-expression purity
- unsupported construct detection

### `arukellt test`

- unit/integration tests
- `examples` から生成したtests
- property tests
- contract runtime instrumentationを有効にしたdev tests

### `arukellt prove`

- VerifiedCore extraction
- VC generation
- incremental prover invocation
- structured counterexample
- proof receipt

対象をpackage、module、function、obligation IDで絞れる。

### `arukellt verify`

release前の統合gateであり、次を実行する。

```text
check
+ test
+ prove
+ specification audit
+ proof receipt validation
+ artifact/manifest binding
```

### `arukellt explain-proof`

obligationの由来、使用した契約・lemma・axiom、solver結果を表示する。solver stdoutの羅列を正式diagnosticとしない。

## 7. Proof obligation model

各obligationは安定IDを持つ。

```text
Obligation {
  id,
  kind,
  function,
  origin,
  assumptions,
  goal,
  semantic_version,
  feature_set
}
```

kind:

- precondition at call
- postcondition at return
- assertion
- invariant initialization
- invariant preservation
- loop exit
- termination
- bounds safety
- division safety
- overflow safety
- frame condition
- representation invariant
- translation equivalence

proof cache keyは、source textではなくcanonical VerifiedCore hash、dependency contract hashes、semantics version、prover configurationから作る。

## 8. Counterexample-driven repair

`disproved` は構造化されたcounterexampleを返す。

```text
Counterexample {
  obligation_id,
  source_span,
  parameter_values,
  old_values,
  heap_projection,
  failing_path,
  violated_clause
}
```

LSPは次を表示する。

- 失敗した契約
- その契約へ至るpath
- 最小化された入力候補
- invariantが失われたiteration
- 呼び出し先のどのpreconditionを満たさないか

diagnosticはAI agentが機械的に再入力できるJSON形式も持つ。

## 9. AI-assisted proof loop

AIは候補生成器であり、proof checkerが判定器である。

```text
human intent/spec
  → AI implementation patch
  → deterministic check/test/prove
  → structured failure
  → AI repair patch
```

### 9.1 specification lock

AIがimplementationとformal specificationを同時に変更して証明を通すことを既定で禁止する。

- `implementation-repair` mode: formal specification hashを固定する。
- `specification-edit` mode: implementation hashを固定し、human reviewを必須にする。
- 両方を変えるPRは二つのcommit/PRへ分割する。

### 9.2 spec weakening検出

formal specification変更時は次を実行する。

- implication check: new preconditionが不必要に強くなっていないか
- implication check: new postconditionが弱くなっていないか
- removed clause detection
- examplesとの整合性
- mutation testing
- vacuity check
- proof coverage差分

`requires false`、常に真のpostcondition、到達不能化によるproof成功を拒否する。

### 9.3 intent drift

`intent` は証明の根拠ではないが、formal specificationとexamplesの変更時にレビュー信号として使う。AIによるdrift判定はinformationalであり、deterministic compiler/prover gateを置き換えない。

## 10. Specification audit

形式仕様自体の品質を検査する。

### 10.1 non-vacuity

- requiresが充足可能か
- assertion/invariant assumptionsが矛盾していないか
- ensuresがrequiresの矛盾だけで証明されていないか

### 10.2 mutation testing

実装または仕様へ制御されたmutationを入れ、証明・examples・testsが検出することを確認する。

例:

- 比較演算子反転
- 境界 `<=` / `<` 変更
- return値変更
- effect追加
- frame condition削除
- overflow mode変更

mutationが大量に生存するpackageをcertifiedへ昇格させない。

### 10.3 coverage

proof coverageは行数ではなくitemとobligationで測る。

```text
public items contracted
public items proved
safety obligations proved
termination obligations proved
axioms used
unknown obligations
unsupported obligations
```

## 11. Dependency model

通常のtargeted proofでは、dependencyの公開contractをreceipt付きassumptionとして使う。dependency実装を毎回再証明しない。

受理条件:

- dependency artifact hash一致
- contract hash一致
- semantics version互換
- required proof policy以上
- trust policyに反するaxiom/unsupportedがない

receiptがないdependencyはproof-required packageから呼べないか、明示的なtrusted boundaryとしてmanifestへ記録する。

## 12. Runtime contractsとの関係

runtime contract checkは形式証明の代替ではない。

用途:

- optional/unchecked codeとの境界
- FFI/host input
- debug/development
- proof model外の環境仮定

proved contractはreleaseで削除可能だが、external boundary checkは別policyで残せる。削除判断もreceiptとsemantic hashに結び付ける。

## 13. Prover strategy

初期backendはWhy3 + SMT solverとする。

- Z3、cvc5、Alt-Ergo等をportfolio運用可能にする。
- backend固有syntaxはVerifiedCoreへ漏らさない。
- solver disagreementは成功にしない。
- proof replay用にVC hash、solver、version、options、resource limitを保存する。

Lean 4は次に使う。

- VerifiedCore semantics
- encoder/decoder
- selected compiler pass
- reusable theorem library

SMTで自動化し、Leanで信頼基盤と恒久定理を縮める。

## 14. Proof-driven compiler development

ユーザープログラムだけでなく、compiler repositoryの変更にも同じ原則を適用する。

compiler passのPRは次を含む。

- pass contract
- input/output validator delta
- negative fixture
- semantic differential test
- translation validationまたは機械証明の方針
- receipt schemaへの影響

optimizer PRはtranslation validationなしで既定有効化しない。validator bypass、fallback、名前解析、型再推論を増やすPRは拒否する。

## 15. 完了条件

Arukelltをproof-driven development対応と呼べる条件は次である。

- program side / logic sideが分離されている。
- `check`、`test`、`prove`、`verify`が一貫したartifact chainを使う。
- contracts、invariants、assertions、termination、models、lemmasを記述できる。
- proof-required packageがreceiptなしでreleaseできない。
- proof failureがsource span付きcounterexampleになる。
- specification lockとspec weakening gateがある。
- proof cacheがsemantic hashに基づく。
- dependency contract receiptを検証する。
- machine integer semanticsを証明できる。
- compiler passにもtranslation validationまたはproof gateがある。
- proof statusとtrusted assumptionsがrelease artifactに残る。
