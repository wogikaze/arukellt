# ADR-038: Operator Overload Trait Surface

ステータス: **PROPOSED** — #688 完了後に実装される演算子オーバーロードの設計 ADR

決定日: 2026-06-26

---

## 文脈

Issue #689 は Arukellt に演算子オーバーロードを導入する。現在 `+`/`-`/`*`/`/`/
`[]` は組み込みスカラー型のみで動作し、ユーザー定義型は演算子構文に参加できない。

#688 で trait method dispatch が実装されたため、演算子を trait メソッド呼び出しに
マッピングできる。本 ADR はそのマッピングのセマンティクスを決定する。

## 決定事項

### D1: 演算子 → trait マッピング

以下の演算子を trait にマッピングする:

| 演算子 | trait | メソッド | 備考 |
|--------|-------|---------|------|
| `a + b` | `Add` | `add(self, rhs) -> Self` | |
| `a - b` | `Sub` | `sub(self, rhs) -> Self` | |
| `a * b` | `Mul` | `mul(self, rhs) -> Self` | |
| `a / b` | `Div` | `div(self, rhs) -> Self` | |
| `a % b` | `Rem` | `rem(self, rhs) -> Self` | |
| `-a` | `Neg` | `neg(self) -> Self` | 単項 |
| `a & b` | `BitAnd` | `bitand(self, rhs) -> Self` | |
| `a \| b` | `BitOr` | `bitor(self, rhs) -> Self` | |
| `a ^ b` | `BitXor` | `bitxor(self, rhs) -> Self` | |
| `a << b` | `Shl` | `shl(self, rhs) -> Self` | |
| `a >> b` | `Shr` | `shr(self, rhs) -> Self` | |
| `!a` | `Not` | `not(self) -> Self` | 単項 |
| `a[i]` | `Index` | `index(self, idx) -> T` | |
| `a[i] = v` | `IndexMut` | `index_set(self, idx, value)` | |
| `*a` | `Deref` | `deref(self) -> Target` | |
| `*a = v` | `DerefMut` | `deref_mut(self) -> Target` | |

### D2: 組み込みスカラーへのフォールバック

演算子の型チェックは以下の優先順位で行う:

1. **ユーザー impl ルックアップ**: レシーバ型に対する `impl Op for T` が存在すれば、
   その trait メソッドに解決する（#688 の trait method dispatch 経由）。
2. **組み込みフォールバック**: impl が存在しない場合、組み込みスカラー演算に
   フォールバックする（`i32 + i32` → WASM `i32.add` 等）。
3. **型エラー**: いずれも該当しない場合は型エラー。

この順序により、既存のコードは変更なしで動作し、ユーザー定義型のみが trait 解決を
経由する。

### D3: `Index` / `IndexMut` の戻り値型

Arukellt には借用 (`&T`) がないため、`Index::index` は値返し (`-> Output`) とする。
`IndexMut` はミュータブル参照の代わりに setter パターンを使用する。

**理想形（本 ADR の正）:**

```
trait Index<Idx, Output> {
    fn index(self: Index, idx: Idx) -> Output
}
trait IndexMut<Idx, Output> {
    fn index_set(self: IndexMut, idx: Idx, value: Output)
}
```

`a[i] = v` は `IndexMut::index_set(a, i, v)` に脱糖する。

### D4: `Deref` の戻り値型

借用がないため、`Deref::deref` は値返し (`-> Target`) とする。`*a` は
`Deref::deref(a)` に脱糖する。`DerefMut` は `deref_mut(self) -> Target` で、
`*a = v` は `Target` 型の setter を経由する。

**理想形（本 ADR の正）:** `Deref<Target>` / `DerefMut<Target>`。

### D5: モジュール配置

`std::core::ops` モジュールに全ての演算子 trait を配置する:

```
std/core/ops.ark  — Add, Sub, Mul, Div, Rem, Neg, BitAnd, BitOr, BitXor,
                    Shl, Shr, Not, Index, IndexMut, Deref, DerefMut
```

組み込み impl は同じファイルに配置する（`impl Add for i32` 等）。

### D6: 提供する trait 面（理想）

次を言語・stdlib の演算子面として提供する:

- `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg` — 算術
- `BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr`, `Not` — ビット / 論理
- `Index`, `IndexMut` — インデックス
- `Deref`, `DerefMut` — デリファレンス

スカラー型への組み込み impl と、ユーザー定義型への `impl` の両方を理想とする。

## 代替案と却下理由

### 代替 A: 全演算子を組み込みのまま（trait なし）

演算子オーバーロードを導入せず、全て組み込みスカラーのみで運用する。

却下理由:
- ユーザー定義の数値型（複素数、行列、量型）が書けない
- stdlib の `Vec<T>` の `+` 結合などが表現できない
- Rust parity の前提が崩れる

### 代替 B: 動的ディスパッチ（vtable）

演算子解決を vtable 経由で行う。

却下理由:
- ADR-036 D1 と矛盾（静的ディスパッチがデフォルト）
- スカラー演算のフォールバックが複雑になる
- LLM フレンドリ方針に反する

## 結果

- 演算子は trait メソッドへマッピングされ、スカラーは組み込みフォールバックを持つ

## 参照

- ADR-036: Trait-based Stdlib Redesign Strategy（D1 静的ディスパッチ）
- Issue #688: Trait method dispatch inside generic functions
- Issue #689: Operator overload trait surface
- Rust `std::ops`: <https://doc.rust-lang.org/std/ops/index.html>
