# RFC-011: 言語 v2 の意味論再設計案

ステータス: DRAFT

提案日: 2026-09-18

関連ADR: [ADR-058](../adr/ADR-058-language-v2-semantics-redesign.md)、[ADR-002](../adr/ADR-002-memory-model.md)、[ADR-003](../adr/ADR-003-generics-strategy.md)、[ADR-007](../adr/ADR-007-targets.md)、[ADR-013](../adr/ADR-013-primary-target.md)、[ADR-014](../adr/ADR-014-stability-labels.md)、[ADR-031](../adr/ADR-031-import-syntax-wit-unification.md)、[ADR-035](../adr/ADR-035-wasm-gc-implementation.md)、[ADR-044](../adr/ADR-044-trait-method-syntax-adopted.md)、[ADR-046](../adr/ADR-046-free-function-eradication.md)

---

## 1. 要約

本RFCは、受領した `new-adr.md` の45項目を、言語意味論、API、仕様、ターゲットの設計論点として整理したものである。

原案の中心は、所有権と借用を言語意味論に置き、GCやlinear memoryなどのメモリ管理をターゲット実装から選べるようにすることにある。

原案には、既存の採択ADRと整合する項目、既存ADRを拡張する項目、既存ADRを変更する項目が含まれる。

本RFCは詳細設計の候補を記録するが、現行挙動の正本でも、採択済みの設計判断でもない。

## 2. 設計上の前提

### 2.1 言語意味論

値の複製、move、借用、可変性、クロージャ捕捉、評価順序、型変換、Trait解決を、特定のWasm命令やGC heap layoutから独立して定義する。

不変な束縛を既定とし、再代入には `let mut` を使う。

暗黙コピーを許す型とmoveされる型を区別し、複製が必要な場所には `Copy` または `clone` を明示する。

借用を導入する場合は、共有借用と可変借用、借用の有効範囲、返却または保存を越えるescapeを型検査の契約に含める。

### 2.2 メモリ管理とターゲット

GC、参照カウント、linear memory、native runtimeは、共通の言語意味論を実装する候補である。

`wasm32-gc` と `wasm32` が異なる意味論を提供するのではなく、同一の意味を各ターゲットの表現へloweringする。

ターゲットが表現できない機能は、別の値の意味に黙って変換せず、target capability errorとして診断する。

この候補は現在の [ADR-002](../adr/ADR-002-memory-model.md)、[ADR-013](../adr/ADR-013-primary-target.md)、[ADR-035](../adr/ADR-035-wasm-gc-implementation.md) を変更するため、採択には後続ADRが必要である。

### 2.3 APIと境界

公開APIの正規形はTrait、メソッド、associated functionとする。

Traitは `Self`、関連型、型引数、coherence、blanket implementationを明示的な解決規則として持つ。

`?` は `Option` と `Result` の構造を保ったまま扱い、異なるエラー型の変換には `From` 系の規則を使う。

`pub` によるソース可視性とComponent exportを分離する。

ソースモジュールの `use` とWITのComponent境界を同じ名前空間として扱わない。

### 2.4 仕様と検証

評価順序、数値変換、overflow、NaN、ゼロ除算、Unicode、slice、脱糖、Trait解決を観測可能な規則として記述する。

仕様の例は、可能な範囲で実行可能なfixture、compile/validate gate、または独立checkerへ接続する。

性能やメモリ使用量は、想定チャートではなく、ターゲット、入力、toolchain、測定方法を固定したreceiptで比較する。

## 3. 45項目の提案カタログ

下表の「扱い」は、このRFCを取り込む時点での既存ADRとの関係を示す。

