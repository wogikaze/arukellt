# ADR-058: 言語 v2 の意味論と仕様境界を再設計する

ステータス: **PROPOSED** — 所有権を含む言語意味論とターゲット実装を分離し、45項目の再設計を個別に判断する

提案日: 2026-09-18

関連: [RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[ADR-002](ADR-002-memory-model.md)、[ADR-003](ADR-003-generics-strategy.md)、[ADR-007](ADR-007-targets.md)、[ADR-013](ADR-013-primary-target.md)、[ADR-014](ADR-014-stability-labels.md)、[ADR-018](ADR-018-language-docs-classification.md)、[ADR-031](ADR-031-import-syntax-wit-unification.md)、[ADR-035](ADR-035-wasm-gc-implementation.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)

---

## 文脈

受領した設計案は、所有権と借用、値の意味論、メモリ管理、Trait、エラー変換、数値変換、文字列、モジュール、文法、ターゲット、仕様検証を一つの再設計案として扱っている。

これらは相互に依存するが、すべてを一つの採択判断にすると、未検証の設計案と既存の採択契約を区別できなくなる。

特に、GCを言語意味論の前提から外す提案は、現在の [ADR-002](ADR-002-memory-model.md)、[ADR-013](ADR-013-primary-target.md)、[ADR-035](ADR-035-wasm-gc-implementation.md) と衝突する。

一方で、Traitとメソッドを正規APIにする、WIT境界とソースモジュールを分ける、例を実行可能な検証対象にする、といった項目は既存の採択ADRまたは提案中のRFCと重なる。

## 目的

言語の理想的な意味論、ターゲットごとの表現、公開API、仕様の検証方法を別々の責務として整理する。

45項目の提案を失わずに記録し、既存の採択契約を暗黙に変更せず、各領域を検証可能な単位で採択できる状態にする。

## 提案する決定

### 1. 言語意味論とターゲット表現を分離する

将来の言語 v2 では、所有権、move、借用、可変性、値と参照の区別、クロージャ捕捉、Trait解決、エラー変換、評価順序、数値変換を言語意味論として定義する。

Wasm GC、linear memory、参照カウント、native runtime などのメモリ管理方式は、その意味論を実現するバックエンドの選択肢として扱う。

ターゲットが扱えない機能は、意味論を別言語に変えるのではなく、コンパイル時の target capability error として報告する。

### 2. 明示的な値操作を候補の基本形とする

原案の候補では、束縛は不変を既定とし、`let mut` で再代入可能性を明示する。

暗黙コピーを行う型と move される型を区別し、必要な複製には `Copy` または明示的な `clone` を使う。

借用とクロージャの捕捉が関数の返却や保存を越えないことを、型検査とライフタイム規則で検査する。

この項目は候補設計であり、`Copy`、`Clone`、`Box`、`Option`、`Result`、enum の表現と診断を別途確定しなければ採択しない。

### 3. 正規APIとComponent境界を分ける

公開APIはTrait、メソッド、associated functionを正規形とし、ユーザー可達なfree functionの重複面は増やさない。

ソースモジュールの参照とComponent ModelのWIT境界は別レイヤーとして扱い、`pub` の可視性とComponent exportを同じ指定にしない。

この方針は [ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[ADR-031](ADR-031-import-syntax-wit-unification.md) と整合する部分を引き継ぎ、矛盾する構文変更は個別の後続判断に分ける。

### 4. 観測可能な意味論を正規仕様にする

評価順序、数値変換、overflow、NaN、ゼロ除算、Unicode、slice、脱糖、名前解決、Trait解決を、実装の偶然ではなく正規仕様の観測可能な規則として記述する。

正規仕様の例は実行可能なfixtureまたは独立した検証に接続し、文書だけの例を仕様の証拠として扱わない。

安定性ラベルは [ADR-014](ADR-014-stability-labels.md) に従い、実装の完成度と互換性保証を混同しない。

### 5. この提案の効力範囲を限定する

このADRが `PROPOSED` である間、現在の `ACCEPTED` ADRは変更しない。

`wasm32-gc` は引き続きprimary、`wasm32` はsupported、WASI P2はprimaryの既定host profileであり、GC表現は [ADR-035](ADR-035-wasm-gc-implementation.md) の契約に従う。

本提案を採択するときは、RFC-011の各項目を検証し、GC依存の変更、ターゲット契約の変更、公開構文の変更、互換性方針の変更について、それぞれ後続の `ACCEPTED` ADRを作成する。

後続ADRが採択されるまでは、既存の `ACCEPTED` ADRをこの提案の後継とはみなさない。

## 代替案と却下理由

### 既存ADRを一括で書き換える

45項目には、既存ADRと整合する提案、未決定の提案、既存ADRを変更する提案が混在している。

一括書き換えでは、採択日、検証根拠、互換性の境界が失われるため採用しない。

### 45項目をすべて個別ADRとして同時に採択する

所有権、メモリ、Trait、構文、仕様検証は依存関係があり、実装と回帰試験がない段階で個別判断を固定すると、後から相互に矛盾するADRが増える。

詳細設計をRFCに置き、採択可能な判断だけを後続ADRへ分ける。

### 現行のGC中心設計を変更しない

これは現在の `ACCEPTED` 契約を維持する限り必要な選択である。

しかし、将来の非GCバックエンド、明示的な所有権、より厳密な言語意味論を検討する設計案自体を記録できないため、この提案の記録先としては不十分である。

## 帰結

RFC-011が45項目の詳細設計と既存ADRとの関係を保持するため、提案の全体像を追跡できる。

現行実装は変更されず、`wasm32-gc`を前提とするセルフホスト検証もこの提案だけでは変わらない。

所有権と借用を採択する場合、言語仕様、型検査、クロージャ、標準ライブラリ、各バックエンド、移行診断を同時に整備する必要がある。

非GCバックエンドを採択する場合も、GC backendと別の言語意味論を作らず、同一意味論からのloweringとtarget capabilityの境界を検証しなければならない。

性能については、原案のイメージチャートを根拠にせず、条件を固定したbenchmark receiptで評価する。

## 再検討条件

次のいずれかに該当した場合、この提案を分割、縮小、または却下する。

- 所有権と借用の規則を、診断と実行可能な回帰試験を含めて定義できない。
- GCと非GCの両方で同一の観測可能意味論を維持できない。
- 既存の安定APIを [ADR-014](ADR-014-stability-labels.md) に従って移行できない。
- 仕様例をfixtureまたは独立checkerで検証できない。
- 実測した性能、バイナリサイズ、メモリ使用量が採択条件を満たさない。

## 関連

- [RFC-011: 言語 v2 の意味論再設計案](../rfcs/011-language-v2-semantics-redesign.md)
- [ADR-002: GC vs non-GC](ADR-002-memory-model.md)
- [ADR-007: コンパイルターゲット整理](ADR-007-targets.md)
- [ADR-013: `wasm32-gc` をプライマリターゲットとする](ADR-013-primary-target.md)
- [ADR-014: 言語仕様と Stdlib API の安定性ラベル](ADR-014-stability-labels.md)
- [ADR-018: 言語ドキュメント分類](ADR-018-language-docs-classification.md)
- [ADR-029: セルフホストネイティブ検証契約](ADR-029-selfhost-native-verification-contract.md)
- [ADR-031: import 構文と WIT パッケージ識別子の統合](ADR-031-import-syntax-wit-unification.md)
- [ADR-035: Wasm GC 内部レイアウト方針](ADR-035-wasm-gc-implementation.md)
- [ADR-044: trait とメソッド構文を言語機能として採択する](ADR-044-trait-method-syntax-adopted.md)
- [ADR-046: 公開 free function の根絶](ADR-046-free-function-eradication.md)
- [ADR-057: セルフホスト fixture 検証を current-only 並列ゲートへ移行](ADR-057-selfhost-current-only-fixture-gate.md)
