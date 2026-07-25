# ADR-039: `?` の Option 対応とエラー型変換

ステータス: **ACCEPTED** — `Option` `?` と Result 異種 Err の `From` 変換を採択する

提案日: 2026-06-26  
決定日: 2026-07-25  
改訂日: 2026-07-11 — 実装済み Result `?` を前提化し、未決範囲を Option / From に限定  
改訂日: 2026-07-25 — ACCEPTED。理想契約と living 実装ギャップを分離

---

## 文脈

**前提（採択前から実装済み・stable）:**

- `expr?` は `Result<T, E>` に対し、同一エラー型 `E` の早期伝播として動作する
- normative: `docs/language/spec.md`、`docs/language/error-handling.md`
- maturity: Try Operator = stable（`docs/language/maturity-matrix.md`）

**本 ADR が固定する拡張:**

1. `Option<T>` に対する `?`
2. 異なるエラー型間の `From<E_source> for E_target` 変換
3. trait 解決を含む型検査規則（#688 / #692 連携）

基本的な Result `?` のパーサー構文・同一型 lowering は前提であり、本 ADR の新設対象ではない。

---

## 決定

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

1. canonical `SemanticTraitId::From` の impl を解決する（import 不要）
2. `return Err(From::from(e))` に脱糖する
3. impl が無ければ型エラー

`expr?` は language syntax desugaring であり、method-call の trait import 規則に従わない
（[RFC-004](../rfcs/004-trait-expressiveness.md) §6）。
名前文字列 `"From"` だけでは解決しない。

`From` trait は #692。

### D3: 型検査（拡張分）

| 適用 | 囲む関数の戻り値 | `expr?` の型 |
|------|------------------|--------------|
| `Result<T, E>`（現行） | `Result<_, E_target>` | `T`（`E`→`E_target` は D2） |
| `Option<T>`（本決定） | `Option<_>` | `T` |
| それ以外 | — | 型エラー |

### D4: MIR lowering（拡張分）

Option / From 変換付き Result は、早期リターン付き match として生成する。
同一型 Result `?` の既存 lowering は変更しない（前提）。

---

## 現行実装ギャップ（理想形ではない）

次は決定の一部ではなく、living implementation の一時状態である。進捗は issue を正とする。

- D2 の living 解決は、登録済み associated method `E_target::from` を MIR が選ぶ形である
  （`SignatureEntry.trait_id` / `SemanticTraitId::From` 配線は #839）
- `wasm32-gc` では異種 Err の From 変換 lowering が未接続（同一型伝播と Option `?` は対象）
- `arukellt run` が WASI P2 component adapter で落ちる場合は #686 / #810 側であり、本 ADR の対象外

---

## 代替案と却下理由

| 案 | 結果 |
|----|------|
| Option `?` を入れず手動 match のまま | 却下（冗長・Rust parity 低下） |
| `try!` マクロ | 却下（マクロ未整備、`?` の方が簡潔） |
| From なしで異種 Err を実行時変換 | 却下（静的型安全性・ADR-036 に反する） |

---

## 参照

- 現行 Result / Option `?`: `docs/language/spec.md` / `error-handling.md`
- fixture: `tests/fixtures/question_mark/`
- ADR-036、Issue #688 / #690 / #692 / #694 / #839
- Rust `?`: <https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator>
