# ADR-070: Language v2 の let-generalization と value restriction

ステータス: **ACCEPTED** — effect system 導入前の `let` 多相化を
構文的な non-expansive/value restriction として固定する

決定日: 2026-09-19

関連: [ADR-014](ADR-014-stability-labels.md)、
[ADR-029](ADR-029-selfhost-native-verification-contract.md)、
[ADR-058](ADR-058-language-v2-semantics-redesign.md)、
[ADR-059](ADR-059-language-v2-ownership-copy-mutability.md)、
[ADR-061](ADR-061-language-v2-closure-capture-lifetime.md)、
[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、
[#864](../../issues/open/864-let-generalization-value-restriction.md)

## 文脈

Arukellt は、型変数を含む `let` binding を内部の `TypeScheme` として保存し、
binding の参照ごとに型変数を instantiate する。これにより、同じ immutable helper を
複数の具象型で使える。

一方、型検査器が effect system を持たない段階で、initializer の実行時効果を
「pure らしい」という推測だけで判定すると、call、allocation、mutable state を含む
値を誤って多相化できる。特に、外側の generic parameter や mutable binding に由来する
型変数まで無条件に量化すると、同じ状態に異なる型を割り当てる unsound な scheme になる。

## 決定

### 1. generalize の入口

`let` binding は、次のすべてを満たすときだけ initializer の型を generalize する。

1. binding が `let mut` ではない。
2. initializer が下記の `NonExpansive` 構文である。
3. inferred type のうち、現在の typing environment または宣言された generic
   parameter から自由に参照される型変数は量化しない。

`TypeScheme` は `vars` と `body` を持ち、`vars` に残った型変数だけを量化する。
scheme の参照は、量化された各変数を fresh type variable へ置き換えて instantiate する。

### 2. NonExpansive 構文

effect system が導入されるまでの `NonExpansive` は、次の構文に限定する。

- 整数、浮動小数点、文字列、真偽値、文字の literal
- closure literal
- 既に immutable な generalized binding の identifier 参照

identifier は、単に AST node が identifier であるという理由だけでは eligible にならない。
monomorphic parameter、mutable binding、expansive initializer から得た binding の alias は
generalize しない。

次の構文は `Expansive` とする。

- 関数 call、method call、index 式
- tuple、array、array repeat、struct literal などの aggregate allocation
- block、`if`、`match`、comprehension、assignment、unary/binary 式
- path や、上記に分類されない式

call が既知の pure function に見えても、effect system がない段階では `Expansive` のまま
とする。aggregate や closure が backend 上で allocation されるかどうかはこの分類を変えない。
literal と closure は source-level value であり、backend の表現（Wasm GC object または
linear-memory object）は型の generalization 規則に影響しない。

### 3. mutable state と environment

`let mut` binding は、その initializer が closure や literal でも monomorphic にする。
closure が外側の binding を参照する場合、外側の binding の型変数は typing environment の
自由変数として扱い、inner `let` の scheme に取り込まない。この規則は、mutable state、
function parameter、外側の generic parameter と closure の型変数を分離する。

この決定は mutable state を禁止するものではない。mutable binding の再代入と borrow/escape
検査は ADR-059/ADR-061 の責務であり、ここではその状態に依存する型変数を多相化しない境界だけを
固定する。

### 4. backend 境界

NonExpansive 判定、environment-aware generalization、scheme instantiation は、AST と
型環境を扱う共通 typechecker の責務とする。Wasm GC、linear-memory fallback、native の
lowering は TypeScheme や value restriction の意味を変更してはならない。

## 帰結

- effect system がなくても、pure 判定の推測に依存せず deterministic に多相化を決められる。
- call と aggregate allocation の結果は、型変数を含んでいても同じ binding 内で共有される。
- immutable generalized closure は複数の具象型で利用できる。
- `let mut` と外側の generic/mutable state は monomorphic のまま残るため、後続の effect
  system でより精密な規則へ置き換えられる。
- Wasm GC と linear-memory fallback は同じ source/typecheck 結果を共有し、backend ごとの
  独自の多相化経路を持たない。

## 却下した代替案

### inferred expression をすべて generalize する

call、allocation、mutable state による identity と評価効果を型検査器が表現できず、
outer environment の型変数を誤って量化するため採用しない。

### pure-looking function/call を名前で判定する

callee の実装、stdlib の alias、backend intrinsic の差が source の型意味論へ漏れ、
名前解決や移行のたびに value restriction が変わるため採用しない。

### 先に effect system を導入する

effect system は将来の置換候補だが、ownership、trait、backend parity と独立した #864 の
完了条件を越える。現段階では構文的な conservative rule を採用し、effect system は別の
設計判断でこの境界を置換する。

## 再検討条件

- effect system または ownership/effect の統合により、call と allocation の effect を
  sound に推論できるようになった場合。
- immutable aggregate constructor の allocation と mutation/identity を型・effectで
  明示でき、現在の conservative rule より広い safe generalization が必要になった場合。
- TypeScheme を backend-specific な representation へ変更する必要が生じた場合。

## 参照

- [Language v2 semantics RFC](../rfcs/011-language-v2-semantics-redesign.md)
- [言語仕様 §2.8](../language/spec.md#28-let-generalization-typescheme)
- [型システム実用ガイド](../language/type-system.md)
