# 型システム

ADR-002 により Wasm GC 前提、ADR-003 により制限付き monomorphization を採用。

---

## プリミティブ型

| 型 | 説明 | Wasm 表現 |
|----|------|----------|
| `i32` | 32ビット符号付き整数 | `i32` |
| `i64` | 64ビット符号付き整数 | `i64` |
| `f32` | 32ビット浮動小数点 | `f32` |
| `f64` | 64ビット浮動小数点 | `f64` |
| `bool` | 真偽値 | `i32` (0 or 1) |
| `char` | Unicode スカラー値 | `i32` |
| `()` | ユニット型 | なし（0 値） |

### 整数リテラル

```
let a: i32 = 42
let b: i64 = 1000000000000
let c = 0xFF        // 16進数
let d = 0b1010      // 2進数
let e = 1_000_000   // 区切り
```

### 浮動小数点リテラル

```
let x: f64 = 3.14
let y: f32 = 2.5f32  // 明示的な f32
let z = 1e10         // 指数表記
```

---

## 複合型

### struct

```
struct Point {
    x: f64,
    y: f64,
}

// 生成
let p = Point { x: 1.0, y: 2.0 }

// フィールドアクセス
let px = p.x
```

### enum

```
enum Color {
    Red,
    Green,
    Blue,
    Rgb(i32, i32, i32),
}

// 生成
let c = Color::Rgb(255, 128, 0)

// パターンマッチ
match c {
    Color::Red => "red",
    Color::Rgb(r, g, b) => "rgb",
    _ => "other",
}
```

### tuple

```
let pair: (i32, String) = (42, "hello")
let (x, y) = pair  // 分解
```

### 配列（固定長）

```
let arr: [i32; 3] = [1, 2, 3]
let first = arr[0]
```

v0 では配列は固定長のみ。可変長配列は `Vec<T>` を使用。

### スライス

```
let arr: [i32; 3] = [1, 2, 3]
let slice: [i32] = as_slice(arr)  // import vec が必要
let length = len(slice)
```

スライス `[T]` は配列または Vec への不変ビュー（参照型）。

**v0 での配列/Vec/スライスの使い分け:**
- 固定サイズ → `[i32; 3]` 配列
- 可変長 → `Vec<i32>` (`vec_new()`, `vec_push` 等)
- 部分参照 → `[i32]` スライス (`as_slice` で作成)

**注意**: 空配列 `[]` は型注釈が必須。実務では `vec_new()` を推奨。

```
// 非推奨: 空配列（型注釈必須、用途限定的）
let empty: [i32; 0] = []

// 推奨: 空 Vec
let v = vec_new()  // 型推論。必要なら let v: Vec<i32> = vec_new()
```

---

## 参照型

GC ヒープ上に配置される型。代入は参照のコピー（オブジェクト共有）。

| 型 | 説明 |
|----|------|
| `String` | 不変の UTF-8 文字列 |
| `Vec<T>` | 可変長配列 |
| `[T]` | 配列スライス（不変ビュー） |
| struct | ユーザー定義構造体 |
| enum | ユーザー定義列挙型 |

### 参照のコピーセマンティクス

```
let s1: String = "hello"
let s2 = s1  // 参照のコピー（s1 と s2 は同じオブジェクト）
```

GC 前提のため、所有権・借用の概念はない。

---

## ジェネリクス

### ジェネリック関数

```
fn identity<T>(x: T) -> T {
    x
}

// 使用
let a = identity(42)         // T = i32（推論）
let b = identity<String>(s)  // 明示指定
```

### 制限（v0）

**重要**: ネストした型引数は禁止。以下は全て ❌：
- `Vec<Vec<i32>>`
- `Vec<Option<T>>`
- `Result<Vec<String>, E>`
- `Option<Result<T, E>>`

- 型パラメータは 2 個まで
- ユーザー定義 generic struct は後回し（stdlib の `Option<T>`, `Result<T, E>`, `Vec<T>` のみ）

---

## 型推論

### 双方向型推論（Bidirectional Type Inference）

synthesis（合成）と checking（検査）の組み合わせ:

```
// synthesis: 式から型を導出
let x = 42        // x: i32

// checking: 期待される型に合わせて検査
let y: i64 = 42   // 42 を i64 としてチェック

// 関数呼び出しでの推論
fn foo(x: i32) -> i64 {
    x as i64
}
let z = foo(42)   // 42 は i32、z は i64
```

### 推論できないケース

```
// エラー: 型が確定しない
let arr = []  // 何の配列？ → 型注釈が必要だが、実務では vec_new() を使う

// 型注釈で解決可能（だが非推奨）
let arr: [i32; 0] = []  // 固定長配列

// 推奨: Vec を使う
let v = vec_new()
vec_push(v, 42)
```

---

## Option と Result

### Option<T>

```
enum Option<T> {
    None,
    Some(T),
}

let x: Option<i32> = Some(42)
let y: Option<i32> = None
```

### Result<T, E>

```
enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn divide(a: i64, b: i64) -> Result<i64, String> {
    if b == 0 {
        Err("division by zero")
    } else {
        Ok(a / b)
    }
}
```

---

## 型の等価性

### 名前的等価（Nominal Equality）

同じ名前の型のみ等しい。構造が同じでも名前が違えば異なる型。

```
struct Point1 { x: f64, y: f64 }
struct Point2 { x: f64, y: f64 }

// Point1 と Point2 は異なる型
```

### ジェネリック型の等価性

型引数が同じなら等しい。

```
Option<i32> == Option<i32>  // 等しい
Option<i32> != Option<i64>  // 異なる
```

---

## 関連

- `docs/language/memory-model.md`: 型の Wasm 表現
- `docs/language/syntax.md`: 型の構文
- ADR-003: ジェネリクス戦略
