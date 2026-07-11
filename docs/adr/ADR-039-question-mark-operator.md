# ADR-039: `?` の Option 対応とエラー型変換

ステータス: **PROPOSED** — 既存の Result `?` を前提に、Option / From 拡張を提案

提案日: 2026-06-26  
改訂日: 2026-07-11 — 実装済み Result `?` を前提化し、未決範囲を Option / From に限定

---

## 文脈

**現行（実装済み・stable）:**

- `expr?` は `Result<T, E>` に対し、同一エラー型 `E` の早期伝播として動作する
- normative: `docs/language/spec.md`、`docs/language/error-handling.md`
- maturity: Try Operator = stable（`docs/language/maturity-matrix.md`）
- fixture 例: `question_mark.ark` 等

**本 ADR が扱う未決範囲:**

1. `Option<T>` に対する `?`
2. 異なるエラー型間の `From<E_source> for E_target` 変換
3. trait 解決を含む型検査規則（#688 / #692 連携）

基本的な Result `?` のパーサー構文・同一型 lowering は**前提**であり、本 ADR の提案対象ではない。

---

## 提案する決定

### D1: `Option<T>` の `?`

```
match expr {
    Some(v) => v,
    None => return None
}
```

- エラー型変換は伴わない（`None` をそのまま伝播）
- `Option` → `Result` 変換は `?` のスコープ外（明示の `ok_or` / `ok_or_else`）
- 囲む関数の戻り値は `Option<_>` であること

### D2: Result の異種エラー変換（`From`）

同一エラー型の Result `?` は現行どおり identity。

`E_source != E_target` のとき:

1. `From<E_source> for E_target` の impl を解決する
2. `return Err(From::from(e))` に脱糖する
3. impl が無ければ型エラー

`From` trait は #692。最小スコープとして必要な `From` impl のみ先行してもよい。

### D3: 型検査（拡張分）

| 適用 | 囲む関数の戻り値 | `expr?` の型 |
|------|------------------|--------------|
| `Result<T, E>`（現行） | `Result<_, E_target>` | `T`（`E`→`E_target` は D2） |
| `Option<T>`（本提案） | `Option<_>` | `T` |
| それ以外 | — | 型エラー |

### D4: MIR lowering（拡張分）

Option / From 変換付き Result は、早期リターン付き match として生成する。
同一型 Result `?` の既存 lowering は変更しない（前提）。

---

## 代替案と却下理由

| 案 | 結果 |
|----|------|
| Option `?` を入れず手動 match のまま | 却下（冗長・Rust parity 低下） |
| `try!` マクロ | 却下（マクロ未整備、`?` の方が簡潔） |
| From なしで異種 Err を実行時変換 | 却下（静的型安全性・ADR-036 に反する） |

---

## 結果（実装計画へ）

作業チェックリストは issue / plan に置く（本 ADR に進捗ダッシュボードを残さない）:

- typechecker: Option `?` と From 解決
- MIR: Option / 変換付き Result の early-return match
- #692: `From` trait
- fixture: Option 伝播、異種 Err + From

---

## 参照

- 現行 Result `?`: `docs/language/spec.md` / `error-handling.md`
- ADR-036、Issue #688 / #690 / #692 / #694
- Rust `?`: https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator
