# ADR-036: Trait-based Stdlib Redesign Strategy

ステータス: **DRAFT** — 688-697 完了後に実行される stdlib 再設計の戦略 ADR

決定日: 2026-06-26

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

## 決定事項

### D1: 静的ディスパッチ (monomorphization) をデフォルトとする

trait method dispatch は **単相化ベースの静的ディスパッチ** をデフォルト方式とする。

- `<T: Trait>` ジェネリック関数の trait method 呼び出しは、コンパイル時に各型引数ごと
  に特殊化され、直接 call 命令に lowering される。
- `dyn Trait` (動的ディスパッチ / vtable) は **将来 issue に切り出す**。本 redesign の
  スコープ外とする。
- 根拠:
  - コンパイラ実装が単純 (既存の monomorphization パスを拡張するだけ)
  - LLM フレンドリ (静的ディスパッチの方針を継承 — 解決規則が透明)
  - コードサイズ増大は v0 スコープでは許容可能
  - Rust も `impl Trait` / ジェネリックをデフォルトとし、`dyn` をオプトインとしている

### D2: 大胆切り替え (bold cutover) — モノモルフィック API を一括削除

688-697 完了を機に、モノモルフィック API を **deprecated 経由ではなく直接削除** する。

- 対象: `std::seq` 全体、prelude の `map_i32_i32`/`filter_i32`/`sort_i32`/`Vec_new_i32` 等
- 手続き: 各削除対象に `[breaking]` issue を作成し、移行ガイドを記載
  - provisional 機能は deprecation period 不要 (ADR-014 準拠)
- 根拠:
  - モノモルフィック API とジェネリック API の並行存在は LLM に混乱を生む
  - `monomorphic-deprecation.md` の「planned」エントリが 50+ 件あり、段階的移行は
    コストに見合わない
  - v0.1 の stdlib は provisional 扱い (ADR-014) であり、破壊的変更のコストが低い

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
Eq (既存)
 └─ PartialEq (新設, #695) — 部分順序のみ (f64 等)
 └─ Ord: Eq (新設, #695)
     └─ PartialOrd: Ord (新設, #695)

Hash (既存)
Clone (新設, #692)
 └─ Copy: Clone (新設, #692) — marker trait
Default (新設, #692)
Display (既存)
 └─ Debug (新設, #696)
From<T> (新設, #692)
Into<T>: From (新設, #692) — blanket impl
TryFrom<T> (新設, #692)
TryInto<T>: TryFrom (新設, #692) — blanket impl
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

### D5: prelude の thin wrapper 化

prelude の free function (`clone`, `eq`, `i32_to_string` 等) は **trait impl への
thin wrapper** として残す。これにより:

- 既存コードの `clone(s)` / `eq(a, b)` 呼び出しは引き続き動作する
- 新規コードは `s.clone()` / `a.eq(b)` のメソッド構文を推奨
- prelude 関数の実装は `impl Clone for String { fn clone(...) }` へ delegate する

## 代替案と却下理由

### 代替 A: 段階的移行 (非破壊)

`monomorphic-deprecation.md` の方針を継続し、deprecated + W0008 警告で誘導する。

却下理由:
- 50+ の「planned」エントリを段階実行するコストが高い
- モノモルフィック版とジェネリック版の並行存在期間が長引くほど LLM 混乱が増す
- v0.1 は provisional 扱いであり、破壊的変更のコストが本来低い

### 代替 B: vtable ベース動的ディスパッチ

trait dispatch を vtable 暗黙パラメータ方式で実装する。

却下理由:
- コンパイラ実装が複雑 (vtable 生成・渡送・call_indirect lowering)
- LLM フレンドリ方針に反する (解決規則が不透明)
- コードサイズメリットは v0 スコープでは不要
- `dyn Trait` が必要になった時点で別途追加可能 (拡張ポイントは残す)

## 結果

本 ADR の決定により以下が必要となる:

- [ ] `docs/stdlib/trait-stdlib-redesign.md` — モジュール別詳細設計doc
- [ ] 各モノモルフィック API 削除に対する `[breaking]` issue 作成
- [ ] `docs/stdlib/expansion-policy.md` の family 分類更新
- [ ] `docs/stdlib/monomorphic-deprecation.md` を「executed」ステータスに更新

## 参照

- ADR-014: Stability Labels
- Issue #688-#697: trait dispatch + stdlib trait 化 issue 群
- `docs/stdlib/expansion-policy.md`
- `docs/stdlib/monomorphic-deprecation.md`
