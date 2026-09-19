# ADR-065: Language v2 の演算子 Trait 解決と magic method 移行

ステータス: **ACCEPTED** — 演算子構文を canonical Trait obligation として解決し、旧 magic method を選択経路から外す

決定日: 2026-09-19

関連: [ADR-038](ADR-038-operator-overload-traits.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[ADR-062](ADR-062-language-v2-trait-core-coherence.md)、[RFC-004](../rfcs/004-trait-expressiveness.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#858](../../issues/open/858-operator-traits-magic-method-retirement.md)

## 文脈

Arukellt の parser は演算子を構文木へ保持し、MIR lowerer は組み込み opcode または
receiver の `型名::method` を選択できる。
しかし、演算子の型検査は現在 operand の同型化に留まり、Trait registry の impl、
coherence、generic bound と同じ候補集合を使っていない。
また、旧仕様には `__add` などの magic method が残っているが、trait-first の公開 API と
同じ意味を別の名前空間で提供すると、二重実装と宣言順依存の原因になる。

## 決定

### 1. Canonical operator protocol

言語構文から参照する Trait と method は次の表で固定する。

| 構文 | Trait | method | 追加の意味 |
|------|-------|--------|------------|
| `a + b` | `Add` | `add` | `Self` を返す |
| `a - b` | `Sub` | `sub` | `Self` を返す |
| `a * b` | `Mul` | `mul` | `Self` を返す |
| `a / b` | `Div` | `div` | `Self` を返す |
| `a % b` | `Rem` | `rem` | `Self` を返す |
| `-a` | `Neg` | `neg` | `Self` を返す |
| `!a` | `Not` | `not` | `bool` または operand の protocol に従う |
| `~a` | `BitNot` | `bitnot` | 整数 protocol。実装状態は compiler gate で確認する |
| `a & b` | `BitAnd` | `bitand` | `Self` を返す |
| `a \| b` | `BitOr` | `bitor` | `Self` を返す |
| `a ^ b` | `BitXor` | `bitxor` | `Self` を返す |
| `a << b` | `Shl` | `shl` | `Self` を返す |
| `a >> b` | `Shr` | `shr` | `Self` を返す |
| `a == b` / `a != b` | `Eq` | `eq` | `!=` は canonical `eq` の結果を反転する |
| `a < b` / `a <= b` / `a > b` / `a >= b` | `Ord` | `cmp` | `Ordering` の結果を関係判定へ射影する |
| `a[i]` | `Index` | `index` | indexer の戻り型契約に従う |
| `a[i] = v` | `IndexMut` | `index_set` | setter protocol |
| `*a` | `Deref` | `deref` | dereference protocol |
| `*a = v` | `DerefMut` | `deref_mut` | setter protocol |

`&&` と `||` は short-circuit の評価順序を持つため、Trait dispatch ではなく言語組み込み
として扱う。
この ADR の equality/order 名は、既存の `Eq::eq` / `Ord::cmp` 実装を二重化しないための
現行 v2 境界である。
`PartialEq` / `PartialOrd` の階層と NaN を含む比較契約を導入する場合は、#695 の設計判断で
この表を後継 ADR により更新する。

### 2. Candidate resolution

演算子は import scope の文字列 lookup ではなく、上表の canonical protocol として解決する。
typechecker は operand の型について既存の Trait registry を問い合わせる。

- concrete receiver は `(TraitRef, SelfType)` の coherence 規則を使う。
- generic receiver は、現在の function の type parameter に対応する Trait bound がある場合だけ許可する。
- candidate が 0 個なら型エラー、複数なら ambiguity/coherence エラーとする。
- `use std::core::ops` の有無で `a + b` の合法性や意味を変えない。
- 組み込み scalar の直接 opcode は canonical Trait impl と同じ観測結果を持つ backend fast path であり、
  独立した magic-method protocol ではない。

### 3. Lowering boundary

MIR は canonical method name と static dispatch の monomorphization 結果から callee を選択する。
receiver の型名だけで同名の inherent method を拾って演算子を成立させてはならない。
GC reference と linear-memory reference の表現差は callee の合法性、Trait candidate、戻り値の
source type を変更しない。

### 4. 旧 magic method の移行

`__add`、`__sub`、`__mul`、`__div`、`__rem`、`__eq`、`__cmp` などの旧名は演算子の解決候補に
含めない。
既存コードは次のように移行する。

```text
impl Type { fn __add(self: Type, rhs: Type) -> Type { ... } }
    → impl Add for Type { fn add(self: Type, rhs: Type) -> Type { ... } }
```

自動 alias や compiler 内の二重実装は作らない。
旧 magic method を持つだけの型は、対応する canonical Trait impl がないため型エラーになる。
stable API の deprecation window が必要になった場合は ADR-014 に従い、専用の移行 issue と削除条件を
先に記録する。

## 帰結

- operator typecheck と MIR overload lowering は同じ protocol table を参照する必要がある。
- stdlib の scalar impl は意味論の正本であり、backend opcode はその実装の最適化経路である。
- user-defined inherent method を operator の暗黙候補に戻す互換修正は、この ADR に反する。
- `PartialEq` / `PartialOrd`、heterogeneous output、dynamic trait object はこの ADR の範囲外である。

## 却下した代替案

### receiver の inherent method 名を暗黙に呼ぶ

`impl Type { fn add(...) }` だけで `a + b` を成立させると、Trait registry の coherence と
generic bound を通らない別の dispatch 面が残るため採用しない。

### magic method と Trait method を同時に候補にする

同じ型に二つの実装を許し、宣言順や backend ごとの lookup 順に意味が依存するため採用しない。

### `&&` / `||` を通常の Trait method にする

右辺を遅延評価する short-circuit 契約を通常の値引数 method では表せないため採用しない。

## 再検討条件

- #695 が `PartialEq` / `PartialOrd` の canonical comparison protocol を採択した場合。
- associated output を使う heterogeneous operator の ABI が別 ADR で固定された場合。
- `IndexMut` / `DerefMut` の place semantics が parser、ownership、GC/fallback backend の全境界で検証可能になった場合。
