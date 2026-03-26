# Core API Cookbook

v0の標準的な書き方パターン集。**これが正解**。v1 ではメソッド構文・for ループ・演算子オーバーロードも使用可能（以下「禁止パターン」参照）。

---

## 原則

1. **関数呼び出し形式（v0/v1 共通）**: `push(v, x)` は常に有効
2. **メソッド構文（v1 M4/M5 以降）**: `v.push(x)` も使用可能
3. **for ループ（v0 以降）**: `for x in values(v)` は v0 から使用可能
4. **型特化関数**: ジェネリック関数は型ごとに提供
5. **while only**: for ループなし

---

## Vec 操作

### 作成

```
// 空のVec
let v: Vec[i32] = Vec_new_i32()

// 容量指定
let v: Vec[i32] = Vec_with_capacity_i32(10)
```

### 追加・削除

```
// 末尾に追加
push(v, 42)

// 末尾から削除
let x: Option[i32] = pop(v)
```

### アクセス

```
// 安全なアクセス
let x: Option[i32] = get(v, 0)
match x {
    Some(val) => print(val),
    None => print("out of bounds"),
}

// 安全でないアクセス（境界チェックなし）
let x: i32 = get_unchecked(v, 0)
```

### 長さ・容量

```
let n: i32 = len(v)
let cap: i32 = capacity(v)
let empty: bool = is_empty(v)
```

### イテレーション

```
let mut i = 0
while i < len(v) {
    let item = get_unchecked(v, i)
    print(item)
    i = i + 1
}
```

### map 的操作

```
fn double(x: i32) -> i32 { x * 2 }

let v: Vec[i32] = vec![1, 2, 3]
let v2: Vec[i32] = map_i32_i32(v, double)
```

### filter 的操作

```
fn is_even(x: i32) -> bool { x % 2 == 0 }

let v: Vec[i32] = vec![1, 2, 3, 4]
let v2: Vec[i32] = filter_i32(v, is_even)
```

### fold 的操作

```
fn sum(acc: i32, x: i32) -> i32 { acc + x }

let v: Vec[i32] = vec![1, 2, 3]
let total: i32 = fold_i32_i32(v, 0, sum)
```

### ソート

```
let v: Vec[i32] = vec![3, 1, 2]
sort_i32(v)  // in-place
```

---

## String 操作

### 作成

```
// 空文字列
let s: String = String_new()

// リテラルから
let s: String = String_from("hello")
```

### 連結

```
let s1: String = String_from("hello")
let s2: String = String_from(" world")
let s3: String = concat(s1, s2)
```

### 長さ

```
let n: i32 = len(s)
let empty: bool = is_empty(s)
```

### スライス

```
let s: String = String_from("hello")
let sub: String = slice(s, 1, 4)  // "ell"
```

### 分割

```
let s: String = String_from("a,b,c")
let parts: Vec[String] = split(s, ",")
```

### 結合

```
let parts: Vec[String] = vec!["a", "b", "c"]
let s: String = join(parts, ",")  // "a,b,c"
```

### 文字追加

```
let s: String = String_from("hello")
push_char(s, '!')  // in-place
```

### clone

```
let s1: String = String_from("hello")
let s2: String = clone(s1)
```

---

## Option 操作

### 作成

```
let x: Option[i32] = Some(42)
let y: Option[i32] = None
```

### チェック

```
let has: bool = is_some(x)
let none: bool = is_none(x)
```

### 取り出し

```
// unwrap（Noneならpanic）
let val: i32 = unwrap(x)

// unwrap_or（Noneならデフォルト値）
let val: i32 = unwrap_or(x, 0)

// パターンマッチ（推奨）
match x {
    Some(val) => print(val),
    None => print("no value"),
}
```

### 変換

```
fn double(x: i32) -> i32 { x * 2 }

let x: Option[i32] = Some(21)
let y: Option[i32] = map_option_i32_i32(x, double)  // Some(42)
```

---

## Result 操作

### 作成

```
let r: Result[i32, String] = Ok(42)
let r: Result[i32, String] = Err("error")
```

### チェック

```
let ok: bool = is_ok(r)
let err: bool = is_err(r)
```

### 取り出し

```
// unwrap（Errならpanic）
let val: i32 = unwrap(r)

// パターンマッチ（推奨）
match r {
    Ok(val) => print(val),
    Err(e) => print(e),
}
```

### ? 演算子

```
fn read_and_parse(dir: DirCap, path: RelPath) -> Result[i32, IOError] {
    let content: String = fs_read_file(dir, path)?
    let n: i32 = parse_int(content)?
    Ok(n * 2)
}
```

