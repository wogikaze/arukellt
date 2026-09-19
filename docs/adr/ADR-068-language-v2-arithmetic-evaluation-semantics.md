# ADR-068: Language v2 の算術 edge と評価順序

ステータス: **ACCEPTED** — 整数・浮動小数点の edge semantics と、式の観測可能な評価順序を
Wasm GC／linear-memory の共通言語契約として固定する

決定日: 2026-09-19

関連: [ADR-013](ADR-013-primary-target.md)、[ADR-029](ADR-029-selfhost-validation.md)、
[ADR-035](ADR-035-wasm-gc-implementation.md)、[ADR-058](ADR-058-language-v2-semantics-redesign.md)、
[ADR-065](ADR-065-language-v2-operator-trait-resolution.md)、[ADR-067](ADR-067-language-v2-numeric-conversion.md)、
[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#862](../../issues/open/862-arithmetic-edge-and-evaluation-order.md)

## 文脈

Language v2 は演算子を canonical Trait protocol として解決し、scalar については同じ
protocol の backend fast path として Wasm opcode を使う（ADR-065）。しかし、overflow、
signed division の trap、shift count、NaN／∞の比較、明示的な float conversion、および
副作用を持つ式の評価順は一つの採択済み契約になっていなかった。

lowerer がこの順序を暗黙に変えると、GC object と linear-memory object、または AST lower と
CoreHIR lower の間で結果が変わる。特に assignment は place の base を RHS の後に lower
できてしまい、place 側の副作用を観測できるプログラムの意味を壊す。

## 決定

### 1. 整数の edge semantics

`i32` と `i64` の組み込み算術は、同じ幅の Wasm integer operation と同じ bit pattern を
返す。

- `+`、`-`、`*` はそれぞれ 2^32 または 2^64 を法とする wraparound とする。
- signed `/` と `%` は divisor が 0 のとき runtime trap とする。
- signed `/` は `MIN / -1` でも runtime trap とする。
- signed `%` の `MIN % -1` は 0 とする。
- `<<` と `>>` の shift count は、`i32` では下位 5 bit、`i64` では下位 6 bitだけを使う。
  count の範囲外や負値を独自に trap にはしない。
- `>>` は signed operand に対する arithmetic right shift とする。bit pattern を使う
  fallback backend は sign bit を明示的に補い、host language の負値 shift の挙動に依存しない。

checked／saturating／wrapping の名前付き API は通常の演算子の trap／wrap 契約を変更しない。
それらは ADR-067 および標準ライブラリの個別 Trait 契約に従う。

### 2. 浮動小数点の edge semantics

`f32` と `f64` はそれぞれ IEEE 754 binary32／binary64 として Wasm arithmetic operation
へ lower する。

- `+`、`-`、`*`、`/` は IEEE/Wasm の有限値、符号付き zero、`+∞`、`-∞`、NaN の結果を使う。
  float の zero division は integer division のような trap ではない。
- NaN は自身を含む全ての値と `==` で false、`!=` で true になる。
  NaN を含む `<`、`<=`、`>`、`>=` は false になる。
- NaN の payload または canonical/non-canonical bit pattern は言語の数値比較契約に含めない。
  `f64_bits_*` のような raw helper は特定の NaN payload を保証しないが、有限値と infinity
  の IEEE bit pattern は保持する。
- `f64_to_f32` は Wasm の binary64→binary32 rounding、`f32_to_f64` は Wasm の exact
  binary32→binary64 widening とする。NaN は NaN のままであり、payload の一致は要求しない。

### 3. 評価順序

副作用、trap、allocation、または observable host operation があるかにかかわらず、次の
順序を source semantics とする。compiler は backend の命令順や最適化でこの順序を変えては
ならない。

- function／method call は callee の receiver を先に評価し、引数を左から右へ評価する。
- 二項演算は左 operand、右 operand の順に評価する。`&&` と `||` は左から評価し、必要な
  場合だけ右を評価する。
- tuple、array、struct、enum payload など aggregate の要素・field は source order で評価する。
- index access は base、index の順に評価する。overloaded `Index` でもこの順序を維持する。
- assignment は place の評価（field の base、index の base と indexを含む）、RHS の評価、
  最後に store の順とする。place の評価が trap になった場合、RHS と store は実行しない。
  単純な local identifier の place 評価には別の observable operation はない。

lowerer は必要に応じて各段階を local temporary に保存し、AST lowerer と CoreHIR lowerer
の双方でこの順序を明示する。

### 4. Target parity

これらは source/typechecker/MIR の共通契約であり、`wasm32-gc` の GC representation と
`wasm32` の linear-memory representation の選択前に確定する。GC の component/core emit と
linear fallback は同じ fixture の compile、validate、runtime で同じ exit／observable result
を返さなければならない。native backend を公開する場合も、C の未定義または未規定な評価順と
signed arithmetic に依存せず、この契約を再現する（RFC-008 §5）。

## 帰結

- scalar fast path を別の意味論として扱わず、Wasm opcode の edge behaviour を言語契約の
  実装として扱える。
- dynamic zero division は `Result` や `Option` へ暗黙変換されない。回復可能な除算は
  caller が事前に条件を検査するか、名前付き safe API を使う。
- 最適化や引数 staging は許可されるが、source order と store order を証明できる temporary
  sequence を保持しなければならない。
- NaN payload を比較する fixture は backend parity の証拠にならない。comparison と明示
  conversion の観測結果を fixture 化する。

## 却下した代替案

### host language の評価順に委ねる

C や emitter 内部の helper 呼び出し順は言語契約にならず、backend 間の observable difference
を許すため採用しない。

### integer overflow を trap または checked value にする

既存 scalar operator と Wasm integer fast path の互換性を壊し、checked／saturating API と
通常演算子の責任境界を曖昧にするため採用しない。

### NaN payload を canonical value として固定する

Wasm が保証しない payload を source-level ABI に持ち込み、GC／linear fallback の余計な
同期制約を作るため採用しない。

## 再検討条件

- unsigned scalar operator を signed scalar protocol と別の canonical Trait 実装として導入する場合。
- IEEE 754 payload／total-order 比較を source-level API として採択する場合。
- `IndexMut` や `DerefMut` の place ABI を ADR-065 の表から更新する場合。
