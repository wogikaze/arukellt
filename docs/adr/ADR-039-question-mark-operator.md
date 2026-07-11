# ADR-039: Question Mark Operator (`?`) and Error Conversion

ステータス: **PROPOSED** — #688/#692 後に実装する `?` 演算子の設計

決定日: 2026-06-26

---

## 文脈

Issue #690 は Arukellt に `?` 演算子を導入する。現在 `Result<T, E>` と
`Option<T>` は存在するが、エラー伝播に手動 `match` が必要で、エラー型変換
（`From<E1> for E2`）もない。

Rust の `?` は `match expr { Ok(v) => v, Err(e) => return Err(From::from(e)) }`
に脱糖し、`From` trait でエラー型変換を行う。本 ADR はこの脱糖と変換の
セマンティクスを決定する。

## 決定事項

### D1: `?` の脱糖規則

`expr?` は以下の規則で脱糖する:

**`Result<T, E>` の場合:**
```
match expr {
    Ok(v) => v,
    Err(e) => return Err(convert_error(e))
}
```

**`Option<T>` の場合:**
```
match expr {
    Some(v) => v,
    None => return None
}
```

`convert_error` は関数の戻り値エラー型と `expr` のエラー型が一致すれば
identity、異なれば `From::from(e)` を呼び出す（D2 参照）。

### D2: エラー型変換（`From` trait 連携）

`?` 演算子は関数の戻り値エラー型（`E_target`）と `expr` のエラー型（`E_source`）
を比較する:

1. **`E_source == E_target`**: 変換なし（identity）
2. **`E_source != E_target`**: `From<E_source> for E_target` の impl を探し、
   `From::from(e)` を呼び出す。impl が存在しない場合は型エラー。

`From` trait は #692 で定義される。#690 の実装は #692 に依存するが、
最小スコープとして `From<E> for E_target` のみを先行実装してもよい。

### D3: `Option` の `?` は変換なし

`Option<T>` の `?` はエラー型変換を伴わない（`None` をそのまま伝播）。
`Option` から `Result` への変換は `?` のスコープ外とし、明示的な
`ok_or` / `ok_or_else` 関数で行う（将来issue）。

### D4: パーサー構文

`?` は後置演算子として構文解析する:

```
postfix_expr := primary_expr postfix*
postfix := '.' ident | '(' args ')' | '[' expr ']' | '?'
```

`?` の優先順位は他の後置演算子（`.`, `()`, `[]`）と同じ左結合で、
メソッドチェーン後に適用される: `foo.bar()? + 1` は `(foo.bar()?) + 1`。

### D5: 型推論

型checker は `expr?` の型を以下のように推論する:

1. `expr` の型を推論
2. `Result<T, E>` なら `expr?` の型は `T`
3. `Option<T>` なら `expr?` の型は `T`
4. それ以外は型エラー（`?` は Result/Option のみに適用可能）

関数の戻り値型が `Result<_, E_target>` でない場合、`?` on Result は
型エラー（`?` は Result を返す関数内でのみ使用可能）。
同様に `?` on Option は Option を返す関数内でのみ使用可能。

### D6: MIR lowering

`?` の MIR lowering は早期リターン付きの match として生成する:

```
// expr? の lowering
let tmp = <lower expr>
match tmp {
    Ok(v) => v,       // continue with v
    Err(e) => {       // early return
        let converted = <convert_error(e)>
        return Err(converted)
    }
}
```

`Option` の場合は `None => return None` となる。

## 代替案と却下理由

### 代替 A: `?` なし（手動 match のまま）

`?` 演算子を導入せず、手動 match でエラー伝播を行う。

却下理由:
- エラー処理が冗長で LLM フレンドリでない
- Rust parity の前提が崩れる
- #694（Error trait ecosystem）の基盤が不足する

### 代替 B: `try!` マクロ

`?` の代わりに `try!(expr)` マクロを導入する。

却下理由:
- マクロシステムが Arukellt にまだない
- `?` の方が構文的に簡潔で Rust と一致する

### 代替 C: `?` に `From` を必須にしない

エラー型が一致しない場合も `?` を許可し、実行時に変換を試みる。

却下理由:
- 静的型安全性が損なわれる
- ADR-036 D1 の静的ディスパッチ方針に反する

## 結果

本 ADR の決定により以下が必要となる:

- [ ] parser: `?` 後置演算子の構文解析
- [ ] typechecker: `?` の型推論とエラー型変換の解決
- [ ] MIR lowering: 早期リターン付き match のコード生成
- [ ] #692: `From` trait 定義（`?` のエラー変換に必要）
- [ ] Fixture: `Result<i32, AppError>` を返す関数で `parse_i32` に `?` を使用
- [ ] Fixture: `Option` 伝播の `?`

## 参照

- ADR-036: Trait-based Stdlib Redesign Strategy（D1 静的ディスパッチ）
- Issue #688: Trait method dispatch inside generic functions
- Issue #690: `?` operator and `From<E>` error conversion
- Issue #692: `Clone` / `Default` / `From` / `Into` / `TryFrom` trait group
- Issue #694: `Error` trait and unified error type ecosystem
- Rust `?` operator: <https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator>
