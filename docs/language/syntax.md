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

## Prelude（自動で見える名前）

以下の名前は import なしで使用可能:

```
// 型
Option, Some, None
Result, Ok, Err
String, Vec

// 組み込み関数
len, println, print, panic

// 数学関数
sqrt, abs, min, max
```

その他の機能は明示的な `import` が必要。

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
fn internal_fn() {
    // ...
}

// pub をつけると外部から参照可能
pub fn public_fn() {
    // ...
}

pub struct PublicStruct {
    field: i32,
}
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

### フィールドアクセスと参照セマンティクス

```
let p = point_new(1.0, 2.0)
let d = point_distance(p, point_new(0.0, 0.0))
let px = p.x  // フィールドアクセス
```

注: struct/enum/String/Vec は参照型。代入や関数引数渡しは参照のコピー（オブジェクト共有）。

```
let p1 = point_new(1.0, 2.0)
let p2 = p1  // p1 と p2 は同じオブジェクトを参照
```

### 演算子

v0 では演算子オーバーロードなし。以下は組み込み演算子で、適用可能型が固定されている。

```
// 算術（i32, i64, f32, f64）
a + b    a - b    a * b    a / b    a % b

// 比較: 等値（全プリミティブ型, String）
// String の == は内容比較（参照同一性ではない）
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

タプルパターンは `let` での分解のみ可。`match` でのタプルパターンは v1。

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

### タプル分解（let のみ）

```
let (a, b) = (1, 2)
```

注: `match` でのタプルパターンは v1。

### enum パターン

```
match option {
    Some(value) => value,
    None => default,
}
```

---

## v1 以降の機能

以下は v1 で追加予定。v0 では使用不可。

### 構造体パターン

```
match point {
    Point { x, y } => x + y,
}
```

### ガード

```
match x {
    n if n > 0 => "positive",
    n if n < 0 => "negative",
    _ => "zero",
}
```

### or-pattern

```
match x {
    1 | 2 | 3 => "small",
    _ => "large",
}
```

### match でのタプルパターン

```
match pair {
    (0, y) => y,
    (x, 0) => x,
    (x, y) => x + y,
}
```

### メソッド構文

```
impl Point {
    fn distance(self, other: Point) -> f64 {
        // ...
    }
}

let d = p.distance(other)
```

---

## コメント

```
// 行コメント

/* ブロックコメント
   複数行 */

/// ドキュメントコメント
/// この関数は...
fn documented_fn() {
    // 実装
}
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
