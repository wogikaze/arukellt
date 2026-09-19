# ADR-067: Language v2 の数値型と明示変換

ステータス: **ACCEPTED** — 具象数値型の暗黙 promotion を廃止し、literal 推論と明示変換の境界を固定する

決定日: 2026-09-19

関連: [ADR-014](ADR-014-stability-labels.md)、[ADR-029](ADR-029-selfhost-validation.md)、[ADR-035](ADR-035-wasm-gc-implementation.md)、[ADR-063](ADR-063-language-v2-conversion-error-boundary.md)、[ADR-065](ADR-065-language-v2-operator-trait-resolution.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#861](../../issues/open/861-numeric-conversion-rules.md)

## 文脈

旧来の型検査は `i32` と `i64` の二項演算や代入で暗黙に広い型へ寄せていた。
仕様はさらに `i32` / `i64` と `f64` の promotion table を記載していたが、実装、
浮動小数点の丸め、checked conversion の失敗表現を一つの規則として定義していなかった。
この状態では、literal の既定型と、すでに具象型になった値の変換が混同され、
`wasm32-gc` と `wasm32` の lowering が異なる型結果を返す余地がある。

## 決定

### 1. 具象数値型の暗黙変換を行わない

異なる具象数値型は、型検査上 `unify` しない。算術、比較、bitwise 演算のオペランドは、
literal 推論を終えた後に同じ具象型でなければならない。関数引数、戻り値、束縛の注釈、
record field、generic parameter への代入にも同じ規則を適用する。

したがって、次のような promotion は存在しない。

```ark
let total = 1i64 + 1       // reject: i64 と i32
let wide: i64 = 1          // reject: i32 literal は i64 へ暗黙変換しない
let mixed = 1i32 + 1.0f64  // reject: i32 と f64
```

変換が必要な場合は、変換元の型と変換先の型を source code に記述する。

### 2. literal inference と concrete conversion を分離する

- suffix のない整数 literal は `i32`、suffix のない浮動小数点 literal は `f64` とする。
- `1i64`、`1.0f32` のような suffix は literal の具象型を直接選択する。
- 型注釈は literal を別の型へ暗黙変換しない。注釈と literal の型が異なる場合は、
  suffix または明示変換を使う。
- いったん変数、引数、record field などへ束縛された値は concrete value であり、
  literal inference の再適用によって型を変更してはならない。

この分離は型検査の共通段階で行い、GC heap、linear memory、Wasm opcode の選択より前に
確定する。

### 3. lossless な `From` / `Into` だけを提供する

`From<T>` と `Into<U>` は、対象の全ての値を意味と値の損失なく表現できる変換だけに使う。
静的 dispatch のため現行実装では対応する impl を明示する。

| source | target | canonical surface | semantics |
|---|---|---|---|
| `i32` | `i64` | `value.into()` | lossless integer widening |
| `f32` | `f64` | `f32_to_f64(value)` | lossless float widening |

`i64 -> i32`、`f64 -> f32`、narrow integer alias の narrowing は `From` / `Into` の impl に
してはならない。既存の prelude free function は ADR-014 の compatibility surface として
残せるが、新規コードの canonical API ではない。

### 4. 明示変換の意味を名前で区別する

整数の narrowing には、同じ値の範囲を暗黙に仮定せず、次の意味を使い分ける。

- `checked`: 表現可能範囲外を `None` として報告する。
- `wrapping`: target の整数幅で二の補数 modulo を適用する。
- `saturating`: target の最小値または最大値へ clamp する。
- `lossless`: 変換前の全値を表現できる場合だけ `From` / `Into` とする。

現行の nominal scalar type model で提供する i64→i32 の canonical methods は、
`to_i32_checked() -> Option<i32>`、`to_i32_wrapping() -> i32`、
`to_i32_saturating() -> i32` とする。`TryFrom` / `TryInto` への一般化、異種 error ABI、
他の target width の fallible conversion は [ADR-063](ADR-063-language-v2-conversion-error-boundary.md)
と #708 の所有範囲に残す。

浮動小数点 narrowing は整数の wrapping / saturating とは異なる。`f64_to_f32` は明示的な
IEEE/Wasm の丸め変換であり、lossless または integer range check と表現してはならない。
整数と浮動小数点の相互変換には promotion を適用せず、対応する明示 cast API が定義された
場合だけ使用する。

### 5. 現行 alias と compatibility API の境界

`u8`、`u16`、`i8`、`i16`、`u32`、`u64` は現在の compiler では i32/i64 representation
alias として扱われ、独立した `TypeInfo` の幅ではない。この事実を理由に、異なる nominal
型を source level で暗黙に混ぜてよいことにはしない。既存の `*_to_*` prelude wrapper と
intrinsic は互換境界として明示呼び出しに限って保持し、各 wrapper の overflow semantics
を docs と fixture で表現する。

### 6. target parity

型結果と conversion mode の選択は `wasm32-gc` と `wasm32` で共通の source/typechecker
契約とする。GC backend は GC value representation、fallback は linear-memory runtime
または scalar Wasm opcode を選べるが、checked の `None`、wrapping の modulo、saturating の
clamp、float rounding の観測結果を target ごとに変えてはならない。

## 帰結

- 暗黙 promotion matrix と、promotion に依存する source code は削除または明示変換へ移行する。
- `From<i64> for i32` のように narrowing を lossless と偽る impl は削除する。
- literal の suffix は型注釈の代用品ではなく、literal 自体の型を選択する構文として文書化する。
- 既存の prelude compatibility wrapper は直ちに削除しない。削除は migration、manifest、
  target parity、ADR-014 の告知期間を満たす別の変更で行う。
- backend parity fixture は同じ `.ark` source を GC と linear の両方で compile、validate、
  runtime 検証する。

## 却下した代替案

### promotion matrix の拡張

全ての整数幅と浮動小数点幅の組み合わせを優先順位付き table で昇格させる案は、literal
と concrete value の境界を隠し、結果型と丸め規則を追いにくくするため採用しない。

### narrowing を `From` / `Into` に残す

呼び出し側から失敗可能性や modulo を読み取れず、`From` の lossless 契約と矛盾するため
採用しない。narrowing は checked / wrapping / saturating の名前付き API に置く。

### runtime `Any` による数値変換

数値を `Any` へ消去して runtime で変換すると、型エラーを遅延させ、GC と fallback の
representation 差を公開 ABIへ持ち込むため採用しない。

## 再検討条件

- narrow integer alias が独立した nominal `TypeInfo` と range semantics を持つようになった場合。
- #708 が `TryFrom` / `TryInto` の共通 error/ABI 契約を採択した場合。
- target parity fixture が Wasm GC と linear fallback の間に意味論差を検出した場合。
