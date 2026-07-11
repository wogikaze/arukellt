# Trait-based Stdlib Redesign — Detailed Design

> 戦略 ADR: [ADR-036](../adr/ADR-036-trait-stdlib-redesign.md)
> 対応 issue 群: #688-#697
> ステータス: **DRAFT** — 688-697 完了後に実行

本 doc は ADR-036 の決定 (静的ディスパッチ優先 + 大胆切り替え) に基づき、
モジュール別の詳細設計・トレイト階層・移行計画を定義する。

---

## 1. トレイト階層 (Trait Hierarchy)

### 1.1 完全階層図

```
PartialEq (#695) — 部分的な等価（反射律を要求しない）
 └─ Eq: PartialEq (既存 #cmp をこの階層へ) — 全等価

PartialOrd: PartialEq (#695) — 部分順序（f64 / F32x4 等）
 └─ Ord: Eq + PartialOrd (#695) — 全順序

  Hash (既存 #hash)     Clone (#692)       Default (#692)     Display (既存 #convert)
                         │                                       │
                       Copy: Clone                              Debug (#696)
                      (marker)

  From<T> (#692) ──blanket──> Into<T> (#692)
  TryFrom<T> (#692) ──blanket──> TryInto<T> (#692)

  Iterator (#691)        IntoIterator (#691)      FromIterator (#692連携)

  Read (#693)            Write (#693)             Seek (#693)
   │                       │
  BufRead: Read (#693)   (BufWriter<W: Write> は struct, trait ではない)

  Error: Display (#694)

  Add (#689)  Sub (#689)  Mul (#689)  Div (#689)  Rem (#689)  Neg (#689)
  BitAnd (#689) BitOr (#689) BitXor (#689) Shl (#689) Shr (#689) Not (#689)
  Index (#689)  IndexMut (#689)  Deref (#689)  DerefMut (#689)
```

### 1.2 supertrait 関係まとめ

| Trait | Supertrait | Issue | 備考 |
|-------|-----------|-------|------|
| `Eq` | `PartialEq` | #695 | 全等価（反射律） |
| `PartialOrd` | `PartialEq` | #695 | 部分順序。`PartialOrd: Ord` は禁止 |
| `Ord` | `Eq` + `PartialOrd` | #695 | 全順序 |
| `Copy` | `Clone` | #692 | marker trait (メソッドなし) |
| `Debug` | — | #696 | Display と並列 (supertrait なし、独立) |
| `Into<T>` | — | #692 | `From<T>` から blanket impl で自動導出 |
| `TryInto<T>` | — | #692 | `TryFrom<T>` から blanket impl |
| `BufRead` | `Read` | #693 | |
| `Error` | `Display` | #694 | source() chaining |

> **設計メモ:** 標準的な包含関係（Rust と同型）を採用する。
> 旧案の `PartialOrd: Ord` は `f64` を PartialOrd のみにできず破綻するため却下（ADR-036 D4）。

### 1.3 組み込み impl マトリクス

各 trait に対する組み込み型の impl 義務を定義する。□=未実装、■=実装済み、◇=本 redesign で追加。

| Type | PartialEq | Eq | PartialOrd | Ord | Hash | Clone | Copy | Default | Display | Debug | From |
|------|-----------|----|------------|-----|------|-------|------|---------|---------|-------|------|
| i32 | ■ | ■ | ◇ | ◇ | ■ | ◇ | ◇ | ◇ | ■ | ◇ | ◇(←i64等) |
| i64 | ■ | ■ | ◇ | ◇ | ■ | ◇ | ◇ | ◇ | ■ | ◇ | ◇ |
| f64 | ◇ | — | ◇ | — | ■ | ◇ | ◇ | ◇ | ■ | ◇ | ◇ |
| bool | ■ | ■ | ◇ | ◇ | ■ | ◇ | ◇ | ◇ | ■ | ◇ | — |
| char | ■ | ■ | ◇ | ◇ | ■ | ◇ | ◇ | ◇ | ■ | ◇ | — |
| String | ■ | ■ | ◇ | ◇ | ■ | ◇ | — | ◇ | ■ | ◇ | ◇ |
| Vec<T> | ◇† | ◇† | ◇† | ◇† | — | ◇ | — | ◇ | — | ◇ | — |
| Option<T> | ◇† | ◇† | ◇† | ◇† | — | ◇ | ◇‡ | ◇ | — | ◇ | — |
| Result<T,E> | — | — | — | — | — | ◇ | — | — | — | ◇ | — |