---

## slice `[T]` 操作

### 作成

```
// Vec からスライス
let v: Vec[i32] = vec![1, 2, 3]
let s: [i32] = as_slice(v)
```

### アクセス

```
let x: Option[i32] = get(s, 0)
let n: i32 = len(s)
```

### イテレーション

```
let mut i = 0
while i < len(s) {
    let item = get_unchecked(s, i)
    print(item)
    i = i + 1
}
```

---

## ファイル操作

### 読み書き

```
fn main(caps: Capabilities) -> Result[(), IOError] {
    let dir: DirCap = cwd(caps)
    
    // 読み込み
    let content: String = fs_read_file(dir, RelPath_from("input.txt"))?
    
    // 書き込み
    fs_write_file(dir, RelPath_from("output.txt"), content)?
    
    Ok(())
}
```

### ストリーム

```
let handle: FileHandle = fs_open(dir, RelPath_from("data.txt"))?
let buf: [u8] = [0; 1024]
let n: i32 = fs_read(handle, buf)?
fs_close(handle)?
```

---

## エラー処理パターン

### パターン1: ? で伝播

```
fn process(dir: DirCap) -> Result[i32, IOError] {
    let content = fs_read_file(dir, RelPath_from("data.txt"))?
    let n = parse_int(content)?
    Ok(n * 2)
}
```

### パターン2: match で分岐

```
let result = fs_read_file(dir, path)
match result {
    Ok(content) => {
        print(content)
        Ok(())
    }
    Err(IOError::NotFound) => {
        print("file not found")
        Ok(())
    }
    Err(e) => Err(e),
}
```

### パターン3: unwrap_or でデフォルト値

```
let content = unwrap_or(fs_read_file(dir, path), "")
```

---

## ループパターン

### 範囲ループ（カウント）

```
let mut i = 0
while i < 10 {
    print(i)
    i = i + 1
}
```

### コレクションループ

```
let items: Vec[i32] = vec![1, 2, 3]
let mut i = 0
while i < len(items) {
    let item = get_unchecked(items, i)
    print(item)
    i = i + 1
}
```

### 条件ループ

```
let mut running = true
while running {
    let input = read_line()
    if input == "quit" {
        running = false
    }
}
```

### 無限ループ

```
loop {
    let input = read_line()
    if input == "quit" {
        break
    }
    print(input)
}
```

---

## v0 制限パターン（v1 では解除）

以下は v0 での制限。v1 では使用可能だが、関数呼び出し形式も引き続き動作する。

### v0 制限: メソッド構文（v1 M4/M5 以降は OK）

```
// v0: 関数呼び出し形式が必要
push(v, 42)
len(s)

// v1: メソッド構文も可能
v.push(42)
s.len()
```

### v0 制限: for ループ（v0 でも実装済み ✅）

```
// ✅ for ループは v0 から使用可能
for x in values(items) {
    print(x)
}

// while ループも常に有効
let mut i = 0
while i < len(items) {
    let x = get_unchecked(items, i)
    print(x)
    i = i + 1
}
```

### ❌ インデックス代入（未実装）

```
// NG
v[i] = 42

// OK
set(v, i, 42)
```

### v0 制限: 演算子オーバーロード（v1 M6 以降は OK）

```
// v0: 明示的な関数呼び出し
let p3 = Point_add(p1, p2)

// v1: impl Add で演算子定義後は使用可能
let p3 = p1 + p2
```

---

## 型特化関数一覧

### Vec

| 操作 | i32 | i64 | f64 | String |
|------|-----|-----|-----|--------|
| new | `Vec_new_i32()` | `Vec_new_i64()` | `Vec_new_f64()` | `Vec_new_String()` |
| map | `map_i32_i32(v, f)` | `map_i64_i64(v, f)` | `map_f64_f64(v, f)` | `map_String_String(v, f)` |
| filter | `filter_i32(v, f)` | `filter_i64(v, f)` | `filter_f64(v, f)` | `filter_String(v, f)` |
| sort | `sort_i32(v)` | `sort_i64(v)` | `sort_f64(v)` | `sort_String(v)` |

### Option

| 操作 | i32 | String |
|------|-----|--------|
| map | `map_option_i32_i32(o, f)` | `map_option_String_String(o, f)` |

---

## 関連

- `docs/stdlib/core.md`: 標準ライブラリ仕様
- `docs/compiler/diagnostics.md`: エラー診断
- `docs/spec/v0-unified-spec.md`: v0統合仕様
