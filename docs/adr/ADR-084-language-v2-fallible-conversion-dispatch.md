# ADR-084: Language v2 の fallible conversion trait と dispatch 形式

ステータス: **ACCEPTED** — fallible conversion は `TryFrom` の associated function を正規形とし、`TryInto` method は target 型推論の採択まで保留する

決定日: 2026-10-23

関連: [ADR-014](ADR-014-stability-labels.md)、[ADR-044](ADR-044-language-v2-trait-first-api.md)、[ADR-046](ADR-046-language-v2-trait-first-api-followup.md)、[ADR-062](ADR-062-language-v2-trait-core-coherence.md)、[ADR-063](ADR-063-language-v2-conversion-error-boundary.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#708](../../issues/open/708-tryfrom-tryinto-traits.md)、[#694](../../issues/open/694-error-trait-unified-ecosystem.md)、[#701](../../issues/open/701-associated-function-syntax.md)

## 文脈

`#708` は fallible conversion の `TryFrom<T>` / `TryInto<U>` の trait 定義、標準 impl、
ABI を決める issue である。ADR-063 は `?` と `From` の境界、および `TryFrom`/`TryInto` を
`From`/`Into` の subtrait にしないことを固定したが、両 trait の呼び出し形式と
impl 選択の規則は #708 の scope に残した。

現行 compiler の generic trait method 呼び出しは receiver 型から impl を選び、
戻り型にしか現れない型引数を期待型から補完しない。`String` に対して
`TryInto<i32>` / `TryInto<i64>` / `TryInto<f64>` を並べた probe では、
`let x: Result<i64, String> = "x".try_into()` が i32 impl を選び、
`i32 vs i64` の typecheck error になった。target 型を path に明示する
`i32::try_from("x")` は typecheck を通り、impl 選択が target 型に固定される。

## 決定

### 1. 正規形は `TryFrom` の associated function

Fallible conversion の公開呼び出し面は `TryFrom<T>` の associated function とする。

```text
trait TryFrom<T> {
    fn try_from(value: T) -> Result<TryFrom, String>
}

impl TryFrom<String> for i32 {
    fn try_from(value: String) -> Result<i32, String>
}
```

source では `U::try_from(value)` と呼ぶ。impl 選択は target 型 `U` の明示 path で行い、
期待型からの impl 推論を要求しない。戻り型は impl 側で具体的な `Result<U, String>` に固定する。

### 2. `TryInto` method は初期 v2 API に含めない

`TryInto<U>` の method 形式 (`x.try_into()`) は、target 型 `U` が戻り型にしか現れない
generic trait method の impl 選択を必要とする。現行 typechecker はその選択を持たず、
複数 impl のとき先頭 impl を選ぶ。期待型-directed な trait method resolution は
type inference 全体への影響が大きく、第二の利用例もまだないため、初期 v2 API には採らない。

`TryFrom` から `TryInto` を暗黙生成しない。両者は `From`/`Into` の subtrait でもない
（ADR-063 §1）。

### 3. Error 型は `String` 固定を初期契約とする

`TryFrom` の error 型は `String` 固定とする。associated `Error` 型と error source chaining は
`#694` と将来の ADR に残す。`?` は ADR-063 の通り `From` だけを使い、
`TryFrom` や `TryInto` へ暗黙フォールバックしない。

### 4. 標準 impl の開始範囲

`#708` の標準 impl は次から開始する。

- `TryFrom<String> for i32` / `i64` / `f64`（parse）
- `TryFrom<i64> for i32`（範囲検査）

impl 集合は issue で管理し、この ADR に進捗や件数を書かない。

### 5. Stability

ADR-014 に従い **experimental** とする。associated function 名、error 型、impl 集合が
固まるまで stable にしない。

## 帰結

- `#708` の fixture は `i32::try_from("42")` の associated 形式で書く。
  `try_into` method の fixture は採択しない。
- compiler は associated trait function の具象戻り型 `Result<U, String>` の解決と、
  call-result temp への let 注釈伝播を実装する必要がある。後者は型伝播の実装ギャップであり、
  この ADR の決定とは独立に issue で追跡する。
- `?` と `From` の意味論、既存 impl の coherence は変更しない。
- `TryInto` method を将来追加する場合は、期待型-directed な trait method resolution の
  設計判断を先に行う。

## 却下した代替案

### `x.try_into()` を target 型推論で解決する

現行 typechecker は戻り型だけの型引数を期待型から補完せず、複数 impl で先頭 impl を選ぶ。
期待型-directed な trait method resolution は inference 全体への影響が大きく、第二の利用例も
ないため、初期 v2 API には採らない。将来の再検討対象とする。

### source 型の inherent method (`s.try_into_i32()`) を正規形にする

公開 API は trait / method / associated function を正規形とする ADR-044/046 の方向に反し、
`TryFrom` の coherence と dual 関係を表現できないため採らない。

### `TryFrom` から `TryInto` を blanket 生成する

method 形式の impl 選択（上記）を前提にするため、同じ blocker を持つ。

## 再検討条件

- target 型が戻り型にしか現れない generic trait method の impl 選択を、inference と coherence の
  両面で設計できた場合（second use case と合わせて）。
- `#694` で associated `Error` 型と error source chaining が確定し、`String` 固定を
  置き換える必要が生じた場合。
- associated function 形式の ergonomics が実利用で不足し、method 形式が必要になった場合。

## 関連文書

- [ADR-014](ADR-014-stability-labels.md) — stability labels
- [ADR-044](ADR-044-language-v2-trait-first-api.md) / [ADR-046](ADR-046-language-v2-trait-first-api-followup.md) — trait-first API
- [ADR-062](ADR-062-language-v2-trait-core-coherence.md) — trait coherence
- [ADR-063](ADR-063-language-v2-conversion-error-boundary.md) — conversion / error boundary
- [RFC-011](../rfcs/011-language-v2-semantics-redesign.md)
- [issue #708](../../issues/open/708-tryfrom-tryinto-traits.md)
