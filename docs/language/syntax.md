# 構文仕様

ADR-004 により trait なし、for 構文なし、メソッド構文なしの v0 仕様。

---

## 重要: 参照型について

**arukellt では struct, enum, String, Vec, [T] はすべて参照型。**

代入や関数引数渡しは参照のコピー（オブジェクト共有）。値のコピーではない。

```
struct Point { x: f64, y: f64 }

let p1 = Point { x: 1.0, y: 2.0 }
let p2 = p1  // p1 と p2 は同じオブジェクトを指す（値コピーではない）

// p1.x を変更すると p2.x も変わる（共有）
```

深いコピーが必要な場合は `clone` を使う:
```
let p2 = clone(p1)  // 別オブジェクト
```

---

## 最小例

```
fn main() {
    // 最小のエントリポイント
}
```

---

## 名前空間の階層

### Prelude（import 不要）

以下は自動で見える:

| カテゴリ | 名前 |
|---------|------|
| 型 | `Option`, `Result`, `String`, `Vec` |
| コンストラクタ | `Some`, `None`, `Ok`, `Err` |
| 基本関数 | `len`, `clone`, `panic` |
| Vec 操作 | `vec_new`, `vec_push`, `vec_pop`, `vec_get` |
| String 操作 | `concat` |
| 出力 | `println`, `print` |
| 数学 | `sqrt`, `abs`, `min`, `max` |

**注**: `Some`/`None`/`Ok`/`Err` だけは裸で書ける。他の enum は `Color::Red` のように修飾必須。

### stdlib（import 必要）

```
import vec    // vec_set, vec_with_capacity, vec_clear, ...
import string // string_slice, string_push_char, ...
import io     // 下記参照
```

### Capability 依存（main 引数から取得）

I/O は capability 経由でのみ使用可能:

```
import io

fn main(caps: io.Caps) -> Result<(), io.Error> {
    io.println(caps, "Hello")           // stdout
    let content = io.read_file(caps, "data.txt")?  // ファイル読み取り
    Ok(())
}
```

`io.Caps` は `main` の引数として渡される。純粋関数からは I/O できない。

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

// v0 ではフィールド更新なし（v1 で追加予定）
// 更新が必要な場合は新しい struct を作成
fn point_move(p: Point, dx: f64, dy: f64) -> Point {
    Point { x: p.x + dx, y: p.y + dy }
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

### 関数呼び出しと可変長配列の操作

```
let result = foo(1, 2)

// String（不変）
let s = "hello"
let s2 = concat(s, " world")
let length = len(s)

// Vec（可変）
let v = vec_new()
vec_push(v, 42)      // v に要素を追加
vec_push(v, 100)
let first = vec_get(v, 0)   // Some(42)
let last = vec_pop(v)       // Some(100)
```

**重要**: Vec は参照型で可変操作可能。`vec_push` は v 自体を変更する（新しい Vec を返さない）。

### フィールドアクセスと参照セマンティクス

```
let p = point_new(1.0, 2.0)
let d = point_distance(p, point_new(0.0, 0.0))
let px = p.x  // フィールドアクセス
```

注: struct/enum/String/Vec/[T] は参照型。代入や関数引数渡しは参照のコピー（オブジェクト共有）。

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
fn fallible() -> Result<i32, MyError> {
    let x = may_fail()?  // Err なら即 return
    Ok(x + 1)
}
```

**v0 制約**: エラー型が一致する場合のみ使用可能。自動変換なし。

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

### 変数パターン（catch-all）

```
match x {
    n => println(n),  // n に束縛（catch-all、最後のアームで使用）
}
```

**注意**: 変数パターンはすべての値にマッチする。他のアームを死なせないよう、最後のアームでのみ使用。

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

v1 で追加予定の機能は `docs/language/syntax-v1-preview.md` を参照。

- メソッド構文（`impl`）
- 構造体パターン
- ガード
- or-pattern
- match でのタプルパターン
- for ループ
- 演算子オーバーロード

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
- `docs/compiler/pipeline.md`: パーサの AST 定義
