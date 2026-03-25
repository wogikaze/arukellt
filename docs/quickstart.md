# Arukellt Quickstart

10分で書き始められるガイド。すべてv0 canonical styleで記述。

> **⚠️ 実装状況について**: 本ガイドのコード例は v0 設計仕様に基づく。
> 大部分の例は現在の実装で動作する（124/124 fixture テスト pass）。
> ファイル I/O 関連の例のみ未実装。各セクションに実装状況を記載。

---

## Hello World

> ✅ **動作確認済み**

```
fn main() {
    print("Hello, world!")
}
```

実行:
```bash
arukellt run hello.ark
```

---

## 基本構文

### 変数

> ✅ **動作確認済み** — let, let mut, 再代入

```
let x = 42              // 不変（再束縛不可）
let mut y = 0           // 可変（再束縛可能）

y = y + 1               // OK
x = x + 1               // コンパイルエラー
```

### 関数

> ✅ **動作確認済み** — 多引数・再帰・ジェネリック・高階関数動作。

```
fn add(a: i32, b: i32) -> i32 {
    a + b
}

let result = add(10, 20)
```

### 型

> ✅ **動作確認済み** — i32, i64, f64, bool, String, Vec<i32> 動作。

```
// プリミティブ
let n: i32 = 42
let f: f64 = 3.14
let b: bool = true
let big: i64 = 1000000

// 複合型
let s: String = String_from("hello")
let v: Vec<i32> = Vec_new_i32()
```

---

## Vec を使う

> ✅ **動作確認済み** — Vec_new_i32, push, pop, get, len, sort_i32, map/filter/fold 動作。

### 作成と操作

```
fn main() {
    let v: Vec<i32> = Vec_new_i32()

    push(v, 10)
    push(v, 20)
    push(v, 30)

    println(i32_to_string(len(v)))  // 3

    let x: i32 = get(v, 0)
    println(i32_to_string(x))      // 10
}
```

### イテレーション

```
fn print_all(v: Vec<i32>) {
    let mut i = 0
    while i < len(v) {
        let item = get(v, i)
        println(i32_to_string(item))
        i = i + 1
    }
}
```

### map/filter

```
fn main() {
    let v: Vec<i32> = Vec_new_i32()
    push(v, 1)
    push(v, 2)
    push(v, 3)
    push(v, 4)
    push(v, 5)

    fn is_even(x: i32) -> bool { x % 2 == 0 }
    let v2 = filter_i32(v, is_even)

    fn double(x: i32) -> i32 { x * 2 }
    let v3 = map_i32_i32(v2, double)

    print_all(v3)  // 4, 8
}
```

---

## String を使う

> ✅ **動作確認済み** — String_from, eq, concat, split, join, slice, println 動作。

### 作成と操作

```
fn main() {
    let s1: String = String_from("hello")
    let s2: String = String_from(" world")

    let s3: String = concat(s1, s2)
    println(s3)                              // hello world

    let sub: String = slice(s3, 0, 5)
    println(sub)                             // hello
}
```

### 分割と結合

> ✅ **動作確認済み**

```
fn main() {
    let s: String = String_from("a,b,c")

    let parts: Vec<String> = split(s, ",")
    let joined: String = join(parts, "-")
    println(joined)                          // a-b-c
}
```

---

## Option を使う

> ✅ **動作確認済み** — Some/None ペイロード、match binding、unwrap/is_some/is_none 動作。

### 基本

```
fn find_first_even(v: Vec<i32>) -> Option<i32> {
    let mut i = 0
    while i < len(v) {
        let item = get(v, i)
        if item % 2 == 0 {
            return Some(item)
        }
        i = i + 1
    }
    None
}

fn main() {
    let v: Vec<i32> = Vec_new_i32()
    push(v, 1)
    push(v, 3)
    push(v, 4)
    push(v, 5)
    let result = find_first_even(v)

    match result {
        Some(val) => println(i32_to_string(val)),
        None => println("not found"),
    }
}
```

---

## Result を使う

> ✅ **動作確認済み** — Ok/Err ペイロード、? 演算子、match binding 動作。

### エラー処理

```
enum ParseError {
    InvalidFormat,
    OutOfRange,
}

fn parse_positive(s: String) -> Result<i32, ParseError> {
    let n = parse_i32(s)

    if n < 0 {
        return Err(OutOfRange)
    }

    Ok(n)
}

fn main() {
    let result = parse_positive(String_from("42"))

    match result {
        Ok(val) => println(i32_to_string(val)),
        Err(InvalidFormat) => println("invalid format"),
        Err(OutOfRange) => println("out of range"),
    }
}
```

