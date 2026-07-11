# RFC-004: Trait expressiveness（Self・型引数・associated function・coherence）

ステータス: **DRAFT**  
関連: [ADR-036](../adr/ADR-036-trait-stdlib-redesign.md)、[ADR-038](../adr/ADR-038-operator-overload-traits.md)、[ADR-039](../adr/ADR-039-question-mark-operator.md)、[ADR-040](../adr/ADR-040-typed-mir-signature-registry.md)、[ADR-042](../adr/ADR-042-intrinsic-layer-separation.md)
提案日: 2026-07-11  
改訂日: 2026-07-11 — associated function・output 一意性・coherence・演算子/`?` の canonical 解決を追加

---

## 目的

ADR-036/038/039 が前提とする trait 面を、**現行型システムで書ける最小表現**に固定する。
associated type を先送りし、型引数と `Self`（メソッド戻り）で代替する。
あわせて、初期 solver が壊れない **coherence / functional dependency** と、
method call と language syntax の解決経路の分離を定義する。

---

## 決定（提案）

### 1. 初期で導入するもの

| 機能 | 初期 | 備考 |
|------|------|------|
| `trait` / `impl Trait for T` | ✅ | #688 |
| メソッド構文 `x.method()` | ✅ | receiver 付き |
| associated function | ✅ | `self` なし。`T::f(...)` / `<T as Trait>::f(...)` |
| `Self`（下記 §2） | ✅ | trait 宣言内と impl 内で意味が異なる |
| 型パラメータ付き trait | ✅ | `From<T>`, `Add<Rhs>`, `TryFrom<T, E>` |
| blanket impl | ✅ | コンパイラ既知の少数（`Into`/`TryInto` 等）。任意ユーザー blanket は後続 |
| associated type | ❌ 初期 | RFC 後継で導入 |
| `derive` | ❌ 初期 | #696 後続 |
| trait import スコープ（method call） | ✅ 最小 | §6 |

### 2. `Self` の意味

| 文脈 | 意味 |
|------|------|
| trait 宣言内 | 将来この trait を実装する型（プレースホルダ） |
| `impl Trait for T` 内 | 現在の impl 対象型 `T` |

「impl 対象型の別名」だけでは不足する。両方を定義する。

### 3. trait method と associated function

```text
trait method:
    第1引数が self（値 receiver）
    x.method(...) で呼べる

trait associated function:
    self 引数を持たない
    <T as Trait>::function(...) または T::function(...) で呼ぶ

初期短縮規則:
    候補が一意なら T::function(...)
    曖昧なら完全修飾 <T as Trait>::function(...) を要求
```

これがないと次が表現できない:

```ark
trait From<T> {
    fn from(value: T) -> Self
}

trait Default {
    fn default() -> Self
}

trait TryFrom<T, E> {
    fn try_from(value: T) -> Result<Self, E>
}
```

ADR-044 が採択しているのは trait / impl / `x.method()`。
`T::default()` と `U::from(x)` の構文・名前解決は本 RFC が補完する。

### 4. associated type の代替と output 一意性

```text
Iterator<Item>          — Item を型引数に
TryFrom<T, E>           — Error を型引数 E に
Add<Rhs>                — 戻りは Self に限定（初期）
Index<Idx, Out>         — Out を型引数に
Deref<Target>           — Target を型引数に
```

heterogeneous `Add`（戻り ≠ Self）や任意 `Output` は associated type 導入後。

**Functional dependency（初期必須）:**

同一の trait input key に対して impl は高々 1 つ。

| Trait | Input key（一意に決める） | 一意に決まる型 |
|-------|---------------------------|----------------|
| `Iterator<Item>` | `Self` | `Self → Item` |
| `TryFrom<T, E>` | `(Self, T)` | `(Self, T) → E` |
| `Index<Idx, Out>` | `(Self, Idx)` | `(Self, Idx) → Out` |
| `Deref<Target>` | `Self` | `Self → Target` |
| `Into<U>` | `(Self, U)` | 高々 1 impl（blanket 経由） |
| `From<T>` | `(Self, T)` | 高々 1 impl |

これがないと、例えば:

```text
impl Iterator<i32> for X
impl Iterator<String> for X
```

や

```text
impl TryFrom<String, ErrorA> for User
impl TryFrom<String, ErrorB> for User
```

が許され、`x.next()` / `s.try_into()` の結果型が決まらない。

### 5. Coherence（初期）

- 重複 impl 禁止（同一 input key）
- overlapping generic impl 禁止
- 初期は **user blanket impl 禁止**（stdlib / compiler が提供する既知 blanket のみ）
- std / compiler 提供 blanket が user impl より優先されて「黙って勝つ」ことはしない（衝突は型エラー）
- `Into` / `TryInto` の直接 impl 禁止（ADR-036）
- impl 候補が複数なら「最初を選ぶ」のではなく **型エラー**

この規則がないまま generic trait dispatch を実装すると再設計になる。

### 6. Method call と language syntax の解決分離

RFC 初期案の「trait を `use` しなければメソッド解決に使わない」は
**method-call syntax にのみ**適用する。

```text
method-call syntax (`x.clone()`, `x.into()`):
    import scope 内の trait を候補にする

language syntax desugaring (`a + b`, `a == b`, `expr?`):
    compiler が canonical SemanticTraitId を直接使う
    import の有無に依存しない

impl discovery:
    canonical trait ID と receiver / type arguments で検索
    名前文字列 "Add" / "From" では解決しない
```

| 構文 | Canonical trait（例） |
|------|------------------------|
| `a + b` | `SemanticTraitId::Add` |
| `a == b` / `a != b` | `SemanticTraitId::PartialEq` |
| `a < b` 等 | `SemanticTraitId::PartialOrd` |
| `expr?`（異種 Err） | `SemanticTraitId::From` |

これは ADR-040 / ADR-042 の semantic spine と統合する。
ユーザーが `use std::core::ops::Add` を書かなかったから `a + b` が失敗する、
という設計は採用しない。

### 7. 変換・演算子との整合

- ユーザーは `From` / `TryFrom` を実装。`Into` / `TryInto` は blanket のみ（ADR-036）
- 演算子 trait の初期署名は ADR-038（本 RFC に従う）
- 比較 trait のメソッド契約は ADR-036（`PartialEq::eq` 等）

### 8. 禁止（初期）

- ユーザー定義の任意 blanket impl
- associated type 構文
- `Into` / `TryInto` の直接実装
- 同一 input key への複数 impl
- language syntax の import 依存解決

---

## 関連

- ADR-036 D4 / ADR-038 / ADR-039 / ADR-040 / ADR-042 / issue #688–#697
