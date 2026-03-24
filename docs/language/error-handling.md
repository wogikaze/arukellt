# エラー処理

## 方針（確定）

- 例外なし
- `Result<T, E>` ベース
- panic は「回復不能なバグ」専用。通常のエラーフローには使わない
- null なし（`Option<T>` で代替）

---

## Result の基本形

```
fn divide(a: f64, b: f64) -> Result<f64, DivideError> {
    if b == 0.0 {
        Err(DivideError::DivByZero)
    } else {
        Ok(a / b)
    }
}
```

エラー型 E はユーザー定義の enum を使う。標準エラー型の階層は持たない（v0）。

---

## panic

回復不能なバグのみ。通常フローに使うのは禁止。

```
fn safe_get(v: Vec<i32>, i: i32) -> i32 {
    if i < 0 {
        panic("negative index")
    }
    if i >= len(v) {
        panic("index out of bounds")
    }
    // 境界チェック済みなので unwrap は安全
    match vec_get(v, i) {
        Some(val) => val,
        None => panic("unreachable"),
    }
}
```

---

## エラー型の設計指針

各モジュールが自分で enum を定義する。標準エラー型の継承階層は持たない。

```
import io

enum AppError {
    Io(io.Error),
    Parse,
    NotFound,
}
```

手動ラップ。v0 では `?` の自動変換なし。

---

## Option と Result の使い分け

| 型 | 使う場面 |
|----|---------|
| `Option<T>` | 値がない、が正常状態。「見つからなかった」等 |
| `Result<T, E>` | 失敗が例外的状態。I/O・パース等 |

---

## 未決定事項

| 事項 | 依存 ADR |
|------|---------|
| `?` のエラー型自動変換（From trait） | ADR-004 |
| panic のキャッチ機構（unwind vs abort） | ADR-002 |
| `assert!` の構文 | syntax-principles.md |
