# Arukellt Quickstart

10分で書き始められるガイド。すべてv0 canonical styleで記述。

---

## Hello World

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

```
let x = 42              // 不変（再束縛不可）
let mut y = 0           // 可変（再束縛可能）

y = y + 1               // OK
x = x + 1               // コンパイルエラー
```

### 関数

```
fn add(a: i32, b: i32) -> i32 {
    a + b
}

let result = add(10, 20)
```

### 型

```
// プリミティブ
let n: i32 = 42
let f: f64 = 3.14
let b: bool = true
let c: char = 'A'

// 複合型
let s: String = String_from("hello")
let v: Vec[i32] = Vec_new_i32()
```

---

## Vec を使う

### 作成と操作

```
fn main() {
    // Vec作成
    let v: Vec[i32] = Vec_new_i32()
    
    // 要素追加
    push(v, 10)
    push(v, 20)
    push(v, 30)
    
    // 長さ
    print(len(v))  // 3
    
    // アクセス
    let x: Option[i32] = get(v, 0)
    match x {
        Some(val) => print(val),
        None => print("not found"),
    }
}
```

### イテレーション

```
fn print_all(v: Vec[i32]) {
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
    let v: Vec[i32] = vec_from_i32([1, 2, 3, 4, 5])
    
    // filter: 偶数のみ
    fn is_even(x: i32) -> bool { x % 2 == 0 }
    let v2 = filter_i32(v, is_even)
    
    // map: 2倍
    fn double(x: i32) -> i32 { x * 2 }
    let v3 = map_i32_i32(v2, double)
    
    print_all(v3)  // 4, 8
}
```

---

## String を使う

### 作成と操作

```
fn main() {
    // String作成
    let s1: String = String_from("hello")
    let s2: String = String_from(" world")
    
    // 連結
    let s3: String = concat(s1, s2)
    print(s3)  // "hello world"
    
    // 長さ
    print(len(s3))  // 11
    
    // スライス
    let sub: String = slice(s3, 0, 5)
    print(sub)  // "hello"
}
```

### 分割と結合

```
fn main() {
    let s: String = String_from("a,b,c")
    
    // 分割
    let parts: Vec[String] = split(s, ",")
    
    // 結合
    let joined: String = join(parts, "-")
    print(joined)  // "a-b-c"
}
```

---

## Option を使う

### 基本

```
fn find_first_even(v: Vec[i32]) -> Option[i32] {
    let mut i = 0
    while i < len(v) {
        let item = get_unchecked(v, i)
        if item % 2 == 0 {
            return Some(item)
        }
        i = i + 1
    }
    None
}

fn main() {
    let v: Vec[i32] = vec_from_i32([1, 3, 4, 5])
    let result = find_first_even(v)
    
    match result {
        Some(val) => print(val),
        None => print("not found"),
    }
}
```

---

## Result を使う

### エラー処理

```
enum ParseError {
    InvalidFormat,
    OutOfRange,
}

fn parse_positive(s: String) -> Result[i32, ParseError] {
    let n = parse_int(s)?  // ? で伝播
    
    if n < 0 {
        return Err(ParseError::OutOfRange)
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

### 基本（capability-based）

```
fn main(caps: Capabilities) -> Result[(), IOError] {
    // capability取得
    let dir: DirCap = cwd(caps)
    
    // ファイル読み込み
    let path = RelPath_from("input.txt")?  // Result<RelPath, IOError>
    let content: String = fs_read_file(dir, path)?
    
    print(content)
    
    Ok(())
}
```

### 読み込んで処理して書き込み

```
fn main(caps: Capabilities) -> Result[(), IOError] {
    let dir = cwd(caps)
    
    // 読み込み
    let path = RelPath_from("input.txt")?
    let content = fs_read_file(dir, path)?
    
    // 処理（行ごとに分割して大文字化）
    let lines: Vec[String] = split(content, "\n")
    let upper_lines = map_String_String(lines, to_upper)
    let result = join(upper_lines, "\n")
    
    // 書き込み
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
fn main(caps: Capabilities) -> Result[(), IOError] {
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
let v: Vec[i32] = Vec_new_i32()
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

| やりたいこと | 書き方 |
|-------------|--------|
| Vec作成 | `let v: Vec[i32] = Vec_new_i32()` |
| Vec追加 | `push(v, 42)` |
| Vec取得 | `get(v, 0)` |
| Vecループ | `while i < len(v)` |
| String作成 | `String_from("hello")` |
| String連結 | `concat(s1, s2)` |
| ファイル読み | `fs_read_file(dir, path)?` |
| エラー処理 | `match result { Ok(v) => ..., Err(e) => ... }` |
