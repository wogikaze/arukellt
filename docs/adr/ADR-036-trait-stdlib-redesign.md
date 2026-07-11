# ADR-036: Trait-based Stdlib Redesign Strategy

ステータス: **PROPOSED** — #688–#697 後に実行する stdlib 再設計の戦略

提案日: 2026-06-26

関連: [ADR-044](ADR-044-trait-method-syntax-adopted.md)（trait / メソッド構文の採択）

---

## 文脈

Issue #688-#697 は Arukellt の stdlib を「モノモルフィックな free function 集」から
「trait ベースのジェネリック API」へ移行する 10 個の連鎖的 issue 群である。

依存グラフは以下の 4 層構造を持つ:

```
Layer 0 (言語基盤):
  688  Trait method dispatch inside generic functions

Layer 1 (trait 定義 + 言語機能):
  689  Operator overload traits (Add/Index/Deref/...)
  690  ? operator + From<E> error conversion
  691  Iterator trait + lazy adapters + FromIterator/collect
  692  Clone/Default/From/Into/TryFrom trait group
  695  Ord/PartialOrd traits

Layer 2 (上位 trait + エコシステム):
  693  Read/Write/BufRead/Seek traits + IO unification  (needs 688+692)
  694  Error trait + unified error ecosystem             (needs 690+692)
  696  Debug trait + format!/write! formatting ecosystem (needs 688+692)

Layer 3 (コレクション拡張):
  697  Vec<T> operation extension                        (needs 691+695)
```

現在の stdlib は以下の構造的負債を抱えている:

- `std::seq` — `Vec<i32>` 専用の eager helper 集 (`map_i32_i32`, `filter_i32`, ...)
- `std::collections::vec.ark` — 7 行の stub (`new_i32` のみ)
- `std::io` — `Reader`/`Writer` が `Vec<i32>` 型エイリアス (byte-layout 約束)
- `std::text::fmt` — 55 行の stub、`Formatter`/`Arguments` なし
- `std::core::error` — `Error` enum はあるが `Error` **trait** ではない
- prelude の `clone`/`eq`/`i32_to_string` が trait impl ではなく free function

688-697 が完了すると、これらすべてを trait ベースのジェネリック API に再構築できる。
本 ADR はその再設計の戦略的方向性を決定する。

## 提案する決定

### D1: 静的ディスパッチ (monomorphization) をデフォルトとする

trait method dispatch は **単相化ベースの静的ディスパッチ** をデフォルト方式とする。

- `<T: Trait>` ジェネリック関数の trait method 呼び出しは、コンパイル時に各型引数ごと
  に特殊化され、直接 call 命令に lowering される。
- `dyn Trait` (動的ディスパッチ / vtable) は **将来 issue に切り出す**。本 redesign の
  スコープ外とする。
- 根拠:
  - コンパイラ実装が単純 (既存の monomorphization パスを拡張するだけ)
  - LLM フレンドリ (静的ディスパッチの方針を継承 — 解決規則が透明)
  - コードサイズ増大は当面許容可能
  - Rust も `impl Trait` / ジェネリックをデフォルトとし、`dyn` をオプトインとしている

### D2: API 削除は ADR-014 の stability に従う

モノモルフィック API（`std::seq`、prelude の `map_i32_i32` / `filter_i32` /
`sort_i32` / `Vec_new_i32` 等）の削除は、**「stdlib 全体が provisional」という前提では行わない**。
manifest 上 `stable` の記号が多数ある（ADR-014）。

| stability | 削除方針 |
|-----------|----------|
| `experimental` | 直接削除可（migration note 推奨） |
| `provisional` | 個別判断。原則 migration note |
| `stable` | **少なくとも 1 リリースの deprecation**（W0008）+ migration guide + 削除リリースを明記 |
| 既に `deprecated_by` | 定めた削除時期に削除 |

「bold cutover」は **experimental / 明示 deprecated 済み** に限定する。
stable 契約を無視した一括削除は ADR-014 違反であり禁止。