`*` f64 は `PartialEq` + `PartialOrd` のみ（NaN で反射律・全順序不可）。`Eq` / `Ord` は実装しない。
`†` 要素型が対応する比較 trait であることを要求。
`‡` T: Copy の場合のみ。

---

## 2. モジュールマップ (Before / After)

### 2.1 std::core 配下

#### Before (現状)

```
std::core/
  mod.ark         — Ordering, Range, cmp_i32, identity
  cmp.ark         — Eq trait + scalar impls, cmp/min/max/clamp (i32 only)
  convert.ark     — Display trait + scalar impls, i32_to_string etc.
  error.ark       — Error enum (concrete), error_message()
  hash.ark        — Hash trait + scalar impls, hash_i32/hash_string
  math.ark        — 数学関数
```

#### After (redesign後)

```
std::core/
  mod.ark         — Ordering, Range (変更なし)
  cmp.ark         — Eq + Ord + PartialOrd trait + scalar impls
                    cmp/min/max/clamp をジェネリック化 (Ord bound)
  convert.ark     — Display + From + Into + TryFrom + TryInto trait
                    i32_to_string 等は Display impl へ delegate
  clone.ark       — [新設] Clone + Copy trait + scalar impls
  default.ark     — [新設] Default trait + scalar impls
  ops.ark         — [新設] Add/Sub/Mul/Div/Neg/Index/IndexMut/Deref/DerefMut
                    + scalar impls (Add for i32 等)
  iter.ark        — [新設] Iterator/IntoIterator/FromIterator trait 定義
                    (adapter 型の実装は std::iter に配置)
  fmt.ark         — [新設] Debug trait + Formatter + Arguments 型定義
                    (format!/write! マクロ展開は std::fmt に配置)
  error.ark       — Error trait (新設) + AppError enum (旧 Error enum からリネーム)
                    impl Error for AppError, impl Error for IoError, ...
  hash.ark        — Hash trait (変更なし、dispatch 有効化のみ)
  math.ark        — 数学関数 (変更なし)
```

### 2.2 std::iter (新設)

`std::seq` を廃止し、`std::iter` を新設する。

```
std::iter/
  mod.ark         — Iterator trait の再エクスポート + 汎用コンビナータ
                    (collect, fold, sum, count, any, all, find, for_each)
  adapters.ark    — Map/Filter/Take/Skip/Zip/Enumerate/Chain/Peekable
                    lazy adapter struct + impl Iterator
  vec_iter.ark    — VecIter<T> struct + impl Iterator for VecIter<T>
                    impl IntoIterator for Vec<T>
  from_iter.ark   — impl FromIterator for Vec<T>
```

**`std::seq` からの移行対応表:**

