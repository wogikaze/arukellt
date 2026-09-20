# ADR-076: Language v2 Iterator の producer / consumer 責務境界

ステータス: **ACCEPTED** — `Iterator` の item 多相な producer 契約と、数値 consumer の capability trait を分離する

決定日: 2026-09-20

関連: [ADR-036](ADR-036-trait-stdlib-redesign.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-062](ADR-062-language-v2-trait-core-coherence.md)、[ADR-066](ADR-066-language-v2-generic-public-api.md)、[ADR-075](ADR-075-language-v2-linear-memory-tracing-runtime.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#691](../../issues/open/691-iterator-trait-lazy-adapters.md)、[#860](../../issues/open/860-iterator-unification-integration.md)

## 文脈

`Iterator` は `Self::Item` を associated type として持つ producer であり、
`wasm32-gc` と linear-memory `wasm32` の両方で同じ source-level iteration
semantics を提供する必要がある。

一方、旧来の実装は `Iterator` の必須 method に `sum -> i32` と
`fold(i32, fn(i32, i32)) -> i32` を含めていた。この契約では `String` や
他の値を item とする iterator が正当に実装できず、数値専用 consumer の ABI
制約が producer の trait 実装へ漏れる。`Vec<Self::Item>` を返す `collect` と
要素数だけを返す `count` には、この制約はない。

ADR-062 は associated type、coherence、static dispatch を採択している。
したがって、item の表現を整数へ pack したり、適用できない数値処理を no-op
として実装したりして境界を隠すことはできない。

## 決定

### 1. `Iterator` は item 多相な producer 契約を持つ

`Iterator` の正規責務は次の通りとする。

- `type Item` と `next(self) -> Option<Self::Item>` による逐次生成。
- `collect(self) -> Vec<Self::Item>` による item-preserving な canonical
  collection。
- `count(self) -> i32` による item-independent な消費。

`collect` の destination を `Vec` 以外へ拡張する場合は、既存の
`FromIterator<Source>` と static dispatch を使う。destination の型を消す
dynamic collection や、item を別の ABI 型へ暗黙変換する経路は作らない。

### 2. 数値 consumer は capability trait へ分離する

数値の集約は `Iterator` の必須 method ではなく、入力 item と出力型を trait
identity に含める capability とする。

```text
IteratorSum<Item, Output> {
    fn sum(self) -> Output
}

IteratorFold<Item, Accumulator> {
    fn fold(self, init: Accumulator,
            f: fn(Accumulator, Item) -> Accumulator) -> Accumulator
}
```

`IteratorSum<Item, Output>` / `IteratorFold<Item, Accumulator>` の実装は、同じ
receiver の `Iterator::Item` と `Item` が一致する場合だけ候補になる。
これは型消去や user-defined blanket impl ではなく、ADR-062 の concrete
coherence と static monomorphization に従う。`i32` の集約は
`IteratorSum<i32, i32>` と `IteratorFold<i32, i32>` で表し、文字列や構造体の
iterator に数値 capability を要求しない。

method 名は `sum` / `fold` のまま維持し、trait import と候補解決は ADR-062
の method-call 規則に従う。将来、加算・畳み込みの一般的な algebraic trait を
導入する場合は、その identity、overflow、失敗表現、callback ABI を別の ADR
で採択する。

### 3. `IntoIterator` / `FromIterator` と target parity

`IntoIterator` は source から concrete iterator への変換、`FromIterator` は
concrete iterator から destination への構築を担当する。両者を
`Iterator` の数値 capability と結合しない。

trait 解決、item 型、consumer の observable result は target 非依存とする。
`wasm32-gc` は typed reference / GC representation を主経路とし、
linear-memory `wasm32` は callback table と linear representation を使うが、
item の順序、終端、collection、overflow の source semantics は一致させる。

### 4. 互換 API の移行

`std::seq` の monomorphic helper は canonical iterator consumer の compatibility
entry として段階的に委譲する。manifest に method-level の deprecation 契約が
存在しない間は、helper を deprecate 済みと記録しない。deprecation と削除は
ADR-014、ADR-036、ADR-066 の migration 条件に従う。

## 帰結

- `String`、`bool`、struct などの item-preserving iterator は `Iterator` と
  `IntoIterator` を実装できる。数値集約が必要な場合だけ対応する capability
  trait を実装する。
- `Iterator` の impl へ無関係な consumer を要求しないため、型/trait の意味論と
  backend ABI の責務が分離される。
- 現行の `std::core::iter` が `i32` lane の `sum` / `fold` を `Iterator` に
  直接持たせている部分は実装ギャップであり、#860 の移行作業で capability
  trait へ移す。既存 compatibility helper の観測結果は回帰 fixture で固定する。
- `dyn Trait`、user blanket、暗黙の item conversion は導入しない。必要になった
  場合は ADR-062 の再検討条件または後続 ADR を通す。

## 却下した代替案

### すべての consumer を `Iterator` に残す

`sum` と `fold` の accumulator を `i32` に固定すると、非数値 item の iterator
を実装できない。`String` 用にダミー値を返す実装は観測可能意味論を壊すため採用
しない。

### item を整数へ pack して共通 ABI にする

文字列、struct、enum の所有権・境界・値の identity を失い、GC と linear-memory
の表現差を source semantics へ漏らす。ADR-060、ADR-062、ADR-075 に反するため
採用しない。

### 任意の user blanket impl で consumer を補う

候補の重複と termination を保証できず、ADR-062 の coherence 境界を破る。
compiler-owned rule を追加する場合も、入力 obligation と候補の一意性を別途
採択しなければならないため、現時点では採用しない。

## 再検討条件

- generic method、`Sum`、`Fold`、`FromIterator` の出力型と callback ABI が
  GC / linear-memory の双方で型安全に固定できる場合。
- 数値以外の consumer（文字列連結、比較、短絡 predicate など）の algebraic
  identity とエラー/overflow semantics が定義された場合。
- capability trait の候補が static coherence だけでは解決できず、dynamic
  dispatch が必要になった場合。その ABI は本 ADR の範囲外とする。

## 関連文書

- [RFC-011: 言語 v2 の意味論再設計案](../rfcs/011-language-v2-semantics-redesign.md)
- [ADR-062: Language v2 の Trait core と coherence](ADR-062-language-v2-trait-core-coherence.md)
- [ADR-075: 言語 v2 の linear-memory tracing runtime](ADR-075-language-v2-linear-memory-tracing-runtime.md)
- [#691: Iterator trait, lazy adapters, FromIterator / collect](../../issues/open/691-iterator-trait-lazy-adapters.md)
- [#860: Iterator Trait 体系を統合完了する](../../issues/open/860-iterator-unification-integration.md)