| 番号 | 対象 | 提案の要点 | 既存契約との扱い |
|---:|---|---|---|
| 1 | 暗黙の共有参照 | 値の所有者を明確にし、暗黙共有をmove、`Copy`、`clone`、借用の規則へ置き換える。 | ADR-002、ADR-035と衝突するため後続ADRが必要。 |
| 2 | `let mut` | 束縛を不変にし、再代入可能性を `let mut` で明示する。 | 新しい言語意味論としてRFCで詳細化する。 |
| 3 | 構造体とタプル | 構造体とタプルで値型、参照型、moveの分類を一致させる。 | 1と同じ型意味論の後続判断に含める。 |
| 4 | 配列の複製 | 配列の複製、move、要素単位の `Copy` を暗黙挙動から分離する。 | 回帰fixtureと移行診断が必要。 |
| 5 | `Option` と `Result` | payloadの所有権とunboxingを明示し、参照型であることを意味論の代用にしない。 | ADR-002、ADR-035の表現と要調整。 |
| 6 | enum | enumの値意味論とheap表現を分け、必要な最適化をbackendへ置く。 | ADR-035のGC layoutとの後続判断が必要。 |
| 7 | `Box<T>` | `Box<T>` が所有権、配置、単なる表現のどれを意味するかを固定する。 | 型とbackendの境界を別RFCで具体化する。 |
| 8 | GC依存 | 言語仕様からGC必須という前提を外し、メモリ管理をbackendの選択肢にする。 | ADR-002、ADR-013、ADR-035を変更するため後続ADRが必要。 |
| 9 | target capability | ターゲットごとの差を機能能力として宣言し、非対応機能をcompile-time errorにする。 | ADR-007、ADR-013のtarget契約と統合する。 |
| 10 | クロージャ | move捕捉、借用捕捉、escape、保持期間を明示し、クロージャ返却を検査する。 | ADR-033を拡張する詳細RFCが必要。 |
| 11 | `pub` とexport | ソース可視性とComponent exportを別の指定と契約にする。 | ADR-006と整合し、公開面の仕様を補う。 |
| 12 | Traitコア | `Self`、関連型、coherence、blanket implementation、静的または動的dispatchを定義する。 | ADR-044、RFC-004、ADR-036と統合する。 |
| 13 | `?` と `From` | `Option`、`Result`、異種エラー変換をTrait解決に接続する。 | ADR-039と整合する。 |
| 14 | magic method | `__add` などのmagic methodを演算子Traitへ移行する。 | ADR-038の提案を詳細化する。 |
| 15 | public free function | `push(v, x)` のような公開free functionをメソッドまたはTrait APIへ寄せる。 | ADR-046と整合する。 |
| 16 | generic API | 型専用helperを減らし、ジェネリックTrait APIへ統一する。 | ADR-003、ADR-044、ADR-046と統合する。 |
| 17 | 型パラメータ数 | 実装都合の上限を言語の理想として固定せず、表現可能なジェネリック規則を定義する。 | ADR-000の暫定制限禁止とADR-003に従う。 |
| 18 | `Any` | 公開 `Any` による型消去を避け、必要な動的多相は明示したTrait境界で表す。 | 公開型システムの個別判断が必要。 |
| 19 | `Error` sentinel | compiler内部のError sentinelをユーザーが使う型システムから分離する。 | 診断とエラー型の個別仕様が必要。 |
| 20 | `i64` と `f64` | 暗黙の数値昇格を禁止し、キャストまたは明示変換を要求する。 | 言語仕様と既存fixtureの移行判断が必要。 |
| 21 | 混合演算 | 数値型の組み合わせごとの変換表と、演算前後の型規則を定義する。 | normative specへ追加する。 |
| 22 | overflowとNaN | overflow、NaN、inf、ゼロ除算をターゲット差なしに観測可能な規則へする。 | Wasmとnativeの差を検証してから採択する。 |
| 23 | 評価順序 | 式の評価を左から右へ固定し、副作用の順序を仕様に記録する。 | 実行可能な意味論fixtureが必要。 |
| 24 | `String` | Unicode、UTF-8、文字単位操作、byte単位操作の境界を分ける。 | stdlibと仕様の同時改訂が必要。 |
| 25 | slice | sliceを独立したviewまたは借用として定義し、`Vec`との戻り値の不整合を解消する。 | 所有権案と依存するため後続判断が必要。 |
| 26 | `HashMap<K,V>` | 型専用のHashMap APIを隠し、ジェネリックな型安全APIにする。 | ADR-003とTrait stdlib案に統合する。 |
| 27 | let-generalization | let一般化とvalue restrictionを型検査規則として形式化する。 | 型システムRFCで扱う。 |
| 28 | 糖衣展開 | 脱糖をstdlibの関数名に依存させず、言語レベルの構文規則として定義する。 | parser、typechecker、stdlibの境界を整理する。 |
| 29 | f-string | f-stringを衛生的に脱糖し、埋め込み式の評価順序と名前解決を保つ。 | 文字列仕様とfixtureを追加する。 |
| 30 | `import` と `use` | ソースモジュールの参照面を整理し、キーワードの役割を重複させない。 | ADR-009、ADR-031との整合確認が必要。 |
| 31 | WIT import | WIT識別子、CLI、manifest、Component境界の受け渡しを一つのcanonical pathにする。 | ADR-031を正本として実装と仕様を一致させる。 |
| 32 | Prelude | 暗黙に使えるPreludeを小さくし、Traitや言語組み込みと明示的importを分離する。 | stdlib API移行とADR-014の確認が必要。 |
| 33 | 真偽値 | `true` と `false` を字句リテラルとして一意に扱い、Preludeとの重複をなくす。 | lexer、name resolution、fixtureを同期する。 |
| 34 | multi-clause関数 | 複数節の関数をどの構文単位としてまとめるかを明示する。 | parserと仕様の個別RFCが必要。 |
| 35 | `where` | pattern match後のmulti-clauseで、`where` の束縛範囲を一貫させる。 | P0相当の文法回帰fixtureを追加する。 |
| 36 | セミコロン | 改行、式、blockの終端規則を組み合わせたときの曖昧さをなくす。 | normative grammarへ反映する。 |
| 37 | Iterator | eager helper、lazy adapter、`IntoIterator`、`FromIterator`を共通のTrait面へ統一する。 | ADR-036とRFC-004に統合する。 |
| 38 | name mangling | name manglingを言語仕様から外し、backendとABIの実装詳細に限定する。 | ADR-006のABI分類と整合する。 |
| 39 | target名 | メモリ方式、WASI profile、出力形式の能力を一つのtarget名に詰め込まない。 | ADR-007、ADR-013の語彙と再整理する。 |
| 40 | `wasm32-gc` とMemory64 | GC、memory width、host profileの既定値を別々の設定として説明する。 | ADR-013、ADR-035、current-stateとの同期が必要。 |
| 41 | 予約キーワード | 将来用途だけの予約を減らし、実装済みまたは採択済みの構文だけを予約する。 | lexer契約と仕様分類を更新する。 |
| 42 | 安定性ラベル | stableを実装完成度ではなく、互換性保証のラベルとして扱う。 | ADR-014と整合する。 |
| 43 | specとredesign | 再設計中の節をstableと誤認させず、仕様の分類と実装状態を同期する。 | ADR-014、ADR-018に従う。 |
| 44 | 実行可能な文書例 | normativeな例をfixtureまたは検証コマンドへ接続し、根拠のないskipを減らす。 | ADR-029、ADR-057、ADR-018と整合する。 |
| 45 | 操作的意味論 | alias、copy、評価、数値、Unicode、クロージャ、Traitの観測可能な規則を一つの仕様体系へ置く。 | 仕様RFCと独立checkerが必要。 |

