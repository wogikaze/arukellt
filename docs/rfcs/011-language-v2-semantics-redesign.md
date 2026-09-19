# RFC-011: 言語 v2 の意味論再設計案

ステータス: DRAFT

提案日: 2026-09-18

関連ADR: [ADR-058](../adr/ADR-058-language-v2-semantics-redesign.md)、[ADR-060](../adr/ADR-060-language-v2-owned-adts-box-slice.md)、[ADR-061](../adr/ADR-061-language-v2-closure-capture-lifetime.md)、[ADR-062](../adr/ADR-062-language-v2-trait-core-coherence.md)、[ADR-063](../adr/ADR-063-language-v2-conversion-error-boundary.md)、[ADR-065](../adr/ADR-065-language-v2-operator-trait-resolution.md)、[ADR-066](../adr/ADR-066-language-v2-generic-public-api.md)、[ADR-067](../adr/ADR-067-language-v2-numeric-conversion.md)、[ADR-002](../adr/ADR-002-memory-model.md)、[ADR-003](../adr/ADR-003-generics-strategy.md)、[ADR-007](../adr/ADR-007-targets.md)、[ADR-013](../adr/ADR-013-primary-target.md)、[ADR-014](../adr/ADR-014-stability-labels.md)、[ADR-031](../adr/ADR-031-import-syntax-wit-unification.md)、[ADR-035](../adr/ADR-035-wasm-gc-implementation.md)、[ADR-044](../adr/ADR-044-trait-method-syntax-adopted.md)、[ADR-046](../adr/ADR-046-free-function-eradication.md)、[ADR-072](../adr/ADR-072-language-v2-prelude-and-keyword-policy.md)

---

## 1. 要約

本RFCは、受領した `new-adr.md` の45項目を、言語意味論、API、仕様、ターゲットの設計論点として整理したものである。

原案の中心は、所有権と借用を言語意味論に置き、GCやlinear memoryなどのメモリ管理をターゲット実装から選べるようにすることにある。

原案には、既存の採択ADRと整合する項目、既存ADRを拡張する項目、既存ADRを変更する項目が含まれる。
この RFC のうち所有権・`let mut`・owned aggregate・配列の move / Copy（1–4）は
[ADR-059](../adr/ADR-059-language-v2-ownership-copy-mutability.md) で採択済みであり、
以下では候補ではなく現行の v2 意味論として扱う。さらに、owned ADT、`Box<T>`、`Slice<T>` の値境界は
[ADR-060](../adr/ADR-060-language-v2-owned-adts-box-slice.md) で採択済みである。
完全な借用 lifetime、clone の詳細、backend の bytes/ABI は引き続き別 issue / ADR の対象である。

本RFCは詳細設計の候補を記録するが、現行挙動の正本でも、採択済みの設計判断でもない。

## 2. 設計上の前提

### 2.1 言語意味論

値の複製、move、借用、可変性、クロージャ捕捉、評価順序、型変換、Trait解決を、特定のWasm命令やGC heap layoutから独立して定義する。

不変な束縛を既定とし、そのbindingを通した再代入と通常の可変操作には `let mut` を使う。mutabilityと`Copy`可能性は別軸として扱う。

暗黙コピーを許す型とmoveされる型を区別し、複製が必要な場所には `Copy` または `clone` を明示する。ユーザー定義struct/enumは、全fieldが`Copy`であっても自動的には `Copy` とせず、明示的な opt-in を要求する。

借用を導入する場合は、共有借用と可変借用、借用の有効範囲、返却または保存を越えるescapeを型検査の契約に含める。

### 2.2 メモリ管理とターゲット

GC、参照カウント、linear memory、native runtimeは、共通の言語意味論を実装する候補である。

`wasm32-gc` と `wasm32` が異なる意味論を提供するのではなく、同一の意味を各ターゲットの表現へloweringする。Wasm GC 対応環境では managed aggregate や共有グラフを `struct` / `array` / `ref` へ直接 lower する経路を優先し、ファイルサイズ、実行性能、ホストGC統合の利点を狙う。

Wasm GC 非対応環境では linear memory 上の allocator と tracing GC runtime 等を用意し、共有・循環参照を含む通常のソースプログラムを同じ観測可能意味論で実行できるようにする。GCの有無だけを理由に通常コードをcompile-time errorへしない。

host固有API、ABI、SIMDなど、本当にターゲット能力に依存する機能はtarget capabilityとして診断する。

