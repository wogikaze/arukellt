# 構文仕様

> **Current-first**: いま動く構文の確認は [../current-state.md](../current-state.md) を基準にしてください。

このページは、現行ブランチで把握しやすい構文の要約です。
設計-only の capability I/O や古い v0 制約の説明は落とし、現在よく使う書き方を優先しています。

## エントリポイント

```ark
fn main() {
}
```

```ark
fn main() -> i32 {
    0
}
```

## import

```ark
import math
import utils as u
```

- 基本形は `import <name>`
- alias 付きは `import <name> as <alias>`
- qualified access は `math::add(1, 2)` の形を使います
- capability 引数付き `main(caps: ...)` は現行の一般的 API ではありません

## 変数と関数

```ark
let x = 42
let mut y = 0

y = y + 1

fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

## 型

```ark
let n: i32 = 42
let big: i64 = 1000000
let f: f64 = 3.14
let b: bool = true
let c: char = 'a'
let s: String = String_from("hello")
```

## struct / enum

```ark
struct Point {
    x: i32,
    y: i32,
}

enum Shape {
    Circle(i32),
    Rect(i32, i32),
}
```

```ark
let p = Point { x: 1, y: 2 }
let x = p.x
```

```ark
let s = Shape::Rect(10, 20)
match s {
    Shape::Circle(r) => println(i32_to_string(r)),
    Shape::Rect(w, h) => println(i32_to_string(w * h)),
}
```

## 制御構文

### if

```ark
let label = if x > 0 {
    String_from("positive")
} else {
    String_from("other")
}
```

### while / loop

```ark
while x < 10 {
    x = x + 1
}

loop {
    if done {
        break
    }
}
```

### for

```ark
for i in 0..10 {
    println(i32_to_string(i))
}

for item in values(v) {
    println(i32_to_string(item))
}
```

## 関数呼び出しスタイル

共通で安全なのは関数呼び出し形式です。

```ark
push(v, 42)
let n = len(v)
let s2 = concat(s1, s2)
```

このブランチでは v1 のメソッド構文もありますが、まずは上の形を基準にするのが安全です。

## match

```ark
match value {
    0 => String_from("zero"),
    1 => String_from("one"),
    _ => String_from("other"),
}
```

```ark
match opt {
    Some(x) => println(i32_to_string(x)),
    None => println(String_from("none")),
}
```

## Result と `?`

```ark
fn parse_positive(s: String) -> Result<i32, String> {
    let n = parse_i32(s)?
    if n < 0 {
        return Err(String_from("negative"))
    }
    Ok(n)
}
```

## v1 実装済み構文

このブランチでは次も入っています。

- `trait`
- `impl`
- メソッド呼び出し
- 演算子オーバーロード
- match guard / or-pattern / struct pattern
- nested generics

詳細は [syntax-v1-preview.md](syntax-v1-preview.md) を参照してください。

## 関連

- [type-system.md](type-system.md)
- [error-handling.md](error-handling.md)
- [../quickstart.md](../quickstart.md)
- [../current-state.md](../current-state.md)
