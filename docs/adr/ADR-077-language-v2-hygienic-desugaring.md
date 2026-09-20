# ADR-077: Language v2 の hygienic desugaring 境界

ステータス: **ACCEPTED** — compiler-owned な receiver / method 参照で糖衣構文を脱糖し、source binding の shadowing を意味論から分離する

決定日: 2026-09-20

トラック: language-v2
Issue: [#865](../../issues/open/865-hygienic-desugaring-fstring.md)
関連: [ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-058](ADR-058-language-v2-semantics-redesign.md)、[ADR-062](ADR-062-language-v2-trait-core-coherence.md)、[ADR-075](ADR-075-language-v2-linear-memory-tracing-runtime.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)

## 文脈

糖衣構文が展開先の stdlib 名を通常の名前解決で参照すると、同じ名前の
ユーザー関数や closure に捕捉される。旧 f-string lowering は
`to_string(expr)` と `concat(lhs, rhs)` を生成していたため、ユーザーが
`to_string` または `concat` を shadow したとき、表示結果ではなく shadow 側の
関数が呼ばれたり、GC artifact の型が壊れたりした。

`for` と comprehension も、source-level の `into_iter`、`next`、`collect`、
`push` の名前へ意味論を委ねてはならない。これらは parser が作る文字列ではなく、
type/trait 情報を持つ semantic lowering の責務である。

## 決定

### 1. canonical mechanism

この構文群では、型付き receiver method を compiler-owned な canonical
semantic reference とする。

- f-string の `{expr}` は `expr.to_string()` へ lower し、`Display` の
  receiver 型に基づく static dispatch を使う。
- f-string の `{expr:?}` は `expr.fmt_debug()` へ lower し、`Debug` の
  receiver 型に基づく static dispatch を使う。
- f-string の literal / converted segment の結合は、型が `String` である
  receiver の `String::concat` method を左から右へ適用する。
- `for` は iterable の concrete owner から `Owner::into_iter` を選び、
  concrete iterator の `Iterator::next` を選ぶ。source の `into_iter`、
  `next`、`collect` という binding を lookup しない。
- comprehension の result builder は `Vec::push` として生成し、source の
  `push` binding を lookup しない。iterator の source conversion は、通常の
  `for` lowering と同じ compiler-owned path を使う。

この機構は source に hidden import を追加することではない。receiver の型、
trait identity、associated type を使って semantic call を確定し、backend は
その call を Wasm GC または linear-memory の canonical ABI へ lower する。
`__intrinsic_*` は source-level canonical name ではなく、この boundary の後段に
限定する。

### 2. 評価順序と hygiene

embedded expression の AST は通常の parser / resolver / typechecker を通る。
f-string の segment は source order で一度ずつ評価し、generated method call が
追加の評価を発生させない。receiver method の解決は、同名の local binding、
closure、module function、stdlib compatibility alias に依存しない。

comprehension と `for` の loop variable、filter、element expression の順序も
既存の core form と同じであり、generated builder/iterator method の名前を
source scope から再解決しない。

### 3. backend parity

Wasm GC は typed reference / GC representation を、linear-memory fallback は
linear representation と tracing runtime を使う。ただし parser、resolved
semantic call、CoreHIR/MIR の observable order、trait result、ownership boundary
は共有する。backend の違いを理由に、糖衣構文の source legality や shadowing
結果を変えない。

## 却下した代替案

### bare free-function expansion

`concat(…, to_string(expr), …)` や `push(result, elem)` を生成する方式は、
ordinary name resolution と closure capture に依存するため採用しない。

### stdlib module path の文字列展開

`std::text::format_debug` のような path を parser が直接生成すると、公開 API
の rename、import policy、compatibility alias が language syntax の意味へ漏れる。
trait receiver identity を失うため採用しない。

### source-visible intrinsic の直接生成

`__intrinsic_concat` などを parser の canonical target にすると、private ABI と
source semantics が結合し、GC / linear backend の representation boundary を
越えてしまう。intrinsic は semantic call の後段でのみ選択する。

## 帰結

- f-string の generated call はユーザーの `concat`、`to_string`、`fmt_debug`
  binding に捕捉されない。
- compiler-owned iteration/comprehension lowering は `into_iter`、`next`、
  `collect`、`push` の source shadowing に捕捉されない。
- `String`、`Display`、`Debug`、`Iterator`、`IntoIterator` の既存 trait/API
  boundary を再利用するため、新しい公開 free function は追加しない。
- receiver method の型解決や trait coherence の変更は ADR-044/062 の再検討条件に
  従う。新しい糖衣構文は、同じ canonical mechanism と parity fixture を別途示す。

## 検証契約

次の fixture と gate を acceptance evidence とする。

- `tests/fixtures/string_interp/fstring_hygiene.ark` は evaluation order、
  `concat` / `to_string` / `fmt_debug` の shadowing、for/comprehension の
  `into_iter` / `next` / `collect` / `push` shadowing を含む。
- `tests/fixtures/diagnostics/fstring_embedded_unresolved.ark` は embedded
  expression の通常の unresolved-name 診断を固定する。
- 同一 fixture を `wasm32-gc` と `wasm32` で compile/validate/runtime 実行し、
  component artifact でも GC path を validate/run する。

## 再検討条件

- generic trait dispatch または dynamic dispatch が必要になり、receiver 型だけ
  では generated call の一意性を保てなくなった場合。
- f-string の format specifier、comprehension の複数 generator、または新しい
  syntax sugar が別の evaluation/ownership contract を要求する場合。
- GC と linear-memory の shared semantic boundary を変更する場合。
