# ADR-069: Language v2 の String / Unicode semantics

ステータス: **ACCEPTED** — `String` の encoding、byte API、Unicode scalar API、および
不正な UTF-8 の境界を Wasm GC／linear-memory の共通言語契約として固定する

決定日: 2026-09-19

関連: [ADR-014](ADR-014-stability-labels.md)、[ADR-029](ADR-029-selfhost-validation.md)、
[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、
[ADR-058](ADR-058-language-v2-semantics-redesign.md)、[ADR-060](ADR-060-language-v2-owned-adt-box-slice.md)、
[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#863](../../issues/done/863-string-unicode-semantics.md)

## 文脈

RFC-011 は `String` の Unicode、UTF-8、文字単位操作、byte 単位操作の境界を分けることを
要求している。現行実装は `char_at` と empty-separator の `chars` が UTF-8 の raw byte を
一単位として扱い、`char_to_string` と `push_char` も char の整数値を一 byte として保存する。
そのため、ASCII 以外では GC target と linear-memory target のどちらを選ぶかだけでなく、
同じ target の API 間でも意味が変わる。

`String` は language-level の text value であり、byte buffer の別名ではない。一方、parser、
binary I/O、Wasm ABI は UTF-8 byte を直接扱う必要があるため、両方の操作を同じ名前空間で
暗黙に混ぜず、境界を型と API 名で示す必要がある。

## 決定

### 1. String representation と char value

- `String` の意味上の値は、常に well-formed UTF-8 byte sequence とする。
- UTF-8 は Unicode scalar value の canonical encoding とする。overlong encoding、surrogate
  code point、`U+10FFFF` を超える code point は `String` の値にならない。
- `char` は Unicode scalar value とし、範囲は `0..=0x10FFFF` から
  `0xD800..=0xDFFF` を除いたものとする。内部表現が i32 であることはこの値域契約を変更しない。
- Unicode の combining mark は独立した scalar である。`chars` は grapheme cluster や
  normalization を実装せず、scalar 単位で分割する。

### 2. byte API と scalar API

次の API は byte index／byte length を扱い、文字 index と混同してはならない。

- `byte_len(s)` と `String::len_bytes()`
- `byte_at(s, i)` と raw compatibility API
- `slice_bytes(s, start, end)`
- `to_utf8_bytes(s)` と `try_from_utf8(bytes)`

`char_at(s, i) -> i32` は既存コードを移行するための deprecated compatibility entry とし、
`i` を byte index、結果を raw byte と明記する。新しい code は `byte_at` を使う。

次の API は Unicode scalar index／値を扱う。

- `len_chars(s)` は scalar count を返す。
- `codepoint_at(s, i) -> Option<char>` は scalar index を受け、範囲外なら `None` を返す。
- `chars(s)` は一 scalar を含む valid UTF-8 `String` の vector を返す。
- `slice_chars(s, start, end)` は scalar index の `[start, end)` を返す。

scalar API の失敗は integer sentinel で表さない。`codepoint_at` の範囲外、invalid source、
`slice_chars` の範囲外は `Option` または空の String を使い、各 API の契約を個別に固定する。

### 3. boundary と slicing

- `slice_bytes` は `0 <= start <= end <= byte_len(s)` かつ `start` と `end` が UTF-8 scalar
  boundary の場合だけ成功する。それ以外は empty String を返す。
- `try_slice_bytes` は同じ検査を行い、成功時 `Some(slice)`、失敗時 `None` を返す。
- `slice_chars` と `try_slice_chars` は scalar index を byte offset へ変換してから同じ
  byte slice を作る。負値、逆順、終端超過はそれぞれ empty／`None` とする。
- `String` の valid invariant を回避する sealed raw API は compiler と stdlib 内部だけが使う。
  それが invalid byte sequence を生成した場合の public text API の結果は未定義にせず、
  `codepoint_at` は `None`、`len_chars` は invalid byte を一 scalar 相当として進める。

### 4. UTF-8 construction と invalid input

- `try_from_utf8(bytes)` は byte 値の範囲、lead byte、continuation、最短形式、surrogate、
  上限を検証し、全体が valid のときだけ `Some(String)` を返す。
- 既存の `from_utf8(bytes)` は migration compatibility の lossy API とし、不正な byte 列を
  その位置から一つの U+FFFD に置換して valid String を返す。byte 値が `0..=255` の外に
  ある入力も不正として同じ扱いにする。
- `char_to_string` と `push_char` は scalar を canonical UTF-8 へ encode する。内部互換境界
  から不正な i32 が到達した場合は U+FFFD に置換し、GC／linear の両 target で同じ結果にする。

### 5. target parity

これらは source/typechecker/MIR の共通契約であり、representation の選択前に適用する。
`wasm32-gc` は主経路、`wasm32` は supported linear-memory fallback とし、両方で同じ
`String` byte sequence、scalar count、boundary result、diagnostic、exit status を返す。
GC array の element layout、linear-memory の header、allocation strategy は backend-private
であり、source-level byte／scalar index を変更する理由にはならない。

## 帰結

- raw byte parser は `byte_at` を使うため、Unicode scalar API の変更が lexer、JSON、WIT、
  binary I/O の byte scan に波及しない。
- `chars` は combining mark を含む scalar ごとに分割するが、ユーザーが見た grapheme cluster
  単位の cursor や normalization は別の API／設計課題である。
- deprecated `char_at` は compatibility window 中だけ残す。既存 call site を `byte_at` へ
  移行し終え、manifest の removal 条件を満たした段階で別 issue で削除する。
- `from_utf8` の lossy behaviour は完全な validation の代替ではない。入力検証が必要な caller
  は `try_from_utf8` を使う。

## 却下した代替案

### char_at の意味だけを raw byte から scalar に置き換える

既存の parser／lexer と byte-oriented fixture を壊し、同じ名前で index 単位が変わるため採用
しない。raw compatibility entry と明示的な scalar API を分離する。

### String を任意 byte sequence とする

各 consumer が invalid UTF-8、continuation boundary、surrogate を個別に判断することになり、
GC／linear parity を検証できないため採用しない。binary data は `Vec<i32>` の byte API とする。

### 不正入力を即時 trap だけで処理する

network、file、parser boundary の回復可能な入力に適用できず、`try_from_utf8` と lossy migration
API の責務を表現できないため採用しない。

## 再検討条件

- Unicode normalization または grapheme cluster API を source-level contract として導入する場合。
- `String` と binary buffer を別の first-class type／ownership ABI に昇格する場合。
- `char` の内部表現を i32 以外へ変更する場合。