## 4. 領域ごとの詳細方針

### 4.1 所有権、借用、値の表現

move-only型、`Copy` 型、明示的な `clone`、共有借用、可変借用を区別する。

構造体、タプル、配列、enum、`Option`、`Result`、`Box`、`Vec`、`String`について、言語上の値とbackendの格納表現を別々に定義する。

GC backendでheap objectを使っても、それが言語上の共有参照や暗黙コピーを意味するとは限らない。

借用検査を採択する場合、最小限の規則として、可変借用の同時存在、共有借用との排他、返却値へのescape、クロージャへの捕捉を検証対象にする。

### 4.2 Traitと標準ライブラリ

`PartialEq`、`Eq`、`PartialOrd`、`Ord`、`Clone`、`Copy`、`Default`、`From`、`Into`、`TryFrom`、`TryInto`、`Iterator`、`IntoIterator`、`FromIterator`、IO系Trait、演算子Traitを重複しない関係として整理する。

`Into` と `TryInto` は初期設計では `From` と `TryFrom` から導出するblanket implementationを候補とするが、coherenceと型推論の規則を先に固定する。

Traitの静的dispatch、`dyn Trait`、associated type、型引数、ユーザー実装の衝突規則は、RFC-004とADR-036の内容を一つの型解決仕様へ統合する。

### 4.3 数値、文字列、コレクション

暗黙変換を減らし、整数、浮動小数点、SIMD、文字、byte、Unicode scalar valueの変換境界を表にする。

文字列のbyte indexと文字単位の位置を同じ値として扱わず、sliceの寿命と所有権を明示する。

HashMapやIteratorのAPIは型専用のhelperを増やさず、Traitとジェネリックなメソッドで表現する。

### 4.4 構文と脱糖

`import`、`use`、Prelude、真偽値、multi-clause、`where`、セミコロン、f-string、Iterator構文を、parserの偶然ではなく仕様上の構文変換として定義する。

糖衣構文が存在する場合、展開先のstdlib名を変更しても言語の意味が変わらないよう、compiler内部のcanonical representationへ直接変換する。

### 4.5 ターゲットとComponent境界

ターゲット名は、値の意味論、メモリ表現、WASI profile、Component emit、Memory64のような独立した能力を一つに混ぜない候補とする。

WIT package identifierはComponent境界の識別子として扱い、ソースモジュールの名前解決と同じ構文規則へ無理に押し込まない。

name manglingは安定した言語APIにせず、必要な範囲でABI資料とbackendのテストに閉じ込める。

### 4.6 仕様、安定性、検証

仕様の各節は、互換性を保証するstable、細部が変わりうるprovisional、設計中のexperimental、未実装のunimplementedを区別する。