この候補は現在の [ADR-002](../adr/ADR-002-memory-model.md)、[ADR-013](../adr/ADR-013-primary-target.md)、[ADR-035](../adr/ADR-035-wasm-gc-implementation.md) の責務境界を変更するため、採択には後続ADRが必要である。

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
| 3 | 構造体とタプル | どちらも owned value とし、代入は move、明示 `Copy` だけ暗黙 copy とする。ユーザー定義型の `Copy` は field 構成だけで自動決定しない。 | ADR-059 で採択済み。 |
| 4 | 配列の複製 | 配列の複製、move、要素単位の `Copy` を暗黙挙動から分離する。 | ADR-059 で採択済み。回帰 fixture は #853 が担当する。 |
| 5 | `Option` と `Result` | 通常のowned ADTとして定義し、payloadに応じたmove/`Copy`を適用する。GC object、tagged value、unboxed等はbackend表現とする。 | ADR-060で採択済み。 |
| 6 | enum | tagged ADTとしての意味とheap/unboxed表現を分離し、`Copy`は明示opt-in、representation optimizationはbackendへ置く。 | ADR-060で採択済み。 |
| 7 | `Box<T>` | heap上の`T`を一意に所有する値とし、borrow、共有所有、tracing GC管理を別概念にする。 | ADR-060で採択済み。 |
| 8 | GC依存 | Wasm GCを対応環境の優先backendとして積極利用しつつ、非対応環境ではlinear-memory runtime fallbackで同じsource semanticsを維持する。 | #854のADT/Box/Slice境界についてADR-060で適用を確認。tracing fallback全体は後続issue。 |
| 9 | target capability | GC有無はbackend capabilityとし通常コードの合法性を変えない。host API、ABI、SIMD等の真にtarget固有な機能だけをcapability errorにする。 | ADR-007、ADR-013のtarget契約と統合する。 |
| 10 | クロージャ | move捕捉、借用捕捉、escape、保持期間を明示し、クロージャ返却を検査する。 | ADR-061で採択済み。#855で shared/mutable borrow、move capture、return/store/callback escape を実装。managed closure environment の lowering は #872/#874。 |
| 11 | `pub` とexport | ソース可視性とComponent exportを別の指定と契約にする。`pub` は source visibility、`export fn` は Component export とする。 | [ADR-071](../adr/ADR-071-component-export-visibility-boundary.md) で採択・実装済み。 |
| 12 | Traitコア | `Self`、関連型、coherence、blanket implementation、静的または動的dispatchを定義する。 | ADR-062で `Self`、associated item、coherence、user blanket禁止、static dispatch、`dyn`境界を採択。 |
| 13 | `?` と `From` | `Option`、`Result`、異種エラー変換をTrait解決に接続する。 | ADR-039、ADR-063で canonical `From`、E0210、`TryFrom` 非暗黙探索を採択。 |
| 14 | magic method | `__add` などのmagic methodを演算子Traitへ移行する。 | ADR-065で採択済み。旧ADR-038を置換する。 |
| 15 | public free function | `push(v, x)` のような公開free functionをメソッドまたはTrait APIへ寄せる。 | ADR-046と整合する。HashMap の constructor / receiver method 面は ADR-066 で採択済み。 |
| 16 | generic API | 型専用helperを減らし、ジェネリックTrait APIへ統一する。 | ADR-003、ADR-044、ADR-046と統合する。HashMap の static generic API は ADR-066 で採択済み。 |
| 17 | 型パラメータ数 | 実装都合の上限を言語の理想として固定せず、表現可能なジェネリック規則を定義する。 | ADR-000の暫定制限禁止とADR-003に従う。 |
| 18 | `Any` | 公開 `Any` による型消去を避け、必要な動的多相は明示したTrait境界で表す。 | `Any` は source type にせず、compiler builtin の内部 placeholder / backend representation に限定することを ADR-066 で採択。 |
| 19 | `Error` sentinel | compiler内部のError sentinelをユーザーが使う型システムから分離する。 | ADR-063で内部 recovery state と nominal `Error` を分離。 |
| 20 | `i64` と `f64` | `i64 -> f64`だけでなく異なる具象数値型の暗黙昇格を禁止し、literal inference と concrete conversion を分離する。 | [ADR-067](../adr/ADR-067-language-v2-numeric-conversion.md) で採択済み。 |
| 21 | 混合演算 | `u32 + i16`等の異種具象数値演算を型エラーとし、巨大な暗黙promotion matrixを作らない。明示変換のlossless/checked/wrapping/saturating規則を定義する。 | [ADR-067](../adr/ADR-067-language-v2-numeric-conversion.md) と normative spec へ反映する。 |
| 22 | overflowとNaN | overflow、NaN、inf、ゼロ除算をターゲット差なしに観測可能な規則へする。 | [ADR-068](../adr/ADR-068-language-v2-arithmetic-evaluation-semantics.md) で Wasm integer/float semantics と target parity を採択済み。 |
| 23 | 評価順序 | 構文ごとに評価順序をArukellt自身の規則として固定する。引数・二項演算・aggregate初期化はsource order、代入はplace評価→右辺評価→storeとする。 | [ADR-068](../adr/ADR-068-language-v2-arithmetic-evaluation-semantics.md) と副作用付き fixture で採択済み。 |
| 24 | `String` | Unicode、UTF-8、文字単位操作、byte単位操作の境界を分ける。 | ADR-069で採択済み。仕様・stdlib・GC／linear parity fixtureへ反映する。 |
| 25 | slice | sliceを独立した非`Copy`の読み取りviewとして定義し、`Vec`との戻り値の不整合を解消する。source binding は現行 API では消費しない。 | ADR-060で採択済み。完全なlifetime/borrow検査は後続。 |
| 26 | `HashMap<K,V>` | 型専用のHashMap APIを隠し、ジェネリックな型安全APIにする。 | ADR-066 で `HashMap::new<K,V>()` と receiver methods、supported pair 診断、旧 API の removal condition を採択。 |
| 27 | let-generalization | effect systemを導入しない段階では、曖昧な「pure判定」ではなく構文的なnon-expansive/value restrictionとして形式化する。 | ADR-070で immutable binding、environment-aware generalization、allocation/call/mutable state の境界を採択済み。 |
| 28 | 糖衣展開 | shadow可能なstdlib名を通常の名前解決で引かず、lang item / pre-resolved symbol / intrinsic等へのhygienic desugaringとして定義する。 | parser、typechecker、stdlibの境界を整理する。 |
| 29 | f-string | f-stringを衛生的に脱糖し、埋め込み式の評価順序と名前解決を保つ。 | 文字列仕様とfixtureを追加する。 |
| 30 | `import` と `use` | ソースモジュールの参照面を整理し、キーワードの役割を重複させない。 | #866で `use` を正規形、legacy `import` を `W0101` 付き移行構文として実装。alias、selective import、`pub use`、loader/resolver/formatter/LSPの同期を確認。仕様は [import-system.md](../spec/import-system.md) を参照。 |
| 31 | WIT import | WIT識別子、CLI、manifest、Component境界の受け渡しを一つのcanonical pathにする。 | [ADR-071](../adr/ADR-071-component-export-visibility-boundary.md) で full version 比較と source/WIT namespace 分離を採択し、ADR-031を精緻化する。 |
| 32 | Prelude | 「最小化」自体を目的にせず、小さく安定した集合を固定する。頻度の低いhelper/Trait/APIは明示importとし、変更は互換性方針に従う。 | [ADR-072](../adr/ADR-072-language-v2-prelude-and-keyword-policy.md) で stable set、legacy auto-import boundary、#878 の移行条件を採択。 |
| 33 | 真偽値 | `true` と `false` を字句リテラルとして一意に扱い、Preludeとの重複をなくす。 | [ADR-072](../adr/ADR-072-language-v2-prelude-and-keyword-policy.md) で manifest value entry を削除し、lexer・name resolution・fixtureを同期。 |
| 34 | multi-clause関数 | 複数節の関数をどの構文単位としてまとめるかを明示する。 | parserと仕様の個別RFCが必要。 |
| 35 | `where` | pattern match後のmulti-clauseで、`where` の束縛範囲を一貫させる。 | P0相当の文法回帰fixtureを追加する。 |
| 36 | セミコロン | 改行、式、blockの終端規則を組み合わせたときの曖昧さをなくす。 | normative grammarへ反映する。 |
| 37 | Iterator | eager helper、lazy adapter、`IntoIterator`、`FromIterator`を共通のTrait面へ統一する。 | ADR-036とRFC-004に統合する。 |
| 38 | name mangling | name manglingを言語仕様から外し、backendとABIの実装詳細に限定する。 | ADR-071 / ADR-006のABI分類と整合し、language spec の具体形式を削除済み。 |
| 39 | target名 | メモリ方式、WASI profile、出力形式の能力を一つのtarget名に詰め込まない。 | ADR-007、ADR-013の語彙と再整理する。 |
| 40 | `wasm32-gc` とMemory64 | GC、memory width、host profileの既定値を別々の設定として説明する。 | ADR-013、ADR-035、current-stateとの同期が必要。 |
| 41 | 予約キーワード | hard reserved / contextual / future-reservedに分類し、未実装という理由だけで将来語を解放しない。 | [ADR-072](../adr/ADR-072-language-v2-prelude-and-keyword-policy.md) と `docs/data/language-keyword-policy.toml` で分類・lexer・LSP・playgroundを同期。 |
| 42 | 安定性ラベル | stableを実装完成度ではなく、互換性保証のラベルとして扱う。 | ADR-014と整合する。 |
| 43 | specとredesign | 再設計中の節をstableと誤認させず、仕様の分類と実装状態を同期する。 | ADR-014、ADR-018に従う。 |
| 44 | 実行可能な文書例 | normativeな例をfixtureまたは検証コマンドへ接続し、根拠のないskipを減らす。 | ADR-029、ADR-057、ADR-018と整合する。 |
| 45 | 操作的意味論 | alias、copy、評価、数値、Unicode、クロージャ、Traitの観測可能な規則を一つの仕様体系へ置く。 | 仕様RFCと独立checkerが必要。 |

