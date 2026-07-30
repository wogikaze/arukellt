# ADR-052: Proof-Driven Development を標準開発モデルにする

ステータス: **PROPOSED**

提案日: 2026-07-30

関連: [ADR-051](ADR-051-verifiable-compiler-architecture.md)、[RFC-009](../rfcs/009-verifiable-compiler-architecture.md)、[RFC-010](../rfcs/010-proof-driven-development.md)

---

## 文脈

Arukelltに契約構文と外部solverを追加するだけでは、形式検証は一部利用者向けの付加機能に留まる。AI agentや人間が高速に変更する開発では、コード生成後に任意でproofを走らせるモデルではなく、仕様・実装・証明失敗・修正を一つのループとして扱う必要がある。

VibeLang型のintent、examples、contracts、effectsは、仕様をコードの近くに置き、AI生成コードのdriftを検出する開発UXとして有用である。ただしintentや実行時contract checkは形式証明ではない。

MoonBit型のprogram side / logic side分離、契約、loop invariant、lemma、Why3/SMTによるVC dischargeは形式検証の基盤になる。ただし証明がoptionalなままでは、release artifactの保証にはならない。

## 決定

ArukelltはProof-Driven Developmentを標準開発モデルとして採用する。

1. `intent`、examples、formal contracts、effectsをfirst-class source metadataとする。
2. `intent`とexamplesはproof semanticsへ含めない。形式的根拠はtyped specificationとmachine-checked proofだけとする。
3. program sideとlogic sideを分離し、predicate、model、lemma、ghost、axiomをlogic sideに置く。
4. `check`、`test`、`prove`、`verify`を同じsnapshot/artifact chain上で実行する。
5. proof-required packageは、現在のsource/semantics/dependency/prover設定に一致するproof receiptなしでreleaseできない。
6. proof resultは`proved | disproved | unknown | unsupported | assumed | skipped`を区別し、required/certified policyではproved以外を成功にしない。
7. proof failureはsource span付きの構造化counterexampleとして返す。
8. AI repairではformal specificationを既定でlockし、implementationとspecificationの同時自動変更を許さない。
9. specification変更はweakening、vacuity、mutation、coverageの監査を必須とする。
10. optimizerおよびcompiler passにもtranslation validationまたは機械証明を要求し、ユーザープログラムとcompiler自身に同じ開発原則を適用する。

## Proof policy

packageは次のpolicyを選ぶ。

- `off`
- `check`
- `optional`
- `required`
- `certified`

`certified`ではaxiom、unknown、unsupported、unmodelled effect、machine/mathematical integer mismatchを禁止し、TrustManifestと再現可能なproof receiptを必須とする。

## AIの位置付け

AIは実装・invariant・lemma・specification候補を生成できるが、proof statusを決定しない。判定器はdeterministic compiler、validator、test runner、proof checker、specification auditだけである。

specification候補は実装修正と別patchとして提示し、formal specification hashを変更する場合はhuman reviewを必須にする。

## 結果

通常の開発完了条件は「compileした」から「宣言した仕様に対する現在のproof policyを満たし、そのreceiptがartifactへ結び付いた」へ変わる。

短期的には仕様、invariant、model、lemmaを書くコストが増える。一方、AI生成コードやoptimizer変更が仕様を壊した時点で機械的に停止でき、仕様を弱めて見かけ上成功させる変更も監査できる。

詳細なsource surface、CLI、counterexample、specification audit、AI repair protocolはRFC-010を正本とする。
