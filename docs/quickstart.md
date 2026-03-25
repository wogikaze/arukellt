# Arukellt Quickstart

10分で書き始められるガイド。すべてv0 canonical styleで記述。

> **⚠️ 実装状況について**: 本ガイドのコード例は v0 設計仕様に基づく。
> 一部の例は現在の実装ではまだ動作しない。各セクションに実装状況を記載。
> 詳細は [`docs/process/v0-status.md`](process/v0-status.md) を参照。

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

> ✅ **動作確認済み** — 2 引数まで完全動作。3 引数以上は未対応。

```
fn add(a: i32, b: i32) -> i32 {
    a + b
}

let result = add(10, 20)
```

### 型

> ⚠️ **部分動作** — i32, bool, String は動作。f64/char はリテラル出力のみ。Vec は未実装。

```
// プリミティブ
let n: i32 = 42
let f: f64 = 3.14
let b: bool = true
let c: char = 'A'

// 複合型
let s: String = String_from("hello")
let v: Vec<i32> = Vec_new_i32()          // ⚠️ Vec は未実装
```

---

## Vec を使う

> 🔲 **未実装** — Vec ランタイムが存在しない。以下は設計仕様。

### 作成と操作

```
fn main() {
    let v: Vec<i32> = Vec_new_i32()

    push(v, 10)
    push(v, 20)
    push(v, 30)

    print(len(v))  // 3

    let x: Option<i32> = get(v, 0)
    match x {
        Some(val) => print(val),       // ⚠️ Some(val) ペイロード未実装
        None => print("not found"),
    }
}
```

### イテレーション

```
fn print_all(v: Vec<i32>) {
    let mut i = 0
    while i < len(v) {
        let item = get_unchecked(v, i)
        print(item)
        i = i + 1
    }
}
```

### map/filter

```
fn main() {
    let v: Vec<i32> = vec_from_i32([1, 2, 3, 4, 5])

    fn is_even(x: i32) -> bool { x % 2 == 0 }
    let v2 = filter_i32(v, is_even)         // ⚠️ closure / 高階関数は未実装

    fn double(x: i32) -> i32 { x * 2 }
    let v3 = map_i32_i32(v2, double)

    print_all(v3)  // 4, 8
}
```

---

## String を使う

> ⚠️ **部分動作** — String_from, eq, println は動作。concat/slice/split/join は未実装。

### 作成と操作

```
fn main() {
    let s1: String = String_from("hello")    // ✅ 動作
    let s2: String = String_from(" world")   // ✅ 動作

    let s3: String = concat(s1, s2)          // 🔲 未実装
    print(s3)

    print(len(s3))                           // 🔲 未実装

    let sub: String = slice(s3, 0, 5)        // 🔲 未実装
    print(sub)
}
```

### 分割と結合

> 🔲 **未実装**

```
fn main() {
    let s: String = String_from("a,b,c")

    let parts: Vec<String> = split(s, ",")   // 🔲 未実装
    let joined: String = join(parts, "-")    // 🔲 未実装
    print(joined)
}
```

---

## Option を使う

> 🔲 **未実装** — Option 型は登録済みだが、Some(val) ペイロードバリアントが未実装。

### 基本

```
fn find_first_even(v: Vec<i32>) -> Option<i32> {
    let mut i = 0
    while i < len(v) {
        let item = get_unchecked(v, i)
        if item % 2 == 0 {
            return Some(item)               // 🔲 ペイロード未実装
        }
        i = i + 1
    }
    None
}

fn main() {
    let v: Vec<i32> = vec_from_i32([1, 3, 4, 5])
    let result = find_first_even(v)

    match result {
        Some(val) => print(val),
        None => print("not found"),
    }
}
```

---

## Result を使う

> 🔲 **未実装** — Result 型は登録済みだが、Ok(val)/Err(e) ペイロードと ? 演算子が未実装。

### エラー処理

```
enum ParseError {
    InvalidFormat,
    OutOfRange,
}

fn parse_positive(s: String) -> Result<i32, ParseError> {
    let n = parse_int(s)?                    // 🔲 ? 演算子は未実装

    if n < 0 {
        return Err(ParseError::OutOfRange)   // 🔲 ペイロード未実装
    }

    Ok(n)
}

fn main() {
    let result = parse_positive(String_from("42"))

    match result {
        Ok(val) => print(val),
        Err(ParseError::InvalidFormat) => print("invalid format"),
        Err(ParseError::OutOfRange) => print("out of range"),
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
| enum | `enum Color { Red, Green, Blue }` | ✅ (unit) |
| match | `match x { 0 => ..., _ => ... }` | ✅ |
| Vec作成 | `let v: Vec<i32> = Vec_new_i32()` | 🔲 |
| Vec追加 | `push(v, 42)` | 🔲 |
| String作成 | `String_from("hello")` | ✅ |
| String連結 | `concat(s1, s2)` | 🔲 |
| ファイル読み | `fs_read_file(dir, path)?` | 🔲 |
| エラー処理 | `match result { Ok(v) => ..., Err(e) => ... }` | 🔲 |