## 4. 領域ごとの詳細方針

### 4.1 所有権、借用、値の表現

move-only型、`Copy` 型、明示的な `clone`、共有借用、可変借用を区別する。`Copy`可能性とmutabilityは独立に扱う。

構造体、タプル、配列、enum、`Option`、`Result`、`Box`、`Vec`、`String`について、言語上は owned value としての意味を先に定義し、backend の格納表現と分離する。ユーザー定義 struct / enum の `Copy` は明示的な opt-in を要求する。

`Option` / `Result` は payload がすべて `Copy` の場合だけ構造的に `Copy` とし、ユーザー enum は明示 `impl Copy` が必要である。
`Box<T>` は非 `Copy` の一意所有値、`Slice<T>` は source collection と range を保持する非 `Copy` の読み取り view である。
`as_slice` は source binding を消費しない。完全な lifetime / mutation 排他はこの RFC の後続項目で扱う。

`Box<T>` はheap上の`T`を一意に所有する値として定義し、`&T` / `&mut T`、共有ownership、tracing GC管理は別概念にする。

GC backendでheap objectを使っても、それが言語上の共有参照や暗黙コピーを意味するとは限らない。

借用検査を採択する場合、最小限の規則として、可変借用の同時存在、共有借用との排他、返却値へのescape、クロージャへの捕捉を検証対象にする。