**個別 ADR の移行期間:** 例として ADR-037（SIMD）は型 identity・名前空間変更のため
少なくとも 1 リリースの deprecation を置く（本節と整合）。

旧「stdlib は provisional なので一括削除可」という根拠は **撤回**する。
### D3: モジュール再編成

現在のモジュール構造を以下のように再編成する:

| 現在 | 移行後 | 変更内容 |
|------|--------|----------|
| `std::seq` | **廃止** → `std::iter` + `std::collections::vec` | eager helper を Iterator adapter + Vec method に置換 |
| `std::collections::vec` (stub) | `std::collections::vec` (本格化) | Vec<T> メソッド表面を拡充 |
| `std::core::cmp` | `std::core::cmp` (拡張) | Ord/PartialOrd 追加 |
| `std::core::convert` | `std::core::convert` (拡張) | From/Into/TryFrom 追加 |
| `std::core::error` (enum) | `std::core::error` (trait) | Error enum → AppError にリネーム、Error trait 新設 |
| `std::core::hash` | `std::core::hash` (維持) | Hash trait は既存のまま dispatch 有効化のみ |
| — (新設) | `std::core::ops` | Add/Sub/Mul/Index/Deref 等 |
| — (新設) | `std::core::iter` | Iterator/IntoIterator/FromIterator |
| — (新設) | `std::core::clone` | Clone/Copy |
| — (新設) | `std::core::default` | Default |
| — (新設) | `std::core::fmt` | Debug/Display/Formatter/Arguments |
| `std::text::fmt` (stub) | `std::fmt` に統合 or `std::text::fmt` (本格化) | format!/write! マクロ基盤 |
| `std::io` (Vec<i32> alias) | `std::io` (trait ベース) | Read/Write/BufRead/Seek |

### D4: トレイト階層の標準化

stdlib trait 階層を以下の通り標準化する:

```
PartialEq (新設/整理, #695) — 部分的な等価（反射律を要求しない。f64 / F32x4 等）
 └─ Eq: PartialEq (既存をこの階層へ位置づけ) — 全等価（反射律を含む）

PartialOrd: PartialEq (新設, #695) — 部分順序（NaN 等で全順序にならない型）
 └─ Ord: Eq + PartialOrd (新設, #695) — 全順序

Hash (既存)
Clone (新設, #692)
 └─ Copy: Clone (新設, #692) — marker trait
Default (新設, #692)
Display (既存) — ユーザー向け表示。Debug の supertrait にはしない
Debug (新設, #696) — 診断・開発者向け。Display と独立

# 変換 trait は独立。Self の向きが逆のため supertrait にしない
From<T> (新設, #692)      — impl From<Source> for Destination { from }
Into<T> (新設, #692)      — ユーザー直接 impl は初期仕様で禁止（下記 blanket のみ）
TryFrom<T, E> (新設, #692) — エラー型は型引数 E（associated type は RFC-004 まで先送り）
TryInto<T, E> (新設, #692) — ユーザー直接 impl は初期仕様で禁止

Iterator (新設, #691)
IntoIterator (新設, #691)
FromIterator (新設, #691)
Read (新設, #693)
Write (新設, #693)
 └─ BufRead: Read (新設, #693)
Seek (新設, #693)
Error: Display (新設, #694)
Add/Sub/Mul/Div/Neg/Index/Deref/... (新設, #689)
```

包含関係を逆転させない（`PartialOrd: Ord` は禁止）。`f64` は `PartialEq` +
`PartialOrd` のみで、`Eq` / `Ord` は実装しない。

**比較 trait のメソッド契約（初期）:**

