# ADR-073: Language v2 multi-clause / where / semicolon grammar

ステータス: **ACCEPTED** — clause group の境界、`where` の scope、改行とセミコロンの意味を固定する
日付: 2026-09-19
トラック: language-v2
Issue: [#869](../../issues/open/869-multi-clause-where-semicolon-grammar.md)
関連: [ADR-058](ADR-058-language-v2-semantics-redesign.md)、[ADR-068](ADR-068-language-v2-arithmetic-evaluation-semantics.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)

## 文脈

selfhost parser は `fn name(pattern) = expr` を複数並べた clause と、末尾の
`where` value binding をすでに脱糖している。
しかし、通常の `skip_newlines` が改行とセミコロンを同じ separator として扱うため、
セミコロンで明示的に終端した declaration が同じ clause group へ取り込まれうる。
`where` binding の終端と clause group の終端も同じ実装上の skip に依存しており、
改行・演算子による継続・セミコロンの組合せが仕様から予測できなかった。

また、clause pattern で導入される binding と、全 clause から共有する `where` binding は
導入時期が異なる。
この二つを同じ scope とみなすと、dispatch 前に評価される `where` 式が、まだ存在しない
特定 clause の pattern binding を参照できるように見えてしまう。

## 決定

### 1. Clause group は明示的な改行境界で形成する

- `fn_clause_group` は clause-form の `fn` declaration から始まる。
- 次の clause は、直前の clause body の外側で **1 個以上の `NEWLINE`** を挟み、同じ名前の
  clause-form `fn` が続くときだけ同じ group に入る。
- `fn` と name の間、type annotation、guard、`=` の前後では、文法が許す範囲で
  `NEWLINE` を continuation whitespace として扱う。
- `;` は item terminator であり、clause group を終了させる。
  `;` の後に同じ名前の clause があっても、別 declaration として解析し、resolver の
  duplicate-definition 診断に委ねる。
- group 内の全 clause は同じ名前と arity を持つ。既存の multi-parameter 未実装制限は
  この ADR では解除しない。

これにより grouping は `skip_newlines` の副作用ではなく、clause separator という
文法上の境界に基づく。

### 2. Clause body の終端と continuation

`=` の右側は一つの expression である。
括弧、配列、record、block などの delimiter 内を除く外側で、expression が完了した後の
`NEWLINE` または `;` は expression の終端になる。

次のいずれかが続く場合、改行は expression の continuation として扱う。

- binary operator、assignment operator、range operator の直後
- member access、call、index など expression suffix の途中
- 開いた delimiter 内の expression

`=` の直後の改行は body の開始を許すが、`=` の直後の `;` は空 expression として
diagnostic にする。

### 3. `where` の位置、binding separator、scope

- `where` は clause group の最後にだけ置ける。
- 最後の clause と `where` の間の改行は許すが、セミコロンを挟んだ `where` は group に
  attach しない。
- `where` の各 binding は `IDENT = expr` で、出現順に評価する。
- binding 同士は 1 個以上の改行、または一つの `;` で分けられる。末尾の `;` も許す。
  `;` はここでは binding の終端であり、すでに開始した `where` block 自体は終了させない。
- `where` binding は synthetic function parameter と、それより前に宣言された `where`
  binding だけを参照できる。後続 binding と、特定 clause の pattern binding は参照できない。
- 各 clause の guard と body は、その clause の pattern binding、および全 `where` binding を
  参照できる。`where` binding は関数の外側からは見えない。

parser は `where` binding を synthetic function body の dispatch より前へ配置する。
この配置が上記の評価順序と scope を実装する正本であり、Wasm backend の表現に依存しない。

### 4. Target parity

上記の token boundary、scope、脱糖結果は source semantics である。
`wasm32-gc` と `wasm32` は同じ parser、resolver、typechecker 契約を使い、GC object と
linear-memory representation の違いによって clause の選択、`where` の評価順序、diagnostic
を変えてはならない。

## 代替案と却下理由

### 改行とセミコロンを同じ clause separator にする

既存の `skip_newlines` だけで実装できるが、明示的に終端した declaration と同名 clause の
区別を失う。formatting、error recovery、duplicate-definition 診断の境界も不安定になるため
却下する。

### Module 全体の同名 function を後から一つへ集約する

非連続 declaration、別 item を挟む declaration、意図した duplicate definition を区別できず、
source order と error location を壊す。現在の source-local desugaring より責務が広くなるため
却下する。

### `where` binding から全 clause の pattern binding を見せる

pattern dispatch より前に評価する binding が、まだ選択されていない clause の局所変数を参照する
ことになり、評価順序と scope が矛盾するため却下する。

## 帰結

parser の lookahead と clause parser は改行だけを clause/group separator として扱い、
semicolon を黙って skip しない。
`where` 内では semicolon を binding separator として明示的に消費する。
既存の block-body function と通常の block statement の optional semicolon 規則は維持する。
新しい source fixture は newline continuation、where 内 semicolon、semicolon による group 終端、
formatter の idempotency を分けて検証する。

## 再検討条件

- multi-parameter clause group の本実装を採択するとき。tuple pattern と synthetic parameter の
  scope をこの ADR に追加する必要がある。
- local recursive `where` function または mutual recursion を導入するとき。
- formatter が syntax-aware indentation を採択し、`where` binding の canonical indentation を
  current formatter contract として変更するとき。

## 関連

- [RFC-011: 言語 v2 の意味論再設計案](../rfcs/011-language-v2-semantics-redesign.md)
- [#869](../../issues/open/869-multi-clause-where-semicolon-grammar.md)
- [#852](../../issues/open/852-language-v2-semantics-full-implementation.md)
