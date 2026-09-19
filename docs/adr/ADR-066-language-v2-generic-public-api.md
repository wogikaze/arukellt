# ADR-066: Language v2 の generic 公開 API と HashMap

ステータス: **ACCEPTED** — `HashMap<K,V>` の source API、静的 monomorphization、内部 `Any` 境界を固定する

決定日: 2026-09-19

関連: [ADR-003](ADR-003-generics-strategy.md)、[ADR-014](ADR-014-stability-labels.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[ADR-058](ADR-058-language-v2-semantics-redesign.md)、[ADR-065](ADR-065-language-v2-operator-trait-resolution.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#859](../../issues/open/859-generic-public-api-any-hashmap.md)

## 文脈

旧来の HashMap API は、`HashMap_i32_i32_new` や
`HashMap_String_i32_get` のように key/value の具体型を関数名へ埋め込んでいた。
この形は実装上の monomorphization と source API を混同し、型専用の free function を
増やし続ける。また、`Any` を公開型として導入すると、型安全な static dispatch と
Wasm GC / linear-memory の表現境界が不明確になる。

ADR-044 と ADR-046 は公開面を trait、method、associated function に寄せることを
採択している。ADR-003 は generic を静的に monomorphize する方針を採択しているため、
HashMap もこの二つの契約に従う必要がある。

## 決定

### 1. Canonical source API

ユーザーが記述する HashMap の正規形を次で固定する。

```ark
let map = HashMap::new<i32, i32>()
let previous = map.insert(1, 10)
let current = map.get(1)
let present = map.contains_key(1)
let removed = map.remove(1)
let count = map.len()
let empty = map.is_empty()
```

`new` は associated function、その他は receiver method とする。`insert`、`get`、
`remove` は `Option<V>` を返し、`contains_key`、`is_empty` は `bool`、`len` は `i32`
を返す。receiver を第一引数に置く公開 free function を canonical API として追加しない。

### 2. Static generic resolution

`HashMap<K,V>` は runtime type erasure を行う dynamic map ではなく、呼び出し地点の
具体型を使う静的 generic API である。

- `HashMap::new<K,V>()` の明示的な型引数と、receiver method の `HashMap<K,V>` を
  typechecker が同じ型ペアとして検査する。
- concrete pair は codegen 前に monomorphize し、GC と linear-memory の各 backend
  adapter へ同じ source signature から lower する。
- 現在の正規 adapter が提供する concrete pair は次の通りである。

  | key | value |
  |-----|-------|
  | `i32` | `i32` |
  | `String` | `i32` |
  | `i32` | `String` |
  | `String` | `String` |
  | `i32` | `fn(i32) -> i32` |
  | `String` | `fn(i32) -> i32` |

- 型変数または未確定型は、周囲の式から解決できるまで generic inference の状態に
  留める。
- concrete pair が adapter 集合にない場合は、`unsupported HashMap type pair` と
  supported pair 集合を示す typecheck error にする。未解決の型を `Any` に落として
  コンパイルを通してはならない。

この pair 集合は現行の公開実装契約であり、将来の拡張を禁止する上限ではない。
新しい pair は adapter、型検査、GC / linear parity fixture、移行文書を同時に追加する。

### 3. `Any` の境界

`Any` はユーザーが宣言・注釈・構築できる公開 source type ではない。`to_string` の
manifest signature にある `any` は、compiler builtin が call site の concrete type を
受け取るための内部 placeholder であり、公開型名ではない。

GC backend の `anyref`、generic type variable、trait dispatch 用の内部 erased value は
codegen 表現としてのみ許可する。HashMap の API、trait bound、公開 ABI の型引数を
`Any` へ置き換える設計は採用しない。

### 4. Target parity

`wasm32-gc` を primary lowering、`wasm32` を supported linear-memory fallback とする
target 契約は変更しない。HashMap の source signature、Option の結果、key equality、
insert / lookup / remove / length の観測結果は両 backend で同じでなければならない。
GC object、parallel array、linear-memory storage は backend の表現差であり、source
level API の合法性や意味論を変えない。

### 5. Compatibility window and removal condition

既存の concrete builtin、compatibility wrapper、raw `std::collections::hash` API は、
この ADR だけでは削除しない。旧 concrete entry は ADR-014 の stability window と既存
issue (#718 / #687) の所有範囲に従う移行面として残し、ドキュメントの推奨例には新しい
method APIだけを掲載する。

HashMap の concrete compatibility entry を削除できる条件を次のように固定する。

1. in-tree の source、fixture、documented example が canonical generic API へ移行済みである。
2. manifest / lint が旧 entry の使用を canonical method へ案内できる。method-level の
   replacement を manifest が表現できない間は、旧 entry を `deprecated` と偽装しない。
3. `wasm32-gc` と `wasm32` の compile、validate、runtime parity fixture が旧 entry に
   依存しない状態で通る。
4. migration guidance と release note に移行手順を記録し、ADR-014 の告知期間を満たす。
5. 次の major release で削除することを release checklist に記録する。

この条件を満たす前に、旧 API を新しい alias の追加で延命したり、`Any` への暗黙変換で
未対応 pair を受理したりしてはならない。

## 帰結

- source API は型引数と receiver method を中心に読めるようになり、backend の mangled
  function name は言語契約から隠れる。
- static monomorphization により、pair ごとの code size と adapter 数は増えうるが、
  GC / linear の ABI を動的 erased value に統一する必要はない。
- raw `std::collections::hash` の既存 stable API、Iterator、他の collection family の
  free function 移行はこの ADR の範囲外であり、各 issue が所有する。
- `std::test` の panic / String ABI のような別の target defect を HashMap API の
  compatibility workaround として取り込まない。

## 却下した代替案

### 公開 `Any` map

全ての key/value を `Any` / `anyref` へ box すれば pair ごとの adapter は減るが、
型検査、ownership、hash/equality、linear fallback の representation が runtime contract
へ漏れるため採用しない。

### concrete free function を canonical として残す

これは monomorphic helper の数を増やし、ADR-046 の公開 API 方針と generic inference
を分断するため採用しない。互換 entry は移行期間だけ保持する。

### GC と linear で別の HashMap 意味論を許す

target ごとに `Option`、collision、remove、size の結果が変わると source program の
移植性が失われるため採用しない。

## 再検討条件

- `HashMap` の dynamic dispatch または custom hasher を採択する後続 ADR が、static
  monomorphization と異なる公開 ABI を必要とする場合。
- `Any` を source-level trait object として導入する場合。そのときは型安全性、ownership、
  GC / fallback ABI を別 ADR で決定する。
- supported pair を拡張し、既存の source semantics または target parity を変更する場合。