<!-- skip-doc-check reason="ADR design sketch; not a runnable snippet" owner="#771" kind="pseudocode" expires="2026-12-31" -->
```ark
trait PartialEq {
    fn eq(self, other: Self) -> bool

    // default
    fn ne(self, other: Self) -> bool {
        !self.eq(other)
    }
}

trait Eq: PartialEq {
    // marker — メソッドなし
}

trait PartialOrd: PartialEq {
    fn partial_cmp(self, other: Self) -> Option<Ordering>
}

trait Ord: Eq + PartialOrd {
    fn cmp(self, other: Self) -> Ordering
}
```

| 演算子 | 解決先 |
|--------|--------|
| `a == b` | `PartialEq::eq` |
| `a != b` | `PartialEq::ne` |
| `a < b` / `<=` / `>` / `>=` | `PartialOrd::partial_cmp` から判定 |

これにより ADR-037 の「`F32x4`: PartialEq のみ / `I32x4`: PartialEq + Eq /
SIMD は PartialOrd を実装しないので `<` 禁止」が自然に導ける。

**破壊的移行（既存 `Eq::eq`）:** 現行 `std::core::cmp::Eq` は `eq` メソッドを持つ。
本 redesign ではメソッドを `PartialEq` へ移し、`Eq` は marker とする。
migration table（`docs/stdlib/monomorphic-deprecation.md` および本 redesign 詳細）に
「`Eq::eq` → `PartialEq::eq`」を追加し、ADR-014 / D2 に従って deprecate → 削除する。

**Blanket implementation（変換）— supertrait ではない:**

```text
# ユーザーは From / TryFrom のみ実装する
# Into / TryInto の直接実装は初期仕様では禁止（blanket と競合するため）

impl<T, U> Into<U> for T where U: From<T> {
    fn into(self) -> U { U::from(self) }
}

impl<T, U, E> TryInto<U, E> for T where U: TryFrom<T, E> {
    fn try_into(self) -> Result<U, E> { U::try_from(self) }
}

# 同一 Source → Destination について複数の From 実装は禁止
```

`Into<T>: From` / `TryInto: TryFrom` という supertrait 表記は禁止
（`From` は変換先、`Into` は変換元に付き、`Self` の向きが逆）。

`Debug` と `Display` は独立。内部構造を出せてもユーザー向け表示を定義したくない型を許す。

`Self` / associated type / blanket の言語機能前提は
[RFC-004](../rfcs/004-trait-expressiveness.md) を参照（初期は型引数で代替）。

### D5: prelude の thin wrapper 化

prelude の free function (`clone`, `eq`, `i32_to_string` 等) は **trait impl への
thin wrapper** として残す。これにより:

- 既存コードの `clone(s)` / `eq(a, b)` 呼び出しは引き続き動作する
- 新規コードは `s.clone()` / `a.eq(b)` のメソッド構文を推奨
- prelude 関数の実装は `impl Clone for String { fn clone(...) }` へ delegate する

## 代替案と却下理由

### 代替 A: experimental 以外もすべて bold cutover

stable / provisional も含め deprecated なしで一括削除する。

却下理由:
- manifest 上多数の API が `stable`（ADR-014）。移行ガイドと事前告知が必要
- 「stdlib 全体が provisional」は事実と異なる

### 代替 B: vtable ベース動的ディスパッチ

trait dispatch を vtable 暗黙パラメータ方式で実装する。

却下理由:
- コンパイラ実装が複雑 (vtable 生成・渡送・call_indirect lowering)
- LLM フレンドリ方針に反する (解決規則が不透明)
- コードサイズメリットは当面不要
- `dyn Trait` が必要になった時点で別途追加可能 (拡張ポイントは残す)

## 結果

採択後の作業チェックリストは
[`docs/plans/trait-stdlib-redesign.md`](../plans/trait-stdlib-redesign.md) に置く
（本 ADR に進捗ダッシュボードを残さない）。

## 参照

- ADR-014: Stability Labels
- Issue #688-#697: trait dispatch + stdlib trait 化 issue 群
- `docs/stdlib/expansion-policy.md`
- `docs/stdlib/monomorphic-deprecation.md`