---

## ファイル読み書き

> 🔲 **未実装** — capability-based I/O は設計済みだが全く実装されていない。

### 基本（capability-based）

```
fn main(caps: Capabilities) -> Result<(), IOError> {
    let dir: DirCap = cwd(caps)

    let path = RelPath_from("input.txt")?
    let content: String = fs_read_file(dir, path)?

    print(content)

    Ok(())
}
```

### 読み込んで処理して書き込み

```
fn main(caps: Capabilities) -> Result<(), IOError> {
    let dir = cwd(caps)

    let path = RelPath_from("input.txt")?
    let content = fs_read_file(dir, path)?

    let lines: Vec<String> = split(content, "\n")
    let upper_lines = map_String_String(lines, to_upper)
    let result = join(upper_lines, "\n")

    fs_write_file(dir, RelPath_from("output.txt"), result)?

    Ok(())
}

fn to_upper(s: String) -> String {
    // 実装は省略
    s
}
```

### エラーハンドリング

```
fn main(caps: Capabilities) -> Result<(), IOError> {
    let dir = cwd(caps)
    let path = RelPath_from("data.txt")?

    let result = fs_read_file(dir, path)

    match result {
        Ok(content) => {
            print(content)
            Ok(())
        }
        Err(IOError::NotFound) => {
            print("file not found, creating default")
            fs_write_file(dir, path, "default content")?
            Ok(())
        }
        Err(e) => Err(e),
    }
}
```

---

## 完全な例：単語カウンター

```
fn main(caps: Capabilities) -> Result[(), IOError] {
    let dir = cwd(caps)
    
    // ファイル読み込み
    let content = fs_read_file(dir, RelPath_from("input.txt"))?
    
    // 単語に分割
    let words: Vec[String] = split(content, " ")
    
    // カウント
    let count = len(words)
    
    // 結果を出力
    let message = concat(String_from("Word count: "), int_to_string(count))
    print(message)
    
    Ok(())
}
```

---

## よくあるエラーと修正

### エラー1: メソッド構文

```
// ❌ 間違い
v.push(42)

// ✅ 正しい
push(v, 42)
```

### エラー2: for ループ

```
// ❌ 間違い
for x in items {
    print(x)
}

// ✅ 正しい
let mut i = 0
while i < len(items) {
    let x = get_unchecked(items, i)
    print(x)
    i = i + 1
}
```

### エラー3: 型推論失敗

```
// ❌ 間違い
let v = Vec::new()

// ✅ 正しい
let v: Vec<i32> = Vec_new_i32()
```

### エラー4: Result unwrap忘れ

```
// ❌ 間違い
let content = fs_read_file(dir, path)
print(content)  // エラー: Result型をprintできない

// ✅ 正しい
let content = fs_read_file(dir, path)?
print(content)
```

---

## 次のステップ

- **詳細な構文**: `docs/language/syntax.md`
- **型システム**: `docs/language/type-system.md`
- **標準ライブラリ**: `docs/stdlib/`
- **Cookbook**: `docs/stdlib/cookbook.md`
- **統合仕様**: `docs/spec/v0-unified-spec.md`

---

## チートシート

| やりたいこと | 書き方 | 実装状況 |
|-------------|--------|---------|
| Hello World | `println("Hello, world!")` | ✅ |
| 変数束縛 | `let x: i32 = 42` | ✅ |
| 関数定義 | `fn add(a: i32, b: i32) -> i32 { a + b }` | ✅ |
| 構造体 | `struct Point { x: i32, y: i32 }` | ✅ |
| enum | `enum Color { Red, Green, Blue }` | ✅ |
| enum payload | `Some(val)`, `Ok(val)`, `Err(e)` | ✅ |
| match | `match x { 0 => ..., _ => ... }` | ✅ |
| Vec作成 | `let v: Vec<i32> = Vec_new_i32()` | ✅ |
| Vec追加 | `push(v, 42)` | ✅ |
| String作成 | `String_from("hello")` | ✅ |
| String連結 | `concat(s1, s2)` | ✅ |
| String分割 | `split(s, ",")` | ✅ |
| Option | `unwrap(opt)`, `is_some(opt)` | ✅ |
| クロージャ | `fn f(x: i32) -> i32 { x + 1 }` + 高階関数 | ✅ |
| ? 演算子 | `let val = risky_fn()?` | ✅ |
| ファイル読み | `fs_read_file(dir, path)?` | 🔲 |
| エラー処理 | `match result { Ok(v) => ..., Err(e) => ... }` | ✅ |