| std::seq (廃止) | std::iter / Vec method (移行先) |
|-----------------|-------------------------------|
| `map_i32_i32(v, f)` | `v.iter().map(f).collect()` |
| `filter_i32(v, f)` | `v.iter().filter(f).collect()` |
| `take_i32(v, n)` | `v.iter().take(n).collect()` |
| `skip_i32(v, n)` | `v.iter().skip(n).collect()` |
| `fold_i32_i32(v, init, f)` | `v.iter().fold(init, f)` |
| `binary_search(v, t)` | `v.binary_search(t)` (Vec method, #695) |
| `min_i32(v)` | `v.iter().min()` (Ord bound) |
| `max_i32(v)` | `v.iter().max()` (Ord bound) |
| `sum_i32(v)` | `v.iter().sum()` (Iterator consumer) |
| `seq_reverse(v)` | `v.reverse()` (Vec method, #697) |
| `seq_contains(v, t)` | `v.contains(t)` (Vec method) |
| `unique(v)` | `v.iter().collect::<HashSet<_>>()` |
| `count_eq(v, t)` | `v.iter().filter(\|x\| x == t).count()` |

### 2.3 std::collections::vec (本格化)

7 行の stub から Vec<T> の本格的メソッド表面へ拡張する。

```
std::collections/
  vec.ark         — Vec<T> 拡張メソッド (#697)
                    windows/chunks/retain/truncate/resize/extend/append
                    drain/splice/sort/sort_by/sort_by_key
                    dedup/dedup_by/binary_search/binary_search_by
                    reverse/contains/into_iter/from_iter
  hash_map.ark    — HashMap<K: Hash, V> (既存、dispatch 有効化)
  hash_set.ark    — HashSet<T: Hash> (既存、dispatch 有効化)
  ...
```

### 2.4 std::io (trait ベース再構築)

`Vec<i32>` 型エイリアス方式を廃止し、Read/Write trait ベースに再構築する。

```
std::io/
  mod.ark         — Read/Write/BufRead/Seek trait 定義
                    IoError enum (既存維持)
                    io::copy<R: Read, W: Write> ジェネリック関数
  buf_reader.ark  — BufReader<R: Read> generic adapter
  buf_writer.ark  — BufWriter<W: Write> generic adapter
  memory.ark      — Cursor<T> / InMemoryBuffer
                    impl Read for Cursor<Vec<u8>>, impl Write for Cursor<...>
  chain.ark       — Chain<R1: Read, R2: Read> adapter
```

**既存 `Vec<i32>` alias からの移行:**

| 現在 | 移行後 |
|------|--------|
| `Reader` = `Vec<i32>` alias | `impl Read for Cursor<Vec<u8>>` |
| `Writer` = `Vec<i32>` alias | `impl Write for Cursor<Vec<u8>>` |
| `reader_from_bytes(bs)` | `Cursor::new(bs)` |
| `reader_read_byte(r)` | `r.read_byte()` (Read trait method) |
| `writer_write_bytes(w, bs)` | `w.write_all(bs)` (Write trait method) |
| `buffered_writer(w)` | `BufWriter::new(w)` |

### 2.5 std::fmt / std::text::fmt (フォーマット層)

```
std::fmt/                    (または std::text::fmt/ を本格化)
  mod.ark         — Debug/Display trait の再エクスポート
                    Formatter, Arguments 型
                    format!/write!/println! マクロ展開基盤
  formatter.ark   — Formatter 実装 (pad, align, fill, precision)
  arguments.ark   — Arguments 中間表現 (format string parse 結果)
```

**`std::text::fmt` (現状 stub) からの移行:**

| 現在 | 移行後 |
|------|--------|
| `format_i32(n)` | `format!("{}", n)` または `n.to_string()` |
| `format_f64(n)` | `format!("{}", n)` |
| `pad_left(s, w, f)` | `format!("{:>w$}", s)` (Formatter 機能) |
| `pad_right(s, w, f)` | `format!("{:<w$}", s)` |

### 2.6 std::error (新設)

```
std::error/                  (または std::core::error.ark に統合)
  mod.ark         — Error trait: Display supertrait + source() method
                    impl Error for IoError
                    impl Error for AppError (旧 std::core::Error enum)
                    impl From<IoError> for AppError
                    impl From<String> for AppError
```

---

## 3. prelude 再設計

### 3.1 prelude 関数の thin wrapper 化

prelude の free function は trait impl への delegate に切り替える。
**関数名は維持** (破壊的変更を最小化) し、実装のみ trait dispatch 経由にする。

<!-- skip-doc-check --><!-- 設計図示用の擬似コード。trait dispatch 未実装のためコンパイル不可 -->

```ark
// prelude.ark (after)

/// String clone — delegates to Clone trait impl
pub fn clone(s: String) -> String {
    s.clone()   // Clone::clone via trait dispatch
}

/// String equality — delegates to Eq trait impl
pub fn eq(a: String, b: String) -> bool {
    a.eq(b)     // Eq::eq via trait dispatch
}

/// i32 to string — delegates to Display trait impl
pub fn i32_to_string(x: i32) -> String {
    x.to_string()   // Display::to_string via trait dispatch
}
```

### 3.2 prelude から削除する関数 (大胆切り替え)

以下のモノモルフィック関数は prelude から **削除** する:

- `Vec_new_i32`, `Vec_new_i64`, `Vec_new_f64`, `Vec_new_String`
  → `Vec::new<T>()` (ジェネリック)
- `Vec_new_i32_with_cap`, `Vec_new_i64_with_cap`, `Vec_new_f64_with_cap`
  → `Vec::with_capacity<T>(n)`
- `sort_i32`, `sort_i64`, `sort_f64`, `sort_String`
  → `v.sort()` (Vec method, Ord bound) または `v.sort_by(cmp)`
- `map_i32_i32`, `filter_i32`, `fold_i32_i32`, `any_i32`, `find_i32`
  → `v.iter().map(f)...` (Iterator adapter)
- `Vec_with_capacity_i32`, `Vec_with_capacity_String`
  → `Vec::with_capacity<T>(n)`

### 3.3 prelude に追加する型・trait

<!-- skip-doc-check --><!-- 設計図示用。対象モジュールが未実装のためコンパイル不可 -->

```ark
// prelude に暗黙インポートされる trait (after)
use std::core::cmp::{Eq, Ord}
use std::core::clone::Clone
use std::core::default::Default
use std::core::convert::{Display, From, Into, TryFrom, TryInto}
use std::core::fmt::Debug
use std::core::iter::{Iterator, IntoIterator, FromIterator}
use std::core::ops::{Add, Sub, Mul, Div, Index, Deref}
use std::core::hash::Hash
```

> **注意**: prelude への trait インポートは「trait をスコープに入れる」効果と
> 「メソッド構文を有効化する」効果の両方を持つ。Arukellt の trait import semantics は
> #688 で確定させる必要がある。

---

## 4. 移行計画

### 4.1 移行フェーズ

```
Phase A: 688-697 完了 (言語機能 + trait 定義)
  ↓
Phase B: stdlib 再構築 (本 doc の設計に従って実装)
  ↓
Phase C: モノモルフィック API 削除 + 移行ガイド公開
  ↓
Phase D: prelude 切り替え + ドキュメント再生成
```

### 4.2 Phase B: stdlib 再構築の実装順序

trait 間の依存関係に従い、以下の順序で実装する:

```
B1. std::core::clone (Clone/Copy)         — #692 trait 定義のみ
B2. std::core::default (Default)          — #692
B3. std::core::convert 拡張 (From/Into)   — #692
B4. std::core::cmp 拡張 (Ord/PartialOrd)  — #695
B5. std::core::ops (Add/Index/Deref/...)  — #689
B6. std::core::iter (Iterator 定義)       — #691
B7. std::iter (adapter 型 + VecIter)      — #691
B8. std::collections::vec 本格化           — #697
B9. std::core::fmt (Debug/Formatter)      — #696
B10. std::fmt (format!/write! 基盤)       — #696
B11. std::core::error (Error trait)       — #694
B12. std::error (impl Error for ...)      — #694
B13. std::io trait ベース再構築            — #693
B14. prelude thin wrapper 化              — 全 trait dispatch 有効化後
```

### 4.3 Phase C: 削除対象と [breaking] issue

以下の `[breaking]` issue を作成する必要がある:

| 削除対象 | 影響範囲 | 移行先 |
|---------|---------|--------|
| `std::seq` モジュール全体 | `map_i32_i32`, `filter_i32`, ... 12関数 | `std::iter` + Vec method |
| prelude `Vec_new_*` (4関数) | Vec コンストラクタ | `Vec::new<T>()` |
| prelude `Vec_new_*_with_cap` (3関数) | Vec コンストラクタ | `Vec::with_capacity<T>(n)` |
| prelude `Vec_with_capacity_*` (2関数) | Vec コンストラクタ | `Vec::with_capacity<T>(n)` |
| prelude `sort_*` (4関数) | Vec sort | `v.sort()` / `v.sort_by()` |
| prelude `map_i32_i32`/`filter_i32`/`fold_i32_i32`/`any_i32`/`find_i32` | 高階関数 | Iterator adapter |
| `std::io` の `Vec<i32>` alias | Reader/Writer 型 | Read/Write trait + Cursor |
| `std::core::error` の `Error` enum | error 型名 | `AppError` にリネーム |
| `std::text::fmt` の `format_i32`/`format_f64`/`pad_left`/`pad_right` | フォーマット | `format!` マクロ |

### 4.4 移行ガイド

統合移行ガイドの構成 (本 redesign 完了時に作成):

1. **概要** — 688-697 で何が変わったか
2. **trait ベース API への移行** — before/after コード例
3. **Vec コンストラクタ** — `Vec_new_i32()` → `Vec::new<i32>()`
4. **イテレータ** — `map_i32_i32(v, f)` → `v.iter().map(f).collect()`
5. **ソート** — `sort_i32(v)` → `v.sort()`
6. **IO** — `reader_from_bytes` → `Cursor::new`
7. **エラー** — `Error` enum → `AppError` + `Error` trait
8. **フォーマット** — `format_i32(n)` → `format!("{}", n)`
9. **prelude 関数の挙動変化** — thin wrapper 化による挙動は不変

---

## 5. manifest.toml 更新計画

`std/manifest.toml` の以下のセクションを更新する:

### 5.1 deprecated → removed

```toml
# 現在: stability = "deprecated", deprecated_by = "Vec::new<i32>"
# 移行後: エントリごと削除

# Vec_new_i32, Vec_new_i64, Vec_new_f64, Vec_new_String — 削除
# Vec_new_i32_with_cap, Vec_new_i64_with_cap, Vec_new_f64_with_cap — 削除
# sort_i32, sort_i64, sort_f64, sort_String — 削除
# map_i32_i32, filter_i32, fold_i32_i32, any_i32, find_i32 — 削除
```

### 5.2 新設 trait エントリ

manifest.toml に trait 定義を登録する形式を #688 で確定させる必要がある。
想定スキーマ:

```toml
[[traits]]
name = "Clone"
module = "std::core::clone"
generic_params = []
stability = "stable"

[[traits]]
name = "Iterator"
module = "std::core::iter"
generic_params = ["T"]
stability = "stable"
```

### 5.3 新設関数エントリ

```toml
# std::iter のジェネリック コンビナータ
[[functions]]
name = "collect"
kind = "prelude_wrapper"
module = "std::iter"
params = ["Iterator<T>"]
returns = "Vec<T>"
stability = "stable"

# Vec method (manifest に method 登録形式が必要、#697 で確定)
```

---

## 6. ドキュメント再生成

本 redesign の完了時に以下を再生成する:

```bash
# stdlib リファレンスページ
python3 scripts/gen/generate-docs.py

# issue index (688-697 を done に移動後)
python3 scripts/gen/generate-issue-index.py

# docs 整合性チェック
python3 scripts/check/check-docs-consistency.py
```

更新対象ドキュメント:

- `docs/stdlib/reference.md` — trait エントリ追加
- `docs/stdlib/modules/core.md` — ops/iter/fmt/clone/default セクション追加
- `docs/stdlib/modules/seq.md` → `docs/stdlib/modules/iter.md` に置換
- `docs/stdlib/modules/io.md` — Read/Write trait ベースに書き換え
- `docs/stdlib/modules/text.md` — fmt セクション拡充
- `docs/stdlib/monomorphic-deprecation.md` — 全エントリを「executed」に更新
- `docs/stdlib/expansion-policy.md` — family 分類更新 (std::seq 廃止、std::iter 新設)
- `docs/stdlib/prelude-migration.md` — thin wrapper 化を反映
- `docs/stdlib/cookbook.md` — trait ベース API のレシピ追加

---

## 7. 未決定事項 (688-697 実装中に確定)

本 redesign の設計は以下の未決定事項に依存する。各 issue の実装中に確定させる:

| 未決定事項 | 確定する issue | 影響 |
|-----------|--------------|------|
| trait import semantics (スコープ規則) | #688 | prelude の trait イポート設計 |
| メソッド構文 (`x.method()` vs `method(x)`) | #688 | 全 API 表面 |
| `Self` 型のサポート有無 | #692 | Clone/Default の trait 定義 |
| associated type の有無 | #691 | Iterator の `Item` 型表現 |
| blanket impl のサポート | #692 | Into/From, TryInto/TryFrom |
| derive 機構 (Debug/Clone 自動導出) | #696 | struct の trait impl 負担 |
| `format!` マクロのパーサー対応 | #696 | フォーマット文字列構文 |
| manifest.toml の trait/method 登録スキーマ | #688 | manifest 更新方式 |

---

## 8. 参照

- [ADR-036: Trait-based Stdlib Redesign Strategy](../adr/ADR-036-trait-stdlib-redesign.md)
- [ADR-014: Stability Labels](../adr/ADR-014-stability-labels.md)
- [Stdlib Expansion Policy](expansion-policy.md)
- [Monomorphic Deprecation Table](monomorphic-deprecation.md)
- Issue #688-#697: trait dispatch + stdlib trait 化 issue 群
- [Issue Dependency Graph](../../issues/open/dependency-graph.md)