設計案の例は、入力、期待結果、実行条件を持つfixtureにして、単なるMarkdownの断片を検証済みの証拠とみなさない。

操作的意味論を追加するときは、現行実装が従っている規則と将来の候補規則を同じ文書で混ぜず、current-stateとRFCを分ける。

## 5. 互換性と移行

このRFCは旧構文との互換性を自動的に保証しない。

stableな言語機能またはstdlib APIを変更する場合は、[ADR-014](../adr/ADR-014-stability-labels.md)に従い、非推奨期間、移行ガイド、削除条件を個別に定める。

provisionalまたはexperimentalな面でも、変更する入力と診断をfixtureで固定してから移行する。

所有権と借用を導入する場合、少なくとも次を同時に定義する。

- 既存の暗黙コピーをどの構文へ変換するか。
- `Copy` と `Clone` をどの型に実装するか。
- 借用エラー、escapeエラー、move後の利用をどう診断するか。
- GC backendでのheap objectと、言語上の所有者をどう対応づけるか。
- `wasm32` と `wasm32-gc` の能力差をどの段階で報告するか。

原案に含まれていたフェーズ日付、完了表示、トップ10の作業順は、実装計画やissueの正本には移さず、各後続ADRの採択後に独立したplanとして作成する。

## 6. 未決事項

次の事項は、このRFCだけでは決定しない。

1. 借用検査を言語の必須規則にするか、限定的な静的検査から始めるか。
2. GC backendで所有権を実行時に表現するか、型検査だけで消去するか。
3. `Box<T>`、`Option<T>`、`Result<T,E>`、enumの値表現とABIをどう固定するか。
4. `dyn Trait`、associated type、coherence、blanket implementationの初期範囲。
5. overflow、NaN、ゼロ除算、Unicode invalid sequenceの各処理結果。
6. `import` と `use` の最終構文、およびWIT parserのcanonical path。
7. `wasm32-gc`、`wasm32`、Memory64、WASI profileをCLIでどう表現するか。
8. stable機能の移行期間と、実行可能な仕様fixtureの必須範囲。

## 7. 採択までの条件

このRFCの項目を採択する前に、領域ごとに次を揃える。

- 現行契約との差分を示す仕様案。
- 既存ADRを変更する場合の後継ADR案。
- 正常系、診断系、ターゲット能力エラーを含む回帰fixture。
- compile、validate、runtime、必要な場合はComponent/WIT検証のreceipt。
- stable APIを変更する場合の移行手順。
- GCと非GCで同じ観測可能結果になることを示す比較。

性能評価は、入力とtoolchainを固定したbenchmarkで行い、原案の相対速度チャートは採択根拠にしない。

## 8. 関連

- [ADR-058: 言語 v2 の意味論と仕様境界を再設計する](../adr/ADR-058-language-v2-semantics-redesign.md)
- [ADR-002: GC vs non-GC](../adr/ADR-002-memory-model.md)
- [ADR-003: generics戦略](../adr/ADR-003-generics-strategy.md)
- [ADR-007: コンパイルターゲット整理](../adr/ADR-007-targets.md)
- [ADR-013: `wasm32-gc` をプライマリターゲットとする](../adr/ADR-013-primary-target.md)
- [ADR-014: 言語仕様と Stdlib API の安定性ラベル](../adr/ADR-014-stability-labels.md)
- [ADR-018: 言語ドキュメント分類](../adr/ADR-018-language-docs-classification.md)
- [ADR-029: セルフホストネイティブ検証契約](../adr/ADR-029-selfhost-native-verification-contract.md)
- [ADR-031: import 構文と WIT パッケージ識別子の統合](../adr/ADR-031-import-syntax-wit-unification.md)
- [ADR-035: Wasm GC 内部レイアウト方針](../adr/ADR-035-wasm-gc-implementation.md)
- [ADR-036: Trait-based Stdlib Redesign Strategy](../adr/ADR-036-trait-stdlib-redesign.md)
- [ADR-038: 演算子オーバーロードをmagic methodからTraitへ移行する](../adr/ADR-038-operator-overload-traits.md)
- [ADR-039: `?` の Option対応とエラー型変換](../adr/ADR-039-question-mark-operator.md)
- [ADR-044: traitとメソッド構文を言語機能として採択する](../adr/ADR-044-trait-method-syntax-adopted.md)
- [ADR-046: 公開 free function の根絶](../adr/ADR-046-free-function-eradication.md)
- [ADR-057: セルフホスト fixture 検証を current-only 並列ゲートへ移行](../adr/ADR-057-selfhost-current-only-fixture-gate.md)
- [RFC-004: Trait expressiveness](004-trait-expressiveness.md)
