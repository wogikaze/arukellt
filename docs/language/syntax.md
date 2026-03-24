# 構文仕様

ADR-004 により trait なし、for 構文なし、メソッド構文なしの v0 仕様。

---

## 最小例

```
fn main() {
    // 最小のエントリポイント
}
```

---

## キーワード

### 予約キーワード

```
fn       struct   enum     let      mut
if       else     match    while    loop
break    continue return   pub      import
as       true     false
```

### v1 以降の予約

```
trait    impl     for      in       async
await    dyn      where    type     const
unsafe   extern   use      mod      super
self     Self
```

---

## プログラム構造

### モジュール

```
// ファイル: src/main.ark
import math
import util.string as ustr

fn main() {
    let result = math.add(1, 2)
}
```

- 1 ファイル = 1 モジュール
- モジュール名はファイルパスから決定
- 循環 import はコンパイルエラー

### 公開範囲

```
// pub がないものは内部のみ
fn internal_fn() { ... }

// pub をつけると外部から参照可能
pub fn public_fn() { ... }

pub struct PublicStruct { ... }
```

---

## 宣言

### 関数宣言

```
fn function_name(param1: Type1, param2: Type2) -> ReturnType {
    // body
}

// 戻り値なし
fn no_return(x: i32) {
    // 暗黙の () を返す
}

// ジェネリック関数
fn identity<T>(x: T) -> T {
    x
}
```

### 構造体宣言

```
struct Point {
    x: f64,
    y: f64,
}

// 関連関数（v0 ではモジュールレベル関数として定義）
fn point_new(x: f64, y: f64) -> Point {
    Point { x: x, y: y }
}

fn point_distance(p: Point, other: Point) -> f64 {
    let dx = p.x - other.x
    let dy = p.y - other.y
    sqrt(dx * dx + dy * dy)
}
```

### 列挙型宣言

```
enum Option<T> {
    None,
    Some(T),
}

enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
}
```

---

## 式

### リテラル

```
42          // i32
3.14        // f64
true        // bool
'a'         // char
"hello"     // String
```

### 変数束縛

```
let x = 42
let y: i64 = 100
let mut z = 0  // 可変
z = z + 1
```

### 関数呼び出し

```
let result = foo(1, 2)
let s = "hello"
let length = len(s)
```

### フィールドアクセス

```
let p = point_new(1.0, 2.0)
let d = point_distance(p, point_new(0.0, 0.0))
let px = p.x  // フィールドアクセス
```

### 演算子

v0 では演算子オーバーロードなし。以下は組み込み演算子で、適用可能型が固定されている。

```
// 算術（i32, i64, f32, f64）
a + b    a - b    a * b    a / b    a % b

// 比較: 等値（全プリミティブ型, String）
a == b   a != b

// 比較: 順序（数値型, char）
a < b    a <= b   a > b   a >= b

// 論理（bool のみ）
a && b   a || b   !a

// ビット演算（i32, i64 のみ）
a & b    a | b    a ^ b    ~a    a << n    a >> n
```

注: 論理否定は `!a`（bool）、ビット否定は `~a`（整数）で区別する。

### ブロック式

```
let x = {
    let a = 1
    let b = 2
    a + b  // 最後の式がブロックの値
}
// x == 3
```

---

## 文

### if 式

```
let result = if x > 0 {
    "positive"
} else if x < 0 {
    "negative"
} else {
    "zero"
}
```

### match 式

v0 の match は以下のパターンのみサポート:
- リテラルパターン
- enum variant パターン
- ワイルドカード `_`
- 変数束縛

```
match value {
    0 => "zero",
    1 => "one",
    _ => "other",
}

// enum のマッチ
match option {
    Some(x) => x,
    None => 0,
}
```

v1 以降のパターン（guard, or-pattern, struct destructuring）:
```
// v1: guard
match x {
    n if n < 0 => "negative",
    _ => "non-negative",
}

// v1: or-pattern
match x {
    1 | 2 => "one or two",
    _ => "other",
}

// v1: struct destructuring
match point {
    Point { x: 0, y } => "on y-axis",
    Point { x, y } => "elsewhere",
}
```

### while ループ

```
let mut i = 0
while i < 10 {
    i = i + 1
}
```

### loop ループ

```
loop {
    if condition {
        break
    }
}

// break with value
let result = loop {
    if found {
        break value
    }
}
```

### break / continue

```
while true {
    if done {
        break
    }
    if skip {
        continue
    }
    // process
}
```

### return

```
fn early_return(x: i32) -> i32 {
    if x < 0 {
        return 0
    }
    x * 2
}
```

### ? 演算子

```
fn fallible() -> Result<i32, Error> {
    let x = may_fail()?  // Err なら即 return
    Ok(x + 1)
}
```

---

## パターン

v0 でサポートするパターン:

### リテラルパターン

```
match x {
    0 => "zero",
    1 => "one",
    _ => "other",
}
```

### 変数パターン

```
match x {
    n => println(n),  // n に束縛
}
```

### ワイルドカードパターン

```
match x {
    _ => "anything",
}
```

### タプルパターン

```
let (a, b) = (1, 2)
```

### enum パターン

```
match option {
    Some(value) => value,
    None => default,
}
```

---

## v1 以降のパターン

以下のパターンは v1 で追加予定:

### 構造体パターン（v1）

```
match point {
    Point { x, y } => x + y,
}
```

### ガード（v1）

```
match x {
    n if n > 0 => "positive",
    n if n < 0 => "negative",
    _ => "zero",
}
```

### or-pattern（v1）

```
match x {
    1 | 2 | 3 => "small",
    _ => "large",
}
```

---

## コメント

```
// 行コメント

/* ブロックコメント
   複数行 */

/// ドキュメントコメント
/// この関数は...
fn documented_fn() { ... }
```

---

## shebang

```
#!/usr/bin/env arukellt run
fn main() {
    // ...
}
```

---

## エントリポイントの形式

### 最小形式

```
fn main() {
    // 戻り値なし
}
```

### 終了コード付き

```
fn main() -> i32 {
    0  // 成功
}
```

### Capability 付き（アプリケーション境界）

```
fn main(caps: Capabilities) -> Result<(), AppError> {
    // caps を通じて I/O にアクセス
    Ok(())
}
```

---

## 関連

- `docs/language/type-system.md`: 型システム詳細
- `docs/compiler-phases.md`: パーサの AST 定義
