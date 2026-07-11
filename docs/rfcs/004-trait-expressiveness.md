# RFC-004: Trait expressiveness（Self・型引数・blanket）

ステータス: **DRAFT**  
関連: [ADR-036](../adr/ADR-036-trait-stdlib-redesign.md)、[ADR-038](../adr/ADR-038-operator-overload-traits.md)、[ADR-039](../adr/ADR-039-try-operator.md)  
提案日: 2026-07-11

---

## 目的

ADR-036/038/039 が前提とする trait 面を、**現行型システムで書ける最小表現**に固定する。
associated type を先送りし、型引数と `Self`（メソッド戻り）で代替する。

---

## 決定（提案）

### 1. 初期で導入するもの

| 機能 | 初期 | 備考 |
|------|------|------|
| `trait` / `impl Trait for T` | ✅ | #688 |
| メソッド構文 `x.method()` | ✅ | |
| `Self`（impl 対象型の別名） | ✅ | `Clone::clone(self) -> Self` |
| 型パラメータ付き trait | ✅ | `From<T>`, `Add<Rhs>`, `TryFrom<T, E>` |
| blanket impl | ✅ | コンパイラ既知の少数（`Into`/`TryInto` 等）。任意ユーザー blanket は後続 |
| associated type | ❌ 初期 | RFC 後継で導入 |
| `derive` | ❌ 初期 | #696 後続 |
| trait import スコープ | ✅ 最小 | trait を use しないとメソッド解決に使わない（詳細は #688） |

### 2. associated type の代替

```text
Iterator<Item>          — Item を型引数に
TryFrom<T, E>           — Error を型引数 E に
Add<Rhs>                — 戻りは Self に限定（初期）
Index<Idx, Out>         — Out を型引数に
Deref<Target>           — Target を型引数に
```

heterogeneous `Add`（戻り ≠ Self）や任意 `Output` は associated type 導入後。

### 3. 変換・演算子との整合

- ユーザーは `From` / `TryFrom` を実装。`Into` / `TryInto` は blanket のみ（ADR-036）
- 演算子 trait の初期署名は ADR-038（本 RFC に従う）

### 4. 禁止（初期）

- ユーザー定義の任意 blanket impl
- associated type 構文
- `Into` / `TryInto` の直接実装

---

## 関連

- ADR-036 D4 / ADR-038 / issue #688–#697