### 4.1.1 クロージャ捕捉の採択結果（ADR-061 / #855）

通常の `|params| body` は、非 `Copy` の外側 binding を shared borrow として捕捉し、body がその place を変更する場合は mutable borrow とする。
`move |params| body` は非 `Copy` binding を作成時に消費する。
`Copy` binding は `move` を付けても Copy 規則に従い消費しない。

borrow capture を含む closure の return、aggregate への保存、callback 引数への受け渡しは enclosing function の外へ escape するため E0213 で拒否する。
borrow 中の source への再代入または再 move は E0214 で拒否する。
これらの検査結果は `wasm32-gc` と `wasm32` で共有し、lambda lifting や managed environment の表現は意味論を変更しない。

### 4.2 Traitと標準ライブラリ

`PartialEq`、`Eq`、`PartialOrd`、`Ord`、`Clone`、`Copy`、`Default`、`From`、`Into`、`TryFrom`、`TryInto`、`Iterator`、`IntoIterator`、`FromIterator`、IO系Trait、演算子Traitを重複しない関係として整理する。

`Into` と `TryInto` は初期設計では `From` と `TryFrom` から導出するblanket implementationを候補とするが、coherenceと型推論の規則を先に固定する。

Traitの静的dispatch、`dyn Trait`、associated type、型引数、ユーザー実装の衝突規則は、RFC-004とADR-036の内容を一つの型解決仕様へ統合する。

### 4.3 数値、文字列、コレクション

