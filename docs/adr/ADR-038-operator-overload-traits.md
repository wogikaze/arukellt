# ADR-038: 演算子オーバーロードを magic method から trait へ移行する

ステータス: **PROPOSED** — 既存の magic method 面を `std::core::ops` trait 面へ置換する

提案日: 2026-06-26  
改訂日: 2026-07-11 — 「新設」ではなく既存 `__add` 等からの移行として書き直し

---

## 文脈

**現行（実装済み）:**

- ユーザー定義型は **magic method 名**（`__add`, `__sub`, `__mul`, …）で演算子に参加できる
- normative: `docs/language/spec.md`（Operator overloading via magic method names）
- `docs/language/syntax.md` も v1 実装済み構文として掲載

**本 ADR の提案:**

magic method 面を、#688 の trait method dispatch に基づく
**`Add` / `Sub` / `Index` 等の trait-based operator surface** へ移行する。

「演算子オーバーロードが存在しない」わけではない。存在する方式を置換する。

---

## 提案する決定

### D1: 演算子 → trait マッピング（移行先）

| 演算子 | trait | メソッド |
|--------|-------|---------|
| `a + b` | `Add` | `add` |
| `a - b` | `Sub` | `sub` |
| `a * b` | `Mul` | `mul` |
| `a / b` | `Div` | `div` |
| `a % b` | `Rem` | `rem` |
| `-a` | `Neg` | `neg` |
| `a & b` / `\|` / `^` | `BitAnd` / `BitOr` / `BitXor` | … |
| `a << b` / `>>` | `Shl` / `Shr` | … |
| `!a` | `Not` | `not` |
| `a[i]` / `a[i]=v` | `Index` / `IndexMut` | `index` / `index_set` |
| `*a` / `*a=v` | `Deref` / `DerefMut` | `deref` / `deref_mut` |

### D2: 解決優先順位（移行後）

1. ユーザー `impl Op for T`（trait dispatch）
2. 組み込みスカラー演算（`i32 + i32` → Wasm `i32.add` 等）
3. 型エラー

### D3: 借用なしの Index / Deref

Arukellt に `&T` がないため、`Index::index` / `Deref::deref` は値返し。
`IndexMut` は setter（`index_set`）パターン。

### D4: モジュール配置

`std/core/ops.ark` に演算子 trait とスカラー組み込み impl を置く。

### D5: magic method の扱い

- **移行期間**: `__add` 等は deprecated（警告）または互換エイリアスとして残してよい
- **理想形**: 公開面は trait のみ。magic method は削除または内部実装詳細へ退避
- 削除タイミング・互換期間は issue / plan（本 ADR は面の置換方針のみ固定）

---

## 代替案と却下理由

| 案 | 結果 |
|----|------|
| magic method のまま恒久運用 | 却下（trait stdlib / ADR-036 と二重面） |
| 動的 vtable 解決 | 却下（ADR-036 静的ディスパッチ） |
| 演算子オーバーロード全廃 | 却下（ユーザー数値型・Vec 結合等が書けない） |

---

## 参照

- 現行 magic methods: `docs/language/spec.md`
- ADR-036、Issue #688 / #689
- Rust `std::ops`: https://doc.rust-lang.org/std/ops/index.html