[ADR-067](../adr/ADR-067-language-v2-numeric-conversion.md) に従い、異なる具象数値型の
暗黙変換は行わない。整数、浮動小数点、SIMD、文字、byte、Unicode scalar valueの明示
変換境界を定義し、literal inference と concrete conversion を分離する。変換は lossless /
checked / wrapping / saturating 等の意味を区別し、型ペアごとの暗黙 promotion matrix は
作らない。

文字列のbyte indexと文字単位の位置を同じ値として扱わず、sliceの寿命と所有権を明示する。

#### 4.3.1 String / Unicode semantics（ADR-069）

String は well-formed UTF-8 の言語値であり、char は Unicode scalar value である。
byte index と scalar index は別の API 面で扱う。byte_at、byte_len、slice_bytes、
to_utf8_bytes、try_from_utf8 は byte 面、len_chars、codepoint_at、chars、slice_chars は
scalar 面に属する。char_at は移行期間だけ残る raw byte compatibility API とする。

slice の byte 境界、strict な try_from_utf8、invalid input を U+FFFD へ置換する from_utf8、
char_to_string / push_char の canonical UTF-8 encoding は ADR-069 を正本とする。
これらの observable result は wasm32-gc と wasm32 で一致させ、GC array と linear-memory
layout の違いを source semantics に漏らさない。

HashMapやIteratorのAPIは型専用のhelperを増やさず、Traitとジェネリックなメソッドで表現する。

#### 4.3.1 HashMap の generic source surface（ADR-066）

HashMap の canonical source surface は `HashMap::new<K,V>()` と receiver method
（`insert`、`get`、`contains_key`、`remove`、`len`、`is_empty`）である。
具体型は static monomorphization で確定し、未対応の concrete pair は `Any` へ暗黙に
erasureせず typecheck error にする。現行の supported pair は `i32` / `String` key と
`i32`、`String`、`fn(i32) -> i32` value の組合せである。

同じ source call は `wasm32-gc` では GC managed representation へ、`wasm32` では
linear-memory representation へ lower する。`Option<V>`、hash/equality、remove、size
の観測結果を target ごとに変えてはならない。type-suffixed entry と raw hash helper は
ADR-014 の移行期間に限る compatibility surface であり、canonical API ではない。

### 4.4 構文と脱糖

`import`、`use`、Prelude、真偽値、multi-clause、`where`、セミコロン、f-string、Iterator構文を、parserの偶然ではなく仕様上の構文変換として定義する。

Preludeは「最小であること」より、小さく安定した集合であることを優先する。
v2 の stable set と移行中の legacy auto-import は分けて扱う。`true` と `false` は lexical literal であり、Prelude value ではない。
予約語は hard reserved / contextual / future-reserved に分類し、未実装であるという理由だけでは将来予約を解除しない。
具体的な分類は [ADR-072](../adr/ADR-072-language-v2-prelude-and-keyword-policy.md) と
[`docs/data/language-keyword-policy.toml`](../data/language-keyword-policy.toml) に従う。

糖衣構文が存在する場合、展開先のstdlib名を通常の名前解決で検索しない。lang item、pre-resolved symbol、intrinsic等のhygienicな参照へ変換し、stdlib APIの名前変更やユーザー定義名のshadowingでソース構文の意味が変わらないようにする。

### 4.5 ターゲットとComponent境界

ターゲット名は、値の意味論、メモリ表現、WASI profile、Component emit、Memory64のような独立した能力を一つに混ぜない候補とする。Wasm GCは対応環境での優先 lowering とし、非対応環境ではlinear-memory runtime fallbackを用意する。通常の共有・循環参照コードについて、GC featureの有無をsource-level capabilityにはしない。

WIT package identifierはComponent境界の識別子として扱い、ソースモジュールの名前解決と同じ構文規則へ無理に押し込まない。`import "namespace:package/interface[@version]"` は Layer C の canonical source form であり、`use ...::...` は Layer S のまま分離する。詳細は ADR-071 に従う。

name manglingは安定した言語APIにせず、必要な範囲でABI資料とbackendのテストに閉じ込める。

### 4.6 仕様、安定性、検証

仕様の各節は、互換性を保証するstable、細部が変わりうるprovisional、設計中のexperimental、未実装のunimplementedを区別する。

設計案の例は、入力、期待結果、実行条件を持つfixtureにして、単なるMarkdownの断片を検証済みの証拠とみなさない。

操作的意味論を追加するときは、現行実装が従っている規則と将来の候補規則を同じ文書で混ぜず、current-stateとRFCを分ける。

## 5. 互換性と移行

このRFCは旧構文との互換性を自動的に保証しない。

stableな言語機能またはstdlib APIを変更する場合は、[ADR-014](../adr/ADR-014-stability-labels.md)に従い、非推奨期間、移行ガイド、削除条件を個別に定める。

HashMap の具体型別 entry から generic method への移行条件は
[ADR-066](../adr/ADR-066-language-v2-generic-public-api.md) と
[HashMap migration guidance](../stdlib/migration-guidance.md) に従う。

provisionalまたはexperimentalな面でも、変更する入力と診断をfixtureで固定してから移行する。

所有権と借用を導入する場合、少なくとも次を同時に定義する。

- 既存の暗黙コピーをどの構文へ変換するか。
- `Copy` と `Clone` をどの型に実装するか。
- 借用エラー、escapeエラー、move後の利用をどう診断するか。
- GC backendでのheap objectと、言語上の所有者をどう対応づけるか。
- `wasm32-gc` のmanaged representationと、`wasm32` fallback runtimeで同じ所有・共有・循環参照意味論をどう対応づけるか。

原案に含まれていたフェーズ日付、完了表示、トップ10の作業順は、実装計画やissueの正本には移さず、各後続ADRの採択後に独立したplanとして作成する。

## 6. 未決事項

次の事項は、このRFCだけでは決定しない。

1. 借用検査を言語の必須規則にするか、限定的な静的検査から始めるか。
2. Wasm GC backendで所有権を実行時にどこまで表現するか、型検査後にどこまで消去するか。またlinear-memory fallbackのcollector/root管理をどのruntime契約にするか。
3. `Box<T>`、`Option<T>`、`Result<T,E>`、enumの値表現とABIをどう固定するか（source-level owned ADT 契約はADR-060で採択済み）。
4. ADR-062 の範囲外である `dyn Trait` の動的 ABI、specialization、higher-kinded projection。
5. Unicode normalization、grapheme cluster、その他の text segmentation API。
6. WIT parserのcanonical pathとComponent境界の受け渡し。Layer C の source form、full version comparison、`export fn` の境界は ADR-071 で採択済みであり、world 宣言など追加 surface は別判断とする。
7. `wasm32-gc`、`wasm32`、Memory64、WASI profileをCLIでどう表現するか。
8. stable機能の移行期間と、実行可能な仕様fixtureの必須範囲。

## 7. 採択までの条件

このRFCの項目を採択する前に、領域ごとに次を揃える。

- 現行契約との差分を示す仕様案。
- 既存ADRを変更する場合の後継ADR案。
- 正常系、診断系、ターゲット能力エラーを含む回帰fixture。
- compile、validate、runtime、必要な場合はComponent/WIT検証のreceipt。
- stable APIを変更する場合の移行手順。
- Wasm GC backendとlinear-memory fallbackで、共有・循環参照を含む同じ観測可能結果になることを示す比較。

性能評価は、入力とtoolchainを固定したbenchmarkで行い、原案の相対速度チャートは採択根拠にしない。少なくとも `.wasm` size、startup、steady-state runtime、allocation throughput、peak memory を個別に記録し、Wasm GCがどのworkloadで有利かを実測する。

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
- [ADR-065: Language v2 の演算子 Trait 解決と magic method 移行](../adr/ADR-065-language-v2-operator-trait-resolution.md)
- [ADR-066: Language v2 の generic 公開 API と HashMap](../adr/ADR-066-language-v2-generic-public-api.md)
- [ADR-068: Language v2 の算術 edge と評価順序](../adr/ADR-068-language-v2-arithmetic-evaluation-semantics.md)
- [ADR-039: `?` の Option対応とエラー型変換](../adr/ADR-039-question-mark-operator.md)
- [ADR-044: traitとメソッド構文を言語機能として採択する](../adr/ADR-044-trait-method-syntax-adopted.md)
- [ADR-046: 公開 free function の根絶](../adr/ADR-046-free-function-eradication.md)
- [ADR-057: セルフホスト fixture 検証を current-only 並列ゲートへ移行](../adr/ADR-057-selfhost-current-only-fixture-gate.md)
- [ADR-071: `pub` と Component export、WIT identity の境界](../adr/ADR-071-component-export-visibility-boundary.md)
- [RFC-004: Trait expressiveness](004-trait-expressiveness.md)
- [ADR-069: Language v2 の String / Unicode semantics](../adr/ADR-069-language-v2-string-unicode-semantics.md)
